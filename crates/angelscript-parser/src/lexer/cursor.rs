/// A cursor over source text that tracks position.
///
/// Provides low-level character access with peek/advance semantics.
/// Tracks byte offset, line number, and column number as it advances.
pub struct Cursor<'src> {
    /// The source text being scanned.
    source: &'src str,
    /// Remaining source text (slice starting at current position).
    rest: &'src str,
    /// Current byte offset from start of source.
    offset: u32,
    /// Current line number (1-indexed).
    line: u32,
    /// Current column number (1-indexed, byte-based).
    column: u32,
}

impl<'src> Cursor<'src> {
    /// Create a new cursor at the start of the source.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            rest: source,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get the full source text.
    #[inline]
    pub fn source(&self) -> &'src str {
        self.source
    }

    /// Current byte offset from start of source.
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Current line number (1-indexed).
    #[inline]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// Current column number (1-indexed, byte-based).
    #[inline]
    pub fn column(&self) -> u32 {
        self.column
    }

    /// Check if we've reached the end of input.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.rest.is_empty()
    }

    /// Peek at the current character without consuming it.
    ///
    /// Optimized with an ASCII fast path to avoid iterator creation
    /// for the common case of ASCII characters.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        let bytes = self.rest.as_bytes();
        let first = *bytes.first()?;
        if first < 128 {
            Some(first as char) // No iterator creation for ASCII
        } else {
            self.rest.chars().next() // UTF-8 path unchanged
        }
    }

    /// Peek at the nth character ahead (0 = current).
    #[inline]
    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest.chars().nth(n)
    }

    /// Check if the current character satisfies a predicate.
    #[inline]
    pub fn check(&self, f: impl Fn(char) -> bool) -> bool {
        self.peek().is_some_and(f)
    }

    /// Check if the upcoming bytes match the given string.
    #[inline]
    pub fn check_str(&self, s: &str) -> bool {
        self.rest.starts_with(s)
    }

    /// Consume the current character and advance.
    ///
    /// Returns the consumed character, or `None` if at EOF.
    /// Updates line/column tracking.
    ///
    /// Optimized with a fast path for ASCII characters, which are the most
    /// common case in source code. Falls back to full UTF-8 handling for
    /// multi-byte characters.
    #[inline(always)]
    pub fn advance(&mut self) -> Option<char> {
        let bytes = self.rest.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        let first_byte = bytes[0];

        // Fast path: ASCII character (most common case)
        if first_byte < 128 {
            let ch = first_byte as char;
            self.rest = unsafe {
                // SAFETY: We know first_byte < 128, so it's valid UTF-8
                // and we're advancing by exactly 1 byte
                std::str::from_utf8_unchecked(&bytes[1..])
            };
            self.offset += 1;

            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }

            Some(ch)
        } else {
            // Slow path: Multi-byte UTF-8 character
            let ch = self.rest.chars().next()?;
            let len = ch.len_utf8() as u32;

            self.rest = &self.rest[len as usize..];
            self.offset += len;
            self.column += len;

            Some(ch)
        }
    }

    /// Advance by n bytes without checking character boundaries.
    ///
    /// # Safety
    /// Caller must ensure `n` lands on a valid UTF-8 boundary.
    pub fn advance_bytes(&mut self, n: usize) {
        debug_assert!(self.rest.is_char_boundary(n));

        for ch in self.rest[..n].chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += ch.len_utf8() as u32;
            }
        }

        self.rest = &self.rest[n..];
        self.offset += n as u32;
    }

    /// Consume if the current character matches.
    #[inline]
    pub fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume characters while the predicate matches.
    ///
    /// Returns the consumed slice.
    pub fn eat_while(&mut self, f: impl Fn(char) -> bool) -> &'src str {
        let start = self.offset as usize;
        while self.check(&f) {
            self.advance();
        }
        &self.source[start..self.offset as usize]
    }

    /// Consume ASCII characters while the predicate matches.
    ///
    /// This is faster than `eat_while` for ASCII-only content (identifiers, numbers)
    /// because it operates directly on bytes without creating char iterators.
    ///
    /// # Note
    /// Does NOT handle newlines - use only for single-line content like identifiers
    /// and numbers where newlines are not expected.
    #[inline]
    pub fn eat_while_ascii(&mut self, f: impl Fn(u8) -> bool) -> &'src str {
        let start = self.offset as usize;
        let bytes = self.rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i] < 128 && f(bytes[i]) {
            i += 1;
        }
        if i > 0 {
            self.rest = &self.rest[i..];
            self.offset += i as u32;
            self.column += i as u32;
        }
        &self.source[start..self.offset as usize]
    }

    /// Get a slice of source from a starting offset to current position.
    #[inline]
    pub fn slice_from(&self, start: u32) -> &'src str {
        &self.source[start as usize..self.offset as usize]
    }
}

