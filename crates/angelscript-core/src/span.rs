//! Source location tracking for error reporting.
//!
//! Provides [`Span`] to track where tokens and errors occur in source code.

use std::fmt;

/// A span of source code, represented by its starting position.
///
/// Similar to Rust compiler diagnostics, we track the line:column
/// where a token starts for debugging and error reporting.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    /// Line number (1-indexed).
    pub line: u32,
    /// Column number (1-indexed, byte-based).
    pub col: u32,
    /// Length in bytes (for additional context).
    pub len: u32,
}

impl Span {
    /// Create a new span from a line, column, and length.
    #[inline]
    pub fn new(line: u32, col: u32, len: u32) -> Self {
        Self { line, col, len }
    }

    /// Create a zero-length span at a position.
    #[inline]
    pub fn point(line: u32, col: u32) -> Self {
        Self { line, col, len: 0 }
    }

    /// Whether this span is empty (zero length).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The length of this span in bytes.
    #[inline]
    pub fn len(&self) -> u32 {
        self.len
    }

    /// Merge two spans into one that starts at the first span and extends to cover both.
    ///
    /// Note: This assumes spans are on the same line or properly ordered.
    /// The resulting span starts at `self` and extends to include `other`.
    #[inline]
    pub fn merge(self, other: Span) -> Span {
        if self.line == other.line {
            // Same line: calculate total length
            let start_col = self.col.min(other.col);
            let end_col = (other.col + other.len).max(self.col + self.len);
            Span {
                line: self.line,
                col: start_col,
                len: end_col - start_col,
            }
        } else {
            // Different lines: just use the first span's position with combined length
            // This is a simplification - for multi-line spans we just approximate
            Span {
                line: self.line,
                col: self.col,
                len: self.len + other.len,
            }
        }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_basics() {
        let span = Span::new(1, 5, 10);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());

        let empty = Span::point(1, 5);
        assert!(empty.is_empty());
    }

    #[test]
    fn span_display() {
        let span = Span::new(3, 15, 5);
        assert_eq!(format!("{}", span), "3:15");
    }

    #[test]
    fn span_merge_same_line() {
        // Two spans on the same line, non-overlapping
        let span1 = Span::new(1, 5, 3); // "foo" at 1:5
        let span2 = Span::new(1, 10, 3); // "bar" at 1:10
        let merged = span1.merge(span2);

        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 5); // Starts at first span
        assert_eq!(merged.len, 8); // 5 to 13 (10 + 3)
    }

    #[test]
    fn span_merge_same_line_overlapping() {
        // Two overlapping spans on the same line
        let span1 = Span::new(1, 5, 5); // 1:5-10
        let span2 = Span::new(1, 8, 4); // 1:8-12
        let merged = span1.merge(span2);

        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 5);
        assert_eq!(merged.len, 7); // 5 to 12
    }

    #[test]
    fn span_merge_same_line_reverse_order() {
        // Merge with second span before first
        let span1 = Span::new(1, 10, 3);
        let span2 = Span::new(1, 5, 3);
        let merged = span1.merge(span2);

        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 5); // Uses minimum col
        assert_eq!(merged.len, 8); // 5 to 13
    }

    #[test]
    fn span_merge_with_point_span() {
        // Merge with a zero-length span
        let span = Span::new(1, 5, 10);
        let point = Span::point(1, 8);
        let merged = span.merge(point);

        assert_eq!(merged.line, 1);
        assert_eq!(merged.col, 5);
        assert_eq!(merged.len, 10); // Point doesn't extend the span
    }

    #[test]
    fn span_merge_different_lines() {
        // Spans on different lines - simplified handling
        let span1 = Span::new(1, 5, 10);
        let span2 = Span::new(3, 10, 5);
        let merged = span1.merge(span2);

        assert_eq!(merged.line, 1); // Uses first span's line
        assert_eq!(merged.col, 5); // Uses first span's column
        assert_eq!(merged.len, 15); // Sum of lengths (approximation)
    }
}
