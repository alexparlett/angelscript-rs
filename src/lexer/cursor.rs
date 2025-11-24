//! Low-level character iteration for the lexer.
//!
//! The [`Cursor`] provides peek/advance operations over source text
//! while tracking line and column positions.

use super::span::Position;

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

    /// Get the remaining source text from current position.
    #[inline]
    pub fn rest(&self) -> &'src str {
        self.rest
    }

    /// Current byte offset from start of source.
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Current position (offset, line, column).
    #[inline]
    pub fn position(&self) -> Position {
        Position::new(self.offset, self.line, self.column)
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
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    /// Peek at the nth character ahead (0 = current).
    #[inline]
    pub fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest.chars().nth(n)
    }

    /// Peek at the current byte without consuming it.
    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        self.rest.as_bytes().first().copied()
    }

    /// Peek at the nth byte ahead (0 = current).
    #[inline]
    pub fn peek_byte_nth(&self, n: usize) -> Option<u8> {
        self.rest.as_bytes().get(n).copied()
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
    pub fn advance(&mut self) -> Option<char> {
        let ch = self.rest.chars().next()?;
        let len = ch.len_utf8() as u32;

        self.rest = &self.rest[len as usize..];
        self.offset += len;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += len;
        }

        Some(ch)
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

    /// Consume if the upcoming bytes match the string.
    #[inline]
    pub fn eat_str(&mut self, s: &str) -> bool {
        if self.check_str(s) {
            self.advance_bytes(s.len());
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

    /// Get a slice of source from a starting offset to current position.
    #[inline]
    pub fn slice_from(&self, start: u32) -> &'src str {
        &self.source[start as usize..self.offset as usize]
    }

    /// Check if the next characters form an identifier continuation.
    ///
    /// Used to ensure keywords don't match partial identifiers.
    #[inline]
    pub fn followed_by_ident_char(&self) -> bool {
        self.check(is_ident_continue)
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
    fn cursor_position_tracking() {
        let mut cursor = Cursor::new("ab\ncd");

        cursor.advance(); // a
        assert_eq!(cursor.position(), Position::new(1, 1, 2));

        cursor.advance(); // b
        assert_eq!(cursor.position(), Position::new(2, 1, 3));

        cursor.advance(); // \n
        assert_eq!(cursor.position(), Position::new(3, 2, 1));

        cursor.advance(); // c
        assert_eq!(cursor.position(), Position::new(4, 2, 2));
    }

    #[test]
    fn cursor_eat() {
        let mut cursor = Cursor::new("hello");

        assert!(cursor.eat('h'));
        assert!(!cursor.eat('h')); // Already consumed
        assert!(cursor.eat('e'));
    }

    #[test]
    fn cursor_eat_str() {
        let mut cursor = Cursor::new("hello world");

        assert!(cursor.eat_str("hello"));
        assert!(!cursor.eat_str("world")); // Space first
        assert!(cursor.eat(' '));
        assert!(cursor.eat_str("world"));
        assert!(cursor.is_eof());
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
    fn cursor_rest() {
        let mut cursor = Cursor::new("hello world");
        assert_eq!(cursor.rest(), "hello world");

        cursor.advance(); // consume 'h'
        assert_eq!(cursor.rest(), "ello world");

        cursor.eat_str("ello ");
        assert_eq!(cursor.rest(), "world");
    }

    #[test]
    fn cursor_peek_byte() {
        let cursor = Cursor::new("abc");
        assert_eq!(cursor.peek_byte(), Some(b'a'));

        let cursor_empty = Cursor::new("");
        assert_eq!(cursor_empty.peek_byte(), None);
    }

    #[test]
    fn cursor_peek_byte_nth() {
        let cursor = Cursor::new("hello");
        assert_eq!(cursor.peek_byte_nth(0), Some(b'h'));
        assert_eq!(cursor.peek_byte_nth(1), Some(b'e'));
        assert_eq!(cursor.peek_byte_nth(4), Some(b'o'));
        assert_eq!(cursor.peek_byte_nth(5), None);
    }

    #[test]
    fn cursor_peek_byte_utf8() {
        let cursor = Cursor::new("héllo");
        assert_eq!(cursor.peek_byte_nth(0), Some(b'h'));
        // 'é' is 2 bytes: 0xC3 0xA9
        assert_eq!(cursor.peek_byte_nth(1), Some(0xC3));
        assert_eq!(cursor.peek_byte_nth(2), Some(0xA9));
        assert_eq!(cursor.peek_byte_nth(3), Some(b'l'));
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

        cursor.eat_str("hello");
        assert_eq!(cursor.slice_from(start), "hello");

        cursor.eat(' ');
        let word_start = cursor.offset();
        cursor.eat_str("world");
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
    fn cursor_followed_by_ident_char() {
        let cursor_yes = Cursor::new("abc");
        assert!(cursor_yes.followed_by_ident_char());

        let cursor_num = Cursor::new("123");
        assert!(cursor_num.followed_by_ident_char()); // digits continue idents

        let cursor_no = Cursor::new(" ");
        assert!(!cursor_no.followed_by_ident_char());

        let cursor_eof = Cursor::new("");
        assert!(!cursor_eof.followed_by_ident_char());
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
        assert_eq!(cursor.rest(), " world");
    }

    #[test]
    fn cursor_advance_bytes_with_newline() {
        let mut cursor = Cursor::new("ab\ncd");

        cursor.advance_bytes(3); // "ab\n"
        assert_eq!(cursor.line(), 2);
        assert_eq!(cursor.column(), 1);
        assert_eq!(cursor.rest(), "cd");
    }
}
