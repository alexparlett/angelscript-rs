# Task 22: TypeHash-Based Type Identity System

## Overview

Replace the current sequential `TypeId(u32)` / `FunctionId(u32)` system with a deterministic hash-based identity system inspired by [Rune's approach](https://docs.rs/rune/latest/rune/struct.Hash.html).

## Problem Statement

The current TypeId/FunctionId system has several pain points:

1. **Registration order dependency**: Types must be registered before they can be referenced
2. **FFI_BIT complexity**: Routing logic scattered throughout CompilationContext to distinguish FFI vs Script types
3. **Two-map lookups**: Requires both `types: HashMap<TypeId, TypeDef>` and `type_names: HashMap<String, TypeId>`
4. **Global properties blocked**: Can't resolve types until registry is sealed (Task 20 Phase 6.4.1 issue)
5. **Cross-references during registration**: Requires careful ordering when types reference each other

## Solution: Deterministic Hash-Based Identity

### Core Concept

Type identity is computed deterministically from the qualified type name using XXHash64. This means:
- A type's hash can be computed before it's registered (forward references)
- No registration order dependencies
- No need for FFI_BIT routing - just check both registries
- Single map replaces dual name+id maps

### TypeHash Struct

```rust
/// A deterministic 64-bit hash identifying a type, function, or method.
///
/// Computed from the qualified name (for types) or name+signature (for functions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct TypeHash(pub u64);

impl TypeHash {
    pub const EMPTY: TypeHash = TypeHash(0);

    pub const fn is_empty(self) -> bool { self.0 == 0 }
    pub const fn as_u64(self) -> u64 { self.0 }
}
```

### Hash Computation

Uses XXHash64 with domain-specific mixing constants:

| Entity | Hash Computation |
|--------|------------------|
| Simple type | `TYPE_MARKER ^ xxhash("int")` |
| Qualified type | `TYPE_MARKER ^ xxhash("Game") ^ SEP ^ xxhash("Player")` |
| Template instance | `template_hash ^ (PARAM[0] ^ arg0_hash) ^ (PARAM[1] ^ arg1_hash)...` |
| Global function | `FUNCTION_MARKER ^ xxhash(name) ^ param_type_hashes...` |
| Method | `METHOD_MARKER ^ owner_type_hash ^ xxhash(name) ^ param_type_hashes...` |
| Constructor | `CONSTRUCTOR_MARKER ^ owner_type_hash ^ param_type_hashes...` |
| Operator | `OPERATOR_MARKER ^ owner_type_hash ^ operator_kind` |

### Mixing Constants

```rust
pub mod hash_constants {
    pub const SEP: u64 = 0x4bc94d6bd06053ad;           // Path component separator
    pub const TYPE: u64 = 0x2fac10b63a6cc57c;          // Type domain
    pub const FUNCTION: u64 = 0x5ea77ffbcdf5f302;      // Function domain
    pub const METHOD: u64 = 0x7d3c8b4a92e15f6d;        // Instance method domain
    pub const OPERATOR: u64 = 0x3e9f5d2a8c7b1403;      // Operator method domain
    pub const CONSTRUCTOR: u64 = 0x9a7f3d5e2b8c4601;   // Constructor domain
    pub const IDENT: u64 = 0x1a095090689d4647;         // Identifier domain
    pub const TEMPLATE_PARAMS: [u64; 32] = [/* ... */]; // Template arg mixing
}
```

### Primitive Hashes

Well-known constant hashes for primitives (computed at compile time):

```rust
pub mod primitives {
    pub const VOID: TypeHash = TypeHash(/* computed */);
    pub const BOOL: TypeHash = TypeHash(/* computed */);
    pub const INT8: TypeHash = TypeHash(/* computed */);
    pub const INT16: TypeHash = TypeHash(/* computed */);
    pub const INT32: TypeHash = TypeHash(/* computed */);  // "int"
    pub const INT64: TypeHash = TypeHash(/* computed */);
    pub const UINT8: TypeHash = TypeHash(/* computed */);
    pub const UINT16: TypeHash = TypeHash(/* computed */);
    pub const UINT32: TypeHash = TypeHash(/* computed */); // "uint"
    pub const UINT64: TypeHash = TypeHash(/* computed */);
    pub const FLOAT: TypeHash = TypeHash(/* computed */);
    pub const DOUBLE: TypeHash = TypeHash(/* computed */);

    pub const NULL: TypeHash = TypeHash(/* computed */);
    pub const VARIABLE_PARAM: TypeHash = TypeHash(/* computed */);
}
```

---

## Design Decisions

1. **Scope**: Complete refactor - replace TypeId/FunctionId entirely with TypeHash
2. **Unified struct**: Single `TypeHash` for types, functions, methods (mixing constants prevent collisions)
3. **Bytecode/VM**: Bytecode uses hashes; VM not built yet; runtime already hash-based - aligns well
4. **Hash collisions**: Trust the math - 64-bit space is astronomically unlikely to collide
5. **Algorithm**: XXHash64 - fast (~15 GB/s), non-cryptographic, well-distributed

---

## Architecture Changes

### Registry Storage

**Before:**
```rust
pub struct FfiRegistry {
    types: FxHashMap<TypeId, TypeDef>,
    type_names: FxHashMap<String, TypeId>,  // Secondary index
    functions: FxHashMap<FunctionId, ResolvedFfiFunctionDef>,
    function_names: FxHashMap<String, Vec<FunctionId>>,
    // ...
}
```

**After:**
```rust
pub struct FfiRegistry {
    types: FxHashMap<TypeHash, TypeDef>,
    // No type_names - hash computed from name
    functions: FxHashMap<TypeHash, ResolvedFfiFunctionDef>,
    function_overloads: FxHashMap<String, Vec<TypeHash>>,  // For overload resolution
    // ...
}
```

### DataType Changes

```rust
// Before
pub struct DataType {
    pub type_id: TypeId,
    pub is_const: bool,
    pub is_handle: bool,
    // ...
}

// After
pub struct DataType {
    pub type_hash: TypeHash,
    pub is_const: bool,
    pub is_handle: bool,
    // ...
}
```

### CompilationContext Simplification

**Before (FFI_BIT routing):**
```rust
pub fn get_type(&self, type_id: TypeId) -> &TypeDef {
    if type_id.is_ffi() {
        self.ffi.get_type(type_id).expect("FFI TypeId not found")
    } else {
        self.script.get_type(type_id)
    }
}
```

**After:**
```rust
pub fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
    self.ffi.get_type(hash).or_else(|| self.script.get_type(hash))
}
```

### Function Overload Handling

Each overload has a unique hash because parameter types are included:

```rust
// void foo(int x)
let hash1 = TypeHash::function("foo", &[primitives::INT32]);

// void foo(float x)
let hash2 = TypeHash::function("foo", &[primitives::FLOAT]);

// hash1 != hash2
```

Secondary index for name-based lookup during overload resolution:
```rust
function_overloads: FxHashMap<String, Vec<TypeHash>>
```

### Template Instantiation

```rust
impl TemplateInstantiator {
    pub fn instantiate(&mut self, template_hash: TypeHash, args: Vec<DataType>) -> TypeHash {
        // Compute instance hash from template + args
        let arg_hashes: Vec<TypeHash> = args.iter().map(|a| a.type_hash).collect();
        let instance_hash = TypeHash::template_instance(template_hash, &arg_hashes);

        // Check if already exists (either FFI or Script)
        if self.exists(instance_hash) {
            return instance_hash;
        }

        // Create and register instance...
        instance_hash
    }
}
```

---

## Implementation Phases

### Phase 1: Add TypeHash Infrastructure
- [ ] Create `src/semantic/types/type_hash.rs`
- [ ] Add `TypeHash` struct with `Debug, Clone, Copy, PartialEq, Eq, Hash, Ord`
- [ ] Add `TypeHasher` builder for computing hashes
- [ ] Add hash constants module
- [ ] Add primitive hash constants
- [ ] Add `twox-hash` crate dependency

### Phase 2: Dual-Key Migration Period
- [ ] Add `type_hash: TypeHash` field to `TypeDef` (computed on construction)
- [ ] Add `function_hash: TypeHash` field to function definitions
- [ ] Keep `TypeId`/`FunctionId` working during transition
- [ ] Add `types_by_hash` secondary map to registries
- [ ] Add hash-based lookup methods alongside existing ones

### Phase 3: Registry Migration
- [ ] Change `FfiRegistry` to use `TypeHash` as primary key
- [ ] Remove `type_names` map (hash computed from name)
- [ ] Update `FfiRegistryBuilder` to compute hashes during registration
- [ ] Change `ScriptRegistry` similarly
- [ ] Update `CompilationContext` to remove FFI_BIT routing

### Phase 4: DataType Migration
- [ ] Change `DataType.type_id: TypeId` to `DataType.type_hash: TypeHash`
- [ ] Update all `DataType` construction sites
- [ ] Update all type comparison code
- [ ] Update template instantiation to use hash-based caching

### Phase 5: Cleanup
- [ ] Remove `TypeId` struct
- [ ] Remove `FunctionId` struct
- [ ] Remove atomic counters (`TYPE_ID_COUNTER`, `FUNCTION_ID_COUNTER`)
- [ ] Remove `FFI_BIT` constant and all `is_ffi()`/`is_script()` checks
- [ ] Remove `next_ffi()`/`next_script()` methods
- [ ] Clean up all routing logic in CompilationContext

### Phase 6: Global Properties (from Task 20 6.4.1)
- [ ] Implement global property builder API on Context
- [ ] Type resolution now works anytime (hash computed from type name string)
- [ ] No sealing requirement

---

## Critical Files

| File | Changes |
|------|---------|
| `src/semantic/types/type_hash.rs` | **NEW** - TypeHash struct, TypeHasher, constants |
| `src/semantic/types/type_def.rs` | Remove TypeId/FunctionId, add TypeHash re-export |
| `src/semantic/types/data_type.rs` | `type_id` → `type_hash` |
| `src/semantic/types/mod.rs` | Update exports |
| `src/ffi/ffi_registry.rs` | HashMap keys TypeId → TypeHash |
| `src/semantic/types/registry.rs` | ScriptRegistry HashMap keys |
| `src/semantic/compilation_context.rs` | Remove FFI_BIT routing |
| `src/semantic/template_instantiator.rs` | Hash-based caching |
| `src/semantic/passes/registration.rs` | Use TypeHash for registration |
| `src/semantic/passes/type_compilation.rs` | Use TypeHash lookups |
| `src/semantic/passes/function_processor/*.rs` | Use TypeHash |
| `src/types/ffi_*.rs` | Update to use TypeHash |
| `src/module.rs` | Update registration to compute hashes |

---

## Performance Considerations

### Hash Computation
- XXHash64: ~15 GB/s throughput on modern CPUs
- Short type names (5-20 chars): ~10-20 nanoseconds
- Computed once at registration, reused thereafter

### Memory
- TypeHash: 8 bytes (vs TypeId: 4 bytes) - minor increase
- Eliminates secondary `type_names` HashMap - memory savings
- Net effect: roughly neutral

### Collision Risk
- 64-bit hash space = 2^64 possible values
- Birthday problem: ~4 billion types before 50% collision chance
- For realistic type counts (<100,000): negligible risk

---

## Related Tasks

- **Task 23**: Ergonomic Module API - Rune-inspired fluent builder API, TypeOf trait, derive macros

---

## References

- [Rune Hash implementation](https://docs.rs/rune/latest/rune/struct.Hash.html)
- [Rune TypeOf trait](https://github.com/rune-rs/rune/blob/main/crates/rune/src/runtime/type_of.rs)
- [Rune RTTI](https://docs.rs/rune/latest/rune/runtime/struct.Rtti.html)
- [XXHash algorithm](https://github.com/Cyan4973/xxHash)
- [twox-hash crate](https://crates.io/crates/twox-hash)
