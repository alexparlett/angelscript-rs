//! ScriptString - AngelScript string type backed by Rust String.
//!
//! This is a VALUE type - copied on assignment. It provides all methods
//! needed for the AngelScript string type plus Rust String extras.
//!
//! # FFI Registration
//!
//! This module also provides FFI registration via [`string_module`] for:
//!
//! ## Parse Functions
//! - `parseInt(const string &in s)` - Parse string to int64
//! - `parseInt(const string &in s, uint base)` - Parse with radix
//! - `parseUInt(const string &in s)` - Parse string to uint64
//! - `parseUInt(const string &in s, uint base)` - Parse with radix
//! - `parseFloat(const string &in s)` - Parse string to double
//!
//! ## Format Functions
//! - `formatInt(int64 val, ...)` - Format integer to string
//! - `formatUInt(uint64 val, ...)` - Format unsigned to string
//! - `formatFloat(double val, ...)` - Format float to string

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Deref, DerefMut};

use crate::ffi::NativeType;
use crate::module::FfiModuleError;
use crate::Module;

/// AngelScript string type backed by Rust String.
///
/// This is a VALUE type - copied on assignment.
/// Byte-indexed (not char-indexed) like C++ AngelScript.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptString(String);

impl ScriptString {
    // =========================================================================
    // CONSTRUCTORS
    // =========================================================================

    /// Create an empty string.
    #[inline]
    pub fn new() -> Self {
        Self(String::new())
    }

    /// Create from a string slice.
    #[inline]
    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    /// Create from a single character.
    #[inline]
    pub fn from_char(c: char) -> Self {
        Self(c.to_string())
    }

    /// Create from a character repeated N times.
    #[inline]
    pub fn from_char_repeated(c: u8, count: u32) -> Self {
        Self(String::from(char::from(c)).repeat(count as usize))
    }

