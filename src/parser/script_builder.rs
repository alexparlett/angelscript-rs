use crate::parser::ast::{Script, ScriptNode};
use crate::parser::error::*;
use crate::parser::lexer::Lexer;
use crate::parser::preprocessor::Preprocessor;
use std::collections::HashSet;

/// Callback trait for handling #include directives
pub trait IncludeCallback {
    /// Called when an #include directive is encountered
    ///
    /// The callback should return the source code for the included file
    ///
    /// # Arguments
    /// * `include_path` - The path specified in the #include directive
    /// * `from_source` - The source code that contains the #include directive
    ///
    /// # Returns
    /// * `Ok(source_code)` with the content of the included file
    /// * `Err(ParseError)` to abort compilation with an error
    fn on_include(&mut self, include_path: &str, from_source: &str) -> Result<String>;
}

/// Callback trait for handling #pragma directives
pub trait PragmaCallback {
    /// Called when a #pragma directive is encountered
    fn on_pragma(&mut self, pragma_text: &str) -> Result<()>;
}

/// Script builder that processes preprocessor directives
pub struct ScriptBuilder {
    defined_words: HashSet<String>,
    include_callback: Option<Box<dyn IncludeCallback>>,
    pragma_callback: Option<Box<dyn PragmaCallback>>,
    included_sources: HashSet<String>, // Track what we've included to prevent circular includes
}

impl ScriptBuilder {
    pub fn new() -> Self {
        Self {
            defined_words: HashSet::new(),
            include_callback: None,
            pragma_callback: None,
            included_sources: HashSet::new(),
        }
    }

    /// Set the include callback
    pub fn set_include_callback<C: IncludeCallback + 'static>(&mut self, callback: C) {
        self.include_callback = Some(Box::new(callback));
    }

    /// Set the pragma callback
    pub fn set_pragma_callback<C: PragmaCallback + 'static>(&mut self, callback: C) {
        self.pragma_callback = Some(Box::new(callback));
    }

    /// Define a word for conditional compilation
    pub fn define_word(&mut self, word: String) {
        self.defined_words.insert(word);
    }

    /// Check if a word is defined
    pub fn is_defined(&self, word: &str) -> bool {
        self.defined_words.contains(word)
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.defined_words.clear();
        self.included_sources.clear();
    }

    /// Parse AngelScript source code with preprocessing
    ///
    /// This is the main entry point for parsing AngelScript code.
    /// It handles:
    /// - Tokenization
    /// - Preprocessor directives (#if, #include, #pragma, etc.)
    /// - Conditional compilation
    /// - Include file resolution
    ///
    /// # Example
    /// ```
    /// let mut builder = ScriptBuilder::new();
    /// builder.define_word("DEBUG".to_string());
    ///
    /// let source = r#"
    ///     #if DEBUG
    ///         void debugFunction() {}
    ///     #endif
    ///     
    ///     void main() {}
    /// "#;
    ///
    /// let script = builder.build_from_source(source)?;
    /// ```
    pub fn build_from_source(&mut self, source: &str) -> Result<Script> {
        // Tokenize
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;

        // Parse with preprocessor
        let preprocessor = Preprocessor::new(tokens, self);
        let mut script = preprocessor.parse()?;

        // Process includes and pragmas
        script.items = self.process_items(source, script.items)?;

        Ok(script)
    }

    fn process_items(
        &mut self,
        current_source: &str,
        items: Vec<ScriptNode>,
    ) -> Result<Vec<ScriptNode>> {
        let mut result = Vec::new();

        for item in items {
            match item {
                ScriptNode::Include(include) => {
                    // Handle include
                    let included_items = self.handle_include(&include.path, current_source)?;
                    result.extend(included_items);
                }
                ScriptNode::Namespace(mut ns) => {
                    // Recursively process namespace contents
                    ns.items = self.process_items(current_source, ns.items)?;
                    result.push(ScriptNode::Namespace(ns));
                }
                ScriptNode::Pragma(pragma) => {
                    // Call pragma callback if set
                    if let Some(ref mut callback) = self.pragma_callback {
                        callback.on_pragma(&pragma.content)?;
                    }
                    // Don't include pragmas in output
                }
                ScriptNode::CustomDirective(_) => {
                    // Skip custom directives (or handle them)
                }
                _ => {
                    // Keep other items as-is
                    result.push(item);
                }
            }
        }

        Ok(result)
    }

    fn handle_include(&mut self, include_path: &str, from_source: &str) -> Result<Vec<ScriptNode>> {
        // Check for circular includes
        if self.included_sources.contains(include_path) {
            // Already included, skip
            return Ok(Vec::new());
        }

        // Get the source code via callback
        let included_source = if let Some(ref mut callback) = self.include_callback {
            callback.on_include(include_path, from_source)?
        } else {
            return Err(ParseError::SyntaxError {
                span: Span::new(
                    Position::new(0, 0, 0),
                    Position::new(0, 0, 0),
                    String::new(),
                ),
                message: format!("No include callback set, cannot resolve: {}", include_path),
            });
        };

        // Mark as included
        self.included_sources.insert(include_path.to_string());

        // Parse the included source recursively
        let included_script =
            self.build_from_source(&included_source)
                .map_err(|e| ParseError::SyntaxError {
                    span: e.span().clone(),
                    message: format!("Failed to parse included file '{}': {}", include_path, e),
                })?;

        Ok(included_script.items)
    }
}

impl Default for ScriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Default include callback that loads files from the filesystem
pub struct DefaultIncludeCallback {
    include_paths: Vec<std::path::PathBuf>,
}

impl DefaultIncludeCallback {
    pub fn new() -> Self {
        Self {
            include_paths: vec![std::path::PathBuf::from(".")],
        }
    }

    pub fn add_include_path(&mut self, path: std::path::PathBuf) {
        self.include_paths.push(path);
    }

    fn resolve_path(&self, filename: &str) -> Option<std::path::PathBuf> {
        use std::path::Path;

        let path = Path::new(filename);

        // Try absolute path
        if path.is_absolute() && path.exists() {
            return Some(path.to_path_buf());
        }

        // Try include paths
        for include_path in &self.include_paths {
            let full_path = include_path.join(filename);
            if full_path.exists() {
                return Some(full_path);
            }
        }

        None
    }
}

impl IncludeCallback for DefaultIncludeCallback {
    fn on_include(&mut self, include_path: &str, _from_source: &str) -> Result<String> {
        let resolved_path =
            self.resolve_path(include_path)
                .ok_or_else(|| ParseError::SyntaxError {
                    span: Span::new(
                        Position::new(0, 0, 0),
                        Position::new(0, 0, 0),
                        include_path.to_string(),
                    ),
                    message: format!("Include file not found: '{}'", include_path),
                })?;

        std::fs::read_to_string(&resolved_path).map_err(|e| ParseError::SyntaxError {
            span: Span::new(
                Position::new(0, 0, 0),
                Position::new(0, 0, 0),
                resolved_path.display().to_string(),
            ),
            message: format!("Failed to read '{}': {}", resolved_path.display(), e),
        })
    }
}

impl Default for DefaultIncludeCallback {
    fn default() -> Self {
        Self::new()
    }
}
