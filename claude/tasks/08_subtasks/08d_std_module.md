# Task 08d: Std Module (I/O Functions)

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08a (ScriptString exists)

---

## Objective

Create the simplest built-in module: basic I/O functions (print, println, eprint, eprintln). This establishes the module pattern for subsequent modules.

## Files to Create

- `src/modules/mod.rs` - Module exports and `default_modules()`
- `src/modules/std.rs` - I/O functions

## Implementation

### src/modules/mod.rs

```rust
//! Built-in modules for AngelScript.
//!
//! These modules provide standard library functionality via FFI registration.

mod std_io;
mod math;
mod string;
mod array;
mod dictionary;

pub use std_io::std_module;
pub use math::math_module;
pub use string::string_module;
pub use array::array_module;
pub use dictionary::dictionary_module;

use crate::ffi::{Module, FfiRegistrationError};

/// Returns all default built-in modules.
///
/// These modules are automatically installed by `Context::new()`.
/// Use `Context::new_raw()` if you don't want built-in modules.
pub fn default_modules<'app>() -> Result<Vec<Module<'app>>, FfiRegistrationError> {
    Ok(vec![
        std_module()?,
        math_module()?,
        string_module()?,
        array_module()?,
        dictionary_module()?,
    ])
}
```

### src/modules/std.rs

```rust
//! Standard I/O functions.
//!
//! Provides print, println, eprint, eprintln functions in the global namespace.

use crate::ffi::{Module, FfiRegistrationError};

/// Creates the std module with I/O functions.
///
/// Functions are registered in the global (root) namespace.
pub fn std_module<'app>() -> Result<Module<'app>, FfiRegistrationError> {
    let mut module = Module::root();

    // Print to stdout without newline
    module.register_fn(
        "void print(const string &in s)",
        |s: &str| {
            print!("{}", s);
        }
    )?;

    // Print to stdout with newline
    module.register_fn(
        "void println(const string &in s)",
        |s: &str| {
            println!("{}", s);
        }
    )?;

    // Print to stderr without newline
    module.register_fn(
        "void eprint(const string &in s)",
        |s: &str| {
            eprint!("{}", s);
        }
    )?;

    // Print to stderr with newline
    module.register_fn(
        "void eprintln(const string &in s)",
        |s: &str| {
            eprintln!("{}", s);
        }
    )?;

    Ok(module)
}
```

## Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `void print(const string &in s)` | Print to stdout, no newline |
| `println` | `void println(const string &in s)` | Print to stdout with newline |
| `eprint` | `void eprint(const string &in s)` | Print to stderr, no newline |
| `eprintln` | `void eprintln(const string &in s)` | Print to stderr with newline |

## Update lib.rs

Add to `src/lib.rs`:
```rust
pub mod modules;
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_module_builds() {
        let module = std_module().expect("std module should build");
        // Module has 4 functions registered
        assert_eq!(module.functions().len(), 4);
    }

    #[test]
    fn test_default_modules() {
        let modules = default_modules().expect("default modules should build");
        // 5 modules: std, math, string, array, dictionary
        assert_eq!(modules.len(), 5);
    }
}
```

## Integration Test

Create a simple script test:
```angelscript
// test: std_io_test.as
void main() {
    print("Hello ");
    println("World!");
    eprint("Error: ");
    eprintln("Something went wrong");
}
```

## Notes

- Uses `&str` parameter type which `FromScript` trait should handle
- No return values - all functions return void
- Functions are side-effect only (I/O)
- In root namespace (no `Module::new(&["std"])` prefix)

## Acceptance Criteria

- [ ] `src/modules/mod.rs` created with exports
- [ ] `src/modules/std.rs` implements std_module()
- [ ] `default_modules()` returns all 5 modules (placeholder Ok for others initially)
- [ ] `src/lib.rs` exports `modules` module
- [ ] All 4 I/O functions registered
- [ ] Unit test passes
- [ ] `cargo build --lib` succeeds
