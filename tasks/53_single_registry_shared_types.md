# Task 53: Single Registry & Shared Types

## Problem Summary

Current dual registry design (`global_registry` + `unit_registry`) doesn't support the `shared` keyword for cross-module type sharing. Adding shared types would require a third registry, making lookups even more complex.

## Current Architecture

```rust
pub struct CompilationContext<'a> {
    global_registry: &'a SymbolRegistry,  // FFI types (immutable)
    unit_registry: SymbolRegistry,         // Script types (mutable)
}

// Every lookup checks two places
fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
    self.unit_registry.get(hash)
        .or_else(|| self.global_registry.get(hash))
}
```

**Problems:**
- Two lookups per resolution
- Adding `shared` would require third registry
- Confusing ownership model

## C++ AngelScript Reference

From `as_scriptengine.h` and `as_module.h`:

```cpp
class asCScriptEngine {
    asCMap<asSNameSpaceNamePair, asCTypeInfo*> allRegisteredTypes;  // FFI
    asCArray<asCTypeInfo *> sharedScriptTypes;                       // Shared script
    asCArray<asCScriptFunction *> scriptFunctions;                   // All functions
};

class asCModule {
    asCMap<asSNameSpaceNamePair, asCTypeInfo*> m_typeLookup;  // Module-local
    asCArray<asCObjectType*> m_classTypes;                     // Owned types
};
```

C++ has three places: engine FFI, engine shared, module local.

## Proposed Design: Single Registry

### TypeSource Enum

```rust
pub enum TypeSource {
    /// FFI type registered from Rust
    Ffi,
    /// Shared script type - visible across all modules
    Shared { first_unit: UnitId },
    /// Module-local script type - only visible within declaring module
    Module { unit_id: UnitId },
}
```

### Hash Strategy

**Key insight:** Module-local types need unit-specific hashes to avoid collisions.

```rust
impl TypeHash {
    /// For FFI and shared types - global namespace
    pub fn from_name(name: &str) -> Self { ... }

    /// For module-local types - includes unit_id to avoid collisions
    pub fn from_module_type(name: &str, unit_id: UnitId) -> Self {
        let base = xxh64(name.as_bytes(), 0);
        let with_unit = base ^ (unit_id.0 as u64).wrapping_mul(hash_constants::UNIT_MARKER);
        TypeHash(hash_constants::TYPE ^ with_unit)
    }
}
```

**Result:**
- `TypeHash::from_name("Player")` - same across all modules (for shared/FFI)
- `TypeHash::from_module_type("GameEntity", UnitId(0))` ≠ `from_module_type("GameEntity", UnitId(1))`

### Registration Flow

```rust
// FFI (before any module compiles)
registry.register(TypeEntry {
    hash: TypeHash::from_name("string"),
    source: TypeSource::Ffi,
    ...
});

// Module A: shared class Player
registry.register(TypeEntry {
    hash: TypeHash::from_name("Player"),  // Global hash
    source: TypeSource::Shared { first_unit: UnitId(0) },
    ...
});

// Module A: class GameEntity
registry.register(TypeEntry {
    hash: TypeHash::from_module_type("GameEntity", UnitId(0)),  // Unit-specific
    source: TypeSource::Module { unit_id: UnitId(0) },
    ...
});

// Module B: shared class Player - finds existing, verifies match
let existing = registry.get(TypeHash::from_name("Player"));
// Validate declarations match or error

// Module B: class GameEntity - different hash, no collision
registry.register(TypeEntry {
    hash: TypeHash::from_module_type("GameEntity", UnitId(1)),
    source: TypeSource::Module { unit_id: UnitId(1) },
    ...
});
```

### Type Resolution

```rust
fn resolve_type(&self, name: &str, current_unit: UnitId) -> Option<TypeHash> {
    // 1. Try module-local first (shadows shared/FFI)
    let module_hash = TypeHash::from_module_type(name, current_unit);
    if self.registry.contains_key(&module_hash) {
        return Some(module_hash);
    }

    // 2. Try shared/FFI
    let global_hash = TypeHash::from_name(name);
    if self.registry.contains_key(&global_hash) {
        return Some(global_hash);
    }

    None
}
```

**Note:** Still two hash computations + lookups for name-based resolution. But hash-based lookups (the common case after resolution) are single lookup.

### Module Cleanup

When discarding a module:

