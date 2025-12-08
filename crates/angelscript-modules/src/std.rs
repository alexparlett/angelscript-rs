//! Standard utility functions for AngelScript.
//!
//! Provides basic I/O, formatting, and utility functions.

use angelscript_registry::Module;
use crate::string::ScriptString;

// =============================================================================
// OUTPUT FUNCTIONS
// =============================================================================

/// Print a string without newline.
#[angelscript_macros::function]
pub fn print(s: ScriptString) {
    print!("{}", s.as_str());
}

/// Print a string with newline.
#[angelscript_macros::function]
pub fn println(s: ScriptString) {
    println!("{}", s.as_str());
}

/// Print an integer.
#[angelscript_macros::function(name = "printi")]
pub fn print_int(n: i64) {
    print!("{}", n);
}

/// Print a float.
#[angelscript_macros::function(name = "printf")]
pub fn print_float(n: f64) {
    print!("{}", n);
}

/// Print a boolean.
#[angelscript_macros::function(name = "printb")]
pub fn print_bool(b: bool) {
    print!("{}", b);
}

// =============================================================================
// FORMAT CONVERSIONS (primitive to string)
// =============================================================================

/// Convert integer to string.
#[angelscript_macros::function(name = "toString")]
pub fn int_to_string(n: i64) -> ScriptString {
    ScriptString::from_str(&n.to_string())
}

/// Convert unsigned integer to string.
#[angelscript_macros::function(name = "toStringu")]
pub fn uint_to_string(n: u64) -> ScriptString {
    ScriptString::from_str(&n.to_string())
}

/// Convert float to string.
#[angelscript_macros::function(name = "toStringf")]
pub fn float_to_string(n: f64) -> ScriptString {
    ScriptString::from_str(&n.to_string())
}

/// Convert float to string with precision.
#[angelscript_macros::function(name = "toStringfp")]
pub fn float_to_string_precision(n: f64, precision: u32) -> ScriptString {
    ScriptString::from_str(&format!("{:.prec$}", n, prec = precision as usize))
}

/// Convert boolean to string ("true" or "false").
#[angelscript_macros::function(name = "toStringb")]
pub fn bool_to_string(b: bool) -> ScriptString {
    ScriptString::from_str(if b { "true" } else { "false" })
}

/// Convert integer to hexadecimal string.
#[angelscript_macros::function(name = "toHex")]
pub fn int_to_hex(n: i64) -> ScriptString {
    ScriptString::from_str(&format!("{:x}", n))
}

/// Convert integer to uppercase hexadecimal string.
#[angelscript_macros::function(name = "toHexUpper")]
pub fn int_to_hex_upper(n: i64) -> ScriptString {
    ScriptString::from_str(&format!("{:X}", n))
}

/// Convert integer to binary string.
#[angelscript_macros::function(name = "toBinary")]
pub fn int_to_binary(n: i64) -> ScriptString {
    ScriptString::from_str(&format!("{:b}", n))
}

/// Convert integer to octal string.
#[angelscript_macros::function(name = "toOctal")]
pub fn int_to_octal(n: i64) -> ScriptString {
    ScriptString::from_str(&format!("{:o}", n))
}

// =============================================================================
// PARSE FUNCTIONS (string to primitive)
// =============================================================================

/// Parse string to integer. Returns 0 on failure.
#[angelscript_macros::function(name = "parseInt")]
pub fn parse_int(s: ScriptString) -> i64 {
    s.as_str().trim().parse().unwrap_or(0)
}

/// Parse string to unsigned integer. Returns 0 on failure.
#[angelscript_macros::function(name = "parseUint")]
pub fn parse_uint(s: ScriptString) -> u64 {
    s.as_str().trim().parse().unwrap_or(0)
}

/// Parse string to float. Returns 0.0 on failure.
#[angelscript_macros::function(name = "parseFloat")]
pub fn parse_float(s: ScriptString) -> f64 {
    s.as_str().trim().parse().unwrap_or(0.0)
}

/// Parse string to boolean. Returns true for "true", "1", "yes", false otherwise.
#[angelscript_macros::function(name = "parseBool")]
pub fn parse_bool(s: ScriptString) -> bool {
    let s = s.as_str().trim().to_lowercase();
    matches!(s.as_str(), "true" | "1" | "yes" | "on")
}