/// Check if a character can start an identifier.
#[inline]
pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Check if a character can continue an identifier.
#[inline]
pub fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Check if a byte can continue an identifier (ASCII-only version).
///
/// This is faster than `is_ident_continue` when working with raw bytes
/// in performance-critical loops.
#[inline]
pub fn is_ident_continue_ascii(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_basics() {
        let mut cursor = Cursor::new("hello");
        assert_eq!(cursor.peek(), Some('h'));
        assert_eq!(cursor.offset(), 0);

        assert_eq!(cursor.advance(), Some('h'));
        assert_eq!(cursor.peek(), Some('e'));
        assert_eq!(cursor.offset(), 1);
    }

    #[test]
    fn cursor_eat() {
        let mut cursor = Cursor::new("hello");

        assert!(cursor.eat('h'));
        assert!(!cursor.eat('h')); // Already consumed
        assert!(cursor.eat('e'));
    }

    #[test]
    fn cursor_eat_while() {
        let mut cursor = Cursor::new("aaabbb");

        let as_ = cursor.eat_while(|c| c == 'a');
        assert_eq!(as_, "aaa");

        let bs = cursor.eat_while(|c| c == 'b');
        assert_eq!(bs, "bbb");

        assert!(cursor.is_eof());
    }

    #[test]
    fn cursor_peek_nth() {
        let cursor = Cursor::new("abc");
        assert_eq!(cursor.peek_nth(0), Some('a'));
        assert_eq!(cursor.peek_nth(1), Some('b'));
        assert_eq!(cursor.peek_nth(2), Some('c'));
        assert_eq!(cursor.peek_nth(3), None);
    }

    #[test]
    fn cursor_utf8() {
        let mut cursor = Cursor::new("héllo");

        cursor.advance(); // h (1 byte)
        assert_eq!(cursor.offset(), 1);
        assert_eq!(cursor.column, 2);

        cursor.advance(); // é (2 bytes)
        assert_eq!(cursor.offset(), 3);
        assert_eq!(cursor.column, 4); // Column counts bytes
    }

    #[test]
    fn is_ident() {
        assert!(is_ident_start('a'));
        assert!(is_ident_start('_'));
        assert!(!is_ident_start('0'));

        assert!(is_ident_continue('a'));
        assert!(is_ident_continue('0'));
        assert!(is_ident_continue('_'));
        assert!(!is_ident_continue('-'));
    }

    #[test]
    fn cursor_check() {
        let cursor = Cursor::new("hello");
        assert!(cursor.check(|c| c == 'h'));
        assert!(cursor.check(|c| c.is_alphabetic()));
        assert!(!cursor.check(|c| c.is_numeric()));

        let empty = Cursor::new("");
        assert!(!empty.check(|_| true));
    }

    #[test]
    fn cursor_check_with_predicate() {
        let cursor = Cursor::new("abc123");
        assert!(cursor.check(is_ident_start));
        assert!(cursor.check(is_ident_continue));

        let numeric = Cursor::new("123");
        assert!(!numeric.check(is_ident_start));
        assert!(numeric.check(|c| c.is_numeric()));
    }

    #[test]
    fn cursor_slice_from() {
        let mut cursor = Cursor::new("hello world");
        let start = cursor.offset();

        cursor.eat_while(|ch| "hello".contains(ch));
        assert_eq!(cursor.slice_from(start), "hello");

        cursor.eat(' ');
        let word_start = cursor.offset();
        cursor.eat_while(|ch| "world".contains(ch));
        assert_eq!(cursor.slice_from(word_start), "world");
    }

    #[test]
    fn cursor_slice_from_with_utf8() {
        let mut cursor = Cursor::new("héllo");
        let start = cursor.offset();

        cursor.advance(); // h
        cursor.advance(); // é
        assert_eq!(cursor.slice_from(start), "hé");
    }

    #[test]
    fn cursor_check_str() {
        let cursor = Cursor::new("hello world");
        assert!(cursor.check_str("hello"));
        assert!(cursor.check_str("hello world"));
        assert!(!cursor.check_str("world"));
        assert!(!cursor.check_str("hello!"));
    }

    #[test]
    fn cursor_source() {
        let source = "test source";
        let mut cursor = Cursor::new(source);

        assert_eq!(cursor.source(), source);

        // source() should return original even after advancing
        cursor.advance();
        cursor.advance();
        assert_eq!(cursor.source(), source);
    }

    #[test]
    fn cursor_line_and_column() {
        let mut cursor = Cursor::new("ab\ncd");

        assert_eq!(cursor.line(), 1);
        assert_eq!(cursor.column(), 1);

        cursor.advance(); // a
        assert_eq!(cursor.line(), 1);
        assert_eq!(cursor.column(), 2);

        cursor.advance(); // b
        assert_eq!(cursor.line(), 1);
        assert_eq!(cursor.column(), 3);

        cursor.advance(); // \n
        assert_eq!(cursor.line(), 2);
        assert_eq!(cursor.column(), 1);

        cursor.advance(); // c
        assert_eq!(cursor.line(), 2);
        assert_eq!(cursor.column(), 2);
    }

    #[test]
    fn cursor_advance_bytes() {
        let mut cursor = Cursor::new("hello world");
        let start = cursor.offset();

        cursor.advance_bytes(5); // "hello"
        assert_eq!(cursor.slice_from(start), "hello");
    }

    #[test]
    fn cursor_advance_bytes_with_newline() {
        let mut cursor = Cursor::new("ab\ncd");

        cursor.advance_bytes(3); // "ab\n"
        assert_eq!(cursor.line(), 2);
        assert_eq!(cursor.column(), 1);
    }

    #[test]
    fn cursor_peek_ascii_fast_path() {
        // ASCII characters should work via fast path
        let cursor = Cursor::new("hello");
        assert_eq!(cursor.peek(), Some('h'));

        // UTF-8 characters should fall back to iterator path
        let utf8_cursor = Cursor::new("héllo");
        assert_eq!(utf8_cursor.peek(), Some('h'));

        // Multi-byte character at start
        let multi_byte = Cursor::new("éllo");
        assert_eq!(multi_byte.peek(), Some('é'));
    }

    #[test]
    fn cursor_eat_while_ascii() {
        let mut cursor = Cursor::new("hello123 world");

        let ident = cursor.eat_while_ascii(|b| b.is_ascii_alphanumeric());
        assert_eq!(ident, "hello123");
        assert_eq!(cursor.offset(), 8);
        assert_eq!(cursor.column(), 9); // 1-indexed

        // Should stop at space
        assert_eq!(cursor.peek(), Some(' '));
    }

    #[test]
    fn cursor_eat_while_ascii_empty() {
        let mut cursor = Cursor::new(" hello");

        // Should return empty slice when first char doesn't match
        let result = cursor.eat_while_ascii(|b| b.is_ascii_alphanumeric());
        assert_eq!(result, "");
        assert_eq!(cursor.offset(), 0);
    }

    #[test]
    fn cursor_eat_while_ascii_stops_at_non_ascii() {
        let mut cursor = Cursor::new("helloéworld");

        // Should stop at the multi-byte 'é' character
        let result = cursor.eat_while_ascii(|b| b.is_ascii_alphanumeric() || b == b'\xc3');
        assert_eq!(result, "hello");
        assert_eq!(cursor.offset(), 5);
    }

    #[test]
    fn is_ident_continue_ascii_works() {
        assert!(is_ident_continue_ascii(b'a'));
        assert!(is_ident_continue_ascii(b'Z'));
        assert!(is_ident_continue_ascii(b'0'));
        assert!(is_ident_continue_ascii(b'9'));
        assert!(is_ident_continue_ascii(b'_'));
        assert!(!is_ident_continue_ascii(b' '));
        assert!(!is_ident_continue_ascii(b'-'));
        assert!(!is_ident_continue_ascii(b'.'));
    }
}
