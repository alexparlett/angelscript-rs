# Task 05: Enum, Interface, and Funcdef Builders

**Status:** Not Started
**Depends On:** Task 01, Task 02
**Estimated Scope:** Additional type registration

---

## Objective

Implement builders for enums, interfaces, and funcdefs.

## Files to Create

- `src/ffi/enum_builder.rs` - EnumBuilder
- `src/ffi/interface.rs` - InterfaceBuilder
- `src/ffi/funcdef.rs` - FuncdefBuilder

## Key Types

```rust
// Enum Builder
pub struct EnumBuilder<'m> {
    module: &'m mut Module,
    name: String,
    values: Vec<(String, i64)>,
}

impl<'m> EnumBuilder<'m> {
    pub fn value(mut self, name: &str, val: i64) -> Self;
    pub fn values(mut self, names: &[&str]) -> Self;  // Auto-increment from 0
    pub fn build(self);
}

// Interface Builder
pub struct InterfaceBuilder<'m> {
    module: &'m mut Module,
    name: String,
    methods: Vec<MethodSignature>,
}

impl<'m> InterfaceBuilder<'m> {
    pub fn method(&mut self, name: &str) -> InterfaceMethodBuilder<'_>;
    pub fn build(self);
}

pub struct InterfaceMethodBuilder<'i> {
    interface: &'i mut InterfaceBuilder<'_>,
    name: String,
    params: Vec<TypeSpec>,
    return_type: TypeSpec,
    is_const: bool,
}

// Funcdef Builder
pub struct FuncdefBuilder<'m> {
    module: &'m mut Module,
    name: String,
    params: Vec<TypeSpec>,
    return_type: TypeSpec,
}

impl<'m> FuncdefBuilder<'m> {
    pub fn param<T: FromScript>(mut self) -> Self;
    pub fn returns<T: ToScript>(mut self) -> Self;
    pub fn build(self);
}
```

## Usage Examples

```rust
// Enum
module.register_enum("Color")
    .value("Red", 0)
    .value("Green", 1)
    .value("Blue", 2)
    .build();

// Interface
module.register_interface("ISerializable")
    .method("serialize").returns::<String>().is_const().done()
    .method("deserialize").param::<&str>().done()
    .build();

// Funcdef
module.register_funcdef("Callback")
    .param::<i32>()
    .param::<&str>()
    .returns::<bool>()
    .build();
```

## Acceptance Criteria

- [ ] Enums can be registered with named values
- [ ] Interfaces can be registered with method signatures
- [ ] Funcdefs can be registered as function pointer types
- [ ] All work with the namespace system
