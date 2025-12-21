//! Unified error types for AngelScript.
//!
//! This module provides a consistent error type hierarchy for all phases
//! of AngelScript processing: lexing, parsing, registration, compilation, and runtime.
//!
//! ## Error Hierarchy
//!
//! ```text
//! AngelScriptError (top-level wrapper)
//! ├── LexError        - Lexer/tokenization errors
//! ├── ParseError      - Parser errors (with ParseErrorKind)
//! ├── RegistrationError - Type/function registration errors
//! ├── CompilationError  - Semantic analysis and compilation errors
//! └── RuntimeError      - Execution/runtime errors
//! ```
//!
//! ## Usage
//!
//! Each phase-specific error type can be used directly for fine-grained handling,
//! or converted to `AngelScriptError` for unified error handling:
//!
//! ```ignore
//! use angelscript_core::{AngelScriptError, ParseError, Span};
//!
//! fn compile(source: &str) -> Result<(), AngelScriptError> {
//!     let tokens = lex(source)?;  // LexError -> AngelScriptError
//!     let ast = parse(tokens)?;   // ParseError -> AngelScriptError
//!     Ok(())
//! }
//! ```

use thiserror::Error;

use crate::Span;

// ============================================================================
// Lexer Errors
// ============================================================================

/// Errors that occur during lexical analysis (tokenization).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum LexError {
    /// An unexpected character was encountered.
    #[error("unexpected character '{ch}' at {span}")]
    UnexpectedChar { ch: char, span: Span },

    /// A string literal was not properly terminated.
    #[error("unterminated string at {span}")]
    UnterminatedString { span: Span },

    /// A heredoc string was not properly terminated.
    #[error("unterminated heredoc at {span}")]
    UnterminatedHeredoc { span: Span },

    /// A block comment was not properly terminated.
    #[error("unterminated comment at {span}")]
    UnterminatedComment { span: Span },

    /// A numeric literal could not be parsed.
    #[error("invalid number at {span}: {detail}")]
    InvalidNumber { span: Span, detail: String },
}

impl LexError {
    /// Get the span where this error occurred.
    pub fn span(&self) -> Span {
        match self {
            LexError::UnexpectedChar { span, .. } => *span,
            LexError::UnterminatedString { span } => *span,
            LexError::UnterminatedHeredoc { span } => *span,
            LexError::UnterminatedComment { span } => *span,
            LexError::InvalidNumber { span, .. } => *span,
        }
    }
}

// ============================================================================
// Parse Errors
// ============================================================================

/// Categories of parse errors.
///
/// This enum provides a structured way to identify error types,
/// enabling better error recovery and more specific error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseErrorKind {
    // Token-level errors
    /// A specific token was expected but not found.
    ExpectedToken,
    /// An unexpected token was encountered.
    UnexpectedToken,
    /// Unexpected end of file.
    UnexpectedEof,

    // Expression errors
    /// An expression was expected.
    ExpectedExpression,
    /// An operator was expected.
    ExpectedOperator,
    /// The expression is invalid.
    InvalidExpression,
    /// A primary expression was expected.
    ExpectedPrimary,

    // Type errors
    /// A type was expected.
    ExpectedType,
    /// The type is invalid.
    InvalidType,
    /// Template arguments were expected.
    ExpectedTemplateArgs,

    // Statement errors
    /// A statement was expected.
    ExpectedStatement,
    /// The statement is invalid.
    InvalidStatement,
    /// A block was expected.
    ExpectedBlock,

    // Declaration errors
    /// A declaration was expected.
    ExpectedDeclaration,
    /// The declaration is invalid.
    InvalidDeclaration,
    /// Function parameters were expected.
    ExpectedParameters,
    /// A class member was expected.
    ExpectedClassMember,
    /// An interface method was expected.
    ExpectedInterfaceMethod,

    // Identifier errors
    /// An identifier was expected.
    ExpectedIdentifier,
    /// A duplicate identifier was found.
    DuplicateIdentifier,

    // Scope/namespace errors
    /// Invalid scope usage.
    InvalidScope,
    /// A namespace was expected.
    ExpectedNamespace,

    // Control flow errors
    /// `break` used outside of a loop.
    BreakOutsideLoop,
    /// `continue` used outside of a loop.
    ContinueOutsideLoop,

    // Syntax errors
    /// Mismatched delimiter (parentheses, brackets, braces).
    MismatchedDelimiter,
    /// Missing semicolon.
    MissingSemicolon,
    /// General syntax error.
    InvalidSyntax,
    /// Invalid escape sequence in string literal.
    InvalidEscapeSequence,

    // Modifier errors
    /// Invalid modifier for this context.
    InvalidModifier,
    /// Conflicting modifiers were specified.
    ConflictingModifiers,

    // Literal errors
    /// A literal value could not be parsed.
    InvalidLiteral,

    // Other
    /// Internal parser error.
    InternalError,
    /// Feature not yet implemented.
    NotImplemented,
}

