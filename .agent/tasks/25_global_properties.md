# Task 25: Global Properties (Revised)

## Overview

Enable FFI registration of global properties that scripts can access. Two categories:
1. **Constants** - Immutable values like `math::PI`
2. **Mutable globals** - Shared state between application and script via `Arc<RwLock<T>>`

**Prerequisites:** Task 01 (Unified Type Registry)

---

## Design Decisions

1. **Constants**: Owned by registry, always immutable
2. **Mutable globals**: Application owns `Arc<RwLock<T>>`, shares clone with engine
3. **Handle semantics**: All FFI global properties are implicitly `@ const` (no handle reassignment)
4. **Object constness**: `.const_()` modifier makes object read-only from script
5. **Type inference**: `DataType` derived from `T::data_type()` on `Any` trait (includes `is_handle` flag)
6. **Namespace**: Inherited from `Module`
7. **Unified API**: Single `global()` method with trait-based dispatch

---

## API Design

### Explicit Registration (Module Builder)

```rust
// Constants - raw primitives are always const
Module::in_namespace(&["math"])
    .global("PI", std::f64::consts::PI)    // const double math::PI
    .global("E", std::f64::consts::E);     // const double math::E

// Mutable global property - use Arc<RwLock<T>>
let score = Arc::new(RwLock::new(0i32));
Module::new()
    .global("g_score", score.clone());  // int g_score (mutable)

// Const object (read-only from script)
let player = Arc::new(RwLock::new(Player::new()));
Module::new()
    .global("g_player", player.clone())
    .const_();  // const Player@ const g_player
```

### Macro Registration (Primitive Constants)

For primitive constants, macros provide a convenient alternative:

```rust
// Basic constant
#[angelscript::global]
pub const PI: f64 = 3.14159265358979;

// With namespace
#[angelscript::global(namespace = "math")]
pub const E: f64 = 2.71828182845905;

// With name override
#[angelscript::global(name = "MAX_INT", namespace = "limits")]
pub const I32_MAX: i32 = i32::MAX;
```

Generates metadata function:
```rust
pub fn PI__global_meta() -> GlobalMeta {
    GlobalMeta {
        name: "PI",
        namespace: None,
        value: ConstantValue::Double(3.14159265358979),
    }
}
```

Module registration:
```rust
Module::in_namespace(&["math"])
    .global_meta(PI__global_meta)
    .global_meta(E__global_meta)
```

**Note**: Macro registration only supports primitive constants. Mutable globals (`Arc<RwLock<T>>`) must use explicit registration because the application needs to hold a reference to interact with the value.

---

## Type Mapping

| Rust `T` in `Arc<RwLock<T>>` | T's TypeKind | AngelScript Declaration |
|------------------------------|--------------|------------------------|
| `i32` | Primitive | `int g_score` |
| `f64` | Primitive | `double g_value` |
| `Vector2` (pod) | Value | `Vector2 g_pos` |
| `Player` (reference) | Reference | `Player@ const g_player` |

With `.const_()`:
- Primitives/Values: `const int g_score`
- Reference types: `const Player@ const g_player`

---

## Core Types

### GlobalPropertyEntry

```rust
/// A global property registered with the engine
pub struct GlobalPropertyEntry {
    pub name: String,
    pub qualified_name: String,
    pub type_hash: TypeHash,
    pub data_type: DataType,
    pub is_const: bool,  // Object constness (const Player@ const)
    pub source: TypeSource,
    pub implementation: GlobalPropertyImpl,
}

/// How the global property value is stored/accessed
pub enum GlobalPropertyImpl {
    /// Constant value (primitives only for now)
    Constant(ConstantValue),
    /// Mutable FFI property via Arc<RwLock<T>>
    Mutable(Box<dyn GlobalPropertyAccessor>),
    /// Script-declared global (slot in unit's global table)
    Script { slot: u32 },
}
```

### ConstantValue

```rust
/// Primitive constant values
#[derive(Debug, Clone, Copy)]
pub enum ConstantValue {
    Bool(bool),
    Int8(i8), Int16(i16), Int32(i32), Int64(i64),
    Uint8(u8), Uint16(u16), Uint32(u32), Uint64(u64),
    Float(f32), Double(f64),
}
```

### GlobalPropertyAccessor Trait

```rust
/// Type-erased accessor for mutable global properties
pub trait GlobalPropertyAccessor: Send + Sync {
    /// Get the data type of this property
    fn data_type(&self) -> DataType;

    /// Read the current value (type-erased)
    fn read(&self) -> Box<dyn std::any::Any>;

    /// Write a new value (type-erased)
    fn write(&self, value: Box<dyn std::any::Any>) -> Result<(), PropertyError>;
}

impl<T: Any + Clone + Send + Sync + 'static> GlobalPropertyAccessor for Arc<RwLock<T>> {
    fn data_type(&self) -> DataType {
        T::data_type()  // Uses our Any trait
    }

    fn read(&self) -> Box<dyn std::any::Any> {
        Box::new(self.read().unwrap().clone())
    }

    fn write(&self, value: Box<dyn std::any::Any>) -> Result<(), PropertyError> {
        let typed = value.downcast::<T>()
            .map_err(|_| PropertyError::TypeMismatch)?;
        *self.write().unwrap() = *typed;
        Ok(())
    }
}
```

### IntoGlobalProperty Trait

