//! Lexer error types and diagnostic formatting.
//!
//! Provides Rust-style error messages with source context, similar to
//! `rustc` error output.

use super::span::Span;
use std::fmt;

/// An error encountered during lexical analysis.
#[derive(Debug, Clone)]
pub struct LexerError {
    /// The kind of error.
    pub kind: LexerErrorKind,
    /// Location in source where the error occurred.
    pub span: Span,
    /// Human-readable error message.
    pub message: String,
}

impl LexerError {
    /// Create a new lexer error.
    pub fn new(kind: LexerErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }

    /// Create an "unexpected character" error.
    pub fn unexpected_char(ch: char, span: Span) -> Self {
        Self::new(
            LexerErrorKind::UnexpectedCharacter,
            span,
            format!("unexpected character '{}'", ch.escape_default()),
        )
    }

    /// Create an "unterminated string" error.
    pub fn unterminated_string(span: Span) -> Self {
        Self::new(
            LexerErrorKind::UnterminatedString,
            span,
            "unterminated string literal".to_string(),
        )
    }

    /// Create an "unterminated heredoc" error.
    pub fn unterminated_heredoc(span: Span) -> Self {
        Self::new(
            LexerErrorKind::UnterminatedString,
            span,
            "unterminated heredoc string (expected closing `\"\"\"`)".to_string(),
        )
    }

    /// Create an "unterminated comment" error.
    pub fn unterminated_comment(span: Span) -> Self {
        Self::new(
            LexerErrorKind::UnterminatedComment,
            span,
            "unterminated block comment (expected closing `*/`)".to_string(),
        )
    }

    /// Create an "invalid number literal" error.
    pub fn invalid_number(span: Span, detail: impl Into<String>) -> Self {
        Self::new(
            LexerErrorKind::InvalidNumberLiteral,
            span,
            format!("invalid number literal: {}", detail.into()),
        )
    }

    /// Create an "invalid escape sequence" error.
    pub fn invalid_escape(ch: char, span: Span) -> Self {
        Self::new(
            LexerErrorKind::InvalidEscapeSequence,
            span,
            format!("invalid escape sequence '\\{}'", ch.escape_default()),
        )
    }

    /// Format this error with source context (Rust-style).
    ///
    /// Returns a multi-line string like:
    /// ```text
    /// error: unterminated string literal
    ///  --> script.as:3:15
    ///   |
    /// 3 |     let x = "hello
    ///   |             ^ unclosed string
    /// ```
    pub fn display(&self, source: &str, filename: &str) -> String {
        let mut out = String::new();

        // Use line/col directly from span
        let line = self.span.line;
        let col = self.span.col;

        // Error header
        out.push_str(&format!("error: {}\n", self.message));

        // Location
        out.push_str(&format!(" --> {}:{}:{}\n", filename, line, col));

        // Find the source line boundaries
        let (line_start, line_end) = find_line_bounds(source, line as usize);

        // Source line
        let line_num_width = line.to_string().len();
        let padding = " ".repeat(line_num_width);

        out.push_str(&format!("{}  |\n", padding));

        // Extract the source line
        let source_line = &source[line_start..line_end];
        out.push_str(&format!("{} | {}\n", line, source_line));

        // Underline (start at column position)
        let underline_start = (col as usize).saturating_sub(1);
        let underline_len = (self.span.len as usize).max(1);

        out.push_str(&format!(
            "{}  | {}{}",
            padding,
            " ".repeat(underline_start),
            "^".repeat(underline_len)
        ));

        out
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LexerError {}

/// Categories of lexer errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexerErrorKind {
    /// Unexpected character in input.
    UnexpectedCharacter,
    /// String literal not closed.
    UnterminatedString,
    /// Block comment not closed.
    UnterminatedComment,
    /// Invalid number literal syntax.
    InvalidNumberLiteral,
    /// Invalid escape sequence in string.
    InvalidEscapeSequence,
}

impl LexerErrorKind {
    /// Get a short description of this error kind.
    pub fn description(self) -> &'static str {
        match self {
            Self::UnexpectedCharacter => "unexpected character",
            Self::UnterminatedString => "unterminated string",
            Self::UnterminatedComment => "unterminated comment",
            Self::InvalidNumberLiteral => "invalid number",
            Self::InvalidEscapeSequence => "invalid escape",
        }
    }
}

