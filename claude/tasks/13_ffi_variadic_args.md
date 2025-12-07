# Task 13: Variadic Function Arguments

**Status:** Not Started
**Depends On:** Tasks 01-11
**Phase:** Post-Migration

---

## Objective

Add support for variadic function arguments (functions accepting variable number of arguments) to the FFI system. This leverages the existing `register_fn_raw` and `method_raw` APIs with the generic calling convention.

## Background

AngelScript supports variadic functions like:
```cpp
engine->RegisterGlobalFunction("void print(const string &in ...)", asFUNCTION(fn), asCALL_GENERIC);
```

Key characteristics:
- `...` suffix on the last parameter type indicates variadic
- All variadic arguments must be the same type
- Compiler adds hidden argument count, accessed via `GetArgCount()`
- Requires generic calling convention

## Design

### No New API Surface

Variadic functions will use the existing `register_fn_raw` and `method_raw` APIs:

```rust
// Variadic global function
module.register_fn_raw("void print(const string &in ...)", |ctx: &mut CallContext| {
    let count = ctx.arg_count();  // Number of variadic args
    for i in 0..count {
        let s: &str = ctx.arg(i)?;
        print!("{}", s);
    }
    Ok(())
})?;

// Variadic with ?&in for any type
module.register_fn_raw("string format(const string &in fmt, const ?&in ...)", |ctx: &mut CallContext| {
    let fmt: &str = ctx.arg(0)?;
    let var_count = ctx.variadic_count();  // Count of variadic args only
    for i in 0..var_count {
        let value = ctx.variadic_arg_any(i)?;  // Access variadic args
        // ... format based on type
    }
    Ok(())
})?;

// Variadic method
module.register_type::<Logger>("Logger")
    .reference_type()
    .method_raw("void log(int level, const string &in ...)", |ctx| {
        let this: &Logger = ctx.this()?;
        let level: i32 = ctx.arg(0)?;
        let msg_count = ctx.variadic_count();
        // ...
        Ok(())
    })?
    .build()?;
```

### Parser Changes

Extend the function parameter parser to recognize `...` suffix:

```rust
// In parse_function_params():
// 1. Parse parameters normally
// 2. If last parameter type ends with `...`, mark as variadic
// 3. Only one variadic parameter allowed, must be last
```

The variadic flag is stored in `NativeFunctionDef`:

```rust
pub struct NativeFunctionDef<'ast> {
    pub id: FunctionId,
    pub name: Ident<'ast>,
    pub template_params: Option<&'ast [Ident<'ast>]>,
    pub params: &'ast [FunctionParam<'ast>],
    pub return_type: ReturnType<'ast>,
    pub is_const: bool,
    pub is_variadic: bool,  // NEW: true if last param has ...
    pub native_fn: NativeFn,
}
```

### CallContext Extensions

Add methods to support variadic function implementation:

```rust
impl<'vm> CallContext<'vm> {
    /// Get total argument count (includes variadic args)
    pub fn arg_count(&self) -> usize;

    /// Get count of variadic arguments only
    /// For `format(fmt, ...)` called as `format("x", a, b, c)`, returns 3
    pub fn variadic_count(&self) -> usize;

    /// Get variadic argument by index (0-based within variadic args)
    pub fn variadic_arg<T: FromScript>(&self, index: usize) -> Result<T, NativeError>;

    /// Get variadic argument as any type
    pub fn variadic_arg_any(&self, index: usize) -> Result<AnyRef<'_>, NativeError>;
}
```

### Semantic Analysis

During semantic analysis, when a variadic function is called:
1. Validate fixed parameters normally
2. Validate all variadic arguments match the variadic parameter type
3. Store argument count for runtime access

## Implementation Steps

1. **Parser**: Extend parameter parsing to recognize `...` suffix
2. **Storage**: Add `is_variadic` field to `NativeFunctionDef` and `NativeMethodDef`
3. **Apply**: Update `apply_to_registry` to register variadic functions
4. **CallContext**: Add `arg_count()`, `variadic_count()`, `variadic_arg()`, `variadic_arg_any()`
5. **Semantic Analysis**: Handle variadic argument validation
6. **Tests**: Add tests for variadic function registration and calls

## Files to Modify

- `src/ast/decl_parser.rs` - Extend parameter parsing for `...`
- `src/ffi/native_fn.rs` - Add variadic methods to CallContext
- `src/ffi/types.rs` - Add is_variadic to NativeFunctionDef/NativeMethodDef
- `src/ffi/apply.rs` - Handle variadic function registration
- `src/semantic/` - Variadic argument validation during type checking

## Acceptance Criteria

- [ ] Parser handles `type ...` suffix on last parameter
- [ ] `is_variadic` field added to `NativeFunctionDef` and `NativeMethodDef`
- [ ] `CallContext::arg_count()` returns total argument count
- [ ] `CallContext::variadic_count()` returns variadic-only count
- [ ] `CallContext::variadic_arg()` and `variadic_arg_any()` access variadic args
- [ ] Semantic analysis validates variadic arguments match declared type
- [ ] Tests cover registration, parsing, and runtime argument access

## Script Usage

```angelscript
// Call variadic functions
print("hello", "world", "!");  // 3 variadic string args

// Mixed fixed and variadic
string result = format("Value: %d, Name: %s", 42, "test");

// With ?&in for any type
log(INFO, "user=", userId, " action=", action);
```
