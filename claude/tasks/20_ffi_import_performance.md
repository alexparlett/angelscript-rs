# Task 20: FFI Import Performance Optimization

## Problem Statement

We've observed a significant performance regression in the benchmarks. A test that used to take ~100μs is now taking ~2.2ms - approximately a 20x slowdown (actually 267x for tiny scripts: 7µs → 2ms).

The root cause is that FFI module import (`import_modules`) happens on every compilation, not once at context creation time. Every call to `Unit::build()` creates a new `Registry` and re-imports all FFI modules from scratch.

### Profiling Results

From profiling analysis:
- `_platform_memmove`: 41.6% (memory copying)
- `drop_in_place<TypeDef>`: 22.0% (destroying TypeDefs)
- `import_modules` inclusive: 68.1%
- `import_type_shell`: 63.4% (most expensive phase)

### Root Cause Analysis

The current flow is:
1. `Context` holds `Vec<Module>` with FFI definitions
2. Each `Unit::build()` creates a fresh `Registry`
3. `Registry::new()` calls `import_modules()` which:
   - Allocates new `TypeDef` for every FFI type
   - Copies all FFI functions
   - Rebuilds template registrations
   - Creates new behavior maps
4. After compilation, `Registry` is dropped, destroying all these allocations
5. Next `Unit::build()` repeats everything

This is O(n) work per compilation where n = FFI types/functions, when it should be O(1).

---

## Chosen Solution: Two-Tier Registry with Arc<FfiRegistry>

After investigation, we're implementing **Option C** (Two-Tier Registry) with the following design:

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Context                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Arc<FfiRegistry>                        │   │
│  │  (immutable, shared across all Units)                │   │
│  │  - FFI types (TypeDef)                               │   │
│  │  - FFI functions (FfiFunctionDef)                    │   │
│  │  - FFI behaviors (TypeBehaviors)                     │   │
│  │  - Template callbacks                                │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                  │
│                    Arc::clone()                              │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Unit (per compilation)                  │   │
│  │  ┌───────────────────────────────────────────────┐  │   │
│  │  │            Registry<'ast>                      │  │   │
│  │  │  - ffi: Arc<FfiRegistry>  (shared, immutable) │  │   │
│  │  │  - script_types: HashMap<TypeId, TypeDef>     │  │   │
│  │  │  - script_functions: HashMap<FunctionId, ...> │  │   │
│  │  │  - template_cache: HashMap<(TypeId, args), ...>│  │   │
│  │  └───────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Why This Approach?

**Alternatives Considered:**

1. **Option A: Clone Registry** - Still O(n) per compilation, just moves the work
2. **Option B: Lazy Import with Caching** - Complex invalidation, doesn't solve core issue
3. **Option C: Two-Tier Registry** ✓ - Clean separation, O(1) per compilation
4. **Option D: Persistent Arena** - Lifetime complexity, doesn't help with sharing

Option C wins because:
- FFI types are immutable after registration
- `Arc` provides zero-cost sharing between Units
- Clear ownership: Context owns FFI, Unit owns script types
- Template instances can still be created per-Unit

---

## Key Design Decisions

### 1. FfiDataType for Lazy Type Resolution

**Problem:** Types in FFI declarations may reference other types not yet registered. For example:
```rust
module.register_function("void process(MyClass@ obj)")?;
// MyClass might not be registered yet!
```

**Solution:** Use `FfiDataType` to defer resolution until all types are registered:

```rust
/// The base type portion of a type reference (without modifiers).
pub enum UnresolvedBaseType {
    /// Simple type name (e.g., "MyClass", "int")
    Simple(String),
    /// Template instantiation (e.g., array<string>, dictionary<string, int>)
    Template {
        name: String,
        args: Vec<FfiDataType>,
    },
}

/// Type reference that may be unresolved during registration.
pub enum FfiDataType {
    /// Already resolved (primitives, already-registered types)
    Resolved(DataType),
    /// Needs resolution during install()
    Unresolved {
        base: UnresolvedBaseType,
        is_const: bool,
        is_handle: bool,
        is_handle_to_const: bool,
        ref_modifier: RefModifier,
    },
}
```

