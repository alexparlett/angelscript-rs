//! Standard I/O functions.
//!
//! Provides print, println, eprint, eprintln functions in the global namespace.
//! These are the simplest built-in functions for script output.
//!
//! # Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | `print(const string &in s)` | Print to stdout without newline |
//! | `println(const string &in s)` | Print to stdout with newline |
//! | `eprint(const string &in s)` | Print to stderr without newline |
//! | `eprintln(const string &in s)` | Print to stderr with newline |
//!
//! # Example
//!
//! ```angelscript
//! void main() {
//!     print("Hello ");
//!     println("World!");
//!     eprint("Error: ");
//!     eprintln("Something went wrong");
//! }
//! ```

use std::io::{self, Write};

use crate::module::FfiModuleError;
use crate::Module;

/// Creates the std module with I/O functions.
///
/// Functions are registered in the global (root) namespace, so scripts
/// can call them directly without a namespace prefix.
///
/// # Returns
///
/// A `Module` containing print, println, eprint, and eprintln functions.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::std_io::std_module;
///
/// let module = std_module().expect("failed to create std module");
/// // Register with engine...
/// ```
pub fn std_module<'app>() -> Result<Module<'app>, FfiModuleError> {
    let mut module = Module::root();

    // Print to stdout without newline
    module.register_fn("void print(const string &in s)", |s: String| {
        print!("{}", s);
        // Flush to ensure immediate output
        let _ = io::stdout().flush();
    })?;

    // Print to stdout with newline
    module.register_fn("void println(const string &in s)", |s: String| {
        println!("{}", s);
    })?;

    // Print to stderr without newline
    module.register_fn("void eprint(const string &in s)", |s: String| {
        eprint!("{}", s);
        // Flush to ensure immediate output
        let _ = io::stderr().flush();
    })?;

    // Print to stderr with newline
    module.register_fn("void eprintln(const string &in s)", |s: String| {
        eprintln!("{}", s);
    })?;

    Ok(module)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_module_creates_successfully() {
        let result = std_module();
        assert!(result.is_ok(), "std module should be created successfully");
    }

    #[test]
    fn test_std_module_has_four_functions() {
        let module = std_module().expect("std module should build");
        assert_eq!(module.functions().len(), 4, "std module should have 4 functions");
    }

    #[test]
    fn test_std_module_is_root_namespace() {
        let module = std_module().expect("std module should build");
        assert!(module.is_root(), "std module should be in root namespace");
    }

    #[test]
    fn test_std_module_function_names() {
        let module = std_module().expect("std module should build");
        let names: Vec<_> = module.functions().iter().map(|f| f.name.name).collect();

        assert!(names.contains(&"print"), "should have print function");
        assert!(names.contains(&"println"), "should have println function");
        assert!(names.contains(&"eprint"), "should have eprint function");
        assert!(names.contains(&"eprintln"), "should have eprintln function");
    }

    #[test]
    fn test_std_module_functions_take_string_param() {
        let module = std_module().expect("std module should build");

        for func in module.functions() {
            assert_eq!(
                func.params.len(),
                1,
                "function {} should take exactly 1 parameter",
                func.name.name
            );
        }
    }

    #[test]
    fn test_std_module_functions_return_void() {
        let module = std_module().expect("std module should build");

        for func in module.functions() {
            assert!(
                func.return_type.ty.is_void(),
                "function {} should return void",
                func.name.name
            );
        }
    }

    #[test]
    fn test_qualified_names_no_namespace() {
        let module = std_module().expect("std module should build");

        // Root namespace means no prefix
        assert_eq!(module.qualified_name("print"), "print");
        assert_eq!(module.qualified_name("println"), "println");
    }
}
