//! Semantic error types for the AngelScript semantic analyzer.
//!
//! Provides comprehensive error reporting for semantic analysis with source location tracking
//! and helpful error messages.

use crate::lexer::Span;
use std::fmt;

/// A semantic error with location and diagnostic information.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticError {
    /// The type of error that occurred.
    pub kind: SemanticErrorKind,
    /// The location in source where the error occurred.
    pub span: Span,
    /// Additional context or message.
    pub message: String,
}

impl SemanticError {
    /// Create a new semantic error.
    pub fn new(kind: SemanticErrorKind, span: Span, message: impl Into<String>) -> Self {
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

impl fmt::Display for SemanticError {
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

impl std::error::Error for SemanticError {}

/// The kind of semantic error that occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticErrorKind {
    // Symbol resolution errors
    /// Reference to an undefined variable.
    UndefinedVariable,
    /// Reference to an undefined function.
    UndefinedFunction,
    /// Reference to an undefined type.
    UndefinedType,
    /// Duplicate declaration in the same scope.
    DuplicateDeclaration,
    /// Variable used in its own initializer.
    UseBeforeDefinition,

    // Context errors
    /// Return statement outside of a function.
    ReturnOutsideFunction,
    /// Break statement outside of a loop or switch.
    BreakOutsideLoop,
    /// Continue statement outside of a loop.
    ContinueOutsideLoop,

    // Type errors
    /// Type mismatch between expected and actual types.
    TypeMismatch,
    /// Invalid operation for the given types.
    InvalidOperation,
    /// Cannot assign to a non-mutable variable.
    AssignToImmutable,
    /// Cannot cast between incompatible types.
    InvalidCast,

    // Template errors
    /// Used template syntax on non-template type.
    NotATemplate,
    /// Wrong number of template arguments.
    WrongTemplateArgCount,

    // Inheritance errors
    /// Class inherits from itself (directly or indirectly).
    CircularInheritance,

    // Field/member errors
    /// Reference to an undefined field.
    UndefinedField,
    /// Reference to an undefined method.
    UndefinedMethod,

    // Function call errors
    /// Wrong number of arguments in function call.
    WrongArgumentCount,
    /// Cannot call a non-function value.
    NotCallable,

    // Other
    /// An internal semantic analyzer error (bug in analyzer).
    InternalError,
}

impl fmt::Display for SemanticErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use SemanticErrorKind::*;
        let msg = match self {
            UndefinedVariable => "undefined variable",
            UndefinedFunction => "undefined function",
            UndefinedType => "undefined type",
            DuplicateDeclaration => "duplicate declaration",
            UseBeforeDefinition => "use before definition",
            ReturnOutsideFunction => "return outside function",
            BreakOutsideLoop => "break outside loop or switch",
            ContinueOutsideLoop => "continue outside loop",
            TypeMismatch => "type mismatch",
            InvalidOperation => "invalid operation",
            AssignToImmutable => "assignment to immutable variable",
            InvalidCast => "invalid cast",
            NotATemplate => "not a template",
            WrongTemplateArgCount => "wrong number of template arguments",
            CircularInheritance => "circular inheritance",
            UndefinedField => "undefined field",
            UndefinedMethod => "undefined method",
            WrongArgumentCount => "wrong number of arguments",
            NotCallable => "not callable",
            InternalError => "internal semantic analyzer error",
        };
        write!(f, "{}", msg)
    }
}

/// A collection of semantic errors.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SemanticErrors {
    errors: Vec<SemanticError>,
}