**Key Design Points:**
- Primitives (`int`, `float`, etc.) are always `Resolved` immediately
- User types become `Unresolved("TypeName")` until install phase
- Template types like `array<MyClass>` use nested `FfiDataType`:
  - Template name `"array"` is unresolved (looked up by string)
  - Args can be independently resolved or unresolved
  - Example: `array<int>` = Template { name: "array", args: [Resolved(int)] }
  - Example: `array<MyClass>` = Template { name: "array", args: [Unresolved("MyClass")] }
- All modifiers (const, handle, ref) are preserved on Unresolved types
- Resolution happens recursively via `resolve()` method

**Implementation (already done in `src/types/ffi_data_type.rs`):**
```rust
impl FfiDataType {
    pub fn resolve<L, I>(&self, lookup: &L, instantiate: &mut I) -> Result<DataType, String>
    where
        L: Fn(&str) -> Option<TypeId>,
        I: FnMut(TypeId, Vec<DataType>) -> Result<TypeId, String>,
    {
        match self {
            FfiDataType::Resolved(dt) => Ok(dt.clone()),
            FfiDataType::Unresolved { base, is_const, is_handle, is_handle_to_const, ref_modifier } => {
                let type_id = match base {
                    UnresolvedBaseType::Simple(name) => {
                        lookup(name).ok_or_else(|| format!("Unknown type: {}", name))?
                    }
                    UnresolvedBaseType::Template { name, args } => {
                        let template_id = lookup(name)?;
                        let resolved_args: Vec<DataType> = args
                            .iter()
                            .map(|arg| arg.resolve(lookup, instantiate))
                            .collect::<Result<_, _>>()?;
                        instantiate(template_id, resolved_args)?
                    }
                };
                Ok(DataType { type_id, is_const, is_handle, is_handle_to_const, ref_modifier })
            }
        }
    }
}
```

---

### 2. Owned FFI Types (No Arena Dependency)

**Problem:** Current FFI parsing uses arenas (`Bump`) for performance, but arena-allocated types have lifetimes that prevent storing in `Arc<FfiRegistry>`.

**Key Insight:** After examining the codebase, we discovered:
- `TypeDef` is **already lifetime-free**! It uses `String`, `Vec`, `FxHashMap` - all owned types.
- Only `FunctionDef<'ast>` has a lifetime due to `default_args: Vec<Option<&'ast Expr<'ast>>>`

**Solution:** Keep arena parsing for speed, but convert to owned types immediately:

```rust
// Parse with temporary arena (fast), convert to owned immediately
let arena = Bump::new();
let parsed = Parser::parse_function_decl(decl, &arena)?;
let ffi_func = FfiFunctionDef::from_parsed(&parsed, native_fn);
// arena dropped here, ffi_func is fully owned
```

**Why This Works:**
- Arena allocation is fast (bump pointer)
- Conversion to owned happens once at registration time
- `FfiRegistry` holds owned types, can be `Arc`-shared
- No self-referential struct issues

---

### 3. FfiExpr for Default Arguments

**Problem:** Default arguments in function declarations are `&'ast Expr<'ast>` - arena-allocated AST nodes. We need owned equivalents.

**Discussion Notes:**
- User initially concerned about storing defaults as strings (requires re-parsing)
- Considered full AST storage but arena ownership is complex
- Settled on owned `FfiExpr` enum covering realistic FFI default arg patterns

**Solution:** Limited but sufficient expression type:

```rust
/// Owned expression for FFI default arguments.
/// Covers common cases without full AST complexity.
pub enum FfiExpr {
    // Literals
    Int(i64),
    UInt(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Null,

    // Enum value: EnumType::Value
    EnumValue { enum_name: String, value_name: String },

    // Constructor call: Type(args...)
    Construct { type_name: String, args: Vec<FfiExpr> },

    // Unary: -expr, !expr
    Unary { op: UnaryOp, expr: Box<FfiExpr> },

    // Binary expressions for simple math
    Binary { left: Box<FfiExpr>, op: BinaryOp, right: Box<FfiExpr> },

    // Identifier (for constants)
    Ident(String),
}
```

**Coverage:**
- `void foo(int x = 0)` → `Int(0)`
- `void foo(float x = -1.5)` → `Unary { op: Neg, expr: Float(1.5) }`
- `void foo(string s = "default")` → `String("default")`
- `void foo(Color c = Color::Red)` → `EnumValue { enum_name: "Color", value_name: "Red" }`
- `void foo(Vec2 v = Vec2(0, 0))` → `Construct { type_name: "Vec2", args: [...] }`

