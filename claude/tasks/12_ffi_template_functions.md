# Task 12: Template Functions Support

**Status:** Not Started
**Depends On:** Tasks 01-11
**Phase:** Post-Migration

---

## Objective

Add support for template functions (functions with their own type parameters) to the FFI system. This leverages the existing `register_fn_raw` and `method_raw` APIs with the generic calling convention.

## Background

AngelScript supports template functions like:
```cpp
engine->RegisterGlobalFunction("T Test<T, U>(T t, U u)", asFUNCTION(fn), asCALL_GENERIC);
```

These differ from template types - they're standalone functions with type parameters that are resolved at each call site.

## Design

### No New API Surface

Template functions will use the existing `register_fn_raw` and `method_raw` APIs:

```rust
// Global template function
module.register_fn_raw("T max<T>(T a, T b)", |ctx: &mut CallContext| {
    // Use arg_any() and type_info() to handle any type
    let type_id = ctx.arg_type_id(0)?;
    match ctx.type_info(type_id)?.kind {
        TypeKind::Int => {
            let a: i64 = ctx.arg(0)?;
            let b: i64 = ctx.arg(1)?;
            ctx.set_return(a.max(b))?;
        }
        TypeKind::Float => {
            let a: f64 = ctx.arg(0)?;
            let b: f64 = ctx.arg(1)?;
            ctx.set_return(a.max(b))?;
        }
        _ => return Err(NativeError::UnsupportedType),
    }
    Ok(())
})?;

// Template method on a class
module.register_type::<Container>("Container")
    .reference_type()
    .method_raw("T get<T>(int index) const", |ctx| {
        let this: &Container = ctx.this()?;
        let index: i32 = ctx.arg(0)?;
        // Return type T based on what was requested
        // ...
        Ok(())
    })?
    .build()?;
```

### Parser Changes

Extend the function signature parser to recognize template parameters on functions:

```rust
// In parse_ffi_function_signature():
// 1. Parse return type (may reference template params like T)
// 2. Parse function name
// 3. If '<' follows name, parse template parameter list: <class T, class U>
// 4. Parse parameter list (may reference template params)
```

The template parameters are stored in `NativeFunctionDef`:

```rust
pub struct NativeFunctionDef<'ast> {
    pub id: FunctionId,
    pub name: Ident<'ast>,
    pub template_params: Option<&'ast [Ident<'ast>]>,  // NEW: ["T", "U"] for template functions
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
    pub is_const: bool,
    pub native_fn: NativeFn,
}
```

### CallContext Extensions

Add methods to support template function implementation:

```rust
impl<'vm> CallContext<'vm> {
    /// Get the TypeId of argument at index (for template functions)
    pub fn arg_type_id(&self, index: usize) -> Result<TypeId, NativeError>;

    /// Get the instantiated template type arguments
    /// For `max<int>(a, b)` call, returns [TypeId for int]
    pub fn template_args(&self) -> &[TypeId];
}
```

### Semantic Analysis

During semantic analysis, when a template function is called:
1. Infer type arguments from the call site arguments (or use explicit `<Type>` syntax)
2. Validate that all uses of each template parameter are consistent
3. Store the instantiated types for runtime access via `ctx.template_args()`

## Implementation Steps

1. **Parser**: Extend `parse_ffi_function_signature` to handle `<class T, ...>` after function name
2. **Storage**: Add `template_params` field to `NativeFunctionDef` and `NativeMethodDef`
3. **Apply**: Update `apply_to_registry` to register template functions with their type parameters
4. **CallContext**: Add `arg_type_id()` and `template_args()` methods
5. **Semantic Analysis**: Handle template function instantiation and type inference
6. **Tests**: Add tests for template function registration and calls

## Files to Modify

- `src/ast/decl_parser.rs` - Extend function signature parsing for template params
- `src/ffi/native_fn.rs` - Add template_args to CallContext
- `src/ffi/types.rs` - Add template_params to NativeFunctionDef/NativeMethodDef
- `src/ffi/apply.rs` - Handle template function registration
- `src/semantic/` - Template function instantiation during type checking

## Acceptance Criteria

- [ ] Parser handles `<class T, ...>` after function name in declarations
- [ ] `template_params` field added to `NativeFunctionDef` and `NativeMethodDef`
- [ ] `CallContext::arg_type_id()` returns TypeId for argument
- [ ] `CallContext::template_args()` returns instantiated type arguments
- [ ] Template functions registered in Registry with type parameters
- [ ] Semantic analysis infers/validates template arguments at call sites
- [ ] Tests cover registration, parsing, and runtime type access

## Script Usage

```angelscript
// Explicit type arguments
int x = max<int>(5, 10);
float f = max<float>(3.14, 2.71);

// Inferred from arguments (if supported)
int y = max(5, 10);  // Infers T = int
```