impl ParseErrorKind {
    /// Returns a human-readable name for this error kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            ParseErrorKind::ExpectedToken => "expected token",
            ParseErrorKind::UnexpectedToken => "unexpected token",
            ParseErrorKind::UnexpectedEof => "unexpected end of file",
            ParseErrorKind::ExpectedExpression => "expected expression",
            ParseErrorKind::ExpectedOperator => "expected operator",
            ParseErrorKind::InvalidExpression => "invalid expression",
            ParseErrorKind::ExpectedPrimary => "expected primary expression",
            ParseErrorKind::ExpectedType => "expected type",
            ParseErrorKind::InvalidType => "invalid type",
            ParseErrorKind::ExpectedTemplateArgs => "expected template arguments",
            ParseErrorKind::ExpectedStatement => "expected statement",
            ParseErrorKind::InvalidStatement => "invalid statement",
            ParseErrorKind::ExpectedBlock => "expected block",
            ParseErrorKind::ExpectedDeclaration => "expected declaration",
            ParseErrorKind::InvalidDeclaration => "invalid declaration",
            ParseErrorKind::ExpectedParameters => "expected parameters",
            ParseErrorKind::ExpectedClassMember => "expected class member",
            ParseErrorKind::ExpectedInterfaceMethod => "expected interface method",
            ParseErrorKind::ExpectedIdentifier => "expected identifier",
            ParseErrorKind::DuplicateIdentifier => "duplicate identifier",
            ParseErrorKind::InvalidScope => "invalid scope",
            ParseErrorKind::ExpectedNamespace => "expected namespace",
            ParseErrorKind::BreakOutsideLoop => "break outside loop",
            ParseErrorKind::ContinueOutsideLoop => "continue outside loop",
            ParseErrorKind::MismatchedDelimiter => "mismatched delimiter",
            ParseErrorKind::MissingSemicolon => "missing semicolon",
            ParseErrorKind::InvalidSyntax => "invalid syntax",
            ParseErrorKind::InvalidEscapeSequence => "invalid escape sequence",
            ParseErrorKind::InvalidModifier => "invalid modifier",
            ParseErrorKind::ConflictingModifiers => "conflicting modifiers",
            ParseErrorKind::InvalidLiteral => "invalid literal",
            ParseErrorKind::InternalError => "internal error",
            ParseErrorKind::NotImplemented => "not implemented",
        }
    }
}

impl std::fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A parse error with location and context.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{kind} at {span}: {message}")]
pub struct ParseError {
    /// The category of this error.
    pub kind: ParseErrorKind,
    /// The source location where the error occurred.
    pub span: Span,
    /// A detailed error message.
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

    /// Create an "expected token" error.
    pub fn expected_token(span: Span, expected: &str, found: &str) -> Self {
        Self::new(
            ParseErrorKind::ExpectedToken,
            span,
            format!("expected {expected}, found {found}"),
        )
    }

    /// Create an "unexpected token" error.
    pub fn unexpected_token(span: Span, token: &str) -> Self {
        Self::new(
            ParseErrorKind::UnexpectedToken,
            span,
            format!("unexpected token: {token}"),
        )
    }

    /// Create an "unexpected EOF" error.
    pub fn unexpected_eof(span: Span) -> Self {
        Self::new(
            ParseErrorKind::UnexpectedEof,
            span,
            "unexpected end of file".to_string(),
        )
    }

    /// Create an "expected identifier" error.
    pub fn expected_identifier(span: Span, found: &str) -> Self {
        Self::new(
            ParseErrorKind::ExpectedIdentifier,
            span,
            format!("expected identifier, found {found}"),
        )
    }

    /// Create an "expected expression" error.
    pub fn expected_expression(span: Span, found: &str) -> Self {
        Self::new(
            ParseErrorKind::ExpectedExpression,
            span,
            format!("expected expression, found {found}"),
        )
    }

