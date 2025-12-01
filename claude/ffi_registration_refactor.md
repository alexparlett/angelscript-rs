# FFI Registration Refactor: Parse-Based Native Function Registration

## Problem

Currently, native/built-in functions (like `string.findFirst`, `string.substr`, `array.length`, etc.) are registered manually in `src/semantic/types/registry.rs` using verbose Rust code:

```rust
let find_first_func_id = FunctionId::next();
let find_first_func_def = FunctionDef {
    id: find_first_func_id,
    name: "findFirst".to_string(),
    namespace: Vec::new(),
    params: vec![string_param.clone(), DataType::simple(self.uint32_type)],
    return_type: DataType::simple(self.int32_type),
    object_type: Some(type_id),
    traits: FunctionTraits { is_const: true, ..default_traits },
    is_native: true,
    default_args: Vec::new(),  // <-- Cannot support default args!
    visibility: Visibility::Public,
    signature_filled: true,
};
```

### Issues with Current Approach

1. **No default argument support**: `FunctionDef.default_args` expects `Vec<Option<&'ast Expr>>` (AST expression references), but `Registry::new()` runs before any arena exists, so we can't create AST expressions.

2. **Verbose and error-prone**: Each method requires ~20 lines of boilerplate Rust code.

3. **Hard to maintain**: Adding new built-in methods requires careful Rust code changes.

4. **Workaround required**: We currently use multiple overloads instead of default args (e.g., `findFirst(str)` and `findFirst(str, start)` instead of `findFirst(str, start = 0)`).

## Proposed Solution

Parse native function declarations as AngelScript code to get proper AST with default argument expressions.

### Approach 1: Builtin Declarations Source

Add a builtin declarations string that gets parsed alongside user code:

```rust
// In module.rs or a new builtins.rs
const BUILTIN_DECLARATIONS: &str = r#"
// String methods (native implementations)
class string {
    uint length() const;
    string substr(uint start, int count = -1) const;
    string substr(uint start) const;
    int findFirst(const string &in substr, uint start = 0) const;
    int findLast(const string &in substr, int start = -1) const;
    bool isEmpty() const;

    // Operators
    string opAdd(const string &in) const;
    string opAdd(int) const;
    string opAdd(float) const;
    string opAddAssign(const string &in);
    bool opEquals(const string &in) const;
    int opCmp(const string &in) const;
    int opIndex(uint) const;
}
"#;
```

### Approach 2: Arena Parameter to Registry

Pass the arena to `Registry::new()` and parse expressions on demand:

```rust
impl<'src, 'ast> Registry<'src, 'ast> {
    pub fn new(arena: &'ast Bump) -> Self {
        // Can now use parse_expression(arena, "0") for default args
    }
}
```

### Approach 3: Deferred Registration

Register builtin method signatures in a later pass (after arena creation):

```rust
// In Compiler::compile, after arena is available:
fn register_builtin_methods(registry: &mut Registry, arena: &Bump) {
    let default_zero = parse_expression("0", arena).unwrap();
    // Now register with proper default_args
}
```

## Recommended Implementation

**Approach 1** is cleanest because:
- Natural AngelScript syntax for declarations
- Default args work automatically through normal parsing
- Easy to read and maintain
- Can be extended for user-provided FFI declarations

### Implementation Steps

1. **Create builtin declarations source**
   - Add `BUILTIN_STRING_DECL`, `BUILTIN_ARRAY_DECL`, etc. as string constants
   - Use `class` syntax with method declarations (no bodies)

2. **Modify `ScriptModule::build()`**
   - Parse builtin declarations into the arena first
   - Merge builtin AST with user AST (or process separately)

3. **Mark functions as native**
   - In `Registrar`, detect functions from builtin source (or with `external` modifier)
   - Set `is_native: true` for these functions

4. **Remove manual registration**
   - Delete `register_builtin_string()`, `register_array_methods()`, etc.
   - Built-in types (primitives, string, array template) still need type registration

5. **Handle type bootstrapping**
   - `string` type must exist before parsing `class string { ... }`
   - Register the type shell first, then parse methods

## Files to Modify

- `src/semantic/types/registry.rs` - Remove manual method registrations
- `src/module.rs` - Add builtin declarations parsing
- `src/semantic/passes/registration.rs` - Handle native function detection
- New: `src/builtins.rs` or `src/semantic/builtins.rs` - Builtin declaration strings

## Benefits

1. **Default arguments work**: `findFirst(str, start = 0)` just works
2. **Less code**: ~500 lines of Rust → ~50 lines of AngelScript declarations
3. **Self-documenting**: Declaration syntax IS the documentation
4. **Extensible**: Same mechanism for user FFI registrations
5. **Consistent**: Same parsing/registration path for all functions

## Example: Before vs After

### Before (Rust)
```rust
// 3a. Register findFirst(const string &in): int
let find_first_func_id = FunctionId::next();
let find_first_func_def = FunctionDef { /* 15 fields */ };
self.functions.insert(find_first_func_id, find_first_func_def);
method_ids.push(find_first_func_id);

// 3b. Register findFirst(const string &in, uint start): int
let find_first2_func_id = FunctionId::next();
let find_first2_func_def = FunctionDef { /* 15 fields */ };
self.functions.insert(find_first2_func_id, find_first2_func_def);
method_ids.push(find_first2_func_id);
```

### After (AngelScript)
```angelscript
int findFirst(const string &in substr, uint start = 0) const;
```

## Related Work

- Current workaround: Multiple overloads instead of default args
- Test scripts use FFI placeholders like `float sqrt(float x) { return x; }`
- Eventually this system should support user-defined FFI bindings
