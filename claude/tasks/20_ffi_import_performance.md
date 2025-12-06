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
**Status:** Complete

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

**Key Implementation Details:**
- `FfiExpr` enum covers: `Int`, `UInt`, `Float`, `Bool`, `String`, `Null`, `EnumValue`, `Construct`, `Unary`, `Binary`, `Ident`, `ScopedIdent`
- `FfiExpr::from_ast()` converts arena-allocated expressions to owned form
- `FfiParam` holds name, `FfiDataType`, and optional `FfiExpr` default value
- `FfiFunctionDef` holds all function metadata with deferred type resolution
- `FfiFunctionDef::resolve()` produces `ResolvedFfiFunctionDef` with concrete `DataType`s
- `FfiResolutionError` provides detailed error messages for resolution failures
- Comprehensive tests: 48 tests for ffi_expr, ffi_function modules

---

### Phase 3: Create FfiRegistry
**Status:** Complete

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

**Key Implementation Details:**
- `FfiRegistry` holds resolved types, functions, behaviors, template callbacks, and namespaces
- `FfiRegistryBuilder::new()` pre-registers all primitive types (void, bool, int, etc.)
- `FfiRegistryBuilder::build()` resolves all `FfiFunctionDef` to `ResolvedFfiFunctionDef`
- Lookup methods mirror `Registry` API: `get_type()`, `lookup_functions()`, `find_method()`, `find_operator_method()`, `find_constructors()`, `find_factories()`, `get_behaviors()`, etc.
- Template callbacks stored as `Arc<dyn Fn>` for thread-safe sharing
- Manual `Debug` impl to handle non-Debug callback types
- Comprehensive tests: 16 tests for registry creation, lookup, resolution errors
- All 2400 tests passing

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

### Phase 6: Refactor Registry Architecture
**Status:** In Progress (6.1-6.6 Complete)

**Goal:** Refactor so `Registry` becomes `ScriptRegistry` (no FFI knowledge), introduce `CompilationContext` as the unified facade, and use TypeId/FunctionId high bits to identify FFI vs Script.

---

#### Phase 6.1: TypeId and FunctionId Refactoring ✓
**Status:** Complete
**File:** `src/semantic/types/type_def.rs`

**TypeId changes:**
1. Added `FFI_BIT` constant (`0x8000_0000`) and helper methods:
   - `is_ffi()` - checks if high bit is set
   - `is_script()` - checks if high bit is clear
2. Updated primitive constants to use FFI bit:
   ```rust
   pub const VOID_TYPE: TypeId = TypeId(0x8000_0000);
   pub const BOOL_TYPE: TypeId = TypeId(0x8000_0001);
   // ... etc for all 12 primitives (0-11)
   ```
3. Added separate atomic counters:
   - `FFI_TYPE_ID_COUNTER` starting at 32 (after primitives + special types)
   - `SCRIPT_TYPE_ID_COUNTER` starting at 0
4. Added `TypeId::next_ffi()` and `TypeId::next_script()` methods
5. Updated special types to use FFI bit:
   - `VARIABLE_PARAM_TYPE = TypeId(0x8000_000C)` (12)
   - `NULL_TYPE = TypeId(0x8000_000D)` (13)
   - `SELF_TYPE = TypeId(0xFFFF_FFFE)` (max - 1, FFI bit set)
   - `FIRST_USER_TYPE_ID = TypeId(0x8000_0020)` (32)

**FunctionId changes:**
1. Added `FFI_BIT` constant and helper methods:
   - `is_ffi()`, `is_script()`
2. Added separate atomic counters:
   - `FFI_FUNCTION_ID_COUNTER` starting at 0
   - `SCRIPT_FUNCTION_ID_COUNTER` starting at 0
3. Added `FunctionId::next_ffi()` and `FunctionId::next_script()` methods

---

