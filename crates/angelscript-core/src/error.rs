use std::fmt;

/// Location in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Source file name or identifier.
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub column: u32,
}

impl SourceLocation {
    pub fn new(file: impl Into<String>, line: u32, column: u32) -> Self {
        SourceLocation {
            file: file.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Severity level for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// A diagnostic message from any phase of compilation.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            location: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: message.into(),
            location: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Info,
            message: message.into(),
            location: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref loc) = self.location {
            write!(f, "{}: {}: {}", loc, self.severity, self.message)
        } else {
            write!(f, "{}: {}", self.severity, self.message)
        }
    }
}

/// Errors from the AngelScript engine.
#[derive(Debug, Clone)]
pub enum Error {
    /// Lexer error (tokenization failed).
    Lex {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Parser error (invalid syntax).
    Parse {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Compilation error (type errors, unresolved symbols, etc.).
    Compile { diagnostics: Vec<Diagnostic> },
    /// Runtime error during VM execution.
    Runtime { message: String },
    /// Script threw an exception.
    ScriptException { message: String },
}

impl Error {
    pub fn lex(message: impl Into<String>) -> Self {
        Error::Lex {
            message: message.into(),
            location: None,
        }
    }

    pub fn lex_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Error::Lex {
            message: message.into(),
            location: Some(location),
        }
    }

    pub fn parse(message: impl Into<String>) -> Self {
        Error::Parse {
            message: message.into(),
            location: None,
        }
    }

    pub fn parse_at(message: impl Into<String>, location: SourceLocation) -> Self {
        Error::Parse {
            message: message.into(),
            location: Some(location),
        }
    }

    pub fn compile(diagnostics: Vec<Diagnostic>) -> Self {
        Error::Compile { diagnostics }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Error::Runtime {
            message: message.into(),
        }
    }

    pub fn script_exception(message: impl Into<String>) -> Self {
        Error::ScriptException {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lex { message, location } => {
                if let Some(loc) = location {
                    write!(f, "lex error at {}: {}", loc, message)
                } else {
                    write!(f, "lex error: {}", message)
                }
            }
            Error::Parse { message, location } => {
                if let Some(loc) = location {
                    write!(f, "parse error at {}: {}", loc, message)
                } else {
                    write!(f, "parse error: {}", message)
                }
            }
            Error::Compile { diagnostics } => {
                write!(f, "compilation failed with {} error(s)", diagnostics.len())?;
                for diag in diagnostics {
                    write!(f, "\n  {}", diag)?;
                }
                Ok(())
            }
            Error::Runtime { message } => write!(f, "runtime error: {}", message),
            Error::ScriptException { message } => write!(f, "script exception: {}", message),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience type alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_location_display() {
        let loc = SourceLocation::new("test.as", 10, 5);
        assert_eq!(format!("{}", loc), "test.as:10:5");
    }

    #[test]
    fn diagnostic_display() {
        let diag = Diagnostic::error("undefined variable 'x'")
            .with_location(SourceLocation::new("test.as", 5, 3));
        let s = format!("{}", diag);
        assert!(s.contains("test.as:5:3"));
        assert!(s.contains("error"));
        assert!(s.contains("undefined variable 'x'"));
    }

    #[test]
    fn error_display() {
        let err = Error::lex("unexpected character");
        assert_eq!(format!("{}", err), "lex error: unexpected character");

        let err = Error::runtime("null pointer dereference");
        assert_eq!(
            format!("{}", err),
            "runtime error: null pointer dereference"
        );
    }
}
