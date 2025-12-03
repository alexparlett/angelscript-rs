# Task 08f: String Module

**Status:** Not Started
**Parent:** Task 08 - Built-in Modules
**Depends On:** 08a (ScriptString), 08d (modules structure)

---

## Objective

Register the `string` type via FFI using the ClassBuilder API. This connects the ScriptString runtime type to the AngelScript type system.

## Files to Create/Modify

- `src/modules/string.rs` - String type registration (new)
- Update `src/modules/mod.rs` - Add export

## Implementation

### src/modules/string.rs

```rust
//! String type registration.
//!
//! Registers the built-in `string` type with all its methods and operators.

use crate::ffi::{Module, FfiRegistrationError};
use crate::runtime::ScriptString;

/// Creates the string module with the string type and global functions.
pub fn string_module<'app>() -> Result<Module<'app>, FfiRegistrationError> {
    let mut module = Module::root();

    // =========================================================================
    // STRING TYPE REGISTRATION
    // =========================================================================

    module.register_type::<ScriptString>("string")
        .value_type()

        // =====================================================================
        // CONSTRUCTORS
        // =====================================================================

        .constructor("void f()", ScriptString::new)?
        .constructor("void f(const string &in)", ScriptString::clone)?
        .constructor("void f(uint8 c)", |c: u8| ScriptString::from(char::from(c)))?
        .constructor("void f(uint8 c, uint count)", |c: u8, count: u32| {
            ScriptString::from_char_repeated(c, count)
        })?

        // =====================================================================
        // OPERATORS
        // =====================================================================

        // Assignment
        .operator("string& opAssign(const string &in)", ScriptString::assign)?

        // Concatenation
        .operator("string opAdd(const string &in)", ScriptString::concat)?
        .operator("string opAdd_r(const string &in)", ScriptString::concat_r)?
        .operator("string& opAddAssign(const string &in)", ScriptString::push_str)?

        // Comparison
        .operator("bool opEquals(const string &in)", ScriptString::eq)?
        .operator("int opCmp(const string &in)", ScriptString::cmp)?

        // Index access
        .operator("uint8 opIndex(uint)", ScriptString::byte_at)?
        .operator("uint8& opIndex(uint)", ScriptString::byte_at_mut)?

        // =====================================================================
        // SIZE AND CAPACITY
        // =====================================================================

        .method("uint length() const", ScriptString::len)?
        .method("bool isEmpty() const", ScriptString::is_empty)?
        .method("uint capacity() const", ScriptString::capacity)?
        .method("void reserve(uint)", ScriptString::reserve)?
        .method("void shrinkToFit()", ScriptString::shrink_to_fit)?
        .method("void clear()", ScriptString::clear)?
        .method("void resize(uint)", ScriptString::resize)?

        // =====================================================================
        // SUBSTRING AND SLICING
        // =====================================================================

        .method("string substr(uint start = 0, int count = -1) const", ScriptString::substr)?
        .method("string slice(uint start, uint end) const", ScriptString::slice)?
        .method("string sliceFrom(uint start) const", ScriptString::slice_from)?
        .method("string sliceTo(uint end) const", ScriptString::slice_to)?

        // =====================================================================
        // SEARCH - SUBSTRING
        // =====================================================================

        .method("int findFirst(const string &in, uint start = 0) const", ScriptString::find_first)?
        .method("int findLast(const string &in, int start = -1) const", ScriptString::find_last)?

        // =====================================================================
        // SEARCH - CHARACTER SETS
        // =====================================================================

        .method("int findFirstOf(const string &in chars, uint start = 0) const", ScriptString::find_first_of)?
        .method("int findFirstNotOf(const string &in chars, uint start = 0) const", ScriptString::find_first_not_of)?
        .method("int findLastOf(const string &in chars, int start = -1) const", ScriptString::find_last_of)?
        .method("int findLastNotOf(const string &in chars, int start = -1) const", ScriptString::find_last_not_of)?

        // =====================================================================
        // MODIFICATION
        // =====================================================================

        .method("void insert(uint pos, const string &in)", ScriptString::insert)?
        .method("void erase(uint pos, int count = -1)", ScriptString::erase)?
        .method("void push(uint8 c)", ScriptString::push)?
        .method("uint8 pop()", ScriptString::pop)?
        .method("void truncate(uint len)", ScriptString::truncate)?
        .method("void replace(uint start, uint end, const string &in)", ScriptString::replace_range)?

        // =====================================================================
        // CASE CONVERSION
        // =====================================================================

        .method("string toLower() const", ScriptString::to_lowercase)?
        .method("string toUpper() const", ScriptString::to_uppercase)?
        .method("string toAsciiLower() const", ScriptString::to_ascii_lowercase)?
        .method("string toAsciiUpper() const", ScriptString::to_ascii_uppercase)?

        // =====================================================================
        // TRIMMING
        // =====================================================================

        .method("string trim() const", ScriptString::trim)?
        .method("string trimStart() const", ScriptString::trim_start)?
        .method("string trimEnd() const", ScriptString::trim_end)?
        .method("string trimMatches(const string &in chars) const", ScriptString::trim_matches)?

        // =====================================================================
        // PREDICATES
        // =====================================================================

        .method("bool startsWith(const string &in) const", ScriptString::starts_with)?
        .method("bool endsWith(const string &in) const", ScriptString::ends_with)?
        .method("bool contains(const string &in) const", ScriptString::contains)?
        .method("bool isAscii() const", ScriptString::is_ascii)?
        .method("bool isAsciiAlphabetic() const", ScriptString::is_ascii_alphabetic)?
        .method("bool isAsciiAlphanumeric() const", ScriptString::is_ascii_alphanumeric)?
        .method("bool isAsciiDigit() const", ScriptString::is_ascii_digit)?
        .method("bool isAsciiHexdigit() const", ScriptString::is_ascii_hexdigit)?
        .method("bool isAsciiWhitespace() const", ScriptString::is_ascii_whitespace)?

        // =====================================================================
        // TRANSFORM
        // =====================================================================

        .method("string repeat(uint count) const", ScriptString::repeat)?
        .method("string replaceAll(const string &in from, const string &in to) const", ScriptString::replace_all)?
        .method("string replaceFirst(const string &in from, const string &in to) const", ScriptString::replace_first)?
        .method("string reversed() const", ScriptString::reversed)?
        .method("uint countOccurrences(const string &in) const", ScriptString::count_occurrences)?

        .build()?;

    // =========================================================================
    // GLOBAL STRING FUNCTIONS
    // =========================================================================

    // Parsing functions
    module.register_fn(
        "int64 parseInt(const string &in, uint base = 10)",
        parse_int
    )?;

    module.register_fn(
        "uint64 parseUInt(const string &in, uint base = 10)",
        parse_uint
    )?;

    module.register_fn(
        "double parseFloat(const string &in)",
        parse_float
    )?;

    // Formatting functions
    module.register_fn(
        "string formatInt(int64 val, const string &in options = \"\", uint width = 0)",
        format_int
    )?;

    module.register_fn(
        "string formatUInt(uint64 val, const string &in options = \"\", uint width = 0)",
        format_uint
    )?;

    module.register_fn(
        "string formatFloat(double val, const string &in options = \"\", uint width = 0, uint precision = 6)",
        format_float
    )?;

    Ok(module)
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn parse_int(s: &ScriptString, base: u32) -> i64 {
    i64::from_str_radix(s.as_str(), base).unwrap_or(0)
}

fn parse_uint(s: &ScriptString, base: u32) -> u64 {
    u64::from_str_radix(s.as_str(), base).unwrap_or(0)
}

fn parse_float(s: &ScriptString) -> f64 {
    s.as_str().parse().unwrap_or(0.0)
}

fn format_int(val: i64, options: &ScriptString, width: u32) -> ScriptString {
    let width = width as usize;
    let s = match options.as_str() {
        "x" | "X" => format!("{:x}", val),
        "o" => format!("{:o}", val),
        "b" => format!("{:b}", val),
        "+" => format!("{:+}", val),
        _ => format!("{}", val),
    };

    if width > 0 && s.len() < width {
        format!("{:>width$}", s).into()
    } else {
        s.into()
    }
}

fn format_uint(val: u64, options: &ScriptString, width: u32) -> ScriptString {
    let width = width as usize;
    let s = match options.as_str() {
        "x" | "X" => format!("{:x}", val),
        "o" => format!("{:o}", val),
        "b" => format!("{:b}", val),
        _ => format!("{}", val),
    };

    if width > 0 && s.len() < width {
        format!("{:>width$}", s).into()
    } else {
        s.into()
    }
}

fn format_float(val: f64, options: &ScriptString, width: u32, precision: u32) -> ScriptString {
    let width = width as usize;
    let precision = precision as usize;
    let s = match options.as_str() {
        "e" | "E" => format!("{:.precision$e}", val),
        "+" => format!("{:+.precision$}", val),
        _ => format!("{:.precision$}", val),
    };

    if width > 0 && s.len() < width {
        format!("{:>width$}", s).into()
    } else {
        s.into()
    }
}
```