    /// Create an "expected type" error.
    pub fn expected_type(span: Span, found: &str) -> Self {
        Self::new(
            ParseErrorKind::ExpectedType,
            span,
            format!("expected type, found {found}"),
        )
    }

    /// Format the error with source context for display.
    ///
    /// This provides a rich error message with the relevant source line
    /// and a caret pointing to the error location.
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

/// A collection of parse errors.
///
/// Used when parsing can continue after encountering errors,
/// allowing multiple errors to be reported at once.
#[derive(Debug, Clone, Default)]
pub struct ParseErrors {
    errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Create a new empty error collection.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
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

    /// Iterate over the errors.
    pub fn iter(&self) -> impl Iterator<Item = &ParseError> {
        self.errors.iter()
    }

    /// Convert to a Vec of errors.
    pub fn into_vec(self) -> Vec<ParseError> {
        self.errors
    }

    /// Convert to a Result, returning Ok(()) if empty or Err with the first error.
    pub fn into_result(self) -> Result<(), ParseError> {
        if let Some(first) = self.errors.into_iter().next() {
            Err(first)
        } else {
            Ok(())
        }
    }
}

impl IntoIterator for ParseErrors {
    type Item = ParseError;
    type IntoIter = std::vec::IntoIter<ParseError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a ParseErrors {
    type Item = &'a ParseError;
    type IntoIter = std::slice::Iter<'a, ParseError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

impl From<ParseError> for ParseErrors {
    fn from(error: ParseError) -> Self {
        Self {
            errors: vec![error],
        }
    }
}

impl std::fmt::Display for ParseErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseErrors {}

// ============================================================================
// Registration Errors
// ============================================================================

/// Errors that occur during type and function registration.
///
/// This error type consolidates errors from both FFI registration
/// (native types/functions) and script module registration.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RegistrationError {
    /// A referenced type was not found.
    #[error("type not found: {0}")]
    TypeNotFound(String),

    /// A type with this name already exists.
    #[error("duplicate type: {0}")]
    DuplicateType(String),

    /// A registration with this name already exists.
    #[error("duplicate registration: {name} already registered as {kind}")]
    DuplicateRegistration {
        /// The name that was duplicated.
        name: String,
        /// What kind of thing was already registered (e.g., "class", "function").
        kind: String,
    },

    /// A duplicate enum value was registered.
    #[error("duplicate enum value: '{value_name}' in enum '{enum_name}'")]
    DuplicateEnumValue {
        /// The enum name.
        enum_name: String,
        /// The duplicate value name.
        value_name: String,
    },

    /// The declaration is invalid.
    #[error("invalid declaration: {0}")]
    InvalidDeclaration(String),

    /// The type is invalid or malformed.
    #[error("invalid type: {0}")]
    InvalidType(String),

    /// A behavior is forbidden for this type kind.
    #[error("type '{type_name}': {behavior} behavior not allowed - {reason}")]
    ForbiddenBehavior {
        /// The type name.
        type_name: String,
        /// The behavior that was forbidden.
        behavior: &'static str,
        /// Why it's forbidden.
        reason: String,
    },

    /// Required behaviors are missing for this type kind.
    #[error("type '{type_name}' is missing required behaviors: {}", missing.join(", "))]
    MissingBehaviors {
        /// The type name.
        type_name: String,
        /// List of missing behavior names.
        missing: Vec<&'static str>,
    },
}

// ============================================================================
// Compilation Errors
// ============================================================================

/// Errors that occur during compilation (semantic analysis, type checking).
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CompilationError {
    /// A referenced type could not be found.
    #[error("at {span}: unknown type '{name}'")]
    UnknownType {
        /// The type name that wasn't found.
        name: String,
        /// Where the type was referenced.
        span: Span,
    },

    /// A referenced function could not be found.
    #[error("at {span}: unknown function '{name}'")]
    UnknownFunction {
        /// The function name that wasn't found.
        name: String,
        /// Where the function was called.
        span: Span,
    },

    /// A referenced variable could not be found.
    #[error("at {span}: unknown variable '{name}'")]
    UnknownVariable {
        /// The variable name that wasn't found.
        name: String,
        /// Where the variable was referenced.
        span: Span,
    },

    /// A symbol name is ambiguous (multiple candidates from different namespaces).
    #[error("at {span}: ambiguous {kind} '{name}': could be {candidates}")]
    AmbiguousSymbol {
        /// What kind of symbol (e.g., "type", "function", "global variable").
        kind: String,
        /// The ambiguous symbol name.
        name: String,
        /// Description of the possible candidates.
        candidates: String,
        /// Where the symbol was referenced.
        span: Span,
    },

    /// A type mismatch was detected.
    #[error("at {span}: {message}")]
    TypeMismatch {
        /// Description of the mismatch.
        message: String,
        /// Where the mismatch occurred.
        span: Span,
    },

    /// An invalid operation was attempted.
    #[error("at {span}: {message}")]
    InvalidOperation {
        /// Description of what's invalid.
        message: String,
        /// Where the operation occurred.
        span: Span,
    },

    /// Circular inheritance was detected.
    #[error("at {span}: circular inheritance for '{name}'")]
    CircularInheritance {
        /// The type involved in the cycle.
        name: String,
        /// Where the type was defined.
        span: Span,
    },

    /// A duplicate definition was found.
    #[error("at {span}: duplicate definition '{name}'")]
    DuplicateDefinition {
        /// The duplicated name.
        name: String,
        /// Where the duplicate was defined.
        span: Span,
    },

    /// A variable was redeclared in the same scope.
    #[error("at {new_span}: variable '{name}' redeclared (originally declared at {original_span})")]
    VariableRedeclaration {
        /// The variable name.
        name: String,
        /// Where the variable was originally declared.
        original_span: Span,
        /// Where the redeclaration occurred.
        new_span: Span,
    },

    /// A generic compilation error.
    #[error("at {span}: {message}")]
    Other {
        /// The error message.
        message: String,
        /// Where the error occurred.
        span: Span,
    },

    /// No string factory configured for string literals.
    #[error(
        "at {span}: no string factory configured - call Context::set_string_factory() or use with_default_modules()"
    )]
    NoStringFactory {
        /// Where the string literal occurred.
        span: Span,
    },