**Edge Cases:**
- Complex expressions fall back to string storage with compile-time parsing
- This matches C++ AngelScript's behavior for complex defaults

---

### 4. FfiFunctionDef Structure

**Purpose:** Owned function definition for FFI, replacing `FunctionDef<'ast>` in the FFI registry.

```rust
/// FFI function parameter with owned types
pub struct FfiParam {
    pub name: String,
    pub data_type: FfiDataType,
    pub default_value: Option<FfiExpr>,
}

/// Owned function definition for FFI registry
pub struct FfiFunctionDef {
    pub id: FunctionId,
    pub name: String,
    pub qualified_name: String,
    pub namespace: Option<String>,
    pub params: Vec<FfiParam>,
    pub return_type: FfiDataType,
    pub traits: FunctionTraits,
    pub native_fn: Option<NativeFunction>,
    /// For methods: the owning type
    pub owner_type: Option<TypeId>,
    /// Operator behavior if this is an operator method
    pub operator: Option<OperatorBehavior>,
}

impl FfiFunctionDef {
    /// Convert from parsed AST function declaration
    pub fn from_parsed(parsed: &FunctionDecl<'_>, native_fn: NativeFunction) -> Self {
        // Convert arena-allocated AST to owned FfiFunctionDef
        // Called immediately after parsing, before arena is dropped
    }

    /// Resolve all FfiDataTypes to concrete DataTypes
    pub fn resolve<L, I>(&self, lookup: &L, instantiate: &mut I) -> Result<ResolvedFunctionDef, String>
    where
        L: Fn(&str) -> Option<TypeId>,
        I: FnMut(TypeId, Vec<DataType>) -> Result<TypeId, String>,
    {
        // Called during Context sealing to produce final resolved types
    }
}
```

---

### 5. FfiRegistry Structure

**Purpose:** Immutable registry holding all resolved FFI data, shared via `Arc`.

```rust
/// Immutable FFI registry, shared across all Units in a Context
pub struct FfiRegistry {
    // Type storage
    types: FxHashMap<TypeId, TypeDef>,
    type_names: FxHashMap<String, TypeId>,

    // Function storage
    functions: FxHashMap<FunctionId, FfiFunctionDef>,
    function_names: FxHashMap<String, Vec<FunctionId>>,  // Overloads

    // Behaviors (lifecycle callbacks)
    behaviors: FxHashMap<TypeId, TypeBehaviors>,

    // Template registration (for instantiation)
    templates: FxHashMap<TypeId, TemplateCallbacks>,

    // Namespace tracking
    namespaces: FxHashSet<String>,
}

impl FfiRegistry {
    pub fn get_type(&self, id: TypeId) -> Option<&TypeDef>;
    pub fn get_type_by_name(&self, name: &str) -> Option<TypeId>;
    pub fn get_function(&self, id: FunctionId) -> Option<&FfiFunctionDef>;
    pub fn get_behaviors(&self, type_id: TypeId) -> Option<&TypeBehaviors>;
    // ... lookup methods
}
```

**FfiRegistryBuilder** for construction phase:

```rust
/// Builder for FfiRegistry, used during module installation
pub struct FfiRegistryBuilder {
    // Mutable collections during registration
    types: FxHashMap<TypeId, TypeDef>,
    unresolved_functions: Vec<FfiFunctionDef>,  // Types not yet resolved
    behaviors: FxHashMap<TypeId, TypeBehaviors>,
    // ...
}

impl FfiRegistryBuilder {
    pub fn register_type(&mut self, type_def: TypeDef) -> TypeId;
    pub fn register_function(&mut self, func: FfiFunctionDef);
    pub fn register_behavior(&mut self, type_id: TypeId, behavior: TypeBehaviors);

    /// Finalize: resolve all types and build immutable registry
    pub fn build(self) -> Result<FfiRegistry, Vec<ResolutionError>> {
        // 1. Build type name lookup table
        // 2. Resolve all FfiDataTypes in functions
        // 3. Validate all references
        // 4. Return immutable FfiRegistry
    }
}
```

---

### 6. Context Sealing Strategy

