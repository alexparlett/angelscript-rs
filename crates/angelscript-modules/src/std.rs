//! Standard I/O functions.
//!
//! Provides print and println functions using the generic calling convention.
//! Functions are registered in the global namespace.
//!
//! # Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | `print(msg, val)` | Print formatted message with value to stdout (no newline) |
//! | `println(msg, val)` | Print formatted message with value to stdout (with newline) |
//! | `println()` | Print empty line to stdout |
//! | `eprint(msg, val)` | Print formatted message with value to stderr (no newline) |
//! | `eprintln(msg, val)` | Print formatted message with value to stderr (with newline) |
//! | `eprintln()` | Print empty line to stderr |
//!
//! The `?&in` parameter type accepts any value type (int, float, bool, string, etc.)
//! and formats it appropriately.
//!
//! # Example
//!
//! ```angelscript
//! void main() {
//!     println("Value: ", 42);
//!     println("Pi: ", 3.14159);
//!     println("Enabled: ", true);
//!     println();  // empty line
//! }
//! ```

use std::io::{self, Write};

use angelscript_ffi::{CallContext, VmSlot};
use angelscript_module::{ModuleError, Module};

/// Format a VmSlot value as a string.
fn format_slot(slot: &VmSlot) -> String {
    match slot {
        VmSlot::Void => String::new(),
        VmSlot::Int(v) => v.to_string(),
        VmSlot::Float(v) => v.to_string(),
        VmSlot::Bool(v) => v.to_string(),
        VmSlot::String(s) => s.clone(),
        VmSlot::Object(_) => "[object]".to_string(),
        VmSlot::Native(_) => "[native]".to_string(),
        VmSlot::NullHandle => "null".to_string(),
    }
}

/// Creates the std module with I/O functions.
///
/// Functions are registered in the global (root) namespace, so scripts
/// can call them directly without a namespace prefix.
///
/// Print functions use the generic calling convention with `?&in` to accept
/// any value type. When variadics are implemented, these can be extended
/// to accept multiple parameters.
///
/// # Returns
///
/// A `Module` containing print, println, eprint, and eprintln functions.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::std_module;
///
/// let module = std_module().expect("failed to create std module");
/// // Register with engine...
/// ```
pub fn std_module<'app>() -> Result<Module<'app>, ModuleError> {
    let mut module = Module::root();

    // =========================================================================
    // PRINT - stdout without newline
    // =========================================================================

    // print(msg) - print string message
    module.register_fn_raw(
        "void print(const string &in msg)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            print!("{}", msg);
            Ok(())
        },
    )?;

    // print(val) - print any value directly
    module.register_fn_raw(
        "void print(?&in val)",
        |ctx: &mut CallContext| {
            let val = ctx.arg_slot(0)?;
            print!("{}", format_slot(val));
            Ok(())
        },
    )?;

    // print(msg, val) - print message followed by any value
    module.register_fn_raw(
        "void print(const string &in msg, ?&in val)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            let val = ctx.arg_slot(1)?;
            print!("{}{}", msg, format_slot(val));
            Ok(())
        },
    )?;

    // =========================================================================
    // PRINTLN - stdout with newline
    // =========================================================================

    // println(msg) - print message followed by any value, with newline
    module.register_fn_raw(
        "void println(const string &in msg)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            println!("{}", msg);
            Ok(())
        },
    )?;

    // println(msg, val) - print message followed by any value, with newline
    module.register_fn_raw(
        "void println(const string &in msg, ?&in val)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            let val = ctx.arg_slot(1)?;
            println!("{}{}", msg, format_slot(val));
            Ok(())
        },
    )?;

    // println() - empty line
    module.register_fn("void println()", || {
        println!();
    })?;

    // =========================================================================
    // EPRINT - stderr without newline
    // =========================================================================

    // eprint(msg, val) - print message followed by any value to stderr
    module.register_fn_raw(
        "void eprint(const string &in msg, ?&in val)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            let val = ctx.arg_slot(1)?;
            eprint!("{}{}", msg, format_slot(val));
            let _ = io::stderr().flush();
            Ok(())
        },
    )?;

    // =========================================================================
    // EPRINTLN - stderr with newline
    // =========================================================================

    // eprintln(msg, val) - print message followed by any value to stderr, with newline
    module.register_fn_raw(
        "void eprintln(const string &in msg, ?&in val)",
        |ctx: &mut CallContext| {
            let msg: String = ctx.arg(0)?;
            let val = ctx.arg_slot(1)?;
            eprintln!("{}{}", msg, format_slot(val));
            Ok(())
        },
    )?;

    // eprintln() - empty line to stderr
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
        // print(msg) + print(val) + print(msg, val) + println(msg) + println(msg, val) + println() + eprint(msg, val) + eprintln(msg, val) + eprintln() = 9
        assert_eq!(
            module.functions().len(),
            9,
            "std module should have 9 functions"
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
        let names: Vec<_> = module.functions().iter().map(|f| f.0.name.as_str()).collect();

        assert!(names.contains(&"print"), "should have print function");
        assert!(names.contains(&"println"), "should have println function");
        assert!(names.contains(&"eprint"), "should have eprint function");
        assert!(names.contains(&"eprintln"), "should have eprintln function");
    }

    #[test]
    fn test_print_function_count() {
        let module = std_module().expect("std module should build");
        let print_count = module
            .functions()
            .iter()
            .filter(|f| f.0.name.as_str() == "print")
            .count();
        // print(msg) + print(val) + print(msg, val) = 3
        assert_eq!(print_count, 3, "should have 3 print functions");
    }

    #[test]
    fn test_println_function_count() {
        let module = std_module().expect("std module should build");
        let println_count = module
            .functions()
            .iter()
            .filter(|f| f.0.name.as_str() == "println")
            .count();
        // println(msg) + println(msg, val) + println() = 3
        assert_eq!(println_count, 3, "should have 3 println functions");
    }

    #[test]
    fn test_all_functions_return_void() {
        let module = std_module().expect("std module should build");

        for (func, _native_fn) in module.functions() {
            assert!(
                func.return_type.is_void(),
                "function {} should return void",
                func.name
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

    #[test]
    fn test_format_slot() {
        assert_eq!(format_slot(&VmSlot::Void), "");
        assert_eq!(format_slot(&VmSlot::Int(42)), "42");
        assert_eq!(format_slot(&VmSlot::Float(3.14)), "3.14");
        assert_eq!(format_slot(&VmSlot::Bool(true)), "true");
        assert_eq!(format_slot(&VmSlot::String("hello".to_string())), "hello");
        assert_eq!(format_slot(&VmSlot::NullHandle), "null");
    }
}
