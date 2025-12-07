//! Parse error types for the AngelScript parser.
//!
//! Provides comprehensive error reporting with source location tracking
//! and helpful error messages.

use angelscript_core::Span;
use std::fmt;

/// A parse error with location and diagnostic information.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The type of error that occurred.
    pub kind: ParseErrorKind,
    /// The location in source where the error occurred.
    pub span: Span,
    /// Additional context or message.
    pub message: String,
}

impl ParseError {
    /// Create a new parse error.
    pub fn new(kind: ParseErrorKind, span: Span, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
        }
    }

    /// Format the error with source context for display.
    pub fn display_with_source(&self, source: &str) -> String {
        let mut output = String::new();

        // Use line and column directly from span
        let line = self.span.line;
        let column = self.span.col;

        // Error header
        output.push_str(&format!("Error at {}:{}: {}\n", line, column, self.kind));

        // Add custom message if present
        if !self.message.is_empty() {
            output.push_str(&format!("  {}\n", self.message));
        }

        // Show the relevant source line
        if let Some(line_text) = Self::get_line(source, line) {
            output.push_str("  |\n");
            output.push_str(&format!("{:>3} | {}\n", line, line_text));

            // Add a caret pointer
            let indent = " ".repeat(column as usize - 1);
            let pointer = if self.span.len <= 1 {
                "^".to_string()
            } else {
                "^".to_string() + &"~".repeat((self.span.len - 1) as usize)
            };
            output.push_str(&format!("  | {}{}\n", indent, pointer));
        }

        output
    }

    /// Get the text of a specific line (1-indexed).
    fn get_line(source: &str, line_num: u32) -> Option<String> {
        source
            .lines()
            .nth(line_num as usize - 1)
            .map(|s| s.to_string())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {:?}{}",
            self.kind,
            self.span,
            if self.message.is_empty() {
                String::new()
            } else {
                format!(": {}", self.message)
            }
        )
    }
}

impl std::error::Error for ParseError {}

/// The kind of parse error that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    // Token-level errors
    /// Expected a specific token but found something else.
    ExpectedToken,
    /// Unexpected token in this context.
    UnexpectedToken,
    /// Unexpected end of file.
    UnexpectedEof,
    
    // Expression errors
    /// Expected an expression.
    ExpectedExpression,
    /// Expected an operator.
    ExpectedOperator,
    /// Invalid expression syntax.
    InvalidExpression,
    /// Expected a primary expression (literal, identifier, etc.).
    ExpectedPrimary,
    
    // Type errors
    /// Expected a type expression.
    ExpectedType,
    /// Invalid type syntax.
    InvalidType,
    /// Expected template argument list.
    ExpectedTemplateArgs,
    
    // Statement errors
    /// Expected a statement.
    ExpectedStatement,
    /// Invalid statement syntax.
    InvalidStatement,
    /// Expected a block (statement surrounded by braces).
    ExpectedBlock,
    
    // Declaration errors
    /// Expected a declaration.
    ExpectedDeclaration,
    /// Invalid declaration syntax.
    InvalidDeclaration,
    /// Expected function parameters.
    ExpectedParameters,
    /// Expected a class member.
    ExpectedClassMember,
    /// Expected an interface method.
    ExpectedInterfaceMethod,
    
    // Identifier errors
    /// Expected an identifier.
    ExpectedIdentifier,
    /// Duplicate identifier in this scope.
    DuplicateIdentifier,
    
    // Scope/namespace errors
    /// Invalid scope resolution.
    InvalidScope,
    /// Expected namespace.
    ExpectedNamespace,
    
    // Control flow errors
    /// Break outside of loop or switch.
    BreakOutsideLoop,
    /// Continue outside of loop.
    ContinueOutsideLoop,
    
    // Syntax errors
    /// Mismatched delimiters (brackets, parens, braces).
    MismatchedDelimiter,
    /// Missing semicolon.
    MissingSemicolon,
    /// Invalid syntax.
    InvalidSyntax,
    
    // Modifier errors
    /// Invalid modifier for this declaration.
    InvalidModifier,
    /// Conflicting modifiers.
    ConflictingModifiers,
    
    // Other
    /// An internal parser error (bug in parser).
    InternalError,
    /// Feature not yet implemented.
    NotImplemented,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseErrorKind::*;
        let msg = match self {
            ExpectedToken => "expected token",
            UnexpectedToken => "unexpected token",
            UnexpectedEof => "unexpected end of file",
            ExpectedExpression => "expected expression",
            ExpectedOperator => "expected operator",
            InvalidExpression => "invalid expression",
            ExpectedPrimary => "expected primary expression",
            ExpectedType => "expected type",
            InvalidType => "invalid type",
            ExpectedTemplateArgs => "expected template arguments",
            ExpectedStatement => "expected statement",
            InvalidStatement => "invalid statement",
            ExpectedBlock => "expected block",
            ExpectedDeclaration => "expected declaration",
            InvalidDeclaration => "invalid declaration",
            ExpectedParameters => "expected function parameters",
            ExpectedClassMember => "expected class member",
            ExpectedInterfaceMethod => "expected interface method",
            ExpectedIdentifier => "expected identifier",
            DuplicateIdentifier => "duplicate identifier",
            InvalidScope => "invalid scope resolution",
            ExpectedNamespace => "expected namespace",
            BreakOutsideLoop => "break outside of loop or switch",
            ContinueOutsideLoop => "continue outside of loop",
            MismatchedDelimiter => "mismatched delimiter",
            MissingSemicolon => "missing semicolon",
            InvalidSyntax => "invalid syntax",
            InvalidModifier => "invalid modifier",
            ConflictingModifiers => "conflicting modifiers",
            InternalError => "internal parser error",
            NotImplemented => "not yet implemented",
        };
        write!(f, "{}", msg)
    }
}

