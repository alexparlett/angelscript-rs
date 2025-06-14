use crate::types::enums::MessageType;
use std::collections::VecDeque;
use std::fmt;
use crate::types::script_data::ScriptData;
use crate::types::script_memory::Void;

/// A single diagnostic message from the AngelScript compiler.
///
/// Diagnostics represent compilation messages such as errors, warnings, and informational
/// messages that occur during script compilation. Each diagnostic includes the message
/// content, location information, and severity level.
///
/// # Examples
///
/// ```rust
/// let diagnostic = Diagnostic {
///     kind: DiagnosticKind::Error,
///     message: "Undefined function 'foo'".to_string(),
///     section: Some("script.as".to_string()),
///     row: 10,
///     col: 5,
/// };
///
/// println!("{}", diagnostic); // script.as:10:5: error: Undefined function 'foo'
/// ```
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The severity level of this diagnostic
    pub kind: DiagnosticKind,
    /// The diagnostic message text
    pub message: String,
    /// The source file or section name where this diagnostic occurred, if available
    pub section: Option<String>,
    /// The line number where this diagnostic occurred (1-based)
    pub row: u32,
    /// The column number where this diagnostic occurred (1-based)
    pub col: u32,
}

/// The severity level of a diagnostic message.
///
/// AngelScript can emit three types of messages during compilation:
/// - **Error**: Compilation errors that prevent successful compilation
/// - **Warning**: Potential issues that don't prevent compilation but may indicate problems
/// - **Info**: Informational messages about the compilation process
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// A compilation error that prevents successful compilation.
    ///
    /// Errors indicate syntax errors, type mismatches, undefined symbols,
    /// or other issues that make the script invalid.
    Error,

    /// A warning about potentially problematic code.
    ///
    /// Warnings indicate code that compiles successfully but may have
    /// unintended behavior, such as unused variables or deprecated features.
    Warning,

    /// An informational message about the compilation process.
    ///
    /// Info messages provide additional context about compilation,
    /// such as optimization notes or debug information.
    Info,
}

impl From<MessageType> for DiagnosticKind {
    fn from(msg_type: MessageType) -> Self {
        match msg_type {
            MessageType::Error => DiagnosticKind::Error,
            MessageType::Warning => DiagnosticKind::Warning,
            MessageType::Information => DiagnosticKind::Info,
        }
    }
}

/// A collection of diagnostic messages from AngelScript compilation.
///
/// `Diagnostics` accumulates all compilation messages (errors, warnings, and info)
/// during script compilation. It provides methods to query the diagnostics,
/// check for errors or warnings, and iterate over the collected messages.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// let mut engine = Engine::create()?;
/// let mut diagnostics = Diagnostics::new();
///
/// // Set up diagnostic collection
/// engine.set_diagnostic_callback(&mut diagnostics)?;
///
/// // Compile script
/// let module = engine.get_module("test", GetModuleFlags::CreateIfNotExists)?;
/// module.add_script_section("script", "void main() { undefined_function(); }")?;
/// let result = module.build();
///
/// // Check results
/// if diagnostics.has_errors() {
///     println!("Compilation failed with {} errors", diagnostics.error_count());
///     for error in diagnostics.errors() {
///         println!("  {}", error);
///     }
/// }
/// ```
///
/// ## Displaying Diagnostics
///
/// ```rust
/// // Print all diagnostics
/// if !diagnostics.is_empty() {
///     println!("Compilation diagnostics:");
///     println!("{}", diagnostics);
/// }
///
/// // Print only errors
/// for error in diagnostics.errors() {
///     eprintln!("Error: {}", error);
/// }
/// ```
#[derive(Debug, Default)]
pub struct Diagnostics {
    diagnostics: VecDeque<Diagnostic>,
    has_errors: bool,
}