/// Parse hexadecimal string to integer. Returns 0 on failure.
#[angelscript_macros::function(name = "parseHex")]
pub fn parse_hex(s: ScriptString) -> i64 {
    let s = s.as_str().trim();
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    i64::from_str_radix(s, 16).unwrap_or(0)
}

/// Parse binary string to integer. Returns 0 on failure.
#[angelscript_macros::function(name = "parseBinary")]
pub fn parse_binary(s: ScriptString) -> i64 {
    let s = s.as_str().trim();
    let s = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")).unwrap_or(s);
    i64::from_str_radix(s, 2).unwrap_or(0)
}

// =============================================================================
// ASSERTION AND DEBUG
// =============================================================================

/// Assert that a condition is true. Panics if false.
#[angelscript_macros::function(name = "assert")]
pub fn assert_true(condition: bool) {
    assert!(condition, "Assertion failed");
}

/// Assert that a condition is true with a custom message.
#[angelscript_macros::function(name = "assertMsg")]
pub fn assert_with_message(condition: bool, message: ScriptString) {
    assert!(condition, "{}", message.as_str());
}

/// Debug print - prints to stderr.
#[angelscript_macros::function(name = "debug")]
pub fn debug_print(s: ScriptString) {
    eprintln!("[DEBUG] {}", s.as_str());
}

// =============================================================================
// TYPE INFO (runtime type inspection)
// =============================================================================

/// Get the type name of an integer.
#[angelscript_macros::function(name = "typeNamei")]
pub fn type_name_int(_: i64) -> ScriptString {
    ScriptString::from_str("int64")
}

/// Get the type name of a float.
#[angelscript_macros::function(name = "typeNamef")]
pub fn type_name_float(_: f64) -> ScriptString {
    ScriptString::from_str("double")
}

/// Get the type name of a boolean.
#[angelscript_macros::function(name = "typeNameb")]
pub fn type_name_bool(_: bool) -> ScriptString {
    ScriptString::from_str("bool")
}

/// Get the type name of a string.
#[angelscript_macros::function(name = "typeNames")]
pub fn type_name_string(_: ScriptString) -> ScriptString {
    ScriptString::from_str("string")
}

// =============================================================================
// RANGE CHECKS
// =============================================================================

/// Check if value is within range [min, max] (inclusive).
#[angelscript_macros::function(name = "inRange")]
pub fn in_range(x: f64, min_val: f64, max_val: f64) -> bool {
    x >= min_val && x <= max_val
}

/// Check if integer value is within range [min, max] (inclusive).
#[angelscript_macros::function(name = "inRangei")]
pub fn in_range_int(x: i64, min_val: i64, max_val: i64) -> bool {
    x >= min_val && x <= max_val
}

// =============================================================================
// BIT MANIPULATION
// =============================================================================

/// Count leading zeros.
#[angelscript_macros::function(name = "clz")]
pub fn count_leading_zeros(n: u64) -> u32 {
    n.leading_zeros()
}

/// Count trailing zeros.
#[angelscript_macros::function(name = "ctz")]
pub fn count_trailing_zeros(n: u64) -> u32 {
    n.trailing_zeros()
}

/// Count number of ones (population count).
#[angelscript_macros::function(name = "popcount")]
pub fn population_count(n: u64) -> u32 {
    n.count_ones()
}

/// Rotate bits left.
#[angelscript_macros::function(name = "rotl")]
pub fn rotate_left(n: u64, count: u32) -> u64 {
    n.rotate_left(count)
}

/// Rotate bits right.
#[angelscript_macros::function(name = "rotr")]
pub fn rotate_right(n: u64, count: u32) -> u64 {
    n.rotate_right(count)
}

/// Reverse bits.
#[angelscript_macros::function(name = "reverseBits")]
pub fn reverse_bits(n: u64) -> u64 {
    n.reverse_bits()
}

/// Swap bytes (endianness conversion).
#[angelscript_macros::function(name = "swapBytes")]
pub fn swap_bytes(n: u64) -> u64 {
    n.swap_bytes()
}

// =============================================================================
// MODULE CREATION
// =============================================================================

