# Task 03: Function Registration

**Status:** Not Started
**Depends On:** Task 01, Task 02
**Estimated Scope:** Function registration API with declaration string parsing

---

## Objective

Implement `Module.register_fn()` and `Module.register_fn_raw()` for registering native functions with declaration string parsing. All metadata (name, parameters, return type, default values) is parsed from AngelScript declaration strings.

## Files to Create/Modify

- `src/ffi/module.rs` - Add `register_fn()` and `register_fn_raw()` methods

## API Design

```rust
impl<'app> Module<'app> {
    /// Register a type-safe native function with declaration string
    ///
    /// Declaration format: "ReturnType name(params)"
    /// Examples:
    ///   - "float sqrt(float x)"
    ///   - "void print(const string &in s)"
    ///   - "int max(int a, int b)"
    ///   - "void log(const string &in msg, int level = 0)"
    pub fn register_fn<F, Args, Ret>(
        &mut self,
        decl: &str,
        f: F,
    ) -> Result<(), FfiRegistrationError>
    where
        F: IntoNativeFn<Args, Ret>;

    /// Register a raw/generic native function with declaration string
    ///
    /// For functions with ?& parameters or complex type handling
    pub fn register_fn_raw<F>(
        &mut self,
        decl: &str,
        f: F,
    ) -> Result<(), FfiRegistrationError>
    where
        F: NativeCallable + Send + Sync + 'static;
}
```

## AST Reuse Strategy

**Key Design Decision:** We reuse the existing AST parser infrastructure rather than creating a separate parsing system. This means:

1. **Same AST types** - `TypeExpr`, `Ident`, `FunctionParam` from `src/ast/`
2. **Same parser** - Leverage existing `Parser` with new entry points for partial parsing
3. **Same arena allocation** - Module owns a `Bump` arena, parsed nodes live there
4. **Consistent semantics** - Declaration strings follow exact AngelScript syntax

### Parser Analysis

Looking at the existing parser (`src/ast/decl_parser.rs`), the `parse_function_or_global_var` method handles function signatures but requires semicolons (line 372 for forward declarations, line 418 for global variables).

### Refactoring Required

The `parse_function_or_global_var` method needs internal refactoring to extract reusable signature parsing:

**Key constraint:** Normal script parsing must not regress - semicolons are still required where the language expects them.

```rust
// src/ast/decl_parser.rs (refactoring)
impl<'ast> Parser<'ast> {
    // New internal method - parses signature without terminator
    fn parse_function_signature_inner(&mut self) -> Result<FunctionSignatureData<'ast>, ParseError> {
        // Parse return type (reuses existing parse_return_type)
        let return_type = self.parse_return_type()?;

        // Parse name
        let name = self.parse_ident()?;

        // Parse parameters (reuses existing parse_function_params)
        let params = self.parse_function_params()?;

        // Parse const modifier
        let is_const = self.eat(TokenKind::Const).is_some();

        // Parse function attributes (reuses existing parse_func_attrs)
        let attrs = self.parse_func_attrs()?;

        Ok(FunctionSignatureData {
            return_type,
            name,
            params,
            is_const,
            attrs,
        })
    }

    // Existing parse_function_or_global_var continues to call
    // parse_function_signature_inner() + handle body/semicolon

    // FFI entry point - accepts EOF, no semicolon
    pub fn parse_ffi_function_signature(&mut self) -> Result<FunctionSignatureData<'ast>, ParseError> {
        let sig = self.parse_function_signature_inner()?;
        self.expect_eof()?;
        Ok(sig)
    }

    /// Helper: expect end of input for FFI parsing
    fn expect_eof(&mut self) -> Result<(), ParseError> {
        if !self.is_eof() {
            return Err(ParseError::new(
                ParseErrorKind::UnexpectedToken,
                self.peek().span,
                "expected end of declaration",
            ));
        }
        Ok(())
    }
}
```

### Parsed Result Types

