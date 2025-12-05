# Task 23: Ergonomic Module API

## Overview

Redesign the FFI registration API inspired by [Rune's Module API](https://github.com/rune-rs/rune/blob/main/crates/rune/src/module/module.rs) for a more ergonomic developer experience.

**Prerequisites:** Task 22 (TypeHash Identity System)

---

## Value Types vs Reference Types

### Explicit Registration

Developer explicitly declares which kind of type they're registering:

- **Value types**: Stack allocated, copy semantics, constructors
- **Reference types**: Heap allocated, ref counted, factories
- **Script classes**: Always reference types (implicit)

```rust
module.value_type::<Vec3>()?      // Stack, copy semantics
    .constructor(...)?;

module.reference_type::<Player>()? // Heap, ref counted
    .factory(...)?;
```

### Constructor vs Factory Distinction

These have **different calling conventions** and must remain distinct:

**Value type constructor** - memory pre-allocated by VM:
```rust
// C++: void Constructor(void *memory) { new(memory) Object(); }
// Rust: fn(&mut MaybeUninit<T>, args...) -> ()
.constructor(|mem: &mut MaybeUninit<Vec3>, x: f32, y: f32, z: f32| {
    mem.write(Vec3 { x, y, z });
})?
```

**Reference type factory** - allocates and returns:
```rust
// C++: CRef* Factory() { return new CRef(); }
// Rust: fn(args...) -> Arc<T>
.factory(|| Arc::new(Player::new()))?
```

---

## Fluent Builder API

### Type Registration

```rust
// Value type
module.value_type::<Vec3>()?
    .constructor(Vec3::construct)?
    .method("length", Vec3::length)?
    .property("x", Vec3::get_x, Vec3::set_x)?
    .op_add(Vec3::add)?
    .op_index(Vec3::index)?;

// Reference type
module.reference_type::<Player>()?
    .factory(Player::create)?
    .addref(Player::add_ref)?
    .release(Player::release)?
    .method("update", Player::update)?;
```

### Functions and Properties

```rust
// Free function
module.function("print", print_fn)?;

// Global property (name, type, value)
module.global_property("g_score", "int", &mut score)?;

// Constant
module.constant("PI", 3.14159f64)?;
```

---

## TypeOf Trait

Trait for Rust types that have an AngelScript type identity:

```rust
/// Trait for Rust types that have an AngelScript type identity.
pub trait TypeOf {
    /// Get the TypeHash for this type.
    fn type_hash() -> TypeHash;

    /// Get the AngelScript type name.
    fn type_name() -> &'static str;

    /// Get the type kind (value or reference).
    fn type_kind() -> TypeKind;
}

/// Whether a type is value (stack) or reference (heap/refcounted).
pub enum TypeKind {
    Value,      // Stack allocated, copy semantics, constructors
    Reference,  // Heap allocated, ref counted, factories
}
```

---

## Derive Macros

### Type Derive

```rust
// Value type
#[derive(AngelScript)]
#[as_value_type]
#[as_name = "Vec3"]  // optional, defaults to struct name
struct Vec3 {
    #[as_property]
    pub x: f32,
    #[as_property]
    pub y: f32,
    #[as_property]
    pub z: f32,
}

// Reference type
#[derive(AngelScript)]
#[as_reference_type]
struct Player {
    name: String,
    health: i32,
}
```

Generates:
```rust
impl TypeOf for Vec3 {
    fn type_hash() -> TypeHash {
        TypeHash::of("Vec3")  // computed at compile time
    }
    fn type_name() -> &'static str {
        "Vec3"
    }
    fn type_kind() -> TypeKind {
        TypeKind::Value
    }
}
```

### Impl Block Attribute

```rust
#[angelscript]
impl Vec3 {
    #[as_constructor]  // For value types - placement init
    pub fn new(x: f32, y: f32, z: f32) -> Self { ... }

    #[as_method]
    pub fn length(&self) -> f32 { ... }

    #[as_method(name = "normalized")]  // rename in script
    pub fn normalize(&self) -> Self { ... }

    #[as_operator(opAdd)]
    pub fn add(&self, other: &Vec3) -> Vec3 { ... }
}

#[angelscript]
impl Player {
    #[as_factory]  // For reference types - allocates and returns
    pub fn create(name: String) -> Arc<Self> { ... }

    #[as_method]
    pub fn update(&mut self) { ... }
}
```

### Simplified Registration

```rust
// Before (manual):
module.value_type::<Vec3>()?
    .constructor(Vec3::construct)
    .method("length", Vec3::length)
    .property("x", Vec3::get_x, Vec3::set_x);

// After (with macros):
module.register::<Vec3>()?;  // Everything from derive + attributes
module.register::<Player>()?;
```

---

## Implementation Phases

### Phase 1: Fluent Builder API
- [ ] Create `ValueTypeBuilder<T>` with fluent methods
- [ ] Create `ReferenceTypeBuilder<T>` with fluent methods
- [ ] Add `module.value_type::<T>()` and `module.reference_type::<T>()`
- [ ] Implement constructor/factory registration
- [ ] Implement method/property/operator registration

### Phase 2: TypeOf Trait
- [ ] Define `TypeOf` trait
- [ ] Define `TypeKind` enum
- [ ] Implement `TypeOf` for primitive types
- [ ] Add manual impl support

### Phase 3: Derive Macro (Struct)
- [ ] Create `angelscript-derive` proc-macro crate
- [ ] Implement `#[derive(AngelScript)]`
- [ ] Support `#[as_value_type]` / `#[as_reference_type]`
- [ ] Support `#[as_name = "..."]`
- [ ] Support `#[as_property]` on fields

### Phase 4: Attribute Macro (Impl Blocks)
- [ ] Implement `#[angelscript]` attribute macro
- [ ] Support `#[as_constructor]` / `#[as_factory]`
- [ ] Support `#[as_method]` with optional rename
- [ ] Support `#[as_operator(...)]`
- [ ] Collect metadata for registration

### Phase 5: Auto-Registration
- [ ] Add `module.register::<T>()?` that uses TypeOf + collected metadata
- [ ] Automatic constructor/factory detection from attributes
- [ ] Automatic method/property registration

---

## Critical Files

| File | Purpose |
|------|---------|
| `src/ffi/module.rs` | Fluent builder API |
| `src/ffi/type_of.rs` | TypeOf trait and TypeKind |
| `angelscript-derive/src/lib.rs` | Proc-macro crate |
| `src/ffi/builders/value_type.rs` | ValueTypeBuilder |
| `src/ffi/builders/reference_type.rs` | ReferenceTypeBuilder |

---

## References

- [Rune Module API](https://github.com/rune-rs/rune/blob/main/crates/rune/src/module/module.rs)
- [Rune TypeOf trait](https://github.com/rune-rs/rune/blob/main/crates/rune/src/runtime/type_of.rs)