## Type Registration Summary

### Constructors (4)
- Default constructor
- Copy constructor
- From single character
- From character repeated N times

### Operators (8)
- `opAssign` - Assignment
- `opAdd`, `opAdd_r` - Concatenation
- `opAddAssign` - Append
- `opEquals` - Equality
- `opCmp` - Comparison
- `opIndex` (const and mutable) - Byte access

### Methods (~35)
See 08a_runtime_string.md for full method list

### Global Functions (6)
- `parseInt`, `parseUInt`, `parseFloat` - Parsing
- `formatInt`, `formatUInt`, `formatFloat` - Formatting

## Default Parameter Handling

Note: Some methods have default parameters (e.g., `substr(uint start = 0, int count = -1)`).

If the FFI system doesn't support default parameters yet, we may need to:
1. Register multiple overloads
2. Or implement default parameter support in declaration parsing

```rust
// Option 1: Multiple overloads
.method("string substr() const", |s: &ScriptString| s.substr(0, -1))?
.method("string substr(uint start) const", |s: &ScriptString, start: u32| s.substr(start, -1))?
.method("string substr(uint start, int count) const", ScriptString::substr)?
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_module_builds() {
        let module = string_module().expect("string module should build");
        // Should have string type registered
        assert!(module.types().iter().any(|t| t.name() == "string"));
    }

    #[test]
    fn test_format_int() {
        assert_eq!(format_int(42, &"".into(), 0).as_str(), "42");
        assert_eq!(format_int(255, &"x".into(), 0).as_str(), "ff");
        assert_eq!(format_int(42, &"".into(), 5).as_str(), "   42");
    }

    #[test]
    fn test_parse_int() {
        assert_eq!(parse_int(&"42".into(), 10), 42);
        assert_eq!(parse_int(&"ff".into(), 16), 255);
        assert_eq!(parse_int(&"invalid".into(), 10), 0);
    }
}
```

## Acceptance Criteria

- [ ] `src/modules/string.rs` created
- [ ] String type registered as value type
- [ ] All constructors registered (4)
- [ ] All operators registered (8)
- [ ] All methods registered (~35)
- [ ] Global functions registered (6)
- [ ] Unit tests pass
- [ ] `cargo build --lib` succeeds
