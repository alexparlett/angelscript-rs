use std::fmt;
use thiserror::Error;
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

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
    pub source: String,
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

// ==================== PARSE ERRORS ====================

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
        let span = self.span();

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

// ==================== SEMANTIC ERRORS ====================

#[derive(Error, Debug, Clone)]
pub enum SemanticError {
    #[error("Undefined symbol '{name}'")]
    UndefinedSymbol {
        name: String,
        location: Option<Span>,
    },

    #[error("Undefined type '{name}'")]
    UndefinedType {
        name: String,
        location: Option<Span>,
    },

    #[error("Undefined function '{name}'")]
    UndefinedFunction {
        name: String,
        location: Option<Span>,
    },

    #[error("Undefined member '{member}' in type '{type_name}'")]
    UndefinedMember {
        type_name: String,
        member: String,
        location: Option<Span>,
    },

    #[error("Type mismatch: expected '{expected}', found '{found}'")]
    TypeMismatch {
        expected: String,
        found: String,
        location: Option<Span>,
    },

    #[error("Cannot assign to '{target}': {reason}")]
    InvalidAssignment {
        target: String,
        reason: String,
        location: Option<Span>,
    },

    #[error("Duplicate definition of '{name}'")]
    DuplicateDefinition {
        name: String,
        location: Option<Span>,
        previous_location: Option<Span>,
    },

    #[error("Invalid operation '{operation}' on type '{type_name}'")]
    InvalidOperation {
        operation: String,
        type_name: String,
        location: Option<Span>,
    },

    #[error("Cannot convert from '{from}' to '{to}'")]
    InvalidConversion {
        from: String,
        to: String,
        location: Option<Span>,
    },

    #[error("Wrong number of arguments: expected {expected}, found {found}")]
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
        location: Option<Span>,
    },

    #[error("Invalid argument type: expected '{expected}', found '{found}'")]
    InvalidArgumentType {
        expected: String,
        found: String,
        location: Option<Span>,
    },

    #[error("Break statement outside of loop")]
    InvalidBreak { location: Option<Span> },

    #[error("Continue statement outside of loop")]
    InvalidContinue { location: Option<Span> },

    #[error("Return statement with value in void function")]
    InvalidReturn { location: Option<Span> },

    #[error("Missing return statement in non-void function '{function}'")]
    MissingReturn {
        function: String,
        location: Option<Span>,
    },

    #[error("Cannot access private member '{member}'")]
    PrivateAccess {
        member: String,
        location: Option<Span>,
    },

    #[error("Cannot access protected member '{member}'")]
    ProtectedAccess {
        member: String,
        location: Option<Span>,
    },

    #[error("Cannot override final method '{method}'")]
    OverrideFinal {
        method: String,
        location: Option<Span>,
    },

    #[error("Abstract method '{method}' not implemented in class '{class}'")]
    UnimplementedAbstract {
        method: String,
        class: String,
        location: Option<Span>,
    },

    #[error("Cannot instantiate abstract class '{class}'")]
    InstantiateAbstract {
        class: String,
        location: Option<Span>,
    },

    #[error("Circular dependency detected: {cycle}")]
    CircularDependency {
        cycle: String,
        location: Option<Span>,
    },

    #[error("Const violation: {message}")]
    ConstViolation {
        message: String,
        location: Option<Span>,
    },

    #[error("Reference type mismatch: {message}")]
    ReferenceMismatch {
        message: String,
        location: Option<Span>,
    },

    #[error("Invalid handle operation: {message}")]
    InvalidHandle {
        message: String,
        location: Option<Span>,
    },

    #[error("Ambiguous call to '{name}': multiple candidates found")]
    AmbiguousCall {
        name: String,
        candidates: Vec<String>,
        location: Option<Span>,
    },

    #[error("Internal compiler error: {message}")]
    Internal {
        message: String,
        location: Option<Span>,
    },
}