```rust
// These wrap existing AST types
pub struct FunctionSignature<'ast> {
    pub return_type: &'ast TypeExpr<'ast>,
    pub name: &'ast Ident<'ast>,
    pub params: Vec<&'ast FunctionParam<'ast>>,
    pub is_const: bool,  // For methods: "void foo() const"
}

pub struct PropertyDecl<'ast> {
    pub type_expr: &'ast TypeExpr<'ast>,
    pub name: &'ast Ident<'ast>,
    pub is_const: bool,
}

pub struct TypeDecl<'ast> {
    pub name: &'ast Ident<'ast>,
    pub template_params: Option<Vec<&'ast Ident<'ast>>>,  // ["T"] or ["K", "V"]
}
```

### Module's Arena

```rust
pub struct Module<'app> {
    // ...
    arena: Bump,  // Owns parsed AST nodes
}

impl<'app> Module<'app> {
    fn parse_function_decl(&self, decl: &str) -> Result<FunctionSignature<'_>, FfiRegistrationError> {
        let lexer = Lexer::new(decl, "ffi");
        let mut parser = Parser::new(lexer, &self.arena);
        parser.parse_ffi_function_signature()  // Uses FFI entry point (accepts EOF)
            .map_err(|e| FfiRegistrationError::ParseError {
                decl: decl.to_string(),
                error: e.to_string()
            })
    }
}
```

## Usage Examples

**Type-Safe Registration:**
```rust
// Simple functions
module.register_fn("float sqrt(float x)", |x: f32| x.sqrt())?;
module.register_fn("void print(const string &in s)", |s: &str| println!("{}", s))?;
module.register_fn("int max(int a, int b)", |a: i32, b: i32| a.max(b))?;

// With default arguments (parsed from declaration)
module.register_fn("void log(const string &in msg, int level = 0)", log_fn)?;

// Multiple defaults
module.register_fn("string format(const string &in s, int width = 10, bool pad = true)", format_fn)?;
```

**Raw/Generic Registration:**
```rust
// For ?& (variable type) parameters
module.register_fn_raw("string format(const string &in fmt, ?&in value)", |ctx| {
    let fmt: &str = ctx.arg(0)?;
    let value = ctx.arg_any(1)?;
    let result = match value {
        AnyRef::Int(n) => format!("{}: {}", fmt, n),
        AnyRef::Float(f) => format!("{}: {}", fmt, f),
        AnyRef::String(s) => format!("{}: {}", fmt, s),
        _ => format!("{}: <unknown>", fmt),
    };
    ctx.set_return(result)?;
    Ok(())
})?;

// For output parameters
module.register_fn_raw("bool parse(const string &in s, int &out result)", |ctx| {
    let s: &str = ctx.arg(0)?;
    match s.parse::<i32>() {
        Ok(n) => {
            ctx.set_arg(1, n)?;
            ctx.set_return(true)?;
        }
        Err(_) => {
            ctx.set_return(false)?;
        }
    }
    Ok(())
})?;
```

## Internal Storage

Functions are stored in Module using parsed AST types (arena-allocated):

```rust
pub(crate) struct NativeFunctionDef<'ast> {
    pub name: &'ast Ident<'ast>,
    pub params: Vec<&'ast FunctionParam<'ast>>,
    pub return_type: &'ast TypeExpr<'ast>,
    pub native_fn: NativeFn,
}
```

## Error Handling

`FfiRegistrationError` includes parse error variants:

```rust
pub enum FfiRegistrationError {
    /// Failed to parse declaration string
    ParseError { decl: String, error: String },
    /// Type mismatch between declaration and closure
    TypeMismatch { expected: String, got: String },
    /// Duplicate function name in same namespace
    DuplicateName { name: String },
    /// Other registration errors...
}
```

## Implementation Notes

1. Module owns a `Bump` arena for storing parsed AST nodes
2. Declaration string is parsed once at registration time
3. Parse errors are returned immediately (fail-fast)
4. The `IntoNativeFn` trait bridges closure types to `NativeFn`
5. Raw functions bypass type inference entirely

## Acceptance Criteria

- [ ] `register_fn` works with type-safe closures
- [ ] `register_fn_raw` works with `CallContext` callbacks
- [ ] Declaration strings are parsed correctly
- [ ] Default arguments are parsed from declarations
- [ ] Parse errors return descriptive `FfiRegistrationError`
- [ ] Variable type parameters (`?&in`, `?&out`) work in raw functions
- [ ] Functions stored with arena-allocated AST types
