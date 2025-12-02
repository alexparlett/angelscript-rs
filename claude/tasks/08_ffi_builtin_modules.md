# Task 08: Built-in Modules

**Status:** Not Started
**Depends On:** Tasks 01-07
**Estimated Scope:** Implement standard library modules

---

## Objective

Implement the built-in modules (std, string, array, dictionary, math) using the FFI registration API, replacing the hardcoded implementations in registry.rs.

## Files to Create

- `src/modules/mod.rs` - Module exports and default_modules()
- `src/modules/std.rs` - print, println, eprint, eprintln
- `src/modules/string.rs` - String type with methods
- `src/modules/array.rs` - array<T> template
- `src/modules/dictionary.rs` - dictionary<K,V> template
- `src/modules/math.rs` - Math functions and constants

## Current Hardcoded Implementation

Located in `src/semantic/types/registry.rs` (~3500 lines), including:
- String type with ~20 methods (length, substr, findFirst, etc.)
- String operators (+, ==, !=, <, >, etc.)
- Array template with methods (length, resize, insertLast, etc.)
- Dictionary template

## New Implementation

```rust
// src/modules/string.rs
pub fn string() -> Module<'static> {
    let mut module = Module::root();
    module.register_type::<ScriptString>("string")
        .value_type()
        .constructor::<()>().native(|| ScriptString::new()).done()
        .constructor::<(&str,)>().native(|s| ScriptString::from(s)).done()
        .const_method("length", |s: &ScriptString| s.len() as u32)
        .const_method("substr", ScriptString::substr)
        .const_method("findFirst", ScriptString::find_first)
        .const_method("isEmpty", |s: &ScriptString| s.is_empty())
        .operator(OperatorBehavior::OpAdd)
            .param::<&str>().returns::<String>()
            .native(|a: &ScriptString, b: &str| a.concat(b)).done()
        .operator(OperatorBehavior::OpEquals)
            .param::<&str>().returns::<bool>()
            .native(|a: &ScriptString, b: &str| a.as_str() == b).done()
        .build();
    module
}

// src/modules/math.rs
pub fn math() -> Module<'static> {
    let mut module = Module::new(&["math"]);

    // Constants
    let mut pi = std::f64::consts::PI;
    module.register_global_property("const double PI", &mut pi)?;

    // Functions
    module.register_fn("sin", |x: f64| x.sin());
    module.register_fn("cos", |x: f64| x.cos());
    module.register_fn("sqrt", |x: f64| x.sqrt());
    module.register_fn("abs", |x: f64| x.abs());
    module.register_fn("floor", |x: f64| x.floor());
    module.register_fn("ceil", |x: f64| x.ceil());

    module
}
```

## Registry Cleanup

Remove from `src/semantic/types/registry.rs`:
- `register_builtin_string()` (~400 lines)
- `register_builtin_template()` for array/dictionary
- All hardcoded method/operator registration

## Acceptance Criteria

- [ ] All built-in types work through FFI registration
- [ ] Existing tests pass with new implementation
- [ ] Registry.rs reduced by ~800+ lines
- [ ] Context::with_default_modules() installs all built-ins
- [ ] Individual modules can be installed selectively