impl SemanticError {
    pub fn location(&self) -> Option<&Span> {
        match self {
            SemanticError::UndefinedSymbol { location, .. }
            | SemanticError::UndefinedType { location, .. }
            | SemanticError::UndefinedFunction { location, .. }
            | SemanticError::UndefinedMember { location, .. }
            | SemanticError::TypeMismatch { location, .. }
            | SemanticError::InvalidAssignment { location, .. }
            | SemanticError::DuplicateDefinition { location, .. }
            | SemanticError::InvalidOperation { location, .. }
            | SemanticError::InvalidConversion { location, .. }
            | SemanticError::ArgumentCountMismatch { location, .. }
            | SemanticError::InvalidArgumentType { location, .. }
            | SemanticError::InvalidBreak { location }
            | SemanticError::InvalidContinue { location }
            | SemanticError::InvalidReturn { location }
            | SemanticError::MissingReturn { location, .. }
            | SemanticError::PrivateAccess { location, .. }
            | SemanticError::ProtectedAccess { location, .. }
            | SemanticError::OverrideFinal { location, .. }
            | SemanticError::UnimplementedAbstract { location, .. }
            | SemanticError::InstantiateAbstract { location, .. }
            | SemanticError::CircularDependency { location, .. }
            | SemanticError::ConstViolation { location, .. }
            | SemanticError::ReferenceMismatch { location, .. }
            | SemanticError::InvalidHandle { location, .. }
            | SemanticError::AmbiguousCall { location, .. }
            | SemanticError::Internal { location, .. } => location.as_ref(),
        }
    }

    /// Format error with source context
    pub fn format_with_source(&self, source: &str) -> String {
        if let Some(span) = self.location() {
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
        } else {
            self.to_string()
        }
    }

    /// Helper constructors for common cases without location
    pub fn undefined_symbol(name: String) -> Self {
        SemanticError::UndefinedSymbol {
            name,
            location: None,
        }
    }

    pub fn undefined_type(name: String) -> Self {
        SemanticError::UndefinedType {
            name,
            location: None,
        }
    }

    pub fn undefined_function(name: String) -> Self {
        SemanticError::UndefinedFunction {
            name,
            location: None,
        }
    }

    pub fn undefined_member(type_name: String, member: String) -> Self {
        SemanticError::UndefinedMember {
            type_name,
            member,
            location: None,
        }
    }

    pub fn type_mismatch(expected: String, found: String) -> Self {
        SemanticError::TypeMismatch {
            expected,
            found,
            location: None,
        }
    }

    pub fn invalid_operation(operation: String, type_name: String) -> Self {
        SemanticError::InvalidOperation {
            operation,
            type_name,
            location: None,
        }
    }

    pub fn internal(message: String) -> Self {
        SemanticError::Internal {
            message,
            location: None,
        }
    }
}

// ==================== CODEGEN ERRORS ====================

#[derive(Debug, Clone)]
pub enum CodegenError {
    UndefinedVariable(String),
    UndefinedFunction(String),
    UndefinedMember(String),
    UnknownType(String),
    UnsupportedOperation(String),
    InvalidLValue,
    InvalidBreak,
    InvalidContinue,
    NotImplemented(String),
    Internal(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CodegenError::UndefinedVariable(name) => write!(f, "Undefined variable: {}", name),
            CodegenError::UndefinedFunction(name) => write!(f, "Undefined function: {}", name),
            CodegenError::UndefinedMember(name) => write!(f, "Undefined member: {}", name),
            CodegenError::UnknownType(name) => write!(f, "Unknown type: {}", name),
            CodegenError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            CodegenError::InvalidLValue => write!(f, "Invalid left-hand side of assignment"),
            CodegenError::InvalidBreak => write!(f, "Break statement outside of loop"),
            CodegenError::InvalidContinue => write!(f, "Continue statement outside of loop"),
            CodegenError::NotImplemented(feature) => write!(f, "Not yet implemented: {}", feature),
            CodegenError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CodegenError {}

// ==================== COMPILE ERRORS ====================

#[derive(Error, Debug, Clone)]
pub enum CompileError {
    #[error("Semantic analysis failed with {} error(s)", .0.len())]
    SemanticErrors(Vec<SemanticError>),

    #[error("Code generation failed: {0}")]
    CodegenError(#[from] CodegenError),

    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),
}

impl CompileError {
    /// Format all errors with source context
    pub fn format_with_source(&self, source: &str) -> String {
        match self {
            CompileError::SemanticErrors(errors) => {
                let mut output = format!(
                    "Semantic analysis failed with {} error(s):\n\n",
                    errors.len()
                );
                for (i, error) in errors.iter().enumerate() {
                    output.push_str(&format!(
                        "Error {}:\n{}\n\n",
                        i + 1,
                        error.format_with_source(source)
                    ));
                }
                output
            }
            CompileError::CodegenError(error) => {
                format!("Code generation failed:\n{}", error)
            }
            CompileError::ParseError(error) => error.format_with_source(source),
        }
    }
}

// Type aliases for convenience
pub type ParseResult<T> = std::result::Result<T, ParseError>;
pub type SemanticResult<T> = std::result::Result<T, SemanticError>;
pub type CodegenResult<T> = std::result::Result<T, CodegenError>;
pub type CompileResult<T> = std::result::Result<T, CompileError>;