```rust
/// Trait for types that can be registered as global properties
pub trait IntoGlobalProperty {
    fn into_global_impl(self) -> GlobalPropertyImpl;
    fn data_type() -> DataType;
    fn is_inherently_const() -> bool;
}

// Raw primitives -> Constant (always const)
impl IntoGlobalProperty for i32 {
    fn into_global_impl(self) -> GlobalPropertyImpl {
        GlobalPropertyImpl::Constant(ConstantValue::Int32(self))
    }
    fn data_type() -> DataType { DataType::simple(primitives::INT32) }
    fn is_inherently_const() -> bool { true }
}

// Arc<RwLock<T>> -> Mutable, uses T::data_type() directly
// The #[derive(Any)] macro generates data_type() with is_handle=true for reference types
impl<T: Any + Clone + Send + Sync + 'static> IntoGlobalProperty for Arc<RwLock<T>> {
    fn into_global_impl(self) -> GlobalPropertyImpl {
        GlobalPropertyImpl::Mutable(Box::new(self))
    }
    fn data_type() -> DataType { T::data_type() }
    fn is_inherently_const() -> bool { false }
}
```

### GlobalMeta (for macros)

```rust
/// Metadata returned by #[angelscript::global] macro
pub struct GlobalMeta {
    pub name: &'static str,
    pub namespace: Option<&'static [&'static str]>,
    pub value: ConstantValue,
}
```

---

## Files to Modify/Create

### Phase 1: Core Types (angelscript-core)

**`crates/angelscript-core/src/entries/global_property.rs`** (new)
- `GlobalPropertyEntry`
- `GlobalPropertyImpl`
- `ConstantValue`

**`crates/angelscript-core/src/global_property_accessor.rs`** (new)
- `GlobalPropertyAccessor` trait
- `PropertyError` enum
- `impl GlobalPropertyAccessor for Arc<RwLock<T>>`

**`crates/angelscript-core/src/into_global_property.rs`** (new)
- `IntoGlobalProperty` trait
- Implementations for all primitive types
- Implementation for `Arc<RwLock<T>>`

**`crates/angelscript-core/src/global_meta.rs`** (new)
- `GlobalMeta` struct (for macro output)

**`crates/angelscript-core/src/entries/mod.rs`**
- Export `GlobalPropertyEntry`, `GlobalPropertyImpl`, `ConstantValue`

**`crates/angelscript-core/src/lib.rs`**
- Export new modules

### Phase 2: Registry Storage (angelscript-registry)

**`crates/angelscript-registry/src/registry.rs`**
- Add `globals: FxHashMap<TypeHash, GlobalPropertyEntry>`
- Add `register_global()` method
- Add `get_global(hash: TypeHash)` method

**Note**: No separate name lookup map needed - globals use `TypeHash::from_ident(name)` for lookup, following the same pattern as other IDENT-based lookups.

### Phase 3: Module API (angelscript-registry)

**`crates/angelscript-registry/src/module.rs`**
```rust
impl Module {
    /// Register a global property (constant or mutable)
    pub fn global<T: IntoGlobalProperty>(
        self,
        name: &str,
        value: T
    ) -> GlobalPropertyBuilder;

    /// Register from macro-generated metadata
    pub fn global_meta(self, meta: fn() -> GlobalMeta) -> Self;
}

pub struct GlobalPropertyBuilder {
    module: Module,
    entry: GlobalPropertyEntry,
}

impl GlobalPropertyBuilder {
    /// Mark as const (object constness for reference types)
    pub fn const_(mut self) -> Module {
        self.entry.is_const = true;
        self.module.pending_globals.push(self.entry);
        self.module
    }

    /// Finish building (for mutable properties)
    pub fn build(self) -> Module {
        self.module.pending_globals.push(self.entry);
        self.module
    }
}
```

### Phase 4: Macro Support (angelscript-macros)

**`crates/angelscript-macros/src/global.rs`** (new)
- `#[angelscript::global]` attribute macro
- Parses: `name`, `namespace` attributes
- Generates `fn NAME__global_meta() -> GlobalMeta`

**`crates/angelscript-macros/src/lib.rs`**
- Export `global` attribute macro

### Phase 5: Tests

**`crates/angelscript-registry/src/tests/global_tests.rs`** (new)
- Primitive constant registration
- Arc<RwLock<T>> mutable registration
- Const vs mutable access semantics
- Namespace qualification
- Name lookup

**`crates/angelscript-macros/tests/global_macro.rs`** (new)
- Macro-based constant registration
- Namespace and name attributes

---

## Implementation Order

1. Add `ConstantValue` enum to core
2. Add `GlobalPropertyEntry` and `GlobalPropertyImpl` to core
3. Add `GlobalPropertyAccessor` trait and impl for `Arc<RwLock<T>>`
4. Add `IntoGlobalProperty` trait and impls
5. Add `GlobalMeta` struct
6. Add storage to `TypeRegistry` (globals field + lookup methods)
7. Add `global()` and `GlobalPropertyBuilder` to `Module`
8. Add `global_meta()` to `Module`
9. Add `#[angelscript::global]` macro
10. Add tests

---

## Future Work (VM/Heap phase)

- Unsafe memory swapping for handle reassignment
- Script-created objects in global properties
- Remove `@ const` restriction when object heap exists
- Non-primitive constants (strings, value types)

---

## Related Tasks

- **Task 01**: Unified Type Registry - globals stored in TypeRegistry
- **Task 23**: Ergonomic Module API - similar builder patterns
