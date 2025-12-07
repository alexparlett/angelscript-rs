# Task 18: Generic Handle (ref) Add-on

**Status:** Not Started
**Depends On:** Tasks 01-08, Task 13 (variable parameter type)
**Phase:** Integration

---

## Objective

Implement the `ref` type as a built-in add-on - a generic container that can hold any handle type and provides type-safe casting.

## Background

The `ref` type is a type-erased handle container. It can hold a handle to any reference type and allows dynamic casting back to specific types. This is useful for:
- Generic storage classes
- Event/messaging systems
- Deferred type resolution

**References:**
- https://angelcode.com/angelscript/sdk/docs/manual/doc_adv_generic_handle.html
- https://angelcode.com/angelscript/sdk/docs/manual/doc_addon_handle.html

## Script Usage

```angelscript
class Foo { int value; }
class Bar { string name; }

void main() {
    Foo@ foo = Foo();
    foo.value = 42;

    // Store in generic ref
    ref@ r = foo;

    // Cast back to specific type
    Foo@ foo2 = cast<Foo>(r);
    if (foo2 !is null) {
        print(foo2.value);  // 42
    }

    // Wrong cast returns null
    Bar@ bar = cast<Bar>(r);
    // bar is null

    // Comparison
    ref@ r2 = foo;
    if (r is r2) { ... }  // true - same object
}
```

## Design

### Rust Implementation

Uses `ObjectHandle` (generational index) not raw pointers:

```rust
/// Type-erased handle container
pub struct ScriptRef {
    /// Handle to the referenced object (None if empty)
    handle: Option<ObjectHandle>,
}

// ObjectHandle already contains:
// - index: u32 (into ObjectHeap.slots)
// - generation: u32 (for use-after-free detection)
// - type_id: TypeId (for runtime type verification)
```

### Type Registration

```rust
pub fn ref_addon() -> Result<Module<'static>, FfiModuleError> {
    let mut module = Module::root();

    module.register_type::<ScriptRef>("ref")
        .value_type()
        .as_handle()  // asOBJ_ASHANDLE flag
        .constructor("void f()", ScriptRef::new)?
        .constructor("void f(const ?&in)", ScriptRef::from_any)?
        .method("ref& opHndlAssign(const ?&in)", ScriptRef::assign)?
        .method("bool opEquals(const ?&in) const", ScriptRef::equals)?
        .method("void opCast(?&out)", ScriptRef::cast)?
        .build()?;

    Ok(module)
}
```

### Key Features

1. **Safe storage**: Uses `ObjectHandle` with generational indices
2. **Reference counting**: AddRef when storing, Release when releasing via ObjectHeap
3. **Dynamic casting**: `opCast` checks TypeId compatibility at runtime
4. **Handle semantics**: Despite being a value type internally, behaves like a handle in scripts

### Required FFI Additions

1. **`TypeKind::AsHandle`** - New variant for generic handle types
2. **`ClassBuilder::as_handle()`** - Method to set the asOBJ_ASHANDLE flag

## Implementation Steps

1. Add `TypeKind::AsHandle` variant to `src/ffi/types.rs`
2. Add `ClassBuilder::as_handle()` method
3. Implement `ScriptRef` struct in `src/modules/ref.rs`
4. Register via `ref_addon()` function
5. Add to default modules or as optional add-on
6. Tests

## Files to Create/Modify

- `src/ffi/types.rs` - Add `TypeKind::AsHandle`
- `src/ffi/class_builder.rs` - Add `as_handle()` method
- `src/modules/ref.rs` - Implement `ScriptRef` and registration
- `src/modules/mod.rs` - Export ref module

## Dependencies

Requires `?&` (variable parameter type) support from Task 13 for method declarations.

## Acceptance Criteria

- [ ] `TypeKind::AsHandle` variant exists
- [ ] `ClassBuilder::as_handle()` sets the flag
- [ ] `ScriptRef` stores any handle type via `ObjectHandle`
- [ ] `opHndlAssign` accepts assignment from any handle
- [ ] `opEquals` compares handle identity
- [ ] `opCast` performs dynamic type casting
- [ ] Reference counting is correct (AddRef/Release via ObjectHeap)
- [ ] Tests cover: store, retrieve, cast success, cast failure, comparison