    /// Template argument count mismatch.
    #[error("at {span}: template expects {expected} type argument(s), got {got}")]
    TemplateArgCountMismatch {
        /// Expected number of type arguments.
        expected: usize,
        /// Actual number of type arguments provided.
        got: usize,
        /// Where the template was instantiated.
        span: Span,
    },

    /// Attempted to instantiate a non-template type as a template.
    #[error("at {span}: '{name}' is not a template")]
    NotATemplate {
        /// The type name.
        name: String,
        /// Where the template was instantiated.
        span: Span,
    },

    /// Template validation callback rejected the instantiation.
    #[error("at {span}: invalid template instantiation '{template}': {message}")]
    TemplateValidationFailed {
        /// The template name.
        template: String,
        /// The error message from the validation callback.
        message: String,
        /// Where the template was instantiated.
        span: Span,
    },

    /// A function was not found.
    #[error("at {span}: function not found: {name}")]
    FunctionNotFound {
        /// The function name.
        name: String,
        /// Where the function was referenced.
        span: Span,
    },

    /// Base class does not have a default constructor for implicit super() call.
    #[error(
        "at {span}: base class '{base_class}' has no default constructor - derived class '{derived_class}' must explicitly call a base constructor with super(...)"
    )]
    NoBaseDefaultConstructor {
        /// The derived class name.
        derived_class: String,
        /// The base class name.
        base_class: String,
        /// Where the derived class constructor is defined.
        span: Span,
    },

    /// Internal compiler error.
    #[error("internal error: {message}")]
    Internal {
        /// The error message.
        message: String,
    },

    /// No matching overload found for function call.
    #[error("at {span}: no matching overload for '{name}({args})'")]
    NoMatchingOverload {
        /// The function name.
        name: String,
        /// The argument types as a string.
        args: String,
        /// Where the call occurred.
        span: Span,
    },

    /// Multiple overloads match with equal priority (ambiguous).
    #[error("at {span}: ambiguous call to '{name}': {candidates}")]
    AmbiguousOverload {
        /// The function name.
        name: String,
        /// Description of the ambiguous candidates.
        candidates: String,
        /// Where the call occurred.
        span: Span,
    },

    /// No operator found for operand types.
    #[error("at {span}: no operator '{op}' for types '{left}' and '{right}'")]
    NoOperator {
        /// The operator (e.g., "+", "-").
        op: String,
        /// The left operand type.
        left: String,
        /// The right operand type.
        right: String,
        /// Where the operation occurred.
        span: Span,
    },

    /// Expression is not an lvalue (cannot be assigned to).
    #[error("at {span}: expression is not an lvalue")]
    NotAnLvalue {
        /// Where the error occurred.
        span: Span,
    },

    /// Cannot modify a const value.
    #[error("at {span}: {message}")]
    CannotModifyConst {
        /// Description of what cannot be modified.
        message: String,
        /// Where the error occurred.
        span: Span,
    },

    /// A referenced field could not be found.
    #[error("at {span}: unknown field '{field}' on type '{type_name}'")]
    UnknownField {
        /// The field name that wasn't found.
        field: String,
        /// The type on which the field was accessed.
        type_name: String,
        /// Where the field was referenced.
        span: Span,
    },

    /// A referenced method could not be found.
    #[error("at {span}: unknown method '{method}' on type '{type_name}'")]
    UnknownMethod {
        /// The method name that wasn't found.
        method: String,
        /// The type on which the method was called.
        type_name: String,
        /// Where the method was called.
        span: Span,
    },

    /// Wrong number of arguments in function/method call.
    #[error("at {span}: {name} expects {expected} argument(s), got {got}")]
    ArgumentCountMismatch {
        /// The function or method name.
        name: String,
        /// Expected number of arguments.
        expected: usize,
        /// Actual number of arguments provided.
        got: usize,
        /// Where the call occurred.
        span: Span,
    },

    /// 'this' keyword used outside of a class method.
    #[error("at {span}: 'this' can only be used inside a class method")]
    ThisOutsideClass {
        /// Where the error occurred.
        span: Span,
    },

    /// Undefined variable.
    #[error("at {span}: undefined variable '{name}'")]
    UndefinedVariable {
        /// The variable name.
        name: String,
        /// Where the variable was referenced.
        span: Span,
    },

    /// Invalid cast between types.
    #[error("at {span}: cannot cast '{from}' to '{to}'")]
    InvalidCast {
        /// The source type.
        from: String,
        /// The target type.
        to: String,
        /// Where the cast occurred.
        span: Span,
    },

    /// Invalid handle type - type does not support handles.
    #[error("at {span}: cannot create handle to '{type_name}': {reason}")]
    InvalidHandleType {
        /// The type name.
        type_name: String,
        /// The reason why handles are not allowed.
        reason: String,
        /// Where the handle was declared.
        span: Span,
    },

    /// Invalid parameter type - type cannot be used as a parameter.
    #[error("at {span}: type '{type_name}' cannot be used as a parameter: {reason}")]
    InvalidParameterType {
        /// The type name.
        type_name: String,
        /// The reason why the type cannot be a parameter.
        reason: String,
        /// Where the parameter was declared.
        span: Span,
    },
}