```rust
impl SymbolRegistry {
    pub fn discard_module(&mut self, unit_id: UnitId) {
        // Remove module-local types
        self.types.retain(|_, entry| {
            !matches!(entry.source, TypeSource::Module { unit_id: u } if u == unit_id)
        });

        // Remove module-local functions
        self.functions.retain(|_, entry| {
            !matches!(entry.source, FunctionSource::Module { unit_id: u } if u == unit_id)
        });

        // Shared types persist (may be referenced by other modules)
    }
}
```

### Shared Type Validation

When Module B declares `shared class Player`:

```rust
fn register_shared_type(&mut self, decl: &ClassDecl, unit_id: UnitId) -> Result<TypeHash, Error> {
    let hash = TypeHash::from_name(&decl.name);

    if let Some(existing) = self.registry.get(&hash) {
        // Verify declarations match
        match &existing.source {
            TypeSource::Shared { .. } => {
                self.validate_shared_match(existing, decl)?;
                Ok(hash)  // Reuse existing
            }
            TypeSource::Ffi => {
                Err(Error::SharedConflictsWithFfi(decl.name))
            }
            TypeSource::Module { .. } => {
                // Shouldn't happen - module types have different hash
                unreachable!()
            }
        }
    } else {
        // First declaration - register it
        self.registry.insert(hash, TypeEntry {
            source: TypeSource::Shared { first_unit: unit_id },
            ...
        });
        Ok(hash)
    }
}
```

---

## Implementation Plan

### Phase 1: Add TypeSource and Hash Variants

1. Add `TypeSource` enum to `angelscript-core`
2. Add `TypeHash::from_module_type()`
3. Add `UNIT_MARKER` hash constant
4. Update `TypeEntry` to use `TypeSource` instead of current `source` field

### Phase 2: Merge Registries

1. Remove `global_registry` / `unit_registry` split in `CompilationContext`
2. Single `registry: &'a mut SymbolRegistry`
3. Update all lookup call sites to use new resolution pattern
4. Add `current_unit: UnitId` to compilation context

### Phase 3: Shared Keyword Support

1. Parse `shared` keyword in class/interface declarations
2. Route to `register_shared_type()` vs `register_module_type()`
3. Implement shared type validation
4. Add `external` keyword for referencing shared types from other modules

### Phase 4: Module Lifecycle

1. Implement `discard_module()`
2. Handle shared type reference counting (don't discard if still referenced)
3. Add tests for multi-module scenarios

---

## Files to Modify

| File | Change |
|------|--------|
| `crates/angelscript-core/src/entries/mod.rs` | Add `TypeSource` enum |
| `crates/angelscript-core/src/type_hash.rs` | Add `from_module_type()` |
| `crates/angelscript-core/src/hash_constants.rs` | Add `UNIT_MARKER` |
| `crates/angelscript-compiler/src/context.rs` | Merge registries, add `current_unit` |
| `crates/angelscript-compiler/src/passes/registration.rs` | Use new registration pattern |
| `crates/angelscript-compiler/src/type_resolver.rs` | Update resolution logic |
| `crates/angelscript-registry/src/registry.rs` | Add `discard_module()` |

---

## Testing

```rust
#[test]
fn module_local_types_are_isolated() {
    // Module A: class Foo { int x; }
    // Module B: class Foo { string y; }
    // Both compile without collision
}

#[test]
fn shared_types_are_reused() {
    // Module A: shared class Player { int health; }
    // Module B: shared class Player { int health; }
    // Both reference same TypeHash
}

#[test]
fn shared_type_mismatch_is_error() {
    // Module A: shared class Player { int health; }
    // Module B: shared class Player { string name; }
    // Compile error on Module B
}

#[test]
fn module_discard_cleans_local_types() {
    // Compile Module A with local type
    // Discard Module A
    // Type no longer in registry
}

#[test]
fn shared_types_survive_module_discard() {
    // Module A declares shared Player
    // Module B references shared Player
    // Discard Module A
    // Player still accessible to Module B
}
```

---

## Relationship to Other Tasks

- **Task 48 (VTable Dispatch):** Independent - can proceed with dual registry
- **Task 53 (This):** Foundation for shared types
- **Future:** `external` keyword, cross-module function imports

---

## Summary

Single registry with `TypeSource` tracking:
1. Eliminates triple-registry problem for shared types
2. Uses unit-prefixed hashes for module isolation
3. Simplifies hash-based lookups to single access
4. Enables proper `shared` keyword support matching C++ feature parity
