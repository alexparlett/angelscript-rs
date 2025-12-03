//! String FFI registration.
//!
//! Registers global string parsing and formatting functions.
//!
//! # Global Functions
//!
//! Parse functions convert strings to numbers:
//! - `parseInt(const string &in s)` - Parse string to int64
//! - `parseInt(const string &in s, uint base)` - Parse with radix
//! - `parseUInt(const string &in s)` - Parse string to uint64
//! - `parseUInt(const string &in s, uint base)` - Parse with radix
//! - `parseFloat(const string &in s)` - Parse string to double
//!
//! Format functions convert numbers to strings:
//! - `formatInt(int64 val)` - Format integer to string
//! - `formatInt(int64 val, const string &in options)` - With format options
//! - `formatInt(int64 val, const string &in options, uint width)` - With width
//! - Similar for `formatUInt` and `formatFloat`
//!
//! Format options:
//! - `"x"` - Hexadecimal
//! - `"o"` - Octal
//! - `"b"` - Binary
//! - `"+"` - Show plus sign for positive numbers
//! - `"e"` - Scientific notation (for floats)

use crate::module::FfiModuleError;
use crate::Module;

/// Creates the string module with global parsing and formatting functions.
///
/// Functions are registered in the root namespace.
///
/// # Note
///
/// The `string` type itself is handled as a built-in type by the semantic
/// analysis system. This module provides utility functions for string/number
/// conversion.
///
/// # Example
///
/// ```ignore
/// use angelscript::modules::string_module;
///
/// let module = string_module().expect("failed to create string module");
/// // Register with engine...
/// ```
pub fn string_module<'app>() -> Result<Module<'app>, FfiModuleError> {
    let mut module = Module::root();

    // =========================================================================
    // PARSING FUNCTIONS
    // =========================================================================

    // parseInt - base 10
    module.register_fn("int64 parseInt(const string &in s)", |s: String| {
        s.trim().parse::<i64>().unwrap_or(0)
    })?;

    // parseInt - with radix
    module.register_fn(
        "int64 parseInt(const string &in s, uint base)",
        |s: String, base: u32| {
            let base = base.clamp(2, 36);
            i64::from_str_radix(s.trim(), base).unwrap_or(0)
        },
    )?;

    // parseUInt - base 10
    module.register_fn("uint64 parseUInt(const string &in s)", |s: String| {
        s.trim().parse::<u64>().unwrap_or(0)
    })?;

    // parseUInt - with radix
    module.register_fn(
        "uint64 parseUInt(const string &in s, uint base)",
        |s: String, base: u32| {
            let base = base.clamp(2, 36);
            u64::from_str_radix(s.trim(), base).unwrap_or(0)
        },
    )?;

    // parseFloat
    module.register_fn("double parseFloat(const string &in s)", |s: String| {
        s.trim().parse::<f64>().unwrap_or(0.0)
    })?;

    // =========================================================================
    // FORMATTING FUNCTIONS - INT
    // =========================================================================

    // formatInt - basic
    module.register_fn("string formatInt(int64 val)", |val: i64| format!("{}", val))?;

    // formatInt - with options
    module.register_fn(
        "string formatInt(int64 val, const string &in options)",
        |val: i64, options: String| format_int_impl(val, &options, 0),
    )?;

    // formatInt - with options and width
    module.register_fn(
        "string formatInt(int64 val, const string &in options, uint width)",
        |val: i64, options: String, width: u32| format_int_impl(val, &options, width),
    )?;

    // =========================================================================
    // FORMATTING FUNCTIONS - UINT
    // =========================================================================

    // formatUInt - basic
    module.register_fn("string formatUInt(uint64 val)", |val: u64| format!("{}", val))?;

    // formatUInt - with options
    module.register_fn(
        "string formatUInt(uint64 val, const string &in options)",
        |val: u64, options: String| format_uint_impl(val, &options, 0),
    )?;

    // formatUInt - with options and width
    module.register_fn(
        "string formatUInt(uint64 val, const string &in options, uint width)",
        |val: u64, options: String, width: u32| format_uint_impl(val, &options, width),
    )?;

    // =========================================================================
    // FORMATTING FUNCTIONS - FLOAT
    // =========================================================================

    // formatFloat - basic
    module.register_fn("string formatFloat(double val)", |val: f64| format!("{}", val))?;

    // formatFloat - with options (uses default precision of 6)
    module.register_fn(
        "string formatFloat(double val, const string &in options)",
        |val: f64, options: String| format_float_impl(val, &options, 0, 6),
    )?;

    // formatFloat - with options and width
    module.register_fn(
        "string formatFloat(double val, const string &in options, uint width)",
        |val: f64, options: String, width: u32| format_float_impl(val, &options, width, 6),
    )?;

    // formatFloat - with options, width, and precision
    module.register_fn(
        "string formatFloat(double val, const string &in options, uint width, uint precision)",
        |val: f64, options: String, width: u32, precision: u32| {
            format_float_impl(val, &options, width, precision)
        },
    )?;

    Ok(module)
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn format_int_impl(val: i64, options: &str, width: u32) -> String {
    let s = match options {
        "x" | "X" => format!("{:x}", val),
        "o" => format!("{:o}", val),
        "b" => format!("{:b}", val),
        "+" => format!("{:+}", val),
        _ => format!("{}", val),
    };

    if width > 0 && s.len() < width as usize {
        format!("{:>width$}", s, width = width as usize)
    } else {
        s
    }
}

fn format_uint_impl(val: u64, options: &str, width: u32) -> String {
    let s = match options {
        "x" | "X" => format!("{:x}", val),
        "o" => format!("{:o}", val),
        "b" => format!("{:b}", val),
        _ => format!("{}", val),
    };

    if width > 0 && s.len() < width as usize {
        format!("{:>width$}", s, width = width as usize)
    } else {
        s
    }
}

fn format_float_impl(val: f64, options: &str, width: u32, precision: u32) -> String {
    let s = match options {
        "e" | "E" => format!("{:.precision$e}", val, precision = precision as usize),
        "+" => format!("{:+.precision$}", val, precision = precision as usize),
        _ => format!("{:.precision$}", val, precision = precision as usize),
    };

    if width > 0 && s.len() < width as usize {
        format!("{:>width$}", s, width = width as usize)
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_module_creates_successfully() {
        let result = string_module();
        assert!(result.is_ok(), "string module should be created successfully");
    }

    #[test]
    fn test_string_module_is_root_namespace() {
        let module = string_module().expect("string module should build");
        assert!(module.is_root(), "string module should be in root namespace");
    }

    #[test]
    fn test_string_module_has_functions() {
        let module = string_module().expect("string module should build");
        // parse functions (5) + format functions (10)
        assert!(
            module.functions().len() >= 10,
            "string module should have functions, got {}",
            module.functions().len()
        );
    }

    #[test]
    fn test_parse_function_names() {
        let module = string_module().expect("string module should build");
        let fn_names: Vec<_> = module.functions().iter().map(|f| f.name.name).collect();

        assert!(fn_names.contains(&"parseInt"), "should have parseInt");
        assert!(fn_names.contains(&"parseUInt"), "should have parseUInt");
        assert!(fn_names.contains(&"parseFloat"), "should have parseFloat");
    }

    #[test]
    fn test_format_function_names() {
        let module = string_module().expect("string module should build");
        let fn_names: Vec<_> = module.functions().iter().map(|f| f.name.name).collect();

        assert!(fn_names.contains(&"formatInt"), "should have formatInt");
        assert!(fn_names.contains(&"formatUInt"), "should have formatUInt");
        assert!(fn_names.contains(&"formatFloat"), "should have formatFloat");
    }

    // Format int tests
    #[test]
    fn test_format_int_decimal() {
        assert_eq!(format_int_impl(42, "", 0), "42");
        assert_eq!(format_int_impl(-42, "", 0), "-42");
        assert_eq!(format_int_impl(0, "", 0), "0");
    }

    #[test]
    fn test_format_int_hex() {
        assert_eq!(format_int_impl(255, "x", 0), "ff");
        assert_eq!(format_int_impl(16, "x", 0), "10");
        assert_eq!(format_int_impl(0, "x", 0), "0");
    }

    #[test]
    fn test_format_int_octal() {
        assert_eq!(format_int_impl(8, "o", 0), "10");
        assert_eq!(format_int_impl(64, "o", 0), "100");
    }

    #[test]
    fn test_format_int_binary() {
        assert_eq!(format_int_impl(5, "b", 0), "101");
        assert_eq!(format_int_impl(8, "b", 0), "1000");
    }

    #[test]
    fn test_format_int_plus_sign() {
        assert_eq!(format_int_impl(42, "+", 0), "+42");
        assert_eq!(format_int_impl(-42, "+", 0), "-42");
        assert_eq!(format_int_impl(0, "+", 0), "+0");
    }

    #[test]
    fn test_format_int_width() {
        assert_eq!(format_int_impl(42, "", 5), "   42");
        assert_eq!(format_int_impl(12345, "", 5), "12345");
        assert_eq!(format_int_impl(123456, "", 5), "123456"); // No truncation
    }

    #[test]
    fn test_format_int_width_zero() {
        assert_eq!(format_int_impl(42, "", 0), "42");
    }

    // Format uint tests
    #[test]
    fn test_format_uint_decimal() {
        assert_eq!(format_uint_impl(42, "", 0), "42");
        assert_eq!(format_uint_impl(0, "", 0), "0");
    }

    #[test]
    fn test_format_uint_hex() {
        assert_eq!(format_uint_impl(255, "x", 0), "ff");
        assert_eq!(format_uint_impl(4096, "x", 0), "1000");
    }

    #[test]
    fn test_format_uint_octal() {
        assert_eq!(format_uint_impl(8, "o", 0), "10");
    }

    #[test]
    fn test_format_uint_binary() {
        assert_eq!(format_uint_impl(5, "b", 0), "101");
    }

    #[test]
    fn test_format_uint_width() {
        assert_eq!(format_uint_impl(42, "", 5), "   42");
    }

    // Format float tests
    #[test]
    fn test_format_float_default() {
        assert_eq!(format_float_impl(3.14159, "", 0, 2), "3.14");
        assert_eq!(format_float_impl(3.14159, "", 0, 4), "3.1416");
    }

    #[test]
    fn test_format_float_scientific() {
        let result = format_float_impl(1234.5, "e", 0, 2);
        assert!(result.contains('e'), "should use scientific notation: {}", result);
    }

    #[test]
    fn test_format_float_plus_sign() {
        assert_eq!(format_float_impl(3.14, "+", 0, 2), "+3.14");
        assert_eq!(format_float_impl(-3.14, "+", 0, 2), "-3.14");
    }

    #[test]
    fn test_format_float_width() {
        assert_eq!(format_float_impl(3.14, "", 10, 2), "      3.14");
    }

    #[test]
    fn test_format_float_precision() {
        assert_eq!(format_float_impl(3.14159265, "", 0, 0), "3");
        assert_eq!(format_float_impl(3.14159265, "", 0, 1), "3.1");
        assert_eq!(format_float_impl(3.14159265, "", 0, 6), "3.141593");
    }

    #[test]
    fn test_count_functions() {
        let module = string_module().expect("string module should build");
        println!("Total string functions: {}", module.functions().len());
        // 5 parse + 3 formatInt + 3 formatUInt + 4 formatFloat = 15
        assert_eq!(module.functions().len(), 15);
    }
}