impl CompilationError {
    /// Get the span where this error occurred.
    pub fn span(&self) -> Span {
        match self {
            CompilationError::UnknownType { span, .. } => *span,
            CompilationError::UnknownFunction { span, .. } => *span,
            CompilationError::UnknownVariable { span, .. } => *span,
            CompilationError::AmbiguousSymbol { span, .. } => *span,
            CompilationError::TypeMismatch { span, .. } => *span,
            CompilationError::InvalidOperation { span, .. } => *span,
            CompilationError::CircularInheritance { span, .. } => *span,
            CompilationError::DuplicateDefinition { span, .. } => *span,
            CompilationError::VariableRedeclaration { new_span, .. } => *new_span,
            CompilationError::Other { span, .. } => *span,
            CompilationError::NoStringFactory { span } => *span,
            CompilationError::TemplateArgCountMismatch { span, .. } => *span,
            CompilationError::NotATemplate { span, .. } => *span,
            CompilationError::TemplateValidationFailed { span, .. } => *span,
            CompilationError::FunctionNotFound { span, .. } => *span,
            CompilationError::Internal { .. } => Span::default(),
            CompilationError::NoMatchingOverload { span, .. } => *span,
            CompilationError::AmbiguousOverload { span, .. } => *span,
            CompilationError::NoOperator { span, .. } => *span,
            CompilationError::NotAnLvalue { span } => *span,
            CompilationError::CannotModifyConst { span, .. } => *span,
            CompilationError::ThisOutsideClass { span } => *span,
            CompilationError::UndefinedVariable { span, .. } => *span,
            CompilationError::UnknownField { span, .. } => *span,
            CompilationError::UnknownMethod { span, .. } => *span,
            CompilationError::ArgumentCountMismatch { span, .. } => *span,
            CompilationError::InvalidCast { span, .. } => *span,
            CompilationError::NoBaseDefaultConstructor { span, .. } => *span,
            CompilationError::InvalidHandleType { span, .. } => *span,
            CompilationError::InvalidParameterType { span, .. } => *span,
        }
    }
}