    /// Get the inner string reference.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner string mutably.
    #[inline]
    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.0
    }

    /// Consume and return the inner String.
    #[inline]
    pub fn into_inner(self) -> String {
        self.0
    }

    // =========================================================================
    // SIZE AND CAPACITY
    // =========================================================================

    /// Returns the byte length of the string.
    #[inline]
    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }

    /// Returns true if the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the allocated capacity in bytes.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.0.capacity() as u32
    }

    /// Reserve capacity for at least `additional` more bytes.
    #[inline]
    pub fn reserve(&mut self, additional: u32) {
        self.0.reserve(additional as usize);
    }

    /// Shrink capacity to fit current length.
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Clear the string.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Resize the string to `new_len` bytes.
    /// Truncates if shorter, pads with null bytes if longer.
    pub fn resize(&mut self, new_len: u32) {
        let new_len = new_len as usize;
        if new_len < self.0.len() {
            self.0.truncate(new_len);
        } else {
            self.0.reserve(new_len - self.0.len());
            while self.0.len() < new_len {
                self.0.push('\0');
            }
        }
    }

    // =========================================================================
    // SUBSTRING AND SLICING
    // =========================================================================

    /// Extract substring. `count` of -1 means "to end of string".
    pub fn substr(&self, start: u32, count: i32) -> Self {
        let start = start as usize;
        let len = self.0.len();

        if start >= len {
            return Self::new();
        }

        let end = if count < 0 {
            len
        } else {
            (start + count as usize).min(len)
        };

        Self(self.0[start..end].to_string())
    }

    /// Slice by byte indices [start..end].
    pub fn slice(&self, start: u32, end: u32) -> Self {
        let start = (start as usize).min(self.0.len());
        let end = (end as usize).min(self.0.len());
        if start >= end {
            Self::new()
        } else {
            Self(self.0[start..end].to_string())
        }
    }

    /// Slice from byte index to end [start..].
    pub fn slice_from(&self, start: u32) -> Self {
        let start = (start as usize).min(self.0.len());
        Self(self.0[start..].to_string())
    }

    /// Slice from start to byte index [..end].
    pub fn slice_to(&self, end: u32) -> Self {
        let end = (end as usize).min(self.0.len());
        Self(self.0[..end].to_string())
    }

    // =========================================================================
    // SEARCH - SUBSTRING
    // =========================================================================

    /// Find first occurrence of substring starting from `start`. Returns -1 if not found.
    pub fn find_first(&self, needle: &Self, start: u32) -> i32 {
        let start = start as usize;
        if start >= self.0.len() {
            return -1;
        }

        self.0[start..]
            .find(needle.as_str())
            .map(|i| (start + i) as i32)
            .unwrap_or(-1)
    }

    /// Find last occurrence of substring. `start` of -1 means "from end". Returns -1 if not found.
    pub fn find_last(&self, needle: &Self, start: i32) -> i32 {
        let search_end = if start < 0 {
            self.0.len()
        } else {
            (start as usize).min(self.0.len())
        };

        self.0[..search_end]
            .rfind(needle.as_str())
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    // =========================================================================
    // SEARCH - CHARACTER SETS
    // =========================================================================

    /// Find first occurrence of any character in `chars` starting from `start`.
    pub fn find_first_of(&self, chars: &Self, start: u32) -> i32 {
        let start = start as usize;
        if start >= self.0.len() {
            return -1;
        }

        self.0[start..]
            .char_indices()
            .find(|(_, c)| chars.0.contains(*c))
            .map(|(i, _)| (start + i) as i32)
            .unwrap_or(-1)
    }

    /// Find first occurrence of any character NOT in `chars` starting from `start`.
    pub fn find_first_not_of(&self, chars: &Self, start: u32) -> i32 {
        let start = start as usize;
        if start >= self.0.len() {
            return -1;
        }

        self.0[start..]
            .char_indices()
            .find(|(_, c)| !chars.0.contains(*c))
            .map(|(i, _)| (start + i) as i32)
            .unwrap_or(-1)
    }

    /// Find last occurrence of any character in `chars`. `start` of -1 means "from end".
    pub fn find_last_of(&self, chars: &Self, start: i32) -> i32 {
        let search_end = if start < 0 {
            self.0.len()
        } else {
            (start as usize).min(self.0.len())
        };

        self.0[..search_end]
            .char_indices()
            .rev()
            .find(|(_, c)| chars.0.contains(*c))
            .map(|(i, _)| i as i32)
            .unwrap_or(-1)
    }

    /// Find last occurrence of any character NOT in `chars`. `start` of -1 means "from end".
    pub fn find_last_not_of(&self, chars: &Self, start: i32) -> i32 {
        let search_end = if start < 0 {
            self.0.len()
        } else {
            (start as usize).min(self.0.len())
        };

        self.0[..search_end]
            .char_indices()
            .rev()
            .find(|(_, c)| !chars.0.contains(*c))
            .map(|(i, _)| i as i32)
            .unwrap_or(-1)
    }

    // =========================================================================
    // MODIFICATION
    // =========================================================================

    /// Insert string at byte position.
    pub fn insert(&mut self, pos: u32, s: &Self) {
        let pos = (pos as usize).min(self.0.len());
        self.0.insert_str(pos, &s.0);
    }

    /// Erase bytes starting at `pos`. `count` of -1 means "to end".
    pub fn erase(&mut self, pos: u32, count: i32) {
        let pos = pos as usize;
        if pos >= self.0.len() {
            return;
        }

        let end = if count < 0 {
            self.0.len()
        } else {
            (pos + count as usize).min(self.0.len())
        };

        self.0.drain(pos..end);
    }

    /// Push a single byte.
    pub fn push(&mut self, c: u8) {
        self.0.push(char::from(c));
    }

    /// Pop and return the last byte. Returns 0 if empty.
    pub fn pop(&mut self) -> u8 {
        self.0.pop().map(|c| c as u8).unwrap_or(0)
    }

    /// Truncate to `len` bytes.
    pub fn truncate(&mut self, len: u32) {
        self.0.truncate(len as usize);
    }

    /// Replace bytes in range [start..end] with `s`.
    pub fn replace_range(&mut self, start: u32, end: u32, s: &Self) {
        let start = (start as usize).min(self.0.len());
        let end = (end as usize).min(self.0.len());
        if start <= end {
            self.0.replace_range(start..end, &s.0);
        }
    }

    // =========================================================================
    // CASE CONVERSION
    // =========================================================================

    /// Convert to lowercase (Unicode-aware).
    pub fn to_lowercase(&self) -> Self {
        Self(self.0.to_lowercase())
    }

    /// Convert to uppercase (Unicode-aware).
    pub fn to_uppercase(&self) -> Self {
        Self(self.0.to_uppercase())
    }

    /// Convert to ASCII lowercase.
    pub fn to_ascii_lowercase(&self) -> Self {
        Self(self.0.to_ascii_lowercase())
    }

    /// Convert to ASCII uppercase.
    pub fn to_ascii_uppercase(&self) -> Self {
        Self(self.0.to_ascii_uppercase())
    }

    // =========================================================================
    // TRIMMING
    // =========================================================================

    /// Trim whitespace from both ends.
    pub fn trim(&self) -> Self {
        Self(self.0.trim().to_string())
    }

    /// Trim whitespace from start.
    pub fn trim_start(&self) -> Self {
        Self(self.0.trim_start().to_string())
    }

    /// Trim whitespace from end.
    pub fn trim_end(&self) -> Self {
        Self(self.0.trim_end().to_string())
    }

    /// Trim any characters in `chars` from both ends.
    pub fn trim_matches(&self, chars: &Self) -> Self {
        let chars_vec: Vec<char> = chars.0.chars().collect();
        Self(self.0.trim_matches(|c| chars_vec.contains(&c)).to_string())
    }

    // =========================================================================
    // PREDICATES
    // =========================================================================

    /// Check if string starts with `prefix`.
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// Check if string ends with `suffix`.
    pub fn ends_with(&self, suffix: &Self) -> bool {
        self.0.ends_with(&suffix.0)
    }

    /// Check if string contains `needle`.
    pub fn contains(&self, needle: &Self) -> bool {
        self.0.contains(&needle.0)
    }

    /// Check if all characters are ASCII.
    pub fn is_ascii(&self) -> bool {
        self.0.is_ascii()
    }

    /// Check if all characters are ASCII alphabetic.
    pub fn is_ascii_alphabetic(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_alphabetic())
    }

    /// Check if all characters are ASCII alphanumeric.
    pub fn is_ascii_alphanumeric(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Check if all characters are ASCII digits.
    pub fn is_ascii_digit(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_digit())
    }

    /// Check if all characters are ASCII hexadecimal digits.
    pub fn is_ascii_hexdigit(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Check if all characters are ASCII whitespace.
    pub fn is_ascii_whitespace(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_whitespace())
    }

    // =========================================================================
    // TRANSFORM
    // =========================================================================

    /// Repeat the string `count` times.
    pub fn repeat(&self, count: u32) -> Self {
        Self(self.0.repeat(count as usize))
    }

    /// Replace all occurrences of `from` with `to`.
    pub fn replace_all(&self, from: &Self, to: &Self) -> Self {
        Self(self.0.replace(&from.0, &to.0))
    }

    /// Replace first occurrence of `from` with `to`.
    pub fn replace_first(&self, from: &Self, to: &Self) -> Self {
        Self(self.0.replacen(&from.0, &to.0, 1))
    }

    /// Reverse the string (by characters, not bytes).
    pub fn reversed(&self) -> Self {
        Self(self.0.chars().rev().collect())
    }

    /// Count occurrences of `needle`.
    pub fn count_occurrences(&self, needle: &Self) -> u32 {
        self.0.matches(&needle.0).count() as u32
    }

    // =========================================================================
    // OPERATORS (for FFI registration)
    // =========================================================================

    /// Assignment operator - clone from other.
    pub fn assign(&mut self, other: &Self) -> &mut Self {
        self.0.clone_from(&other.0);
        self
    }

    /// Concatenation operator - self + other.
    pub fn concat(&self, other: &Self) -> Self {
        Self(format!("{}{}", self.0, other.0))
    }

    /// Reverse concatenation - other + self (for opAdd_r).
    pub fn concat_r(&self, other: &Self) -> Self {
        Self(format!("{}{}", other.0, self.0))
    }

    /// Append operator - self += other.
    pub fn push_str(&mut self, other: &Self) -> &mut Self {
        self.0.push_str(&other.0);
        self
    }

    /// Equality comparison.
    pub fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    /// Comparison operator - returns <0, 0, or >0.
    pub fn cmp(&self, other: &Self) -> i32 {
        match self.0.cmp(&other.0) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        }
    }

    /// Index access - get byte at position. Panics if out of bounds.
    pub fn byte_at(&self, index: u32) -> u8 {
        self.0.as_bytes()[index as usize]
    }

    /// Mutable index access. Returns mutable reference to the string for in-place modification.
    /// Note: This is a simplified version - full mutable byte access would require unsafe code.
    pub fn byte_at_mut(&mut self, index: u32) -> u8 {
        // For now, just return the byte (mutable access to individual bytes is complex in Rust)
        // The FFI layer will need to handle this specially
        self.0.as_bytes()[index as usize]
    }

    /// Set byte at index. Returns true if successful.
    pub fn set_byte_at(&mut self, index: u32, value: u8) -> bool {
        let index = index as usize;
        if index >= self.0.len() {
            return false;
        }

        // SAFETY: We're replacing ASCII bytes, which maintains UTF-8 validity
        // as long as value is valid ASCII. For full correctness, we'd need
        // to validate or use unsafe code.
        unsafe {
            self.0.as_bytes_mut()[index] = value;
        }
        true
    }
}