/// A collection of parse errors.
#[derive(Debug, Clone, Default)]
pub struct ParseErrors {
    errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Create a new empty error collection.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Add an error to the collection.
    pub fn push(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if there are any errors.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Get all errors.
    pub fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    /// Consume and return the errors.
    pub fn into_vec(self) -> Vec<ParseError> {
        self.errors
    }
}

impl From<ParseError> for ParseErrors {
    fn from(error: ParseError) -> Self {
        let mut errors = ParseErrors::new();
        errors.push(error);
        errors
    }
}

impl FromIterator<ParseError> for ParseErrors {
    fn from_iter<T: IntoIterator<Item = ParseError>>(iter: T) -> Self {
        Self {
            errors: iter.into_iter().collect(),
        }
    }
}

impl fmt::Display for ParseErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() {
            write!(f, "no errors")
        } else if self.errors.len() == 1 {
            write!(f, "{}", self.errors[0])
        } else {
            writeln!(f, "{} errors:", self.errors.len())?;
            for (i, error) in self.errors.iter().enumerate() {
                writeln!(f, "  {}: {}", i + 1, error)?;
            }
            Ok(())
        }
    }
}

impl std::error::Error for ParseErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 6, 3),
            "expected ';'",
        );
        let display = format!("{}", error);
        assert!(display.contains("expected token"));
        assert!(display.contains("expected ';'"));
    }

    #[test]
    fn error_with_source() {
        let source = "int x = 5\nint y = 10;";
        let error = ParseError::new(
            ParseErrorKind::MissingSemicolon,
            Span::new(1, 10, 0),
            "expected ';' after expression",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:10"));
        assert!(display.contains("int x = 5"));
    }

    #[test]
    fn multiple_errors() {
        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "error 1",
        ));
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 6, 1),
            "error 2",
        ));
        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn all_parse_error_kinds_display() {
        // Token-level errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedToken), "expected token");
        assert_eq!(format!("{}", ParseErrorKind::UnexpectedToken), "unexpected token");
        assert_eq!(format!("{}", ParseErrorKind::UnexpectedEof), "unexpected end of file");

        // Expression errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedExpression), "expected expression");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedOperator), "expected operator");
        assert_eq!(format!("{}", ParseErrorKind::InvalidExpression), "invalid expression");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedPrimary), "expected primary expression");

        // Type errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedType), "expected type");
        assert_eq!(format!("{}", ParseErrorKind::InvalidType), "invalid type");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedTemplateArgs), "expected template arguments");

        // Statement errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedStatement), "expected statement");
        assert_eq!(format!("{}", ParseErrorKind::InvalidStatement), "invalid statement");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedBlock), "expected block");

        // Declaration errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedDeclaration), "expected declaration");
        assert_eq!(format!("{}", ParseErrorKind::InvalidDeclaration), "invalid declaration");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedParameters), "expected function parameters");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedClassMember), "expected class member");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedInterfaceMethod), "expected interface method");

        // Identifier errors
        assert_eq!(format!("{}", ParseErrorKind::ExpectedIdentifier), "expected identifier");
        assert_eq!(format!("{}", ParseErrorKind::DuplicateIdentifier), "duplicate identifier");

        // Scope/namespace errors
        assert_eq!(format!("{}", ParseErrorKind::InvalidScope), "invalid scope resolution");
        assert_eq!(format!("{}", ParseErrorKind::ExpectedNamespace), "expected namespace");

        // Control flow errors
        assert_eq!(format!("{}", ParseErrorKind::BreakOutsideLoop), "break outside of loop or switch");
        assert_eq!(format!("{}", ParseErrorKind::ContinueOutsideLoop), "continue outside of loop");

        // Syntax errors
        assert_eq!(format!("{}", ParseErrorKind::MismatchedDelimiter), "mismatched delimiter");
        assert_eq!(format!("{}", ParseErrorKind::MissingSemicolon), "missing semicolon");
        assert_eq!(format!("{}", ParseErrorKind::InvalidSyntax), "invalid syntax");

        // Modifier errors
        assert_eq!(format!("{}", ParseErrorKind::InvalidModifier), "invalid modifier");
        assert_eq!(format!("{}", ParseErrorKind::ConflictingModifiers), "conflicting modifiers");

        // Other
        assert_eq!(format!("{}", ParseErrorKind::InternalError), "internal parser error");
        assert_eq!(format!("{}", ParseErrorKind::NotImplemented), "not yet implemented");
    }

    #[test]
    fn display_with_source_multichar_span() {
        let source = "int abc = 123;";
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 5, 3), // "abc"
            "test error",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:5"));
        assert!(display.contains("int abc = 123;"));
        assert!(display.contains("^~~")); // Multi-char pointer
    }

    #[test]
    fn display_with_source_no_message() {
        let source = "int x;";
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 5, 1),
            "",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:5"));
        assert!(!display.contains("  \n  ")); // Should not have empty message line
    }

    #[test]
    fn display_with_source_invalid_line() {
        let source = "int x;";
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(100, 1, 1), // Line that doesn't exist
            "error on non-existent line",
        );
        let display = error.display_with_source(source);
        // Should still show error header even if line not found
        assert!(display.contains("100:1"));
    }

    #[test]
    fn parse_errors_empty() {
        let errors = ParseErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);
        assert_eq!(format!("{}", errors), "no errors");
    }

    #[test]
    fn parse_errors_single() {
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "single error",
        );
        let errors = ParseErrors::from(error.clone());
        assert_eq!(errors.len(), 1);
        assert!(!errors.is_empty());
        let display = format!("{}", errors);
        assert!(display.contains("expected token"));
    }

    #[test]
    fn parse_errors_multiple_display() {
        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "error 1",
        ));
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedExpression,
            Span::new(2, 1, 1),
            "error 2",
        ));
        errors.push(ParseError::new(
            ParseErrorKind::MissingSemicolon,
            Span::new(3, 1, 1),
            "error 3",
        ));

        let display = format!("{}", errors);
        assert!(display.contains("3 errors:"));
        assert!(display.contains("1:"));
        assert!(display.contains("2:"));
        assert!(display.contains("3:"));
    }

    #[test]
    fn parse_errors_from_iter() {
        let error_vec = vec![
            ParseError::new(ParseErrorKind::ExpectedToken, Span::new(1, 1, 1), "err1"),
            ParseError::new(ParseErrorKind::ExpectedType, Span::new(2, 1, 1), "err2"),
        ];

        let errors: ParseErrors = error_vec.into_iter().collect();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn parse_errors_into_vec() {
        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "error",
        ));

        let vec = errors.into_vec();
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn parse_errors_errors_method() {
        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "error",
        ));

        let err_slice = errors.errors();
        assert_eq!(err_slice.len(), 1);
    }

    #[test]
    fn parse_error_std_error_impl() {
        let error = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "test",
        );

        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn parse_errors_std_error_impl() {
        let errors = ParseErrors::new();

        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &errors;
    }
}