**Lifecycle:**
```
Context::new()
    │
    ▼ (mutable phase - can register)
context.install(&module)?  ──► FfiRegistryBuilder accumulates
context.install(&module2)?
    │
    ▼ (sealing trigger)
context.create_unit()
    │
    ▼ (sealing happens)
FfiRegistryBuilder::build() ──► Arc<FfiRegistry>
    │
    ▼ (immutable phase - cannot register)
context.install(&module3)?  ──► Error::AlreadySealed
context.create_unit()  ──► OK, uses Arc::clone()
context.create_unit()  ──► OK, uses Arc::clone()
```

**Implementation:**

```rust
pub struct Context {
    // During registration phase
    builder: Option<FfiRegistryBuilder>,
    // After sealing
    ffi_registry: Option<Arc<FfiRegistry>>,
    // ... other fields
}

impl Context {
    pub fn install(&mut self, module: &Module) -> Result<(), ContextError> {
        let builder = self.builder.as_mut()
            .ok_or(ContextError::AlreadySealed)?;
        module.install_into(builder)?;
        Ok(())
    }

    pub fn create_unit(&mut self) -> Result<Unit, ContextError> {
        // Seal on first call
        if self.ffi_registry.is_none() {
            let builder = self.builder.take()
                .expect("Must have builder before sealing");
            self.ffi_registry = Some(Arc::new(builder.build()?));
        }

        let ffi = Arc::clone(self.ffi_registry.as_ref().unwrap());
        Ok(Unit::new(ffi))
    }
}
```

**Error Handling:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("Context is already sealed - cannot register new types after create_unit()")]
    AlreadySealed,
    #[error("Type resolution failed: {0}")]
    ResolutionError(String),
    // ...
}
```

---

### 7. Two-Tier Registry Lookup

**Registry Structure:**

```rust
pub struct Registry<'ast> {
    /// Shared FFI registry (immutable)
    ffi: Arc<FfiRegistry>,

    /// Script-defined types (mutable during compilation)
    script_types: FxHashMap<TypeId, TypeDef>,
    script_type_names: FxHashMap<String, TypeId>,

    /// Script-defined functions
    script_functions: FxHashMap<FunctionId, FunctionDef<'ast>>,

    /// Template instance cache (created during compilation)
    template_instances: FxHashMap<(TypeId, Vec<DataType>), TypeId>,

    /// Script-defined global variables
    script_globals: FxHashMap<String, GlobalVarDef>,
}
```

**Lookup Priority:**

```rust
impl<'ast> Registry<'ast> {
    pub fn get_type(&self, id: TypeId) -> Option<TypeDefView<'_>> {
        // 1. Check script types first (can shadow FFI)
        if let Some(def) = self.script_types.get(&id) {
            return Some(TypeDefView::Script(def));
        }
        // 2. Fall back to FFI
        self.ffi.get_type(id).map(TypeDefView::Ffi)
    }

    pub fn get_type_by_name(&self, name: &str) -> Option<TypeId> {
        // Script types take precedence
        self.script_type_names.get(name).copied()
            .or_else(|| self.ffi.get_type_by_name(name))
    }

    pub fn get_function(&self, id: FunctionId) -> Option<FunctionDefView<'_, 'ast>> {
        if let Some(def) = self.script_functions.get(&id) {
            return Some(FunctionDefView::Script(def));
        }
        self.ffi.get_function(id).map(FunctionDefView::Ffi)
    }
}
```

---

### 8. FunctionDefView for Unified Access

**Problem:** Code that works with functions needs to handle both FFI and script functions uniformly.

**Solution:** View enum that provides common interface:

```rust
pub enum FunctionDefView<'r, 'ast> {
    Ffi(&'r FfiFunctionDef),
    Script(&'r FunctionDef<'ast>),
}

impl<'r, 'ast> FunctionDefView<'r, 'ast> {
    pub fn name(&self) -> &str {
        match self {
            FunctionDefView::Ffi(f) => &f.name,
            FunctionDefView::Script(f) => &f.name,
        }
    }

    pub fn return_type(&self) -> &DataType {
        match self {
            FunctionDefView::Ffi(f) => f.resolved_return_type(),
            FunctionDefView::Script(f) => &f.return_type,
        }
    }

    pub fn params(&self) -> impl Iterator<Item = ParamView<'_>> {
        // Unified parameter access
    }