#### Phase 6.2: FfiRegistry Updates ✓
**Status:** Complete
**Files:**
- `src/ffi/ffi_registry.rs`
- `src/ffi/class_builder.rs`
- `src/ffi/interface_builder.rs`
- `src/ffi/enum_builder.rs`
- `src/ffi/native_fn.rs`
- `src/module.rs`
- `src/types/ffi_convert.rs`
- `src/types/ffi_*.rs` (doc comment updates)

**Changes:**
1. Updated `FfiRegistryBuilder::new()`:
   - Now stores `TypeDef::Primitive` entries in `types` HashMap
   - Added "int" and "uint" aliases
2. Updated `register_type()` to use `TypeId::next_ffi()`
3. Updated all builders to use `TypeId::next_ffi()`:
   - `class_builder.rs::build()`
   - `interface_builder.rs::build()`
   - `enum_builder.rs::build()`
4. Updated `NativeFn::new()` to use `FunctionId::next_ffi()`
5. Updated `module.rs` for funcdefs, template params, and properties
6. Updated `ffi_convert.rs::signature_to_ffi_function_def()`
7. Updated all doc comments to reference `next_ffi()` methods

---

#### Phase 6.3: Rename Registry to ScriptRegistry ✓
**Status:** Complete
**Files:**
- `src/semantic/types/registry.rs` (major changes)
- `src/semantic/types/mod.rs` (update exports)
- `src/semantic/mod.rs` (re-export with alias)
- `src/semantic/passes/registration.rs` (use constants)
- `src/semantic/passes/type_compilation.rs` (use constants)
- `src/semantic/passes/function_processor/*.rs` (use constants)
- `src/semantic/types/conversion.rs` (use ScriptRegistry)
- `src/semantic/const_eval.rs` (use ScriptRegistry)
- `src/semantic/compiler.rs` (stub import_modules)

**Changes:**
1. Renamed `Registry<'ast>` to `ScriptRegistry<'ast>`
2. Removed fields: `template_callbacks`, `template_cache`, `void_type`, `bool_type`, etc.
3. Updated `new()` to initialize empty HashMap (no primitives)
4. Removed all `import_*` methods (FFI handled by FfiRegistry)
5. Added stub `instantiate_template()` that returns error (moves to CompilationContext)
6. Replaced `registry.void_type` etc. with `VOID_TYPE` constants throughout codebase
7. Updated `compiler.rs::compile_with_modules()` to stub out module import (Phase 6.5)
8. Added `Registry` alias for backwards compatibility during transition
9. Updated tests: removed primitive tests, ignored template tests pending Phase 6.4

**Note:** ~554 tests fail because they expect primitives in Registry. These tests need
CompilationContext (Phase 6.4) to work properly with the new architecture.

---

#### Phase 6.4: Create CompilationContext ✓
**Status:** Complete
**New files:**
- `src/semantic/compilation_context.rs`
- `src/semantic/template_instantiator.rs`

```rust
pub struct CompilationContext<'ast> {
    ffi: Arc<FfiRegistry>,
    script: ScriptRegistry<'ast>,
    type_by_name: FxHashMap<String, TypeId>,      // Unified name lookup
    func_by_name: FxHashMap<String, Vec<FunctionId>>,  // Unified name lookup
    template_instantiator: TemplateInstantiator,
}
```

**Changes:**

1. **Created `CompilationContext`** (`src/semantic/compilation_context.rs`):
   - Unified type/function lookup with `type_by_name` and `func_by_name` maps
   - Initialized from FFI registry on construction
   - Routes ID-based lookups by `is_ffi()` bit
   - Delegates registration to ScriptRegistry and updates unified maps

2. **Created `TemplateInstantiator`** (`src/semantic/template_instantiator.rs`):
   - Single-responsibility struct for template instantiation
   - Caches template instances to avoid duplicates
   - Handles validation callbacks for FFI templates
   - Copies behaviors from template to instance

3. **Implemented unified lookup methods**:
   - `lookup_type()` - single HashMap lookup (unified map)
   - `get_type()` - routes by `is_ffi()` bit
   - `lookup_functions()` - single HashMap lookup (unified map)
   - All behavior/method/operator lookups route by `is_ffi()`