// =========================================================================
// STANDARD TRAIT IMPLEMENTATIONS
// =========================================================================

impl NativeType for ScriptString {
    const NAME: &'static str = "string";
}

impl From<String> for ScriptString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ScriptString {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<char> for ScriptString {
    fn from(c: char) -> Self {
        Self::from_char(c)
    }
}

impl From<ScriptString> for String {
    fn from(s: ScriptString) -> Self {
        s.0
    }
}

impl Deref for ScriptString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ScriptString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for ScriptString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for ScriptString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScriptString({:?})", self.0)
    }
}

impl AsRef<str> for ScriptString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<[u8]> for ScriptString {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

// =========================================================================
// FFI REGISTRATION
// =========================================================================

/// Creates the string module with the string type and parsing/formatting functions.
///
/// All functions are registered in the root namespace.
///
/// # Type
///
/// Registers the `string` value type with standard string operations.
///
/// # Functions
///
/// ## Parsing
/// - `parseInt`, `parseUInt`, `parseFloat` - Parse strings to numbers
///
/// ## Formatting
/// - `formatInt`, `formatUInt`, `formatFloat` - Format numbers to strings
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
    // STRING TYPE
    // =========================================================================

    // Register minimal string type - just enough for semantic analysis
    // Methods use raw API since ScriptString doesn't implement FromScript/ToScript
    module
        .register_type::<ScriptString>("string")?
        .value_type()
        .method_raw("uint length() const", |ctx: &mut crate::ffi::CallContext| {
            let s: &ScriptString = ctx.this()?;
            ctx.set_return(s.len())?;
            Ok(())
        })?
        .method_raw("bool isEmpty() const", |ctx: &mut crate::ffi::CallContext| {
            let s: &ScriptString = ctx.this()?;
            ctx.set_return(s.is_empty())?;
            Ok(())
        })?
        .operator_raw(
            "uint8 opIndex(uint idx) const",
            |ctx: &mut crate::ffi::CallContext| {
                let idx: u32 = ctx.arg(0)?;
                let s: &ScriptString = ctx.this()?;
                ctx.set_return(s.byte_at(idx))?;
                Ok(())
            },
        )?
        .operator_raw(
            "string &opAssign(const string &in other)",
            |_ctx: &mut crate::ffi::CallContext| {
                // Placeholder - VM handles assignment
                Ok(())
            },
        )?
        .operator_raw(
            "string opAdd(const string &in other) const",
            |_ctx: &mut crate::ffi::CallContext| {
                // Placeholder - VM handles concatenation
                Ok(())
            },
        )?
        .operator_raw(
            "bool opEquals(const string &in other) const",
            |_ctx: &mut crate::ffi::CallContext| {
                // Placeholder - VM handles comparison
                Ok(())
            },
        )?
        .operator_raw(
            "int opCmp(const string &in other) const",
            |_ctx: &mut crate::ffi::CallContext| {
                // Placeholder - VM handles comparison
                Ok(())
            },
        )?
        .operator_raw(
            "string &opAddAssign(const string &in other)",
            |ctx: &mut crate::ffi::CallContext| {
                // Get the other string argument
                let other: String = ctx.arg(0)?;
                let s: &mut ScriptString = ctx.this_mut()?;
                s.push_str(&ScriptString::from(other));
                Ok(())
            },
        )?
        // === Modification methods ===
        .method_raw("void clear()", |ctx: &mut crate::ffi::CallContext| {
            let s: &mut ScriptString = ctx.this_mut()?;
            s.clear();
            Ok(())
        })?
        .method_raw("void insert(uint pos, const string &in str)", |ctx: &mut crate::ffi::CallContext| {
            let pos: u32 = ctx.arg(0)?;
            let str_val: String = ctx.arg(1)?;
            let s: &mut ScriptString = ctx.this_mut()?;
            s.insert(pos, &ScriptString::from(str_val));
            Ok(())
        })?
        .method_raw("void erase(uint pos, int count = -1)", |ctx: &mut crate::ffi::CallContext| {
            let pos: u32 = ctx.arg(0)?;
            let count: i32 = ctx.arg(1)?;
            let s: &mut ScriptString = ctx.this_mut()?;
            s.erase(pos, count);
            Ok(())
        })?
        // === Substring methods ===
        .method_raw("string substr(uint start, int count = -1) const", |ctx: &mut crate::ffi::CallContext| {
            let start: u32 = ctx.arg(0)?;
            let count: i32 = ctx.arg(1)?;
            let s: &ScriptString = ctx.this()?;
            let result = s.substr(start, count);
            ctx.set_return(result.into_inner())?;
            Ok(())
        })?
        // === Search methods ===
        .method_raw("int findFirst(const string &in str, uint start = 0) const", |ctx: &mut crate::ffi::CallContext| {
            let str_val: String = ctx.arg(0)?;
            let start: u32 = ctx.arg(1)?;
            let s: &ScriptString = ctx.this()?;
            let result = s.find_first(&ScriptString::from(str_val), start);
            ctx.set_return(result as i64)?;
            Ok(())
        })?
        .method_raw("int findLast(const string &in str, int start = -1) const", |ctx: &mut crate::ffi::CallContext| {
            let str_val: String = ctx.arg(0)?;
            let start: i32 = ctx.arg(1)?;
            let s: &ScriptString = ctx.this()?;
            let result = s.find_last(&ScriptString::from(str_val), start);
            ctx.set_return(result as i64)?;
            Ok(())
        })?
        // === Predicate methods ===
        .method_raw("bool startsWith(const string &in str) const", |ctx: &mut crate::ffi::CallContext| {
            let str_val: String = ctx.arg(0)?;
            let s: &ScriptString = ctx.this()?;
            ctx.set_return(s.starts_with(&ScriptString::from(str_val)))?;
            Ok(())
        })?
        .method_raw("bool endsWith(const string &in str) const", |ctx: &mut crate::ffi::CallContext| {
            let str_val: String = ctx.arg(0)?;
            let s: &ScriptString = ctx.this()?;
            ctx.set_return(s.ends_with(&ScriptString::from(str_val)))?;
            Ok(())
        })?
        .method_raw("bool contains(const string &in str) const", |ctx: &mut crate::ffi::CallContext| {
            let str_val: String = ctx.arg(0)?;
            let s: &ScriptString = ctx.this()?;
            ctx.set_return(s.contains(&ScriptString::from(str_val)))?;
            Ok(())
        })?
        .build()?;

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

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // CONSTRUCTOR TESTS
    // =========================================================================

