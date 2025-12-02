# Task 06: Template Builder

**Status:** Not Started
**Depends On:** Task 01, Task 02, Task 04
**Estimated Scope:** Template type registration

---

## Objective

Implement TemplateBuilder for registering generic template types like `array<T>` and `dictionary<K,V>`.

## Files to Create

- `src/ffi/template.rs` - TemplateBuilder, TemplateInstanceBuilder

## Key Types

```rust
pub struct TemplateInstanceInfo {
    pub template_name: String,
    pub sub_types: Vec<TypeSpec>,
}

pub struct TemplateValidation {
    pub is_valid: bool,
    pub error: Option<String>,
    pub needs_gc: bool,
}

pub struct TemplateBuilder<'m> {
    module: &'m mut Module,
    name: String,
    param_count: usize,
    validator: Option<Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,
    instance_builder: Option<Box<dyn Fn(&mut TemplateInstanceBuilder, &[TypeSpec]) + Send + Sync>>,
}

impl<'m> TemplateBuilder<'m> {
    pub fn params(mut self, count: usize) -> Self;
    pub fn validator<F>(mut self, f: F) -> Self;
    pub fn on_instantiate<F>(mut self, f: F) -> Self;
    pub fn build(self);
}

pub struct TemplateInstanceBuilder {
    methods: Vec<InstanceMethod>,
    operators: Vec<(OperatorBehavior, InstanceMethod)>,
    properties: Vec<InstanceProperty>,
}

impl TemplateInstanceBuilder {
    pub fn method(&mut self, name: &str) -> InstanceMethodBuilder<'_>;
    pub fn operator(&mut self, op: OperatorBehavior) -> InstanceOperatorBuilder<'_>;
    pub fn property(&mut self, name: &str) -> InstancePropertyBuilder<'_>;
}

/// Placeholder for template type parameter
pub struct SubType(pub usize);
```

## Usage Example

```rust
module.register_template("array")
    .params(1)
    .validator(|info| TemplateValidation::valid())
    .on_instantiate(|builder, sub_types| {
        builder.method("insertLast")
            .param_subtype(0)  // T
            .returns::<()>()
            .native(array_insert_last)
            .done();

        builder.method("length")
            .returns::<u32>()
            .is_const()
            .native(array_length)
            .done();

        builder.operator(OperatorBehavior::OpIndex)
            .param::<i32>()
            .returns_ref_subtype(0)  // &T
            .native(array_index)
            .done();
    })
    .build();
```

## Implementation Notes

- Template validation runs at compile time
- SubType(0), SubType(1) reference type parameters
- Specializations can be registered separately
- Factory receives hidden TypeInfo for subtype access

## Acceptance Criteria

- [ ] Single-parameter templates work (array<T>)
- [ ] Multi-parameter templates work (dictionary<K,V>)
- [ ] Validation callback rejects invalid instantiations
- [ ] SubType parameters resolve correctly
- [ ] Template specializations can override generic