4. **Updated exports** (`src/semantic/mod.rs`):
   - Added `template_instantiator` module
   - Re-exported `CompilationContext`

5. **Re-enabled ignored tests**:
   - `init_list_empty_error` ✓
   - `init_list_simple_int` ✓
   - `init_list_nested` ✓
   - `init_list_type_promotion` ✓

   Added `create_test_context_with_array()` helper that creates FfiRegistry
   with array template, wraps in CompilationContext for template instantiation.

**Test Results:**
- All 4 re-enabled tests pass
- 1869 total tests pass (up from 1851 in Phase 6.3)
- 554 tests still fail (need Phase 6.5 to use CompilationContext in compiler)

---

#### Phase 6.4.1: Add Global Property Support to FfiRegistry
**Status:** DEFERRED to Task 22 (TypeHash Identity System)

**Reason:** Global property registration requires type resolution, which currently requires a sealed
registry. The TypeHash-based identity system (Task 22) eliminates this requirement by allowing types
to be referenced by their deterministic hash before registration. This simplifies the global property
API significantly.

**See:** `claude/tasks/22_typehash_identity.md` Phase 6 for the deferred implementation.

---

#### Phase 6.4.2: ScriptParam Refactoring ✓
**Status:** Complete

Unified default argument handling between FFI and Script functions by introducing `ScriptParam`.

**Changes:**

1. **Created `ScriptParam<'ast>` struct** (`src/semantic/types/registry.rs`):
   ```rust
   pub struct ScriptParam<'ast> {
       pub name: String,
       pub data_type: DataType,
       pub default: Option<&'ast Expr<'ast>>,
   }
   ```