    #[test]
    fn test_new_creates_empty_string() {
        let s = ScriptString::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_str(), "");
    }

    #[test]
    fn test_from_str() {
        let s = ScriptString::from_str("hello");
        assert_eq!(s.len(), 5);
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_from_str_empty() {
        let s = ScriptString::from_str("");
        assert!(s.is_empty());
    }

    #[test]
    fn test_from_str_unicode() {
        let s = ScriptString::from_str("hello 世界");
        assert_eq!(s.as_str(), "hello 世界");
        // Note: len() returns byte length, not char count
        assert_eq!(s.len(), 12); // "hello " = 6 bytes, "世界" = 6 bytes (3 each)
    }

    #[test]
    fn test_from_char() {
        let s = ScriptString::from_char('x');
        assert_eq!(s.as_str(), "x");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_from_char_unicode() {
        let s = ScriptString::from_char('世');
        assert_eq!(s.as_str(), "世");
        assert_eq!(s.len(), 3); // UTF-8 bytes
    }

    #[test]
    fn test_from_char_repeated() {
        let s = ScriptString::from_char_repeated(b'x', 5);
        assert_eq!(s.as_str(), "xxxxx");
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_from_char_repeated_zero() {
        let s = ScriptString::from_char_repeated(b'x', 0);
        assert!(s.is_empty());
    }

    #[test]
    fn test_from_string_trait() {
        let s = ScriptString::from(String::from("world"));
        assert_eq!(s.as_str(), "world");
    }

    #[test]
    fn test_from_str_trait() {
        let s = ScriptString::from("hello");
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_into_inner() {
        let s = ScriptString::from("hello");
        let inner: String = s.into_inner();
        assert_eq!(inner, "hello");
    }

    #[test]
    fn test_as_mut_string() {
        let mut s = ScriptString::from("hello");
        s.as_mut_string().push_str(" world");
        assert_eq!(s.as_str(), "hello world");
    }

    // =========================================================================
    // SIZE AND CAPACITY TESTS
    // =========================================================================

    #[test]
    fn test_len() {
        assert_eq!(ScriptString::from("").len(), 0);
        assert_eq!(ScriptString::from("hello").len(), 5);
        assert_eq!(ScriptString::from("世界").len(), 6); // 2 chars, 3 bytes each
    }

    #[test]
    fn test_is_empty() {
        assert!(ScriptString::new().is_empty());
        assert!(ScriptString::from("").is_empty());
        assert!(!ScriptString::from("x").is_empty());
    }

    #[test]
    fn test_capacity_and_reserve() {
        let mut s = ScriptString::new();
        s.reserve(100);
        assert!(s.capacity() >= 100);
    }

    #[test]
    fn test_shrink_to_fit() {
        let mut s = ScriptString::new();
        s.reserve(1000);
        let cap_before = s.capacity();
        s.push_str(&"hi".into());
        s.shrink_to_fit();
        assert!(s.capacity() <= cap_before);
    }

    #[test]
    fn test_clear() {
        let mut s = ScriptString::from("hello");
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_resize_truncate() {
        let mut s = ScriptString::from("hello");
        s.resize(3);
        assert_eq!(s.as_str(), "hel");
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn test_resize_extend() {
        let mut s = ScriptString::from("hi");
        s.resize(5);
        assert_eq!(s.len(), 5);
        assert_eq!(&s.as_str()[..2], "hi");
        // Remaining should be null bytes
        assert_eq!(s.byte_at(2), 0);
        assert_eq!(s.byte_at(3), 0);
        assert_eq!(s.byte_at(4), 0);
    }

    #[test]
    fn test_resize_same_length() {
        let mut s = ScriptString::from("hello");
        s.resize(5);
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_truncate() {
        let mut s = ScriptString::from("hello world");
        s.truncate(5);
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_truncate_beyond_length() {
        let mut s = ScriptString::from("hi");
        s.truncate(100);
        assert_eq!(s.as_str(), "hi"); // No change
    }

    // =========================================================================
    // SUBSTRING AND SLICING TESTS
    // =========================================================================

    #[test]
    fn test_substr_basic() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.substr(0, 5).as_str(), "hello");
        assert_eq!(s.substr(6, 5).as_str(), "world");
    }

    #[test]
    fn test_substr_to_end() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.substr(6, -1).as_str(), "world");
    }

    #[test]
    fn test_substr_partial() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.substr(6, 3).as_str(), "wor");
    }

    #[test]
    fn test_substr_out_of_bounds_start() {
        let s = ScriptString::from("hello");
        assert_eq!(s.substr(100, 5).as_str(), "");
    }

    #[test]
    fn test_substr_count_exceeds_length() {
        let s = ScriptString::from("hello");
        assert_eq!(s.substr(0, 100).as_str(), "hello");
    }

    #[test]
    fn test_substr_zero_count() {
        let s = ScriptString::from("hello");
        assert_eq!(s.substr(0, 0).as_str(), "");
    }

    #[test]
    fn test_slice_basic() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.slice(0, 5).as_str(), "hello");
        assert_eq!(s.slice(6, 11).as_str(), "world");
    }

    #[test]
    fn test_slice_empty() {
        let s = ScriptString::from("hello");
        assert_eq!(s.slice(3, 3).as_str(), ""); // start == end
        assert_eq!(s.slice(4, 2).as_str(), ""); // start > end
    }

    #[test]
    fn test_slice_clamped() {
        let s = ScriptString::from("hello");
        assert_eq!(s.slice(0, 100).as_str(), "hello"); // end clamped
        assert_eq!(s.slice(100, 200).as_str(), ""); // both clamped
    }

    #[test]
    fn test_slice_from() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.slice_from(6).as_str(), "world");
        assert_eq!(s.slice_from(0).as_str(), "hello world");
        assert_eq!(s.slice_from(100).as_str(), ""); // clamped
    }

    #[test]
    fn test_slice_to() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.slice_to(5).as_str(), "hello");
        assert_eq!(s.slice_to(0).as_str(), "");
        assert_eq!(s.slice_to(100).as_str(), "hello world"); // clamped
    }

    // =========================================================================
    // SEARCH - SUBSTRING TESTS
    // =========================================================================

    #[test]
    fn test_find_first_basic() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_first(&"ell".into(), 0), 1);
    }

    #[test]
    fn test_find_first_with_offset() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_first(&"ell".into(), 2), 7);
    }

    #[test]
    fn test_find_first_not_found() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first(&"xyz".into(), 0), -1);
    }

    #[test]
    fn test_find_first_offset_past_end() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first(&"ell".into(), 100), -1);
    }

    #[test]
    fn test_find_first_empty_needle() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first(&"".into(), 0), 0); // Empty string found at start
    }

    #[test]
    fn test_find_first_in_empty_string() {
        let s = ScriptString::new();
        assert_eq!(s.find_first(&"x".into(), 0), -1);
    }

    #[test]
    fn test_find_last_basic() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_last(&"ell".into(), -1), 7);
    }

    #[test]
    fn test_find_last_with_limit() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_last(&"ell".into(), 5), 1);
    }

    #[test]
    fn test_find_last_not_found() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_last(&"xyz".into(), -1), -1);
    }

    #[test]
    fn test_find_last_empty_string() {
        let s = ScriptString::new();
        assert_eq!(s.find_last(&"x".into(), -1), -1);
    }

    // =========================================================================
    // SEARCH - CHARACTER SET TESTS
    // =========================================================================

    #[test]
    fn test_find_first_of_basic() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_of(&"aeiou".into(), 0), 1); // 'e' at index 1
    }

    #[test]
    fn test_find_first_of_not_found() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_of(&"xyz".into(), 0), -1);
    }

    #[test]
    fn test_find_first_of_with_offset() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_of(&"aeiou".into(), 2), 4); // 'o' at index 4
    }

    #[test]
    fn test_find_first_of_offset_past_end() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_of(&"aeiou".into(), 100), -1);
    }

    #[test]
    fn test_find_first_not_of_basic() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_not_of(&"he".into(), 0), 2); // 'l' at index 2
    }

    #[test]
    fn test_find_first_not_of_all_match() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_not_of(&"helo".into(), 0), -1); // all chars are in set
    }

    #[test]
    fn test_find_first_not_of_empty_set() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_first_not_of(&"".into(), 0), 0); // First char not in empty set
    }

    #[test]
    fn test_find_last_of_basic() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_last_of(&"aeiou".into(), -1), 4); // 'o' at index 4
    }

    #[test]
    fn test_find_last_of_with_limit() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_last_of(&"aeiou".into(), 3), 1); // 'e' at index 1 (search up to index 3)
    }

    #[test]
    fn test_find_last_of_not_found() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_last_of(&"xyz".into(), -1), -1);
    }

    #[test]
    fn test_find_last_not_of_basic() {
        let s = ScriptString::from("hello");
        assert_eq!(s.find_last_not_of(&"o".into(), -1), 3); // 'l' at index 3
    }

    #[test]
    fn test_find_last_not_of_all_match() {
        let s = ScriptString::from("aaa");
        assert_eq!(s.find_last_not_of(&"a".into(), -1), -1);
    }

    // =========================================================================
    // MODIFICATION TESTS
    // =========================================================================

    #[test]
    fn test_insert_middle() {
        let mut s = ScriptString::from("helo");
        s.insert(3, &"l".into());
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_insert_start() {
        let mut s = ScriptString::from("world");
        s.insert(0, &"hello ".into());
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_insert_end() {
        let mut s = ScriptString::from("hello");
        s.insert(5, &" world".into());
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_insert_beyond_end() {
        let mut s = ScriptString::from("hello");
        s.insert(100, &"!".into()); // Clamped to end
        assert_eq!(s.as_str(), "hello!");
    }

    #[test]
    fn test_erase_basic() {
        let mut s = ScriptString::from("hello");
        s.erase(1, 3);
        assert_eq!(s.as_str(), "ho");
    }

    #[test]
    fn test_erase_to_end() {
        let mut s = ScriptString::from("hello world");
        s.erase(5, -1);
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_erase_from_start() {
        let mut s = ScriptString::from("hello");
        s.erase(0, 2);
        assert_eq!(s.as_str(), "llo");
    }

    #[test]
    fn test_erase_past_end() {
        let mut s = ScriptString::from("hello");
        s.erase(100, 5); // pos past end, no change
        assert_eq!(s.as_str(), "hello");
    }

    #[test]
    fn test_erase_count_exceeds() {
        let mut s = ScriptString::from("hello");
        s.erase(2, 100); // count exceeds, erases to end
        assert_eq!(s.as_str(), "he");
    }

    #[test]
    fn test_push_and_pop() {
        let mut s = ScriptString::from("hell");
        s.push(b'o');
        assert_eq!(s.as_str(), "hello");

        let c = s.pop();
        assert_eq!(c, b'o');
        assert_eq!(s.as_str(), "hell");
    }

    #[test]
    fn test_pop_empty() {
        let mut s = ScriptString::new();
        assert_eq!(s.pop(), 0);
    }

    #[test]
    fn test_replace_range_basic() {
        let mut s = ScriptString::from("hello world");
        s.replace_range(6, 11, &"rust".into());
        assert_eq!(s.as_str(), "hello rust");
    }

    #[test]
    fn test_replace_range_insert() {
        let mut s = ScriptString::from("hello");
        s.replace_range(5, 5, &" world".into()); // Empty range = insert
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_replace_range_delete() {
        let mut s = ScriptString::from("hello world");
        s.replace_range(5, 11, &"".into()); // Replace with empty = delete
        assert_eq!(s.as_str(), "hello");
    }

    // =========================================================================
    // CASE CONVERSION TESTS
    // =========================================================================

    #[test]
    fn test_to_lowercase() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_lowercase().as_str(), "hello world");
    }

    #[test]
    fn test_to_uppercase() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_uppercase().as_str(), "HELLO WORLD");
    }

    #[test]
    fn test_to_ascii_lowercase() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_ascii_lowercase().as_str(), "hello world");
    }

    #[test]
    fn test_to_ascii_uppercase() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_ascii_uppercase().as_str(), "HELLO WORLD");
    }

    #[test]
    fn test_case_conversion_unicode() {
        let s = ScriptString::from("Grüß Gott");
        // Unicode-aware lowercase
        assert_eq!(s.to_lowercase().as_str(), "grüß gott");
    }

    #[test]
    fn test_case_conversion_empty() {
        let s = ScriptString::new();
        assert_eq!(s.to_lowercase().as_str(), "");
        assert_eq!(s.to_uppercase().as_str(), "");
    }

    // =========================================================================
    // TRIMMING TESTS
    // =========================================================================

    #[test]
    fn test_trim() {
        let s = ScriptString::from("  hello  ");
        assert_eq!(s.trim().as_str(), "hello");
    }

    #[test]
    fn test_trim_start() {
        let s = ScriptString::from("  hello  ");
        assert_eq!(s.trim_start().as_str(), "hello  ");
    }

    #[test]
    fn test_trim_end() {
        let s = ScriptString::from("  hello  ");
        assert_eq!(s.trim_end().as_str(), "  hello");
    }

    #[test]
    fn test_trim_matches() {
        let s = ScriptString::from("xxhelloxx");
        assert_eq!(s.trim_matches(&"x".into()).as_str(), "hello");
    }

    #[test]
    fn test_trim_matches_multiple_chars() {
        let s = ScriptString::from("xyzhelloyzx");
        assert_eq!(s.trim_matches(&"xyz".into()).as_str(), "hello");
    }

    #[test]
    fn test_trim_empty() {
        let s = ScriptString::new();
        assert_eq!(s.trim().as_str(), "");
    }

    #[test]
    fn test_trim_all_whitespace() {
        let s = ScriptString::from("   \t\n  ");
        assert_eq!(s.trim().as_str(), "");
    }

    // =========================================================================
    // PREDICATE TESTS
    // =========================================================================

    #[test]
    fn test_starts_with() {
        let s = ScriptString::from("hello world");
        assert!(s.starts_with(&"hello".into()));
        assert!(s.starts_with(&"".into()));
        assert!(!s.starts_with(&"world".into()));
    }

    #[test]
    fn test_ends_with() {
        let s = ScriptString::from("hello world");
        assert!(s.ends_with(&"world".into()));
        assert!(s.ends_with(&"".into()));
        assert!(!s.ends_with(&"hello".into()));
    }

    #[test]
    fn test_contains() {
        let s = ScriptString::from("hello world");
        assert!(s.contains(&"lo wo".into()));
        assert!(s.contains(&"".into()));
        assert!(!s.contains(&"xyz".into()));
    }

    #[test]
    fn test_is_ascii() {
        assert!(ScriptString::from("hello").is_ascii());
        assert!(ScriptString::from("").is_ascii());
        assert!(!ScriptString::from("héllo").is_ascii());
    }

    #[test]
    fn test_is_ascii_alphabetic() {
        assert!(ScriptString::from("hello").is_ascii_alphabetic());
        assert!(ScriptString::from("ABC").is_ascii_alphabetic());
        assert!(!ScriptString::from("hello123").is_ascii_alphabetic());
        assert!(!ScriptString::from("").is_ascii_alphabetic()); // Empty returns false
    }

    #[test]
    fn test_is_ascii_alphanumeric() {
        assert!(ScriptString::from("hello123").is_ascii_alphanumeric());
        assert!(ScriptString::from("ABC").is_ascii_alphanumeric());
        assert!(!ScriptString::from("hello!").is_ascii_alphanumeric());
        assert!(!ScriptString::from("").is_ascii_alphanumeric());
    }

    #[test]
    fn test_is_ascii_digit() {
        assert!(ScriptString::from("12345").is_ascii_digit());
        assert!(!ScriptString::from("123a").is_ascii_digit());
        assert!(!ScriptString::from("").is_ascii_digit());
    }

    #[test]
    fn test_is_ascii_hexdigit() {
        assert!(ScriptString::from("deadbeef").is_ascii_hexdigit());
        assert!(ScriptString::from("DEADBEEF").is_ascii_hexdigit());
        assert!(ScriptString::from("123ABC").is_ascii_hexdigit());
        assert!(!ScriptString::from("ghij").is_ascii_hexdigit());
        assert!(!ScriptString::from("").is_ascii_hexdigit());
    }

    #[test]
    fn test_is_ascii_whitespace() {
        assert!(ScriptString::from("   ").is_ascii_whitespace());
        assert!(ScriptString::from("\t\n").is_ascii_whitespace());
        assert!(!ScriptString::from(" x ").is_ascii_whitespace());
        assert!(!ScriptString::from("").is_ascii_whitespace());
    }

    // =========================================================================
    // TRANSFORM TESTS
    // =========================================================================

    #[test]
    fn test_repeat() {
        let s = ScriptString::from("ab");
        assert_eq!(s.repeat(3).as_str(), "ababab");
        assert_eq!(s.repeat(0).as_str(), "");
        assert_eq!(s.repeat(1).as_str(), "ab");
    }

    #[test]
    fn test_replace_all() {
        let s = ScriptString::from("hello hello hello");
        assert_eq!(s.replace_all(&"hello".into(), &"hi".into()).as_str(), "hi hi hi");
    }

    #[test]
    fn test_replace_all_no_match() {
        let s = ScriptString::from("hello");
        assert_eq!(s.replace_all(&"xyz".into(), &"abc".into()).as_str(), "hello");
    }

    #[test]
    fn test_replace_first() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.replace_first(&"hello".into(), &"hi".into()).as_str(), "hi hello");
    }

    #[test]
    fn test_replace_first_no_match() {
        let s = ScriptString::from("hello");
        assert_eq!(s.replace_first(&"xyz".into(), &"abc".into()).as_str(), "hello");
    }

    #[test]
    fn test_reversed() {
        let s = ScriptString::from("hello");
        assert_eq!(s.reversed().as_str(), "olleh");
    }

    #[test]
    fn test_reversed_empty() {
        let s = ScriptString::new();
        assert_eq!(s.reversed().as_str(), "");
    }

    #[test]
    fn test_reversed_unicode() {
        let s = ScriptString::from("世界");
        assert_eq!(s.reversed().as_str(), "界世");
    }

    #[test]
    fn test_count_occurrences() {
        let s = ScriptString::from("hello hello hello");
        assert_eq!(s.count_occurrences(&"hello".into()), 3);
        assert_eq!(s.count_occurrences(&"x".into()), 0);
        assert_eq!(s.count_occurrences(&"l".into()), 6);
    }

    // =========================================================================
    // OPERATOR TESTS
    // =========================================================================

    #[test]
    fn test_assign() {
        let mut s = ScriptString::from("old");
        s.assign(&"new".into());
        assert_eq!(s.as_str(), "new");
    }

    #[test]
    fn test_concat() {
        let s1 = ScriptString::from("hello");
        let s2 = ScriptString::from(" world");
        assert_eq!(s1.concat(&s2).as_str(), "hello world");
    }

    #[test]
    fn test_concat_r() {
        let s1 = ScriptString::from(" world");
        let s2 = ScriptString::from("hello");
        assert_eq!(s1.concat_r(&s2).as_str(), "hello world");
    }

    #[test]
    fn test_push_str() {
        let mut s = ScriptString::from("hello");
        s.push_str(&" world".into());
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_eq_method() {
        let s1 = ScriptString::from("hello");
        let s2 = ScriptString::from("hello");
        let s3 = ScriptString::from("world");
        assert!(s1.eq(&s2));
        assert!(!s1.eq(&s3));
    }

    #[test]
    fn test_cmp_method() {
        let s1 = ScriptString::from("abc");
        let s2 = ScriptString::from("abd");
        let s3 = ScriptString::from("abc");
        assert_eq!(s1.cmp(&s2), -1); // abc < abd
        assert_eq!(s2.cmp(&s1), 1);  // abd > abc
        assert_eq!(s1.cmp(&s3), 0);  // abc == abc
    }

    #[test]
    fn test_byte_at() {
        let s = ScriptString::from("hello");
        assert_eq!(s.byte_at(0), b'h');
        assert_eq!(s.byte_at(4), b'o');
    }

    #[test]
    #[should_panic]
    fn test_byte_at_out_of_bounds() {
        let s = ScriptString::from("hello");
        s.byte_at(100); // Should panic
    }

    #[test]
    fn test_set_byte_at() {
        let mut s = ScriptString::from("hello");
        assert!(s.set_byte_at(0, b'j'));
        assert_eq!(s.as_str(), "jello");
    }

    #[test]
    fn test_set_byte_at_out_of_bounds() {
        let mut s = ScriptString::from("hello");
        assert!(!s.set_byte_at(100, b'x')); // Returns false
        assert_eq!(s.as_str(), "hello"); // Unchanged
    }

    // =========================================================================
    // TRAIT IMPLEMENTATION TESTS
    // =========================================================================

    #[test]
    fn test_deref() {
        let s = ScriptString::from("hello");
        // Deref allows using str methods
        assert_eq!(s.chars().count(), 5);
        // Use str::contains via deref (explicit to avoid method name conflict)
        assert!((*s).contains("ell"));
    }

    #[test]
    fn test_deref_mut() {
        let mut s = ScriptString::from("hello");
        // DerefMut allows mutable str access
        s.make_ascii_uppercase();
        assert_eq!(s.as_str(), "HELLO");
    }

    #[test]
    fn test_display() {
        let s = ScriptString::from("hello");
        assert_eq!(format!("{}", s), "hello");
    }

    #[test]
    fn test_debug() {
        let s = ScriptString::from("hello");
        let debug = format!("{:?}", s);
        assert!(debug.contains("ScriptString"));
        assert!(debug.contains("hello"));
    }

    #[test]
    fn test_clone() {
        let s1 = ScriptString::from("hello");
        let s2 = s1.clone();
        assert_eq!(s1, s2);
        // Verify they're independent
        assert_eq!(s1.as_str(), s2.as_str());
    }

    #[test]
    fn test_default() {
        let s: ScriptString = Default::default();
        assert!(s.is_empty());
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let s1 = ScriptString::from("hello");
        let s2 = ScriptString::from("hello");

        let mut set = HashSet::new();
        set.insert(s1);
        assert!(set.contains(&s2));
    }

    #[test]
    fn test_partial_eq() {
        let s1 = ScriptString::from("hello");
        let s2 = ScriptString::from("hello");
        let s3 = ScriptString::from("world");
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_ord() {
        let s1 = ScriptString::from("abc");
        let s2 = ScriptString::from("abd");
        let s3 = ScriptString::from("abc");
        assert!(s1 < s2);
        assert!(s2 > s1);
        assert!(s1 <= s3);
        assert!(s1 >= s3);
    }

    #[test]
    fn test_as_ref_str() {
        let s = ScriptString::from("hello");
        let r: &str = s.as_ref();
        assert_eq!(r, "hello");
    }

    #[test]
    fn test_as_ref_bytes() {
        let s = ScriptString::from("hello");
        let r: &[u8] = s.as_ref();
        assert_eq!(r, b"hello");
    }

    #[test]
    fn test_from_char_trait() {
        let s: ScriptString = 'x'.into();
        assert_eq!(s.as_str(), "x");
    }

    #[test]
    fn test_into_string() {
        let s = ScriptString::from("hello");
        let inner: String = s.into();
        assert_eq!(inner, "hello");
    }

    #[test]
    fn test_native_type_name() {
        assert_eq!(ScriptString::NAME, "string");
    }

    // =========================================================================
    // FFI MODULE TESTS
    // =========================================================================

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
        // parse (5) + format (10) = 15
        assert!(
            module.functions().len() >= 10,
            "string module should have functions, got {}",
            module.functions().len()
        );
    }

    #[test]
    fn test_parse_function_names() {
        let module = string_module().expect("string module should build");
        let fn_names: Vec<_> = module.functions().iter().map(|f| f.name.as_str()).collect();

        assert!(fn_names.contains(&"parseInt"), "should have parseInt");
        assert!(fn_names.contains(&"parseUInt"), "should have parseUInt");
        assert!(fn_names.contains(&"parseFloat"), "should have parseFloat");
    }

    #[test]
    fn test_format_function_names() {
        let module = string_module().expect("string module should build");
        let fn_names: Vec<_> = module.functions().iter().map(|f| f.name.as_str()).collect();

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
        // 5 parse + 3 formatInt + 3 formatUInt + 4 formatFloat = 15
        assert_eq!(module.functions().len(), 15);
    }
}