impl SemanticErrors {
    /// Create a new empty error collection.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Add an error to the collection.
    pub fn push(&mut self, error: SemanticError) {
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
    pub fn errors(&self) -> &[SemanticError] {
        &self.errors
    }

    /// Consume and return the errors.
    pub fn into_vec(self) -> Vec<SemanticError> {
        self.errors
    }
}

impl From<SemanticError> for SemanticErrors {
    fn from(error: SemanticError) -> Self {
        let mut errors = SemanticErrors::new();
        errors.push(error);
        errors
    }
}

impl FromIterator<SemanticError> for SemanticErrors {
    fn from_iter<T: IntoIterator<Item = SemanticError>>(iter: T) -> Self {
        Self {
            errors: iter.into_iter().collect(),
        }
    }
}

impl fmt::Display for SemanticErrors {
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

impl std::error::Error for SemanticErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 6, 3),
            "variable 'foo' is not defined",
        );
        let display = format!("{}", error);
        assert!(display.contains("undefined variable"));
        assert!(display.contains("variable 'foo' is not defined"));
    }

    #[test]
    fn error_with_source() {
        let source = "int x = foo;\nint y = 10;";
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 9, 3),
            "variable 'foo' is not defined",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:9"));
        assert!(display.contains("int x = foo;"));
        assert!(display.contains("^~~")); // Multi-char pointer for "foo"
    }

    #[test]
    fn multiple_errors() {
        let mut errors = SemanticErrors::new();
        errors.push(SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error 1",
        ));
        errors.push(SemanticError::new(
            SemanticErrorKind::DuplicateDeclaration,
            Span::new(1, 6, 1),
            "error 2",
        ));
        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn all_semantic_error_kinds_display() {
        // Symbol resolution errors
        assert_eq!(format!("{}", SemanticErrorKind::UndefinedVariable), "undefined variable");
        assert_eq!(format!("{}", SemanticErrorKind::UndefinedFunction), "undefined function");
        assert_eq!(format!("{}", SemanticErrorKind::UndefinedType), "undefined type");
        assert_eq!(format!("{}", SemanticErrorKind::DuplicateDeclaration), "duplicate declaration");
        assert_eq!(format!("{}", SemanticErrorKind::UseBeforeDefinition), "use before definition");

        // Context errors
        assert_eq!(format!("{}", SemanticErrorKind::ReturnOutsideFunction), "return outside function");
        assert_eq!(format!("{}", SemanticErrorKind::BreakOutsideLoop), "break outside loop or switch");
        assert_eq!(format!("{}", SemanticErrorKind::ContinueOutsideLoop), "continue outside loop");

        // Type errors
        assert_eq!(format!("{}", SemanticErrorKind::TypeMismatch), "type mismatch");

        // Template errors
        assert_eq!(format!("{}", SemanticErrorKind::NotATemplate), "not a template");
        assert_eq!(format!("{}", SemanticErrorKind::WrongTemplateArgCount), "wrong number of template arguments");

        // Inheritance errors
        assert_eq!(format!("{}", SemanticErrorKind::CircularInheritance), "circular inheritance");

        // Other
        assert_eq!(format!("{}", SemanticErrorKind::InternalError), "internal semantic analyzer error");
    }

    #[test]
    fn display_with_source_single_char_span() {
        let source = "int x = y;";
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 9, 1), // "y"
            "variable 'y' is not defined",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:9"));
        assert!(display.contains("int x = y;"));
        assert!(display.contains("^")); // Single-char pointer
        assert!(!display.contains("~~")); // No tildes for single char
    }

    #[test]
    fn display_with_source_multichar_span() {
        let source = "int result = undefined_var;";
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 14, 13), // "undefined_var"
            "variable 'undefined_var' is not defined",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:14"));
        assert!(display.contains("int result = undefined_var;"));
        assert!(display.contains("^~~~~~~~~~~~~")); // Multi-char pointer (13 chars)
    }

    #[test]
    fn display_with_source_no_message() {
        let source = "int x;";
        let error = SemanticError::new(
            SemanticErrorKind::DuplicateDeclaration,
            Span::new(1, 5, 1),
            "",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("1:5"));
        assert!(display.contains("duplicate declaration"));
        assert!(!display.contains("  \n  ")); // Should not have empty message line
    }

    #[test]
    fn display_with_source_invalid_line() {
        let source = "int x;";
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(100, 1, 1), // Line that doesn't exist
            "error on non-existent line",
        );
        let display = error.display_with_source(source);
        // Should still show error header even if line not found
        assert!(display.contains("100:1"));
        assert!(display.contains("undefined variable"));
    }

    #[test]
    fn semantic_errors_empty() {
        let errors = SemanticErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);
        assert_eq!(format!("{}", errors), "no errors");
    }

    #[test]
    fn semantic_errors_single() {
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "single error",
        );
        let errors = SemanticErrors::from(error.clone());
        assert_eq!(errors.len(), 1);
        assert!(!errors.is_empty());
        let display = format!("{}", errors);
        assert!(display.contains("undefined variable"));
    }

    #[test]
    fn semantic_errors_multiple_display() {
        let mut errors = SemanticErrors::new();
        errors.push(SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error 1",
        ));
        errors.push(SemanticError::new(
            SemanticErrorKind::DuplicateDeclaration,
            Span::new(2, 1, 1),
            "error 2",
        ));
        errors.push(SemanticError::new(
            SemanticErrorKind::UseBeforeDefinition,
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
    fn semantic_errors_from_iter() {
        let error_vec = vec![
            SemanticError::new(SemanticErrorKind::UndefinedVariable, Span::new(1, 1, 1), "err1"),
            SemanticError::new(SemanticErrorKind::UndefinedFunction, Span::new(2, 1, 1), "err2"),
        ];

        let errors: SemanticErrors = error_vec.into_iter().collect();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn semantic_errors_into_vec() {
        let mut errors = SemanticErrors::new();
        errors.push(SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error",
        ));

        let vec = errors.into_vec();
        assert_eq!(vec.len(), 1);
    }

    #[test]
    fn semantic_errors_errors_method() {
        let mut errors = SemanticErrors::new();
        errors.push(SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error",
        ));

        let err_slice = errors.errors();
        assert_eq!(err_slice.len(), 1);
    }

    #[test]
    fn semantic_error_std_error_impl() {
        let error = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "test",
        );

        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn semantic_errors_std_error_impl() {
        let errors = SemanticErrors::new();

        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &errors;
    }

    #[test]
    fn context_errors() {
        let error = SemanticError::new(
            SemanticErrorKind::ReturnOutsideFunction,
            Span::new(5, 5, 6),
            "return statement must be inside a function",
        );
        let display = format!("{}", error);
        assert!(display.contains("return outside function"));

        let error = SemanticError::new(
            SemanticErrorKind::BreakOutsideLoop,
            Span::new(10, 5, 5),
            "break statement must be inside a loop or switch",
        );
        let display = format!("{}", error);
        assert!(display.contains("break outside loop or switch"));

        let error = SemanticError::new(
            SemanticErrorKind::ContinueOutsideLoop,
            Span::new(15, 5, 8),
            "continue statement must be inside a loop",
        );
        let display = format!("{}", error);
        assert!(display.contains("continue outside loop"));
    }

    #[test]
    fn use_before_definition_error() {
        let source = "int x = x + 1;";
        let error = SemanticError::new(
            SemanticErrorKind::UseBeforeDefinition,
            Span::new(1, 9, 1),
            "variable 'x' is used in its own initializer",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("use before definition"));
        assert!(display.contains("int x = x + 1;"));
        assert!(display.contains("variable 'x' is used in its own initializer"));
    }

    #[test]
    fn duplicate_declaration_error() {
        let source = "int x = 5;\nint x = 10;";
        let error = SemanticError::new(
            SemanticErrorKind::DuplicateDeclaration,
            Span::new(2, 5, 1),
            "variable 'x' is already declared in this scope",
        );
        let display = error.display_with_source(source);
        assert!(display.contains("duplicate declaration"));
        assert!(display.contains("int x = 10;"));
    }

    #[test]
    fn semantic_errors_equality() {
        let error1 = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error",
        );
        let error2 = SemanticError::new(
            SemanticErrorKind::UndefinedVariable,
            Span::new(1, 1, 1),
            "error",
        );
        assert_eq!(error1, error2);

        let errors1 = SemanticErrors::from(error1.clone());
        let errors2 = SemanticErrors::from(error2);
        assert_eq!(errors1, errors2);
    }
}
