//! ScriptString - AngelScript string type backed by Rust String.
//!
//! This is a VALUE type - copied on assignment. It provides all methods
//! needed for the AngelScript string type.

use std::fmt;
use std::ops::{Deref, DerefMut};

use angelscript_macros::Any;
use angelscript_registry::Module;

/// AngelScript string type backed by Rust String.
///
/// This is a VALUE type - copied on assignment.
/// Byte-indexed (not char-indexed) like C++ AngelScript.
#[derive(Any, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[angelscript(name = "string", value)]
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
    #[allow(clippy::should_implement_trait)]
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
    // SIZE AND CAPACITY (exposed to AngelScript)
    // =========================================================================

    /// Returns the byte length of the string.
    #[angelscript_macros::function(instance, const, name = "length")]
    pub fn len(&self) -> u32 {
        self.0.len() as u32
    }

    /// Returns true if the string is empty.
    #[angelscript_macros::function(instance, const, name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the allocated capacity in bytes.
    #[angelscript_macros::function(instance, const)]
    pub fn capacity(&self) -> u32 {
        self.0.capacity() as u32
    }

    /// Reserve capacity for at least `additional` more bytes.
    #[angelscript_macros::function(instance)]
    pub fn reserve(&mut self, additional: u32) {
        self.0.reserve(additional as usize);
    }

    /// Shrink capacity to fit current length.
    #[angelscript_macros::function(instance, name = "shrinkToFit")]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Clear the string.
    #[angelscript_macros::function(instance)]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Resize the string to `new_len` bytes.
    /// Truncates if shorter, pads with null bytes if longer.
    #[angelscript_macros::function(instance)]
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
    #[angelscript_macros::function(instance, const)]
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
    #[angelscript_macros::function(instance, const)]
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
    #[angelscript_macros::function(instance, const, name = "sliceFrom")]
    pub fn slice_from(&self, start: u32) -> Self {
        let start = (start as usize).min(self.0.len());
        Self(self.0[start..].to_string())
    }

    /// Slice from start to byte index [..end].
    #[angelscript_macros::function(instance, const, name = "sliceTo")]
    pub fn slice_to(&self, end: u32) -> Self {
        let end = (end as usize).min(self.0.len());
        Self(self.0[..end].to_string())
    }

    // =========================================================================
    // SEARCH - SUBSTRING
    // =========================================================================

    /// Find first occurrence of substring starting from `start`. Returns -1 if not found.
    #[angelscript_macros::function(instance, const, name = "findFirst")]
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
    #[angelscript_macros::function(instance, const, name = "findLast")]
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
    #[angelscript_macros::function(instance, const, name = "findFirstOf")]
    pub fn find_first_of(&self, chars: Self, start: u32) -> i32 {
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
    #[angelscript_macros::function(instance, const, name = "findFirstNotOf")]
    pub fn find_first_not_of(&self, chars: Self, start: u32) -> i32 {
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
    #[angelscript_macros::function(instance, const, name = "findLastOf")]
    pub fn find_last_of(&self, chars: Self, start: i32) -> i32 {
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
    #[angelscript_macros::function(instance, const, name = "findLastNotOf")]
    pub fn find_last_not_of(&self, chars: Self, start: i32) -> i32 {
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
    #[angelscript_macros::function(instance)]
    pub fn insert(&mut self, pos: u32, s: Self) {
        let pos = (pos as usize).min(self.0.len());
        self.0.insert_str(pos, &s.0);
    }

    /// Erase bytes starting at `pos`. `count` of -1 means "to end".
    #[angelscript_macros::function(instance)]
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
    #[angelscript_macros::function(instance)]
    pub fn push(&mut self, c: u8) {
        self.0.push(char::from(c));
    }

    /// Pop and return the last byte. Returns 0 if empty.
    #[angelscript_macros::function(instance)]
    pub fn pop(&mut self) -> u8 {
        self.0.pop().map(|c| c as u8).unwrap_or(0)
    }

    /// Truncate to `len` bytes.
    #[angelscript_macros::function(instance)]
    pub fn truncate(&mut self, len: u32) {
        self.0.truncate(len as usize);
    }

    /// Replace bytes in range [start..end] with `s`.
    #[angelscript_macros::function(instance, name = "replaceRange")]
    pub fn replace_range(&mut self, start: u32, end: u32, s: Self) {
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
    #[angelscript_macros::function(instance, const, name = "toLowercase")]
    pub fn to_lowercase(&self) -> Self {
        Self(self.0.to_lowercase())
    }

    /// Convert to uppercase (Unicode-aware).
    #[angelscript_macros::function(instance, const, name = "toUppercase")]
    pub fn to_uppercase(&self) -> Self {
        Self(self.0.to_uppercase())
    }

    /// Convert to ASCII lowercase.
    #[angelscript_macros::function(instance, const, name = "toAsciiLowercase")]
    pub fn to_ascii_lowercase(&self) -> Self {
        Self(self.0.to_ascii_lowercase())
    }

    /// Convert to ASCII uppercase.
    #[angelscript_macros::function(instance, const, name = "toAsciiUppercase")]
    pub fn to_ascii_uppercase(&self) -> Self {
        Self(self.0.to_ascii_uppercase())
    }

    // =========================================================================
    // TRIMMING
    // =========================================================================

    /// Trim whitespace from both ends.
    #[angelscript_macros::function(instance, const)]
    pub fn trim(&self) -> Self {
        Self(self.0.trim().to_string())
    }

    /// Trim whitespace from start.
    #[angelscript_macros::function(instance, const, name = "trimStart")]
    pub fn trim_start(&self) -> Self {
        Self(self.0.trim_start().to_string())
    }

    /// Trim whitespace from end.
    #[angelscript_macros::function(instance, const, name = "trimEnd")]
    pub fn trim_end(&self) -> Self {
        Self(self.0.trim_end().to_string())
    }

    /// Trim any characters in `chars` from both ends.
    #[angelscript_macros::function(instance, const, name = "trimMatches")]
    pub fn trim_matches(&self, chars: Self) -> Self {
        let chars_vec: Vec<char> = chars.0.chars().collect();
        Self(self.0.trim_matches(|c| chars_vec.contains(&c)).to_string())
    }

    // =========================================================================
    // PREDICATES
    // =========================================================================

    /// Check if string starts with `prefix`.
    #[angelscript_macros::function(instance, const, name = "startsWith")]
    pub fn starts_with(&self, prefix: Self) -> bool {
        self.0.starts_with(&prefix.0)
    }

    /// Check if string ends with `suffix`.
    #[angelscript_macros::function(instance, const, name = "endsWith")]
    pub fn ends_with(&self, suffix: Self) -> bool {
        self.0.ends_with(&suffix.0)
    }

    /// Check if string contains `needle`.
    #[angelscript_macros::function(instance, const)]
    pub fn contains(&self, needle: Self) -> bool {
        self.0.contains(&needle.0)
    }

    /// Check if all characters are ASCII.
    #[angelscript_macros::function(instance, const, name = "isAscii")]
    pub fn is_ascii(&self) -> bool {
        self.0.is_ascii()
    }

    /// Check if all characters are ASCII alphabetic.
    #[angelscript_macros::function(instance, const, name = "isAsciiAlphabetic")]
    pub fn is_ascii_alphabetic(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_alphabetic())
    }

    /// Check if all characters are ASCII alphanumeric.
    #[angelscript_macros::function(instance, const, name = "isAsciiAlphanumeric")]
    pub fn is_ascii_alphanumeric(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Check if all characters are ASCII digits.
    #[angelscript_macros::function(instance, const, name = "isAsciiDigit")]
    pub fn is_ascii_digit(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_digit())
    }

    /// Check if all characters are ASCII hexadecimal digits.
    #[angelscript_macros::function(instance, const, name = "isAsciiHexdigit")]
    pub fn is_ascii_hexdigit(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Check if all characters are ASCII whitespace.
    #[angelscript_macros::function(instance, const, name = "isAsciiWhitespace")]
    pub fn is_ascii_whitespace(&self) -> bool {
        !self.0.is_empty() && self.0.chars().all(|c| c.is_ascii_whitespace())
    }

    // =========================================================================
    // TRANSFORM
    // =========================================================================

    /// Repeat the string `count` times.
    #[angelscript_macros::function(instance, const, name = "repeat")]
    pub fn repeat_n(&self, count: u32) -> Self {
        Self(self.0.repeat(count as usize))
    }

    /// Replace all occurrences of `from` with `to`.
    #[angelscript_macros::function(instance, const, name = "replaceAll")]
    pub fn replace_all(&self, from: Self, to: Self) -> Self {
        Self(self.0.replace(&from.0, &to.0))
    }

    /// Replace first occurrence of `from` with `to`.
    #[angelscript_macros::function(instance, const, name = "replaceFirst")]
    pub fn replace_first(&self, from: Self, to: Self) -> Self {
        Self(self.0.replacen(&from.0, &to.0, 1))
    }

    /// Reverse the string (by characters, not bytes).
    #[angelscript_macros::function(instance, const)]
    pub fn reversed(&self) -> Self {
        Self(self.0.chars().rev().collect())
    }

    /// Count occurrences of `needle`.
    #[angelscript_macros::function(instance, const, name = "countOccurrences")]
    pub fn count_occurrences(&self, needle: Self) -> u32 {
        self.0.matches(&needle.0).count() as u32
    }

    // =========================================================================
    // OPERATORS (for FFI registration)
    // =========================================================================

    /// Concatenation operator - self + other.
    #[angelscript_macros::function(operator = Operator::Add, const)]
    pub fn concat(&self, other: Self) -> Self {
        Self(format!("{}{}", self.0, other.0))
    }

    /// Append operator - self += other.
    #[angelscript_macros::function(operator = Operator::AddAssign)]
    pub fn append(&mut self, other: Self) {
        self.0.push_str(&other.0);
    }

    /// Equality comparison.
    #[angelscript_macros::function(operator = Operator::Equals, const)]
    pub fn eq_op(&self, other: Self) -> bool {
        self.0 == other.0
    }

    /// Comparison operator - returns <0, 0, or >0.
    #[angelscript_macros::function(operator = Operator::Cmp, const)]
    pub fn cmp_op(&self, other: Self) -> i32 {
        match self.0.cmp(&other.0) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// Index access - get byte at position.
    #[angelscript_macros::function(operator = Operator::Index, const)]
    pub fn byte_at(&self, index: u32) -> u8 {
        self.0.as_bytes().get(index as usize).copied().unwrap_or(0)
    }
}

// =========================================================================
// STANDARD TRAIT IMPLEMENTATIONS
// =========================================================================

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
// STRING FACTORY
// =========================================================================

/// Default string factory that creates [`ScriptString`] values.
///
/// This factory interprets raw bytes as UTF-8, using lossy conversion
/// for invalid sequences. It's automatically set by `Context::with_default_modules()`.
///
/// # Example
///
/// ```ignore
/// use angelscript_modules::string::ScriptStringFactory;
///
/// ctx.set_string_factory(Box::new(ScriptStringFactory));
/// ```
pub struct ScriptStringFactory;

impl angelscript_core::StringFactory for ScriptStringFactory {
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
        // Interpret as UTF-8, lossy conversion for invalid sequences
        let s = String::from_utf8_lossy(data);
        Box::new(ScriptString::from(s.as_ref()))
    }

    fn type_hash(&self) -> angelscript_core::TypeHash {
        <ScriptString as angelscript_core::Any>::type_hash()
    }
}

// =========================================================================
// MODULE CREATION
// =========================================================================

/// Creates the string module with the string value type.
///
/// # Example
///
/// ```ignore
/// use angelscript_modules::string;
///
/// let module = string::module();
/// // Install with context...
/// ```
pub fn module() -> Module {
    Module::new()
        .ty::<ScriptString>()
        // Basic operations
        .function(ScriptString::len__meta)
        .function(ScriptString::is_empty__meta)
        .function(ScriptString::capacity__meta)
        .function(ScriptString::reserve__meta)
        .function(ScriptString::shrink_to_fit__meta)
        .function(ScriptString::clear__meta)
        .function(ScriptString::resize__meta)
        // Substrings
        .function(ScriptString::substr__meta)
        .function(ScriptString::slice__meta)
        .function(ScriptString::slice_from__meta)
        .function(ScriptString::slice_to__meta)
        // Search
        .function(ScriptString::find_first__meta)
        .function(ScriptString::find_last__meta)
        .function(ScriptString::find_first_of__meta)
        .function(ScriptString::find_first_not_of__meta)
        .function(ScriptString::find_last_of__meta)
        .function(ScriptString::find_last_not_of__meta)
        // Modification
        .function(ScriptString::insert__meta)
        .function(ScriptString::erase__meta)
        .function(ScriptString::push__meta)
        .function(ScriptString::pop__meta)
        .function(ScriptString::truncate__meta)
        .function(ScriptString::replace_range__meta)
        // Case conversion
        .function(ScriptString::to_lowercase__meta)
        .function(ScriptString::to_uppercase__meta)
        .function(ScriptString::to_ascii_lowercase__meta)
        .function(ScriptString::to_ascii_uppercase__meta)
        // Trimming
        .function(ScriptString::trim__meta)
        .function(ScriptString::trim_start__meta)
        .function(ScriptString::trim_end__meta)
        .function(ScriptString::trim_matches__meta)
        // Predicates
        .function(ScriptString::starts_with__meta)
        .function(ScriptString::ends_with__meta)
        .function(ScriptString::contains__meta)
        .function(ScriptString::is_ascii__meta)
        .function(ScriptString::is_ascii_alphabetic__meta)
        .function(ScriptString::is_ascii_alphanumeric__meta)
        .function(ScriptString::is_ascii_digit__meta)
        .function(ScriptString::is_ascii_hexdigit__meta)
        .function(ScriptString::is_ascii_whitespace__meta)
        // Transformations
        .function(ScriptString::repeat_n__meta)
        .function(ScriptString::replace_all__meta)
        .function(ScriptString::replace_first__meta)
        .function(ScriptString::reversed__meta)
        .function(ScriptString::count_occurrences__meta)
        // Operators
        .function(ScriptString::concat__meta)
        .function(ScriptString::append__meta)
        .function(ScriptString::eq_op__meta)
        .function(ScriptString::cmp_op__meta)
        .function(ScriptString::byte_at__meta)
}

// =========================================================================
// TESTS
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_from_str_unicode() {
        let s = ScriptString::from_str("hello 世界");
        assert_eq!(s.as_str(), "hello 世界");
        assert_eq!(s.len(), 12); // "hello " = 6 bytes, "世界" = 6 bytes
    }

    #[test]
    fn test_from_char() {
        let s = ScriptString::from_char('x');
        assert_eq!(s.as_str(), "x");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_from_char_repeated() {
        let s = ScriptString::from_char_repeated(b'x', 5);
        assert_eq!(s.as_str(), "xxxxx");
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn test_substr() {
        let s = ScriptString::from("hello world");
        assert_eq!(s.substr(0, 5).as_str(), "hello");
        assert_eq!(s.substr(6, 5).as_str(), "world");
        assert_eq!(s.substr(6, -1).as_str(), "world");
    }

    #[test]
    fn test_find_first() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_first(&"ell".into(), 0), 1);
        assert_eq!(s.find_first(&"ell".into(), 2), 7);
        assert_eq!(s.find_first(&"xyz".into(), 0), -1);
    }

    #[test]
    fn test_find_last() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.find_last(&"ell".into(), -1), 7);
        assert_eq!(s.find_last(&"ell".into(), 5), 1);
    }

    #[test]
    fn test_insert_and_erase() {
        let mut s = ScriptString::from("helo");
        s.insert(3, "l".into());
        assert_eq!(s.as_str(), "hello");

        s.erase(1, 3);
        assert_eq!(s.as_str(), "ho");
    }

    #[test]
    fn test_case_conversion() {
        let s = ScriptString::from("Hello World");
        assert_eq!(s.to_lowercase().as_str(), "hello world");
        assert_eq!(s.to_uppercase().as_str(), "HELLO WORLD");
    }

    #[test]
    fn test_trim() {
        let s = ScriptString::from("  hello  ");
        assert_eq!(s.trim().as_str(), "hello");
        assert_eq!(s.trim_start().as_str(), "hello  ");
        assert_eq!(s.trim_end().as_str(), "  hello");
    }

    #[test]
    fn test_predicates() {
        let s = ScriptString::from("hello world");
        assert!(s.starts_with("hello".into()));
        assert!(s.ends_with("world".into()));
        assert!(s.contains("lo wo".into()));
    }

    #[test]
    fn test_replace() {
        let s = ScriptString::from("hello hello");
        assert_eq!(s.replace_all("hello".into(), "hi".into()).as_str(), "hi hi");
        assert_eq!(
            s.replace_first("hello".into(), "hi".into()).as_str(),
            "hi hello"
        );
    }

    #[test]
    fn test_concat() {
        let s1 = ScriptString::from("hello");
        let s2 = ScriptString::from(" world");
        assert_eq!(s1.concat(s2).as_str(), "hello world");
    }

    #[test]
    fn test_append() {
        let mut s = ScriptString::from("hello");
        s.append(" world".into());
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_cmp() {
        let s1 = ScriptString::from("abc");
        let s2 = ScriptString::from("abd");
        let s3 = ScriptString::from("abc");
        assert_eq!(s1.cmp_op(s2.clone()), -1);
        assert_eq!(s2.cmp_op(s1.clone()), 1);
        assert_eq!(s1.cmp_op(s3), 0);
    }

    #[test]
    fn test_byte_at() {
        let s = ScriptString::from("hello");
        assert_eq!(s.byte_at(0), b'h');
        assert_eq!(s.byte_at(4), b'o');
        assert_eq!(s.byte_at(100), 0); // Out of bounds returns 0
    }

    #[test]
    fn test_reversed() {
        let s = ScriptString::from("hello");
        assert_eq!(s.reversed().as_str(), "olleh");
    }

    #[test]
    fn test_count_occurrences() {
        let s = ScriptString::from("hello hello hello");
        assert_eq!(s.count_occurrences("hello".into()), 3);
        assert_eq!(s.count_occurrences("x".into()), 0);
    }

    #[test]
    fn test_module_creates() {
        use angelscript_registry::HasClassMeta;
        let meta = ScriptString::__as_type_meta();
        assert_eq!(meta.name, "string");
    }

    #[test]
    fn test_script_string_factory_creates_string() {
        use angelscript_core::StringFactory;
        let factory = ScriptStringFactory;
        let value = factory.create(b"hello world");
        let s = value.downcast::<ScriptString>().unwrap();
        assert_eq!(s.as_str(), "hello world");
    }

    #[test]
    fn test_script_string_factory_type_hash() {
        use angelscript_core::{Any, StringFactory};
        let factory = ScriptStringFactory;
        assert_eq!(factory.type_hash(), ScriptString::type_hash());
    }

    #[test]
    fn test_script_string_factory_handles_non_utf8() {
        use angelscript_core::StringFactory;
        let factory = ScriptStringFactory;
        // Invalid UTF-8 sequence followed by valid ASCII
        let data = vec![0xFF, 0xFE, b'a', b'b', b'c'];
        let value = factory.create(&data);
        let s = value.downcast::<ScriptString>().unwrap();
        // from_utf8_lossy replaces invalid sequences with replacement character
        assert!(s.as_str().contains("abc"));
    }

    #[test]
    fn test_script_string_factory_empty() {
        use angelscript_core::StringFactory;
        let factory = ScriptStringFactory;
        let value = factory.create(b"");
        let s = value.downcast::<ScriptString>().unwrap();
        assert!(s.is_empty());
    }
}