/// Creates the std module with utility functions.
pub fn module() -> Module {
    Module::new()
        // Output
        .function_meta(print__meta)
        .function_meta(println__meta)
        .function_meta(print_int__meta)
        .function_meta(print_float__meta)
        .function_meta(print_bool__meta)
        // Format conversions
        .function_meta(int_to_string__meta)
        .function_meta(uint_to_string__meta)
        .function_meta(float_to_string__meta)
        .function_meta(float_to_string_precision__meta)
        .function_meta(bool_to_string__meta)
        .function_meta(int_to_hex__meta)
        .function_meta(int_to_hex_upper__meta)
        .function_meta(int_to_binary__meta)
        .function_meta(int_to_octal__meta)
        // Parse functions
        .function_meta(parse_int__meta)
        .function_meta(parse_uint__meta)
        .function_meta(parse_float__meta)
        .function_meta(parse_bool__meta)
        .function_meta(parse_hex__meta)
        .function_meta(parse_binary__meta)
        // Assertion and debug
        .function_meta(assert_true__meta)
        .function_meta(assert_with_message__meta)
        .function_meta(debug_print__meta)
        // Type info
        .function_meta(type_name_int__meta)
        .function_meta(type_name_float__meta)
        .function_meta(type_name_bool__meta)
        .function_meta(type_name_string__meta)
        // Range checks
        .function_meta(in_range__meta)
        .function_meta(in_range_int__meta)
        // Bit manipulation
        .function_meta(count_leading_zeros__meta)
        .function_meta(count_trailing_zeros__meta)
        .function_meta(population_count__meta)
        .function_meta(rotate_left__meta)
        .function_meta(rotate_right__meta)
        .function_meta(reverse_bits__meta)
        .function_meta(swap_bytes__meta)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_to_string() {
        assert_eq!(int_to_string(42).as_str(), "42");
        assert_eq!(int_to_string(-123).as_str(), "-123");
        assert_eq!(int_to_string(0).as_str(), "0");
    }

    #[test]
    fn test_float_to_string() {
        assert_eq!(float_to_string(3.14).as_str(), "3.14");
        assert_eq!(float_to_string_precision(3.14159, 2).as_str(), "3.14");
    }

    #[test]
    fn test_bool_to_string() {
        assert_eq!(bool_to_string(true).as_str(), "true");
        assert_eq!(bool_to_string(false).as_str(), "false");
    }

    #[test]
    fn test_int_to_hex() {
        assert_eq!(int_to_hex(255).as_str(), "ff");
        assert_eq!(int_to_hex_upper(255).as_str(), "FF");
        assert_eq!(int_to_hex(16).as_str(), "10");
    }

    #[test]
    fn test_int_to_binary() {
        assert_eq!(int_to_binary(5).as_str(), "101");
        assert_eq!(int_to_binary(8).as_str(), "1000");
    }

    #[test]
    fn test_parse_int() {
        assert_eq!(parse_int("42".into()), 42);
        assert_eq!(parse_int("-123".into()), -123);
        assert_eq!(parse_int("  42  ".into()), 42);
        assert_eq!(parse_int("invalid".into()), 0);
    }

    #[test]
    fn test_parse_float() {
        assert!((parse_float("3.14".into()) - 3.14).abs() < f64::EPSILON);
        assert!((parse_float("invalid".into())).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true".into()));
        assert!(parse_bool("1".into()));
        assert!(parse_bool("yes".into()));
        assert!(parse_bool("on".into()));
        assert!(!parse_bool("false".into()));
        assert!(!parse_bool("0".into()));
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("ff".into()), 255);
        assert_eq!(parse_hex("0xFF".into()), 255);
        assert_eq!(parse_hex("10".into()), 16);
    }

    #[test]
    fn test_parse_binary() {
        assert_eq!(parse_binary("101".into()), 5);
        assert_eq!(parse_binary("0b1000".into()), 8);
    }

    #[test]
    fn test_in_range() {
        assert!(in_range(5.0, 0.0, 10.0));
        assert!(!in_range(15.0, 0.0, 10.0));
        assert!(in_range(0.0, 0.0, 10.0));
        assert!(in_range(10.0, 0.0, 10.0));
    }

    #[test]
    fn test_bit_ops() {
        assert_eq!(count_leading_zeros(1), 63);
        assert_eq!(count_trailing_zeros(8), 3);
        assert_eq!(population_count(0b1010101), 4);
    }

    #[test]
    fn test_module_creates() {
        let m = module();
        assert!(m.namespace.is_empty());
        assert!(!m.functions.is_empty());
    }
}
