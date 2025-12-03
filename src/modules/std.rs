//! Standard I/O functions.
//!
//! Provides print and println functions with overloads for common types.
//! Functions are registered in the global namespace.
//!
//! # Functions
//!
//! Each print function has overloads for: string, int, uint, float, double, bool
//!
//! | Function | Description |
//! |----------|-------------|
//! | `print(...)` | Print to stdout without newline |
//! | `println(...)` | Print to stdout with newline |
//! | `eprint(...)` | Print to stderr without newline |
//! | `eprintln(...)` | Print to stderr with newline |
//!
//! # Example
//!
//! ```angelscript
//! void main() {
//!     print("Value: ");
//!     println(42);
//!     println(3.14159);
//!     println(true);
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
/// Each print function has overloads for common types:
/// - string, int, uint, int64, uint64, float, double, bool
///
/// # Returns
///
/// A `Module` containing print, println, eprint, and eprintln functions
/// with overloads for multiple types.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::std_module;
///
/// let module = std_module().expect("failed to create std module");
/// // Register with engine...
/// ```
pub fn std_module<'app>() -> Result<Module<'app>, FfiModuleError> {
    let mut module = Module::root();

    // =========================================================================
    // PRINT - stdout without newline
    // =========================================================================

    module.register_fn("void print(const string &in s)", |s: String| {
        print!("{}", s);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(int val)", |v: i32| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(uint val)", |v: u32| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(int64 val)", |v: i64| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(uint64 val)", |v: u64| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(float val)", |v: f32| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(double val)", |v: f64| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    module.register_fn("void print(bool val)", |v: bool| {
        print!("{}", v);
        let _ = io::stdout().flush();
    })?;

    // =========================================================================
    // PRINTLN - stdout with newline
    // =========================================================================

    module.register_fn("void println(const string &in s)", |s: String| {
        println!("{}", s);
    })?;

    module.register_fn("void println(int val)", |v: i32| {
        println!("{}", v);
    })?;

    module.register_fn("void println(uint val)", |v: u32| {
        println!("{}", v);
    })?;

    module.register_fn("void println(int64 val)", |v: i64| {
        println!("{}", v);
    })?;

    module.register_fn("void println(uint64 val)", |v: u64| {
        println!("{}", v);
    })?;

    module.register_fn("void println(float val)", |v: f32| {
        println!("{}", v);
    })?;

    module.register_fn("void println(double val)", |v: f64| {
        println!("{}", v);
    })?;

    module.register_fn("void println(bool val)", |v: bool| {
        println!("{}", v);
    })?;

    // No-argument println for empty line
    module.register_fn("void println()", || {
        println!();
    })?;

    // =========================================================================
    // EPRINT - stderr without newline
    // =========================================================================

    module.register_fn("void eprint(const string &in s)", |s: String| {
        eprint!("{}", s);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(int val)", |v: i32| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(uint val)", |v: u32| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(int64 val)", |v: i64| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(uint64 val)", |v: u64| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(float val)", |v: f32| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(double val)", |v: f64| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    module.register_fn("void eprint(bool val)", |v: bool| {
        eprint!("{}", v);
        let _ = io::stderr().flush();
    })?;

    // =========================================================================
    // EPRINTLN - stderr with newline
    // =========================================================================

    module.register_fn("void eprintln(const string &in s)", |s: String| {
        eprintln!("{}", s);
    })?;

    module.register_fn("void eprintln(int val)", |v: i32| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(uint val)", |v: u32| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(int64 val)", |v: i64| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(uint64 val)", |v: u64| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(float val)", |v: f32| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(double val)", |v: f64| {
        eprintln!("{}", v);
    })?;

    module.register_fn("void eprintln(bool val)", |v: bool| {
        eprintln!("{}", v);
    })?;

    // No-argument eprintln for empty line
    module.register_fn("void eprintln()", || {
        eprintln!();
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
    fn test_std_module_function_count() {
        let module = std_module().expect("std module should build");
        // 8 print + 9 println + 8 eprint + 9 eprintln = 34
        assert_eq!(
            module.functions().len(),
            34,
            "std module should have 34 functions"
        );
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
    fn test_print_overload_count() {
        let module = std_module().expect("std module should build");
        let print_count = module
            .functions()
            .iter()
            .filter(|f| f.name.name == "print")
            .count();
        assert_eq!(print_count, 8, "should have 8 print overloads");
    }

    #[test]
    fn test_println_overload_count() {
        let module = std_module().expect("std module should build");
        let println_count = module
            .functions()
            .iter()
            .filter(|f| f.name.name == "println")
            .count();
        // 8 typed + 1 no-arg = 9
        assert_eq!(println_count, 9, "should have 9 println overloads");
    }

    #[test]
    fn test_all_functions_return_void() {
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