/// Find the byte boundaries for a given line number.
///
/// Returns `(line_start, line_end)` where:
/// - `line_start` is byte offset of line start
/// - `line_end` is byte offset of line end (excluding newline)
/// - `line_num` is 1-indexed
fn find_line_bounds(source: &str, line_num: usize) -> (usize, usize) {
    let mut current_line = 1;
    let mut line_start = 0;

    for (i, ch) in source.char_indices() {
        if current_line == line_num {
            // Found the target line, now find its end
            let line_end = source[i..]
                .find('\n')
                .map(|offset| i + offset)
                .unwrap_or(source.len());
            return (line_start, line_end);
        }

        if ch == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // If we didn't find the line, return the last line
    (line_start, source.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_line_bounds_simple() {
        let source = "abc\ndef\nghi";
        // Lines: 1="abc", 2="def", 3="ghi"

        assert_eq!(find_line_bounds(source, 1), (0, 3));
        assert_eq!(find_line_bounds(source, 2), (4, 7));
        assert_eq!(find_line_bounds(source, 3), (8, 11));
    }

    #[test]
    fn error_display() {
        let source = "let x = \"hello";
        let err = LexerError::unterminated_string(Span::new(1, 9, 6));
        let display = err.display(source, "test.as");

        assert!(display.contains("error: unterminated string"));
        assert!(display.contains("--> test.as:1:9"));
        assert!(display.contains("let x = \"hello"));
        assert!(display.contains("^^^^^^"));
    }

    #[test]
    fn unexpected_char_error() {
        let err = LexerError::unexpected_char('$', Span::new(1, 6, 1));
        assert_eq!(err.kind, LexerErrorKind::UnexpectedCharacter);
        assert_eq!(err.span, Span::new(1, 6, 1));
    }

    #[test]
    fn error_new() {
        let err = LexerError::new(
            LexerErrorKind::InvalidNumberLiteral,
            Span::new(2, 10, 5),
            "custom message",
        );
        assert_eq!(err.kind, LexerErrorKind::InvalidNumberLiteral);
        assert_eq!(err.span, Span::new(2, 10, 5));
        assert_eq!(err.message, "custom message");
    }

    #[test]
    fn unterminated_string_error() {
        let err = LexerError::unterminated_string(Span::new(3, 5, 10));
        assert_eq!(err.kind, LexerErrorKind::UnterminatedString);
        assert_eq!(err.span, Span::new(3, 5, 10));
        assert_eq!(err.message, "unterminated string literal");
    }

    #[test]
    fn unterminated_heredoc_error() {
        let err = LexerError::unterminated_heredoc(Span::new(5, 1, 20));
        assert_eq!(err.kind, LexerErrorKind::UnterminatedString);
        assert_eq!(err.span, Span::new(5, 1, 20));
        assert!(err.message.contains("heredoc"));
        assert!(err.message.contains("\"\"\""));
    }

    #[test]
    fn unterminated_comment_error() {
        let err = LexerError::unterminated_comment(Span::new(10, 15, 5));
        assert_eq!(err.kind, LexerErrorKind::UnterminatedComment);
        assert_eq!(err.span, Span::new(10, 15, 5));
        assert!(err.message.contains("block comment"));
        assert!(err.message.contains("*/"));
    }

    #[test]
    fn invalid_number_error() {
        let err = LexerError::invalid_number(Span::new(2, 8, 4), "no digits after 0x");
        assert_eq!(err.kind, LexerErrorKind::InvalidNumberLiteral);
        assert_eq!(err.span, Span::new(2, 8, 4));
        assert!(err.message.contains("invalid number literal"));
        assert!(err.message.contains("no digits after 0x"));
    }

    #[test]
    fn invalid_number_error_with_string() {
        let detail = String::from("floating point overflow");
        let err = LexerError::invalid_number(Span::new(1, 1, 10), detail);
        assert!(err.message.contains("floating point overflow"));
    }

    #[test]
    fn invalid_escape_error() {
        let err = LexerError::invalid_escape('x', Span::new(1, 12, 2));
        assert_eq!(err.kind, LexerErrorKind::InvalidEscapeSequence);
        assert_eq!(err.span, Span::new(1, 12, 2));
        assert!(err.message.contains("invalid escape"));
        assert!(err.message.contains("\\x"));
    }

    #[test]
    fn invalid_escape_with_special_char() {
        let err = LexerError::invalid_escape('\t', Span::new(1, 5, 2));
        assert!(err.message.contains("\\\\t")); // Should be escaped in message
    }

    #[test]
    fn error_kind_descriptions() {
        assert_eq!(
            LexerErrorKind::UnexpectedCharacter.description(),
            "unexpected character"
        );
        assert_eq!(
            LexerErrorKind::UnterminatedString.description(),
            "unterminated string"
        );
        assert_eq!(
            LexerErrorKind::UnterminatedComment.description(),
            "unterminated comment"
        );
        assert_eq!(
            LexerErrorKind::InvalidNumberLiteral.description(),
            "invalid number"
        );
        assert_eq!(
            LexerErrorKind::InvalidEscapeSequence.description(),
            "invalid escape"
        );
    }

    #[test]
    fn error_display_trait() {
        let err = LexerError::new(
            LexerErrorKind::UnexpectedCharacter,
            Span::new(1, 1, 1),
            "test message",
        );
        assert_eq!(format!("{}", err), "test message");
    }

    #[test]
    fn error_multiline_source() {
        let source = "line 1\nline 2\nline 3 with error\nline 4";
        let err = LexerError::unexpected_char('$', Span::new(3, 6, 1));
        let display = err.display(source, "multi.as");

        assert!(display.contains("multi.as:3:6"));
        assert!(display.contains("line 3 with error"));
        assert!(display.contains("     ^")); // Underline at column 6
    }
}
