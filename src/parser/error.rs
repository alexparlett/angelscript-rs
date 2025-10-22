use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

impl Span {
    pub fn new(start: Position, end: Position, source: String) -> Self {
        Self { start, end, source }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start,
            end: other.end,
            source: format!("{}{}", self.source, other.source),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.start.line == self.end.line {
            write!(f, "{}:{}", self.start.line, self.start.column)
        } else {
            write!(
                f,
                "{}:{}-{}:{}",
                self.start.line, self.start.column, self.end.line, self.end.column
            )
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Unexpected token at {span}: expected {expected}, found '{found}'")]
    UnexpectedToken {
        span: Span,
        expected: String,
        found: String,
    },

    #[error("Unexpected end of input at {span}: expected {expected}")]
    UnexpectedEof { span: Span, expected: String },

    #[error("Invalid operator at {span}: '{operator}'")]
    InvalidOperator { span: Span, operator: String },

    #[error("Invalid expression at {span}: {message}")]
    InvalidExpression { span: Span, message: String },

    #[error("Unmatched delimiter at {span}: expected '{expected}', found '{found}'")]
    UnmatchedDelimiter {
        span: Span,
        expected: char,
        found: String,
    },

    #[error("Invalid number literal at {span}: {message}")]
    InvalidNumber { span: Span, message: String },

    #[error("Invalid string literal at {span}: {message}")]
    InvalidString { span: Span, message: String },

    #[error("Type error at {span}: {message}")]
    TypeError { span: Span, message: String },

    #[error("Syntax error at {span}: {message}")]
    SyntaxError { span: Span, message: String },
}

impl ParseError {
    pub fn span(&self) -> &Span {
        match self {
            ParseError::UnexpectedToken { span, .. }
            | ParseError::UnexpectedEof { span, .. }
            | ParseError::InvalidOperator { span, .. }
            | ParseError::InvalidExpression { span, .. }
            | ParseError::UnmatchedDelimiter { span, .. }
            | ParseError::InvalidNumber { span, .. }
            | ParseError::InvalidString { span, .. }
            | ParseError::TypeError { span, .. }
            | ParseError::SyntaxError { span, .. } => span,
        }
    }

    /// Format error with source context
    pub fn format_with_source(&self, source: &str) -> String {
        let span = match self {
            _ => self.span(),
        };

        let lines: Vec<&str> = source.lines().collect();
        let line_idx = span.start.line.saturating_sub(1);

        if line_idx >= lines.len() {
            return self.to_string();
        }

        let line = lines[line_idx];
        let col = span.start.column.saturating_sub(1);
        let length = span.end.offset.saturating_sub(span.start.offset).max(1);

        format!(
            "{}\n  --> {}:{}\n   |\n{:3} | {}\n   | {}{}",
            self,
            span.start.line,
            span.start.column,
            span.start.line,
            line,
            " ".repeat(col),
            "^".repeat(length.min(line.len().saturating_sub(col)))
        )
    }
}

pub type Result<T> = std::result::Result<T, ParseError>;
