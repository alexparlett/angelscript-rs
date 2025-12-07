# Task 16: Garbage Collection and Weak Reference Behaviors

**Status:** Not Started
**Depends On:** Tasks 01-11
**Phase:** Advanced Features (post-migration)

---

## Objective

Add support for garbage collection and weak reference behaviors to handle cyclic references and weak reference semantics for reference-counted objects.

## Background

AngelScript provides optional GC support for handling cyclic references between objects, and weak reference support for non-owning references.

### Garbage Collection Behaviors

These behaviors allow the engine to detect and break reference cycles:

```cpp
// Report current reference count
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_GETREFCOUNT,
    "int f()", asMETHOD(MyType, GetRefCount), asCALL_THISCALL);

// Set GC flag (mark as potentially part of a cycle)
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_SETGCFLAG,
    "void f()", asMETHOD(MyType, SetGCFlag), asCALL_THISCALL);

// Get GC flag
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_GETGCFLAG,
    "bool f()", asMETHOD(MyType, GetGCFlag), asCALL_THISCALL);

// Enumerate all references held by this object
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_ENUMREFS,
    "void f(int&in)", asFUNCTION(MyType_EnumRefs), asCALL_CDECL_OBJLAST);

// Release all references (break cycles during collection)
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_RELEASEREFS,
    "void f(int&in)", asFUNCTION(MyType_ReleaseRefs), asCALL_CDECL_OBJLAST);
```

### Weak Reference Behavior

For `weakref<T>` support:

```cpp
// Return shared weak reference flag object
r = engine->RegisterObjectBehaviour("MyType", asBEHAVE_GET_WEAKREF_FLAG,
    "int &f()", asMETHOD(MyType, GetWeakRefFlag), asCALL_THISCALL);
```

## Design

### ClassBuilder Methods

```rust
module.register_type::<MyType>("MyType")
    .reference_type()
    .addref(MyType::add_ref)?
    .release(MyType::release)?
    // GC behaviors
    .gc_getrefcount(MyType::get_ref_count)?
    .gc_setflag(MyType::set_gc_flag)?
    .gc_getflag(MyType::get_gc_flag)?
    .gc_enumrefs(my_type_enum_refs)?
    .gc_releaserefs(my_type_release_refs)?
    // Weak reference behavior
    .get_weakref_flag(MyType::get_weakref_flag)?
    .build()?;
```

### Behaviors Struct Extension

```rust
pub struct Behaviors {
    // Existing
    pub factory: Option<NativeFn>,
    pub addref: Option<NativeFn>,
    pub release: Option<NativeFn>,
    pub construct: Option<NativeFn>,
    pub destruct: Option<NativeFn>,
    pub copy_construct: Option<NativeFn>,
    pub assign: Option<NativeFn>,
    pub list_construct: Option<ListBehavior>,
    pub list_factory: Option<ListBehavior>,

    // GC behaviors (NEW)
    pub gc_getrefcount: Option<NativeFn>,
    pub gc_setflag: Option<NativeFn>,
    pub gc_getflag: Option<NativeFn>,
    pub gc_enumrefs: Option<NativeFn>,
    pub gc_releaserefs: Option<NativeFn>,

    // Weak reference (NEW)
    pub get_weakref_flag: Option<NativeFn>,
}
```

### GC Callback Context

For `enumrefs` and `releaserefs`, the native function receives a GC context:

```rust
fn my_type_enum_refs(this: &MyType, gc: &mut GcContext) {
    // Report all references this object holds
    if let Some(ref child) = this.child {
        gc.enum_ref(child);
    }
    if let Some(ref other) = this.other {
        gc.enum_ref(other);
    }
}

fn my_type_release_refs(this: &mut MyType, gc: &GcContext) {
    // Release references to break cycles
    this.child = None;
    this.other = None;
}
```

### Weak Reference Flag

The weak reference flag is a shared object that tracks whether the target is still alive:

```rust
pub struct WeakRefFlag {
    alive: AtomicBool,
    ref_count: AtomicUsize,
}

impl MyType {
    fn get_weakref_flag(&self) -> &WeakRefFlag {
        &self.weak_flag
    }
}
```

## Implementation Steps

1. **Behaviors**: Add GC and weakref fields to `Behaviors` struct
2. **ClassBuilder**: Add builder methods for all GC and weakref behaviors
3. **GcContext**: Create type for enumrefs/releaserefs callbacks
4. **WeakRefFlag**: Create shared weak reference flag type
5. **Apply**: Register GC and weakref behaviors in Registry
6. **Tests**: Cover GC cycle detection and weak reference semantics

## Files to Modify

- `src/ffi/types.rs` - Extend Behaviors struct
- `src/ffi/class_builder.rs` - Add GC and weakref builder methods
- `src/ffi/gc.rs` - NEW: GcContext type
- `src/ffi/weakref.rs` - NEW: WeakRefFlag type
- `src/ffi/apply.rs` - Handle GC and weakref behavior registration

## Acceptance Criteria

- [ ] `Behaviors` struct has all GC and weakref fields
- [ ] `ClassBuilder` has methods: `gc_getrefcount()`, `gc_setflag()`, `gc_getflag()`, `gc_enumrefs()`, `gc_releaserefs()`
- [ ] `ClassBuilder::get_weakref_flag()` registers weak reference behavior
- [ ] `GcContext` provides `enum_ref()` method for reporting references
- [ ] `WeakRefFlag` type for shared weak reference state
- [ ] Tests cover GC behavior registration
- [ ] Tests cover weak reference flag behavior

## Script Usage

```angelscript
// Weak references
class Node {
    Node@ next;
}

Node@ n = Node();
weakref<Node> w(n);
@n = null;  // Node destroyed
if (w.get() is null) {
    // Weak reference correctly reports target gone
}

// GC handles cycles automatically
Node@ a = Node();
Node@ b = Node();
@a.next = b;
@b.next = a;  // Cycle created
@a = null;
@b = null;
// GC will detect and break the cycle
```

## Notes

- GC support is optional - types without GC behaviors use simple reference counting
- Only needed for types that can form reference cycles
- Weak references require the target type to implement `get_weakref_flag`
