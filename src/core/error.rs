use crate::core::span::Span;
use crate::core::types::{FunctionId, TypeId};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found '{found}'")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Option<Span>,
    },

    #[error("Unexpected end of input: expected {expected}")]
    UnexpectedEof {
        expected: String,
        span: Option<Span>,
    },

    #[error("Invalid operator: '{operator}'")]
    InvalidOperator {
        operator: String,
        span: Option<Span>,
    },

    #[error("Invalid expression: {message}")]
    InvalidExpression { message: String, span: Option<Span> },

    #[error("Unmatched delimiter: expected '{expected}', found '{found}'")]
    UnmatchedDelimiter {
        expected: char,
        found: String,
        span: Option<Span>,
    },

    #[error("Invalid number literal: {message}")]
    InvalidNumber { message: String, span: Option<Span> },

    #[error("Invalid string literal: {message}")]
    InvalidString { message: String, span: Option<Span> },

    #[error("Type error: {message}")]
    TypeError { message: String, span: Option<Span> },

    #[error("Syntax error: {message}")]
    SyntaxError { message: String, span: Option<Span> },
}

impl ParseError {
    pub fn span(&self) -> Option<&Span> {
        match self {
            ParseError::UnexpectedToken { span, .. }
            | ParseError::UnexpectedEof { span, .. }
            | ParseError::InvalidOperator { span, .. }
            | ParseError::InvalidExpression { span, .. }
            | ParseError::UnmatchedDelimiter { span, .. }
            | ParseError::InvalidNumber { span, .. }
            | ParseError::InvalidString { span, .. }
            | ParseError::TypeError { span, .. }
            | ParseError::SyntaxError { span, .. } => span.as_ref(),
        }
    }
}

#[derive(Error, Debug, Clone)]
pub enum SemanticError {
    #[error("Undefined symbol '{name}'")]
    UndefinedSymbol { name: String, span: Option<Span> },

    #[error("Undefined type '{name}'")]
    UndefinedType { name: String, span: Option<Span> },

    #[error("Undefined function '{name}'")]
    UndefinedFunction { name: String, span: Option<Span> },

    #[error("Undefined member '{member}' in type '{type_name}'")]
    UndefinedMember {
        type_name: String,
        member: String,
        span: Option<Span>,
    },

    #[error("Duplicate function defined '{name}'")]
    DuplicateFunction {
        name: String,
        span: Option<Span>,
    },

    #[error("Type mismatch: expected '{expected}', found '{found}'")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Option<Span>,
    },

    #[error("Cannot assign to '{target}': {reason}")]
    InvalidAssignment {
        target: String,
        reason: String,
        span: Option<Span>,
    },

    #[error("Duplicate definition of '{name}'")]
    DuplicateDefinition {
        name: String,
        span: Option<Span>,
        previous_span: Option<Span>,
    },

    #[error("Invalid operation '{operation}' on type '{type_name}'")]
    InvalidOperation {
        operation: String,
        type_name: String,
        span: Option<Span>,
    },

    #[error("Cannot convert from '{from}' to '{to}'")]
    InvalidConversion {
        from: String,
        to: String,
        span: Option<Span>,
    },

    #[error("Wrong number of arguments: expected {expected}, found {found}")]
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
        span: Option<Span>,
    },

    #[error("Invalid argument type: expected '{expected}', found '{found}'")]
    InvalidArgumentType {
        expected: String,
        found: String,
        span: Option<Span>,
    },

    #[error("Break statement outside of loop")]
    InvalidBreak { span: Option<Span> },

    #[error("Continue statement outside of loop")]
    InvalidContinue { span: Option<Span> },

    #[error("Return statement with value in void function")]
    InvalidReturn { span: Option<Span> },

    #[error("Missing return statement in non-void function '{function}'")]
    MissingReturn {
        function: String,
        span: Option<Span>,
    },

    #[error("Cannot access private member '{member}'")]
    PrivateAccess { member: String, span: Option<Span> },

    #[error("Cannot access protected member '{member}'")]
    ProtectedAccess { member: String, span: Option<Span> },

    #[error("Cannot override final method '{method}'")]
    OverrideFinal { method: String, span: Option<Span> },

    #[error("Abstract method '{method}' not implemented in class '{class}'")]
    UnimplementedAbstract {
        method: String,
        class: String,
        span: Option<Span>,
    },

    #[error("Cannot instantiate abstract class '{class}'")]
    InstantiateAbstract { class: String, span: Option<Span> },

    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String, span: Option<Span> },

    #[error("Const violation: {message}")]
    ConstViolation { message: String, span: Option<Span> },

    #[error("Reference type mismatch: {message}")]
    ReferenceMismatch { message: String, span: Option<Span> },

    #[error("Invalid handle operation: {message}")]
    InvalidHandle { message: String, span: Option<Span> },

    #[error("Ambiguous call to '{name}': multiple candidates found")]
    AmbiguousCall {
        name: String,
        candidates: Vec<String>,
        span: Option<Span>,
    },

    #[error("Internal compiler error: {message}")]
    Internal { message: String, span: Option<Span> },
}