impl Diagnostics {
    /// Creates a new, empty diagnostics collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// assert!(diagnostics.is_empty());
    /// assert!(!diagnostics.has_errors());
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a diagnostic to the collection.
    ///
    /// If the diagnostic is an error, this will set the internal error flag.
    /// This method is typically called by the AngelScript message callback
    /// and not directly by user code.
    ///
    /// # Arguments
    ///
    /// * `diagnostic` - The diagnostic to add
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut diagnostics = Diagnostics::new();
    ///
    /// diagnostics.add_diagnostic(Diagnostic {
    ///     kind: DiagnosticKind::Warning,
    ///     message: "Unused variable 'x'".to_string(),
    ///     section: Some("script.as".to_string()),
    ///     row: 5,
    ///     col: 10,
    /// });
    ///
    /// assert_eq!(diagnostics.warning_count(), 1);
    /// assert!(!diagnostics.has_errors());
    /// ```
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        if diagnostic.kind == DiagnosticKind::Error {
            self.has_errors = true;
        }
        self.diagnostics.push_back(diagnostic);
    }

    /// Returns `true` if the collection contains any error diagnostics.
    ///
    /// This is a fast operation that doesn't require iterating through
    /// all diagnostics, as the error state is tracked internally.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut diagnostics = Diagnostics::new();
    /// assert!(!diagnostics.has_errors());
    ///
    /// diagnostics.add_diagnostic(Diagnostic {
    ///     kind: DiagnosticKind::Error,
    ///     message: "Syntax error".to_string(),
    ///     section: None,
    ///     row: 1,
    ///     col: 1,
    /// });
    ///
    /// assert!(diagnostics.has_errors());
    /// ```
    pub fn has_errors(&self) -> bool {
        self.has_errors
    }

    /// Returns `true` if the collection contains any warning diagnostics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut diagnostics = Diagnostics::new();
    /// assert!(!diagnostics.has_warnings());
    ///
    /// diagnostics.add_diagnostic(Diagnostic {
    ///     kind: DiagnosticKind::Warning,
    ///     message: "Unused variable".to_string(),
    ///     section: None,
    ///     row: 1,
    ///     col: 1,
    /// });
    ///
    /// assert!(diagnostics.has_warnings());
    /// ```
    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::Warning)
    }

    /// Returns `true` if the collection contains no diagnostics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// assert!(diagnostics.is_empty());
    ///
    /// let mut diagnostics = Diagnostics::new();
    /// // ... add some diagnostics ...
    /// diagnostics.clear();
    /// assert!(diagnostics.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Removes all diagnostics from the collection.
    ///
    /// After calling this method, the collection will be empty and
    /// `has_errors()` will return `false`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut diagnostics = Diagnostics::new();
    /// // ... add diagnostics during compilation ...
    ///
    /// // Clear for next compilation
    /// diagnostics.clear();
    /// assert!(diagnostics.is_empty());
    /// assert!(!diagnostics.has_errors());
    /// ```
    pub fn clear(&mut self) {
        self.diagnostics.clear();
        self.has_errors = false;
    }

    /// Returns an iterator over all diagnostics in the collection.
    ///
    /// The diagnostics are returned in the order they were added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// for diagnostic in diagnostics.iter() {
    ///     println!("{}: {}", diagnostic.kind, diagnostic.message);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    /// Returns an iterator over only the error diagnostics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// if diagnostics.has_errors() {
    ///     println!("Compilation errors:");
    ///     for error in diagnostics.errors() {
    ///         println!("  {}", error);
    ///     }
    /// }
    /// ```
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Error)
    }

    /// Returns an iterator over only the warning diagnostics.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// if diagnostics.has_warnings() {
    ///     println!("Compilation warnings:");
    ///     for warning in diagnostics.warnings() {
    ///         println!("  {}", warning);
    ///     }
    /// }
    /// ```
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Warning)
    }

    /// Returns the total number of diagnostics in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// println!("Total diagnostics: {}", diagnostics.count());
    /// ```
    pub fn count(&self) -> usize {
        self.diagnostics.len()
    }

    /// Returns the number of error diagnostics in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// if diagnostics.has_errors() {
    ///     println!("Found {} compilation errors", diagnostics.error_count());
    /// }
    /// ```
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Error)
            .count()
    }

    /// Returns the number of warning diagnostics in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// if diagnostics.has_warnings() {
    ///     println!("Found {} compilation warnings", diagnostics.warning_count());
    /// }
    /// ```
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Warning)
            .count()
    }

    /// Returns the number of info diagnostics in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// println!("Info messages: {}", diagnostics.info_count());
    /// ```
    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::Info)
            .count()
    }

    /// Writes all diagnostics to the provided writer.
    ///
    /// Each diagnostic is written on its own line in the format:
    /// `section:row:col: kind: message` or `row:col: kind: message` if no section.
    ///
    /// # Arguments
    ///
    /// * `writer` - The writer to output diagnostics to
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the writer fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// // Write to stderr
    /// diagnostics.emit(&mut io::stderr())?;
    ///
    /// // Write to a string
    /// let mut output = Vec::new();
    /// diagnostics.emit(&mut output)?;
    /// let output_str = String::from_utf8(output)?;
    /// ```
    pub fn emit<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        for diagnostic in &self.diagnostics {
            writeln!(writer, "{}", diagnostic)?;
        }
        Ok(())
    }
}

impl fmt::Display for Diagnostic {
    /// Formats a diagnostic for display.
    ///
    /// The format is: `section:row:col: kind: message` or `row:col: kind: message` if no section.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostic = Diagnostic {
    ///     kind: DiagnosticKind::Error,
    ///     message: "Undefined symbol 'foo'".to_string(),
    ///     section: Some("main.as".to_string()),
    ///     row: 10,
    ///     col: 5,
    /// };
    ///
    /// assert_eq!(diagnostic.to_string(), "main.as:10:5: error: Undefined symbol 'foo'");
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_str = match self.kind {
            DiagnosticKind::Error => "error",
            DiagnosticKind::Warning => "warning",
            DiagnosticKind::Info => "info",
        };

        if let Some(section) = &self.section {
            write!(
                f,
                "{}:{}:{}: {}: {}",
                section, self.row, self.col, kind_str, self.message
            )
        } else {
            write!(
                f,
                "{}:{}: {}: {}",
                self.row, self.col, kind_str, self.message
            )
        }
    }
}

impl fmt::Display for Diagnostics {
    /// Formats all diagnostics for display.
    ///
    /// Each diagnostic is displayed on its own line.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let diagnostics = Diagnostics::new();
    /// // ... populate with diagnostics ...
    ///
    /// println!("Compilation results:\n{}", diagnostics);
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            writeln!(f, "{}", diagnostic)?;
        }
        Ok(())
    }
}

impl ScriptData for Diagnostics {
    fn to_script_ptr(&mut self) -> *mut Void {
        self as *mut Diagnostics as *mut Void
    }

    fn from_script_ptr(ptr: *mut Void) -> Self
    where
        Self: Sized
    {
        unsafe { ptr.cast::<Self>().read() }
    }
}