    pub fn traits(&self) -> &FunctionTraits {
        match self {
            FunctionDefView::Ffi(f) => &f.traits,
            FunctionDefView::Script(f) => &f.traits,
        }
    }
}
```

**Note:** `TypeDef` doesn't need a view type because it's already lifetime-free and can be stored directly in both FFI and script registries.

---

## Implementation Phases

### Phase 0: Create src/types/ module ✓
**Status:** Complete

- Moved `TypeKind`, `ReferenceKind` from `src/types.rs` to `src/types/type_kind.rs`
- Created module structure with proper re-exports
- All tests passing (2326 tests)

**Files:**
- `src/types/mod.rs` (new)
- `src/types/type_kind.rs` (new, moved from src/types.rs)

---

### Phase 1: Add FfiDataType ✓
**Status:** Complete

- Created `src/types/ffi_data_type.rs`
- Implemented `UnresolvedBaseType` for simple and template types
- Implemented `FfiDataType` with Resolved/Unresolved variants
- Implemented recursive `resolve()` method with lookup and instantiate callbacks
- Added comprehensive tests (24 tests)

**Key Implementation Details:**
- Primitives immediately resolve to `FfiDataType::Resolved(DataType)`
- User types become `FfiDataType::Unresolved { base: Simple(name), ... }`
- Template types use nested FfiDataType for arguments
- All modifiers preserved on unresolved types
- Helper constructors: `unresolved_simple()`, `unresolved_handle()`, `unresolved_const()`, `unresolved_template_simple()`

---

### Phase 2: Add FfiExpr and FfiFunctionDef
**Status:** In Progress

**Tasks:**
1. Create `src/types/ffi_expr.rs`:
   - Define `FfiExpr` enum with literal variants
   - Add `EnumValue`, `Construct`, `Unary`, `Binary` variants
   - Implement conversion from parsed AST expressions
   - Add resolution method for enum values and constructors

2. Create `src/types/ffi_function.rs`:
   - Define `FfiParam` struct
   - Define `FfiFunctionDef` struct
   - Implement `from_parsed()` for AST conversion
   - Implement `resolve()` for type resolution

3. Update `src/types/mod.rs`:
   - Add new module declarations
   - Re-export public types

**Files:**
- `src/types/ffi_expr.rs` (new)
- `src/types/ffi_function.rs` (new)
- `src/types/mod.rs` (modify)

---

### Phase 3: Create FfiRegistry
**Status:** Pending

**Tasks:**
1. Create `src/types/ffi_registry.rs`:
   - Define `FfiRegistry` struct with type/function storage
   - Implement lookup methods
   - Define `FfiRegistryBuilder` for construction phase
   - Implement `build()` method with type resolution

2. Handle template callbacks:
   - Store template instantiation callbacks
   - Support runtime template instantiation

**Files:**
- `src/types/ffi_registry.rs` (new)
- `src/types/mod.rs` (modify)

---

### Phase 4: Update Registration Builders
**Status:** Pending

**Tasks:**
1. Update `src/ffi/class_builder.rs`:
   - Change to produce `TypeDef` + unresolved method signatures
   - Use `FfiDataType` for parameter/return types
   - Keep arena parsing, convert to owned immediately

2. Update `src/ffi/function_builder.rs`:
   - Produce `FfiFunctionDef` instead of registering directly
   - Parse default args into `FfiExpr`
   - Handle method binding to owner type

3. Update `src/module.rs`:
   - Change `Module` to hold `FfiRegistryBuilder` data
   - Implement `install_into(builder)` method

**Files:**
- `src/ffi/class_builder.rs` (modify)
- `src/ffi/function_builder.rs` (modify)
- `src/module.rs` (modify)

---

### Phase 5: Update Context
**Status:** Pending

**Tasks:**
1. Update `src/context.rs`:
   - Add `builder: Option<FfiRegistryBuilder>` field
   - Add `ffi_registry: Option<Arc<FfiRegistry>>` field
   - Implement sealing logic in `create_unit()`
   - Add `ContextError::AlreadySealed` variant
   - Update `install()` to use builder

2. Add explicit seal method (optional):
   - `context.seal()` for explicit sealing
   - Useful for checking errors before first unit

**Files:**
- `src/context.rs` (modify)
- `src/error.rs` (modify, add ContextError)

---

### Phase 6: Refactor Registry
**Status:** Pending

**Tasks:**
1. Update `src/semantic/types/registry.rs`:
   - Add `ffi: Arc<FfiRegistry>` field
   - Add script-specific HashMaps
   - Implement two-tier lookup methods
   - Update all existing methods for new structure

2. Create view types:
   - `FunctionDefView` enum
   - Unified parameter/return type access

3. Update callers:
   - Compiler passes that access registry
   - Type resolution code
   - Overload resolution

**Files:**
- `src/semantic/types/registry.rs` (major refactor)
- `src/semantic/types/mod.rs` (modify exports)

---

### Phase 7: Update Compiler and Unit
**Status:** Pending

**Tasks:**
1. Update `src/unit.rs`:
   - Accept `Arc<FfiRegistry>` in constructor
   - Pass to compiler

2. Update `src/semantic/compiler.rs`:
   - Create `Registry` with FFI reference
   - Remove `import_modules()` call
   - Use new two-tier lookup

3. Remove old import code:
   - Delete `import_modules()` function
   - Clean up related helpers

**Files:**
- `src/unit.rs` (modify)
- `src/semantic/compiler.rs` (modify)
- `src/semantic/types/registry.rs` (remove import_*)

---

## Expected Performance

| Scenario | Before | After | Improvement |
|----------|--------|-------|-------------|
| Context creation | ~0ms | ~2ms | N/A (one-time cost) |
| Unit::build() tiny script | ~2ms | ~10µs | **200x faster** |
| Unit::build() small script | ~2.2ms | ~100µs | **22x faster** |
| Unit::build() large script | ~3.5ms | ~1.5ms | **2.3x faster** |

**Why the improvement varies:**
- Tiny/small scripts: Nearly all time was FFI import, now eliminated
- Large scripts: FFI import was smaller fraction, actual compilation dominates

---

## Files Summary

### New Files
| File | Phase | Purpose |
|------|-------|---------|
| `src/types/mod.rs` | 0 ✓ | Module structure |
| `src/types/type_kind.rs` | 0 ✓ | TypeKind, ReferenceKind |
| `src/types/ffi_data_type.rs` | 1 ✓ | FfiDataType, UnresolvedBaseType |
| `src/types/ffi_expr.rs` | 2 | FfiExpr for default args |
| `src/types/ffi_function.rs` | 2 | FfiFunctionDef, FfiParam |
| `src/types/ffi_registry.rs` | 3 | FfiRegistry, FfiRegistryBuilder |

### Modified Files
| File | Phase | Changes |
|------|-------|---------|
| `src/ffi/class_builder.rs` | 4 | Use FfiDataType |
| `src/ffi/function_builder.rs` | 4 | Produce FfiFunctionDef |
| `src/module.rs` | 4 | Hold builder data, install_into() |
| `src/context.rs` | 5 | Sealing, Arc<FfiRegistry> |
| `src/semantic/types/registry.rs` | 6 | Two-tier architecture |
| `src/semantic/compiler.rs` | 7 | Remove import_modules |
| `src/unit.rs` | 7 | Accept Arc<FfiRegistry> |

---

## Validation Checklist

### Functional Tests
- [ ] `cargo test --lib` passes (all unit tests)
- [ ] `cargo test --test integration_tests` passes
- [ ] FFI type registration works
- [ ] FFI function registration works
- [ ] Template instantiation works
- [ ] Default arguments work
- [ ] Operator overloading works
- [ ] Multiple Units from same Context work correctly

### Performance Tests
- [ ] `cargo bench --bench module_benchmarks` shows improvement
- [ ] Tiny script benchmark < 100µs (down from ~2ms)
- [ ] Small script benchmark < 500µs (down from ~2.2ms)
- [ ] No regression in large script compilation

### Error Handling
- [ ] Context sealing works (install after create_unit errors)
- [ ] Unresolved type errors are clear
- [ ] Missing FFI type errors are helpful

### Code Quality
- [ ] No new clippy warnings
- [ ] Documentation updated
- [ ] No unsafe code added

---

## Open Questions / Future Work

1. **Template Instance Caching:** Should template instances created during compilation be cached in `FfiRegistry` for reuse across Units? Currently planned to cache per-Unit.

2. **Incremental Registration:** Could we support adding more FFI types after sealing? Would require careful invalidation.

3. **Thread Safety:** `Arc<FfiRegistry>` enables multi-threaded compilation. Worth pursuing?

4. **Memory Layout:** Could we use a single allocation for all FFI data (arena-style) to improve cache locality?