impl SemanticError {
    pub fn span(&self) -> Option<&Span> {
        match self {
            SemanticError::UndefinedSymbol { span, .. }
            | SemanticError::UndefinedType { span, .. }
            | SemanticError::UndefinedFunction { span, .. }
            | SemanticError::DuplicateFunction { span, .. }
            | SemanticError::UndefinedMember { span, .. }
            | SemanticError::TypeMismatch { span, .. }
            | SemanticError::InvalidAssignment { span, .. }
            | SemanticError::DuplicateDefinition { span, .. }
            | SemanticError::InvalidOperation { span, .. }
            | SemanticError::InvalidConversion { span, .. }
            | SemanticError::ArgumentCountMismatch { span, .. }
            | SemanticError::InvalidArgumentType { span, .. }
            | SemanticError::InvalidBreak { span }
            | SemanticError::InvalidContinue { span }
            | SemanticError::InvalidReturn { span }
            | SemanticError::MissingReturn { span, .. }
            | SemanticError::PrivateAccess { span, .. }
            | SemanticError::ProtectedAccess { span, .. }
            | SemanticError::OverrideFinal { span, .. }
            | SemanticError::UnimplementedAbstract { span, .. }
            | SemanticError::InstantiateAbstract { span, .. }
            | SemanticError::CircularDependency { span, .. }
            | SemanticError::ConstViolation { span, .. }
            | SemanticError::ReferenceMismatch { span, .. }
            | SemanticError::InvalidHandle { span, .. }
            | SemanticError::AmbiguousCall { span, .. }
            | SemanticError::Internal { span, .. } => span.as_ref(),
        }
    }

    pub fn format_detailed(&self) -> String {
        let mut output = format!("error: {}\n", self);

        if let Some(span) = self.span() {
            output.push_str(&format!("  --> {}\n", span.format()));
        }

        match self {
            SemanticError::DuplicateDefinition { previous_span, .. } => {
                if let Some(prev) = previous_span {
                    output.push_str(&format!("note: Previously defined at: {}\n", prev.format()));
                }
            }
            SemanticError::AmbiguousCall { candidates, .. } => {
                output.push_str("note: Candidates are:\n");
                for candidate in candidates {
                    output.push_str(&format!("  - {}\n", candidate));
                }
            }
            _ => {}
        }

        output
    }

    pub fn undefined_symbol(name: String) -> Self {
        SemanticError::UndefinedSymbol { name, span: None }
    }

    pub fn undefined_type(name: String) -> Self {
        SemanticError::UndefinedType { name, span: None }
    }

    pub fn undefined_function(name: String) -> Self {
        SemanticError::UndefinedFunction { name, span: None }
    }

    pub fn undefined_member(type_name: String, member: String) -> Self {
        SemanticError::UndefinedMember {
            type_name,
            member,
            span: None,
        }
    }

    pub fn type_mismatch(expected: String, found: String) -> Self {
        SemanticError::TypeMismatch {
            expected,
            found,
            span: None,
        }
    }

    pub fn invalid_operation(operation: String, type_name: String) -> Self {
        SemanticError::InvalidOperation {
            operation,
            type_name,
            span: None,
        }
    }

    pub fn internal(message: String) -> Self {
        SemanticError::Internal {
            message,
            span: None,
        }
    }
}

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

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    pub fn format_detailed(&self) -> String {
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
                        error.format_detailed()
                    ));
                }
                output
            }
            CompileError::CodegenError(error) => {
                format!("Code generation failed:\n{}", error)
            }
            CompileError::ParseError(error) => {
                format!("Parse error:\n{}", error)
            }
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;
pub type SemanticResult<T> = Result<T, SemanticError>;
pub type CodegenResult<T> = Result<T, CodegenError>;
pub type CompileResult<T> = Result<T, CompileError>;
