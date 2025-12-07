//! Lexer error types and diagnostic formatting.
//!
//! Provides Rust-style error messages with source context, similar to
//! `rustc` error output.

use super::span::Span;
use std::fmt;

/// An error encountered during lexical analysis.
#[derive(Debug, Clone)]
pub struct LexerError {
    /// Location in source where the error occurred.
    pub span: Span,
    /// Human-readable error message.
    pub message: String,
}

impl LexerError {
    /// Create a new lexer error.
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    /// Create an "unexpected character" error.
    pub fn unexpected_char(ch: char, span: Span) -> Self {
        Self::new(
            span,
            format!("unexpected character '{}'", ch.escape_default()),
        )
    }

    /// Create an "unterminated string" error.
    pub fn unterminated_string(span: Span) -> Self {
        Self::new(
            span,
            "unterminated string literal".to_string(),
        )
    }

    /// Create an "unterminated heredoc" error.
    pub fn unterminated_heredoc(span: Span) -> Self {
        Self::new(
            span,
            "unterminated heredoc string (expected closing `\"\"\"`)".to_string(),
        )
    }

    /// Create an "unterminated comment" error.
    pub fn unterminated_comment(span: Span) -> Self {
        Self::new(
            span,
            "unterminated block comment (expected closing `*/`)".to_string(),
        )
    }

    /// Create an "invalid number literal" error.
    pub fn invalid_number(span: Span, detail: impl Into<String>) -> Self {
        Self::new(
            span,
            format!("invalid number literal: {}", detail.into()),
        )
    }
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LexerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_char_error() {
        let err = LexerError::unexpected_char('$', Span::new(1, 6, 1));
        assert_eq!(err.span, Span::new(1, 6, 1));
    }

    #[test]
    fn error_new() {
        let err = LexerError::new(
            Span::new(2, 10, 5),
            "custom message",
        );
        assert_eq!(err.span, Span::new(2, 10, 5));
        assert_eq!(err.message, "custom message");
    }

    #[test]
    fn unterminated_string_error() {
        let err = LexerError::unterminated_string(Span::new(3, 5, 10));
        assert_eq!(err.span, Span::new(3, 5, 10));
        assert_eq!(err.message, "unterminated string literal");
    }

    #[test]
    fn unterminated_heredoc_error() {
        let err = LexerError::unterminated_heredoc(Span::new(5, 1, 20));
        assert_eq!(err.span, Span::new(5, 1, 20));
        assert!(err.message.contains("heredoc"));
        assert!(err.message.contains("\"\"\""));
    }

    #[test]
    fn unterminated_comment_error() {
        let err = LexerError::unterminated_comment(Span::new(10, 15, 5));
        assert_eq!(err.span, Span::new(10, 15, 5));
        assert!(err.message.contains("block comment"));
        assert!(err.message.contains("*/"));
    }

    #[test]
    fn invalid_number_error() {
        let err = LexerError::invalid_number(Span::new(2, 8, 4), "no digits after 0x");
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
    fn error_display_trait() {
        let err = LexerError::new(
            Span::new(1, 1, 1),
            "test message",
        );
        assert_eq!(format!("{}", err), "test message");
    }
}
