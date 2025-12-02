# Task 03: Function Builder

**Status:** Not Started
**Depends On:** Task 01, Task 02
**Estimated Scope:** Function registration API

---

## Objective

Implement the FunctionBuilder for registering native functions with both type-safe and raw/generic calling conventions.

## Files to Create/Modify

- `src/ffi/function.rs` - FunctionBuilder implementation

## Key Types

```rust
pub struct FunctionBuilder<'m> {
    module: &'m mut Module,
    name: String,
    params: Vec<ParamDef>,
    return_type: Option<TypeSpec>,
    native_fn: Option<NativeFn>,
    default_exprs: Vec<Option<String>>,
}

impl<'m> FunctionBuilder<'m> {
    pub fn with_signature(mut self, sig: &str) -> Self;
    pub fn param<T: FromScript>(mut self, name: &str) -> Self;
    pub fn param_ref<T: FromScript>(mut self, name: &str, modifier: RefModifier) -> Self;
    pub fn param_any_in(mut self, name: &str) -> Self;
    pub fn param_any_out(mut self, name: &str) -> Self;
    pub fn param_with_default<T: FromScript>(mut self, name: &str, default_expr: &str) -> Self;
    pub fn returns<T: ToScript>(mut self) -> Self;
    pub fn native<F>(mut self, f: F) -> Self;
    pub fn build(self);
}
```

## Two Calling Conventions

**Type-Safe (High-Level):**
```rust
module.register_fn("sqrt", |x: f64| x.sqrt());
```

**Generic (Low-Level):**
```rust
module.register_fn_raw("format", |ctx: &mut CallContext| {
    let fmt: &str = ctx.arg::<&str>(0)?;
    let any_val = ctx.arg_any(1)?;
    ctx.set_return(result);
    Ok(())
}).param::<&str>("fmt").param_any_in("value").returns::<String>();
```

## Implementation Notes

- Type-safe functions infer signature from closure via `IntoNativeFn` trait
- Raw functions require explicit signature declaration
- Support for `?&in` and `?&out` variable type parameters
- Default arguments stored as expression strings (parsed during apply)

## Acceptance Criteria

- [ ] Type-safe function registration works with closures
- [ ] Raw function registration works with CallContext
- [ ] Variable type parameters (?&) work
- [ ] Default arguments can be specified
- [ ] Signature inference works for common types
