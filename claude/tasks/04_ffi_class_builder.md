# Task 04: Class Builder

**Status:** Not Started
**Depends On:** Task 01, Task 02, Task 03
**Estimated Scope:** Type registration API

---

## Objective

Implement ClassBuilder for registering native types (value types and reference types) with constructors, methods, properties, operators, and behaviors.

## Files to Create/Modify

- `src/ffi/class.rs` - ClassBuilder, MethodBuilder, PropertyBuilder, OperatorBuilder

## Key Types

```rust
pub struct ClassBuilder<'m, T: NativeType> {
    module: &'m mut Module,
    name: String,
    type_kind: TypeKind,
    constructors: Vec<ConstructorDef>,
    methods: Vec<MethodDef>,
    properties: Vec<PropertyDef>,
    operators: Vec<OperatorDef>,
    behaviors: Behaviors,
    _marker: PhantomData<T>,
}

pub enum TypeKind {
    Value { size: usize, align: usize, is_pod: bool },
    Reference { kind: ReferenceKind },
}

pub enum ReferenceKind {
    Standard,    // Full handle support with AddRef/Release
    Scoped,      // RAII-style, no handles (asOBJ_SCOPED)
    SingleRef,   // App-controlled lifetime (asOBJ_NOHANDLE)
    GenericHandle, // Type-erased container (asOBJ_ASHANDLE)
}

pub struct Behaviors {
    pub factory: Option<NativeFn>,
    pub addref: Option<NativeFn>,
    pub release: Option<NativeFn>,
    pub construct: Option<NativeFn>,
    pub destruct: Option<NativeFn>,
    pub copy_construct: Option<NativeFn>,
    pub assign: Option<NativeFn>,
}

impl<'m, T: NativeType> ClassBuilder<'m, T> {
    pub fn value_type(mut self) -> Self;
    pub fn reference_type(mut self) -> Self;
    pub fn factory<F>(mut self, f: F) -> Self;
    pub fn addref<F>(mut self, f: F) -> Self;
    pub fn release<F>(mut self, f: F) -> Self;
    pub fn constructor<Args>(mut self) -> ConstructorBuilder<'m, T, Args>;
    pub fn destructor<F>(mut self, f: F) -> Self;
    pub fn method<F, Args, Ret>(mut self, name: &str, f: F) -> Self;
    pub fn const_method<F, Args, Ret>(mut self, name: &str, f: F) -> Self;
    pub fn method_raw<F>(mut self, name: &str, f: F) -> MethodBuilder<'m, T>;
    pub fn property(mut self, name: &str) -> PropertyBuilder<'m, T>;
    pub fn operator(mut self, op: OperatorBehavior) -> OperatorBuilder<'m, T>;
    pub fn build(self);
}
```

## Implementation Notes

- Value types: size/alignment inferred from `size_of::<T>()` and `align_of::<T>()`
- Reference types require factory, addref, release behaviors
- Methods can be const or mutable
- Properties have getter and optional setter
- Operators map to AngelScript operator overloading

## Acceptance Criteria

- [ ] Value types can be registered with constructors
- [ ] Reference types can be registered with factory/addref/release
- [ ] Methods work with type-safe and raw conventions
- [ ] Properties with getters and setters work
- [ ] Operators can be registered
- [ ] Scoped and SingleRef variants work