2. **Updated `FunctionDef`**:
   - Changed `params: Vec<DataType>` → `params: Vec<ScriptParam<'ast>>`
   - Removed `default_args: Vec<Option<&'ast Expr<'ast>>>` field
   - Defaults now inline on each param (like FFI's `ResolvedFfiParam.default_value`)

3. **Updated `FunctionRef` unified interface** (`src/semantic/compilation_context.rs`):
   - `required_param_count()` - works for both FFI and Script
   - `has_defaults()` - unified default arg detection
   - `param_types()` - extracts `Vec<DataType>` from params

4. **Updated all call sites**:
   - `type_compilation.rs`: Creates `Vec<ScriptParam>` with inline defaults
   - `registration.rs`: Removed `default_args` from `FunctionDef` construction
   - `expr_checker.rs`: Changed `param.type_id` → `param.data_type.type_id`
   - `overload_resolver.rs`: Uses `param.data_type` for type comparisons
   - `function_processor/mod.rs`: Extracts `(name, data_type)` from `ScriptParam`

5. **Updated `update_function_signature`**:
   - Now takes `Vec<ScriptParam<'ast>>` instead of separate params + default_args

**Test Results:**
- Library compiles successfully
- 554 tests still fail (blocked on Phase 6.5 - need CompilationContext in compiler)

---

#### Phase 6.5: Update Compiler Infrastructure ✓
**Status:** Complete

**Solution:** Updated all compilation passes to use `CompilationContext` instead of `ScriptRegistry`
directly, enabling unified access to both FFI and Script types.

**Files Changed:**
- `src/semantic/compiler.rs` - Added `compile_with_ffi()`, updated `CompilationResult` to use `context`
- `src/semantic/passes/registration.rs` - Added `register_with_context()`, `RegistrationDataWithContext`
- `src/semantic/passes/type_compilation.rs` - Changed to use `CompilationContext`
- `src/semantic/passes/function_processor/*.rs` - Changed to use `CompilationContext`
- `src/semantic/types/conversion.rs` - Updated `can_convert_to()` to accept `CompilationContext`
- `src/semantic/const_eval.rs` - Updated `ConstEvaluator` to use `CompilationContext`

**API Changes:**

1. `Compiler::compile_with_ffi()`:
   - New primary entry point accepting `Arc<FfiRegistry>`
   - Creates `CompilationContext` and passes through all passes
   - `compile()` and `compile_with_modules()` deprecated (still work via default FFI)

2. `Registrar::register_with_context()`:
   - New method accepting pre-built `CompilationContext`
   - Returns `RegistrationDataWithContext` with `.context` field

3. `TypeCompiler::compile()`:
   - Now accepts `CompilationContext` instead of `ScriptRegistry`
   - Returns `TypeCompilationData` with `.context` field

4. `FunctionCompiler::compile()`:
   - Now accepts `&CompilationContext` instead of `&Registry`
   - All sub-modules use `self.context` for lookups

5. `DataType::can_convert_to()`:
   - Changed signature to accept `&CompilationContext` instead of `&ScriptRegistry`
   - Now properly supports FFI type lookups for conversions

6. `ConstEvaluator::new()`:
   - Now accepts `&CompilationContext` instead of `&ScriptRegistry`
   - Enum lookups work with both FFI and Script enums

**Test Results:**
- 2403 tests passing (up from 1869 in Phase 6.4)
- 20 tests failing - all related to FFI module installation
  - These tests use deprecated `compile_with_modules()` which no longer installs modules
  - Need Phase 6.6 to update module installation API

**Remaining Failures (20 tests):**
All fail with "undefined type 'string'" or "undefined type 'array'" because:
- Tests call `Compiler::compile_with_modules(&script, &[string_module])`
- This deprecated method ignores the modules parameter
- Need to update tests to use `compile_with_ffi()` with modules installed in `FfiRegistryBuilder`

**Notes:**
- `CompilationContext::default()` creates a context with primitives only (no string/array/dictionary)
- Tests that need FFI types (string, array) must build an `FfiRegistry` with those modules installed
- This is working as designed - Phase 6.6 will address module installation API

---

#### Phase 6.6: Update Context and Unit ✓
**Status:** Complete
**Files:**
- `src/context.rs`
- `src/unit.rs`
- `src/semantic/compiler.rs`
- `src/semantic/compilation_context.rs`
- `src/semantic/template_instantiator.rs`
- `src/semantic/passes/function_processor/expr_checker.rs`
- `src/semantic/passes/type_compilation.rs`
- `src/ffi/class_builder.rs`
- `src/types/ffi_convert.rs`

**Changes:**

1. **`Unit::build()` uses `compile_with_ffi()`**:
   - Gets `Arc<FfiRegistry>` from Context
   - Passes to `Compiler::compile_with_ffi()`
   - Falls back to default FFI registry (primitives only) if no context

2. **Deleted `compile_with_modules()`**:
   - Removed deprecated method from `src/semantic/compiler.rs`
   - Updated all tests to use `compile_with_ffi()` with proper FFI registries
   - Added test helpers: `create_ffi_with_string()`, `create_ffi_with_array()`, `create_ffi_with_string_and_array()`

3. **Template specialization for operator methods**:
   - `TemplateInstantiator` now creates specialized `FunctionDef` with substituted types
   - When instantiating `array<int>`, operator methods get `int` return types (not `T`)
   - Added `substitute_type()` helper to replace template params with concrete types

4. **Unified function lookup via `FunctionRef`**:
   - `CompilationContext::get_function()` returns `FunctionRef` (works for FFI and Script)
   - Added `get_script_function()` for script-only access
   - `FunctionRef` provides unified interface: `param_count()`, `param_type()`, `return_type()`, `traits()`, `name()`
   - Updated all operator method lookups to use unified interface

5. **Fixed operator behavior detection**:
   - Moved auto-detection from `signature_to_ffi_function()` to `operator()` and `operator_raw()` in class_builder
   - Operators are now properly registered with their behavior when using the builder API

6. **Updated `find_operator_method_with_mutability()`**:
   - Now implemented directly in `CompilationContext` using unified function lookup
   - Handles cases where script types (template instances) have FFI function IDs in their `operator_methods`

**Test Results:**
- 2423 tests passing
- All FFI module tests working with proper registry setup

---

#### Phase 6.7: Cleanup and Testing ✓
**Status:** Complete

**Changes:**
1. Removed deprecated `Compiler::compile()` (no-args version)
2. Renamed `Compiler::compile_with_ffi()` → `Compiler::compile(script, ffi)`
3. Removed `Registry` type alias from `src/semantic/mod.rs`
4. Updated all ~420 test usages to pass `default_ffi()` helper
5. Fixed unused import warning in `conversion.rs`
6. Updated doc comments to reference `CompilationContext` instead of `Registry`

**Test Results:**
- 2423 tests passing
- All FFI module tests working with proper registry setup

**Design Notes:**
- Template instances are always Script types (created in ScriptRegistry)
- Template definitions can be FFI types (e.g., `array<T>`)
- Primitives are FFI types - `CompilationContext.lookup_type()` checks FfiRegistry first
- The high bit approach allows O(1) routing without string comparisons
- `get_type()`/`get_function()` dispatch by checking `is_ffi()` on the ID
- `lookup_type()`/`lookup_functions()` check FFI first, then Script (for shadowing)

**Files:**
- `src/semantic/types/type_def.rs` (TypeId/FunctionId FFI bit)
- `src/semantic/types/registry.rs` (rename to ScriptRegistry, use HashMap)
- `src/ffi/ffi_registry.rs` (use FFI TypeIds, store primitives)
- `src/semantic/compilation_context.rs` (NEW: unified facade)
- `src/semantic/types/mod.rs` (update exports)
- `src/semantic/compiler.rs` (use CompilationContext)
- `src/semantic/passes/registration.rs` (use CompilationContext)
- `src/semantic/passes/type_compilation.rs` (use CompilationContext)
- `src/semantic/passes/function_processor/*.rs` (use CompilationContext)
- `src/context.rs` (pass FfiRegistry to compilation)
- `src/unit.rs` (create CompilationContext)

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

3. Remove old import code from `src/semantic/types/registry.rs`:
   - Delete `import_modules()` function
   - Delete `import_type_shell()` function
   - Delete `import_type_details()` function
   - Delete `import_function()` function
   - Delete `import_behavior()` function
   - Delete `import_enum()` function
   - Delete `import_interface()` function
   - Delete `import_funcdef()` function
   - Delete any other `import_*` helper functions

4. Remove deprecated NativeX types from `src/ffi/types.rs`:
   - Delete `NativeFunctionDef<'ast>` struct
   - Delete `NativeTypeDef<'ast>` struct
   - Delete `NativeMethodDef<'ast>` struct
   - Delete `NativePropertyDef<'ast>` struct
   - Delete `NativeInterfaceDef<'ast>` struct
   - Delete `NativeInterfaceMethod<'ast>` struct
   - Delete `NativeFuncdefDef<'ast>` struct
   - Update module exports in `src/ffi/types.rs`
   - Update `src/ffi/mod.rs` exports if needed

5. Rename and move `NativeEnumDef` (still needed for FfiRegistry):
   - Rename `NativeEnumDef` to `FfiEnumDef`
   - Move from `src/module.rs` to `src/types/ffi_enum.rs`
   - Update `src/types/mod.rs` to export `FfiEnumDef`
   - Update all usages in `src/module.rs`, `src/ffi/enum_builder.rs`, `src/lib.rs`

6. Struct to keep unchanged:
   - `NativeFn` in `src/ffi/native_fn.rs` - actual function pointer wrapper (used by FfiFunctionDef)

**Files:**
- `src/unit.rs` (modify)
- `src/semantic/compiler.rs` (modify)
- `src/semantic/types/registry.rs` (remove all import_* functions)
- `src/ffi/types.rs` (remove NativeX structs)
- `src/types/ffi_enum.rs` (new - FfiEnumDef moved from module.rs)
- `src/module.rs` (modify - remove NativeEnumDef, use FfiEnumDef)
- `src/ffi/enum_builder.rs` (modify - use FfiEnumDef)
- `src/lib.rs` (modify - update re-export)

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