// ============================================================================
// Runtime Errors
// ============================================================================

/// Errors that occur during script execution.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum RuntimeError {
    /// A type mismatch occurred at runtime.
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// The expected type.
        expected: String,
        /// The actual type.
        actual: String,
    },

    /// A null handle was used where a valid object was required.
    #[error("null handle cannot be converted to {target_type}")]
    NullHandle {
        /// The type that was expected.
        target_type: String,
    },

    /// An integer value overflowed during conversion.
    #[error("integer overflow: {value} doesn't fit in {target_type}")]
    IntegerOverflow {
        /// The value that overflowed.
        value: i64,
        /// The target type.
        target_type: String,
    },

    /// Invalid UTF-8 was encountered.
    #[error("invalid UTF-8 string")]
    InvalidUtf8,

    /// A stale handle was used (object was freed).
    #[error("stale handle: object at index {index} has been freed")]
    StaleHandle {
        /// The index of the freed object.
        index: u32,
    },

    /// A native function panicked.
    #[error("native function panicked: {message}")]
    NativePanic {
        /// The panic message.
        message: String,
    },

    /// Division by zero.
    #[error("division by zero")]
    DivisionByZero,

    /// Stack overflow.
    #[error("stack overflow")]
    StackOverflow,

    /// A generic runtime error.
    #[error("{message}")]
    Other {
        /// The error message.
        message: String,
    },

    /// A global property access error.
    #[error("property error: {0}")]
    Property(#[from] crate::PropertyError),
}

// ============================================================================
// Unified Error Type
// ============================================================================

/// The unified error type for all AngelScript operations.
///
/// This enum wraps all phase-specific error types, enabling unified
/// error handling across the entire compilation and execution pipeline.
///
/// Each variant uses `#[from]` to enable automatic conversion with the `?` operator.
///
/// ## Example
///
/// ```ignore
/// use angelscript_core::AngelScriptError;
///
/// fn process_script(source: &str) -> Result<(), AngelScriptError> {
///     let tokens = lex(source)?;      // LexError -> AngelScriptError
///     let ast = parse(tokens)?;       // ParseError -> AngelScriptError
///     let module = compile(ast)?;     // CompilationError -> AngelScriptError
///     execute(module)?;               // RuntimeError -> AngelScriptError
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AngelScriptError {
    /// A lexer error.
    #[error(transparent)]
    Lex(#[from] LexError),

    /// A parse error.
    #[error(transparent)]
    Parse(#[from] ParseError),

    /// A registration error.
    #[error(transparent)]
    Registration(#[from] RegistrationError),

    /// A compilation error.
    #[error(transparent)]
    Compilation(#[from] CompilationError),

    /// A runtime error.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

impl AngelScriptError {
    /// Check if this is a lexer error.
    pub fn is_lex(&self) -> bool {
        matches!(self, AngelScriptError::Lex(_))
    }

    /// Check if this is a parse error.
    pub fn is_parse(&self) -> bool {
        matches!(self, AngelScriptError::Parse(_))
    }

    /// Check if this is a registration error.
    pub fn is_registration(&self) -> bool {
        matches!(self, AngelScriptError::Registration(_))
    }

    /// Check if this is a compilation error.
    pub fn is_compilation(&self) -> bool {
        matches!(self, AngelScriptError::Compilation(_))
    }

    /// Check if this is a runtime error.
    pub fn is_runtime(&self) -> bool {
        matches!(self, AngelScriptError::Runtime(_))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_error_display() {
        let err = LexError::UnexpectedChar {
            ch: '@',
            span: Span::new(1, 5, 1),
        };
        assert_eq!(format!("{err}"), "unexpected character '@' at 1:5");
    }

    #[test]
    fn lex_error_span() {
        let span = Span::new(3, 10, 5);
        let err = LexError::UnterminatedString { span };
        assert_eq!(err.span(), span);
    }

    #[test]
    fn parse_error_display() {
        let err = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 10, 3),
            "expected ';', found '}'",
        );
        assert_eq!(
            format!("{err}"),
            "expected token at 1:10: expected ';', found '}'"
        );
    }

    #[test]
    fn parse_error_constructors() {
        let span = Span::new(5, 20, 5);

        let err = ParseError::expected_token(span, "';'", "'}'");
        assert_eq!(err.kind, ParseErrorKind::ExpectedToken);
        assert!(err.message.contains("expected ';'"));

        let err = ParseError::unexpected_token(span, "'@'");
        assert_eq!(err.kind, ParseErrorKind::UnexpectedToken);

        let err = ParseError::unexpected_eof(span);
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEof);
    }

    #[test]
    fn parse_errors_collection() {
        let mut errors = ParseErrors::new();
        assert!(errors.is_empty());

        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "test".to_string(),
        ));
        errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            Span::new(2, 1, 1),
            "test2".to_string(),
        ));

        assert_eq!(errors.len(), 2);
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_errors_into_result() {
        let empty = ParseErrors::new();
        assert!(empty.into_result().is_ok());

        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "first error".to_string(),
        ));
        errors.push(ParseError::new(
            ParseErrorKind::UnexpectedToken,
            Span::new(2, 1, 1),
            "second error".to_string(),
        ));

        let result = errors.into_result();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("first"));
    }

    #[test]
    fn registration_error_display() {
        let err = RegistrationError::DuplicateRegistration {
            name: "MyClass".to_string(),
            kind: "class".to_string(),
        };
        assert_eq!(
            format!("{err}"),
            "duplicate registration: MyClass already registered as class"
        );
    }

    #[test]
    fn compilation_error_display() {
        let err = CompilationError::UnknownType {
            name: "Foo".to_string(),
            span: Span::new(10, 5, 3),
        };
        assert_eq!(format!("{err}"), "at 10:5: unknown type 'Foo'");
    }

    #[test]
    fn compilation_error_span() {
        let span = Span::new(5, 10, 8);
        let err = CompilationError::TypeMismatch {
            message: "test".to_string(),
            span,
        };
        assert_eq!(err.span(), span);
    }

    #[test]
    fn runtime_error_display() {
        let err = RuntimeError::TypeMismatch {
            expected: "int".to_string(),
            actual: "string".to_string(),
        };
        assert_eq!(format!("{err}"), "type mismatch: expected int, got string");
    }

    #[test]
    fn angelscript_error_from_lex() {
        let lex_err = LexError::UnexpectedChar {
            ch: '#',
            span: Span::new(1, 1, 1),
        };
        let err: AngelScriptError = lex_err.into();
        assert!(err.is_lex());
        assert!(!err.is_parse());
    }

    #[test]
    fn angelscript_error_from_parse() {
        let parse_err = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 1, 1),
            "test".to_string(),
        );
        let err: AngelScriptError = parse_err.into();
        assert!(err.is_parse());
    }

    #[test]
    fn angelscript_error_from_registration() {
        let reg_err = RegistrationError::TypeNotFound("Foo".to_string());
        let err: AngelScriptError = reg_err.into();
        assert!(err.is_registration());
    }

    #[test]
    fn angelscript_error_from_compilation() {
        let comp_err = CompilationError::UnknownType {
            name: "Bar".to_string(),
            span: Span::new(1, 1, 3),
        };
        let err: AngelScriptError = comp_err.into();
        assert!(err.is_compilation());
    }

    #[test]
    fn angelscript_error_from_runtime() {
        let rt_err = RuntimeError::DivisionByZero;
        let err: AngelScriptError = rt_err.into();
        assert!(err.is_runtime());
    }

    #[test]
    fn angelscript_error_transparent_display() {
        let lex_err = LexError::UnterminatedString {
            span: Span::new(5, 10, 20),
        };
        let err: AngelScriptError = lex_err.into();
        // #[error(transparent)] means it uses the inner error's Display
        assert_eq!(format!("{err}"), "unterminated string at 5:10");
    }

    #[test]
    fn parse_error_kind_as_str() {
        assert_eq!(ParseErrorKind::ExpectedToken.as_str(), "expected token");
        assert_eq!(
            ParseErrorKind::UnexpectedEof.as_str(),
            "unexpected end of file"
        );
        assert_eq!(ParseErrorKind::InvalidSyntax.as_str(), "invalid syntax");
    }
}
