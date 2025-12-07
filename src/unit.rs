//! Compilation unit API.
//!
//! This module provides the compilation unit for working with AngelScript.
//! Users register source files, build the unit, and execute functions.
//!
//! # Example
//!
//! ```ignore
//! use angelscript::{Context, Unit};
//! use std::sync::Arc;
//!
//! // Create a context with default modules
//! let ctx = Arc::new(Context::with_default_modules().unwrap());
//!
//! // Create a unit from the context
//! let mut unit = ctx.create_unit();
//!
//! // Add source files
//! unit.add_source("player.as", r#"
//!     class Player {
//!         int health;
//!         Player(int h) { health = h; }
//!     }
//! "#)?;
//!
//! unit.add_source("main.as", r#"
//!     void main() {
//!         Player p = Player(100);
//!     }
//! "#)?;
//!
//! // Build the unit (parse + compile all sources)
//! unit.build()?;
//!
//! // Execute functions
//! unit.call("main", &[])?;
//! ```

use crate::context::Context;
use angelscript_core::{AngelScriptError, CompilationError};
use angelscript_ffi::FfiRegistryBuilder;
use angelscript_compiler::{Compiler, CompiledModule};
use angelscript_parser::ast::{Parser, ParseError};
use bumpalo::Bump;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A compilation unit ready for execution.
///
/// This is the main entry point for working with AngelScript. Users:
/// 1. Create a unit with `Context::create_unit()` or `Unit::new()`
/// 2. Add source files with `add_source()`
/// 3. Build the unit with `build()`
/// 4. Execute functions with `call()`
///
/// All parsing and compilation happens internally during `build()`.
pub struct Unit<'app> {
    /// Reference to the context (if created via Context::create_unit)
    context: Option<Arc<Context<'app>>>,

    /// Source files to compile (filename → source code)
    sources: HashMap<String, String>,

    /// Hash of each source file (for change detection)
    source_hashes: HashMap<String, u64>,

    /// Files marked as dirty (need recompilation)
    dirty_files: HashSet<String>,

    /// Memory arena for AST allocation (created during build)
    arena: Bump,

    /// Compiled module (available after build)
    compiled: Option<CompiledModule>,

    /// Whether the module has been built
    is_built: bool,
}

impl Default for Unit<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'app> Unit<'app> {
    /// Create a new empty compilation unit without a context.
    ///
    /// This unit will not have access to FFI modules. For access to
    /// built-in types like string, array, and dictionary, use
    /// `Context::create_unit()` instead.
    pub fn new() -> Self {
        Self {
            context: None,
            sources: HashMap::new(),
            source_hashes: HashMap::new(),
            dirty_files: HashSet::new(),
            arena: Bump::new(),
            compiled: None,
            is_built: false,
        }
    }

    /// Create a compilation unit with a context.
    ///
    /// The unit will have access to all modules installed in the context.
    /// This is typically called via `Context::create_unit()`.
    pub fn with_context(context: Arc<Context<'app>>) -> Self {
        Self {
            context: Some(context),
            sources: HashMap::new(),
            source_hashes: HashMap::new(),
            dirty_files: HashSet::new(),
            arena: Bump::new(),
            compiled: None,
            is_built: false,
        }
    }

    /// Compute a simple hash of source code for change detection.
    fn hash_source(source: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    /// Add a source file to the unit.
    ///
    /// The source will be parsed and compiled when `build()` is called.
    ///
    /// # Parameters
    ///
    /// - `filename`: Name for error reporting (e.g., "player.as")
    /// - `source`: The AngelScript source code
    ///
    /// # Errors
    ///
    /// Returns an error if the unit has already been built.
    /// Use `update_source()` to change a file after building, or `clear()` to rebuild from scratch.
    pub fn add_source(&mut self, filename: impl Into<String>, source: impl Into<String>) -> Result<(), UnitError> {
        if self.is_built {
            return Err(UnitError::AlreadyBuilt);
        }

        let filename = filename.into();
        let source = source.into();
        let hash = Self::hash_source(&source);

        self.sources.insert(filename.clone(), source);
        self.source_hashes.insert(filename.clone(), hash);
        self.dirty_files.insert(filename);

        Ok(())
    }

    /// Update a source file and mark it for recompilation.
    ///
    /// This allows hot-reloading: you can change a source file and call `rebuild()`
    /// to recompile only the changed files.
    ///
    /// # Parameters
    ///
    /// - `filename`: The file to update (must already exist)
    /// - `source`: The new source code
    ///
    /// # Returns
    ///
    /// Returns `true` if the source actually changed (based on hash comparison),
    /// `false` if it's identical to the existing source.
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist in the unit.
    pub fn update_source(&mut self, filename: impl AsRef<str>, source: impl Into<String>) -> Result<bool, UnitError> {
        let filename = filename.as_ref();

        // Check source_hashes since sources is cleared after build
        if !self.source_hashes.contains_key(filename) {
            return Err(UnitError::FileNotFound(filename.to_string()));
        }

        let source = source.into();
        let new_hash = Self::hash_source(&source);
        let old_hash = self.source_hashes.get(filename).copied();

        // Check if source actually changed
        let changed = Some(new_hash) != old_hash;

        if changed {
            self.sources.insert(filename.to_string(), source);
            self.source_hashes.insert(filename.to_string(), new_hash);
            self.dirty_files.insert(filename.to_string());
        }

        Ok(changed)
    }

    /// Rebuild the module, recompiling only changed files.
    ///
    /// This is used for hot-reloading after calling `update_source()`.
    ///
    /// # Errors
    ///
    /// Returns errors if parsing or compilation fails.
    pub fn rebuild(&mut self) -> Result<(), BuildError> {
        if !self.is_built {
            return self.build();
        }

        if self.dirty_files.is_empty() {
            // Nothing changed
            return Ok(());
        }

        // For now, just rebuild everything
        // TODO: Implement true incremental compilation
        self.is_built = false;
        self.build()?;
        self.dirty_files.clear();

        Ok(())
    }

    /// Check if there are pending changes that need recompilation.
    pub fn has_pending_changes(&self) -> bool {
        !self.dirty_files.is_empty()
    }

    /// Get the list of files that have changed and need recompilation.
    pub fn dirty_files(&self) -> &HashSet<String> {
        &self.dirty_files
    }

    /// Build the module by parsing and compiling all sources.
    ///
    /// This performs:
    /// 1. Parsing all source files
    /// 2. Semantic analysis (3 passes)
    /// 3. Bytecode generation
    ///
    /// After building, you can call functions with `call()`.
    ///
    /// # Errors
    ///
    /// Returns errors if parsing or compilation fails.
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn build(&mut self) -> Result<(), BuildError> {
        if self.is_built {
            return Err(BuildError::AlreadyBuilt);
        }

        if self.sources.is_empty() {
            return Err(BuildError::NoSources);
        }

        // Parse all sources
        let (scripts, all_parse_errors) = {
            let mut all_parse_errors = Vec::new();
            let mut scripts = Vec::new();

            for (filename, source) in &self.sources {
                let (script, parse_errors) = Parser::parse_lenient(source, &self.arena);

                if !parse_errors.is_empty() {
                    all_parse_errors.push((filename.clone(), parse_errors));
                }

                scripts.push((filename.clone(), script));
            }

            (scripts, all_parse_errors)
        };

        // If there were parse errors, fail early
        if !all_parse_errors.is_empty() {
            return Err(BuildError::ParseErrors(all_parse_errors));
        }

        // For now, we only support single-file compilation
        // TODO: Implement multi-file compilation with shared registry
        if scripts.len() > 1 {
            return Err(BuildError::MultiFileNotSupported);
        }

        // Get FFI registry from context (if available)
        let ffi_registry = self
            .context
            .as_ref()
            .and_then(|ctx| ctx.ffi_registry().cloned())
            .unwrap_or_else(|| {
                // Create default FFI registry with primitives only
                Arc::new(FfiRegistryBuilder::new().build().unwrap())
            });

        // Compile the script(s) with FFI registry
        let compilation_result = {
            if scripts.len() == 1 {
                // Single file - compile with FFI registry from context
                Compiler::compile(&scripts[0].1, ffi_registry)
            } else {
                // Multi-file - TODO: implement
                todo!("Multi-file compilation not yet implemented")
            }
        };

        // Check for compilation errors
        if !compilation_result.is_success() {
            return Err(BuildError::CompilationErrors(compilation_result.errors));
        }

        // Store the compiled module and registry
        self.compiled = Some(compilation_result.module);
        
        self.is_built = true;
        self.dirty_files.clear();

        // Clear source strings - they're no longer needed since the lexer copies
        // all string content into the arena. We keep the hashes for hot-reload
        // change detection.
        self.sources.clear();

        Ok(())
    }

    /// Check if the unit has been built.
    pub fn is_built(&self) -> bool {
        self.is_built
    }

    /// Get the compiled module (available after build).
    pub fn compiled(&self) -> Option<&CompiledModule> {
        self.compiled.as_ref()
    }

    // TODO: Cannot return CompilationContext with lifetimes - need to redesign module API
    // /// Get the compilation context (available after build).
    // pub fn compilation_context(&self) -> Option<&CompilationContext> {
    //     self.compilation_context.as_ref()
    // }

    /// Clear the unit and reset to empty state.
    ///
    /// This allows you to reuse the unit for a different set of sources.
    pub fn clear(&mut self) {
        self.sources.clear();
        self.source_hashes.clear();
        self.dirty_files.clear();
        self.arena.reset();
        self.compiled = None;
        // self.compilation_context = None;  // TODO: Cannot store CompilationContext with lifetimes
        self.is_built = false;
    }

    /// Get the number of source files in the module.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Get the number of compiled functions (available after build).
    pub fn function_count(&self) -> usize {
        self.compiled.as_ref().map_or(0, |c| c.functions.len())
    }

    /// Get the number of registered types (available after build).
    pub fn type_count(&self) -> usize {
        // TODO: Cannot access registry with lifetimes
        // self.registry.as_ref().map_or(0, |r| r.type_count())
        0
    }
}


/// Errors that can occur when adding sources or managing the unit.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UnitError {
    /// The unit has already been built
    #[error("Unit has already been built. Use update_source() for hot reloading or clear() to rebuild.")]
    AlreadyBuilt,

    /// File not found in unit
    #[error("File '{0}' not found in unit")]
    FileNotFound(String),
}

/// Errors that can occur during unit building.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// No sources have been added
    #[error("No sources added to unit")]
    NoSources,

    /// Unit has already been built
    #[error("Unit has already been built")]
    AlreadyBuilt,

    /// Parse errors occurred
    #[error("Parse errors in {} file(s)", .0.len())]
    ParseErrors(Vec<(String, Vec<ParseError>)>),

    /// Compilation errors occurred
    #[error("Compilation errors: {0:?}")]
    CompilationErrors(Vec<CompilationError>),

    /// Multi-file compilation not yet supported
    #[error("Multi-file compilation not yet implemented")]
    MultiFileNotSupported,
}

impl BuildError {
    /// Convert to a vector of `AngelScriptError`.
    ///
    /// This extracts the underlying parse or compilation errors, enabling
    /// unified error handling with the top-level `AngelScriptError` type.
    ///
    /// For variants that don't contain underlying errors (NoSources, AlreadyBuilt,
    /// MultiFileNotSupported), this returns an empty vector.
    pub fn into_errors(self) -> Vec<AngelScriptError> {
        match self {
            BuildError::ParseErrors(file_errors) => {
                file_errors
                    .into_iter()
                    .flat_map(|(_, errors)| errors)
                    .map(AngelScriptError::from)
                    .collect()
            }
            BuildError::CompilationErrors(errors) => {
                errors.into_iter().map(AngelScriptError::from).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Get the first error as an `AngelScriptError`, if any.
    ///
    /// This is useful when you only need to report a single error.
    pub fn first_error(&self) -> Option<AngelScriptError> {
        match self {
            BuildError::ParseErrors(file_errors) => {
                file_errors
                    .iter()
                    .flat_map(|(_, errors)| errors)
                    .next()
                    .cloned()
                    .map(AngelScriptError::from)
            }
            BuildError::CompilationErrors(errors) => {
                errors.first().cloned().map(AngelScriptError::from)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_unit() {
        let unit = Unit::new();
        assert!(!unit.is_built());
        assert_eq!(unit.source_count(), 0);
    }

    #[test]
    fn add_source() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        assert_eq!(unit.source_count(), 1);
    }

    #[test]
    fn build_simple_unit() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();

        unit.build().unwrap();

        assert!(unit.is_built());
        assert!(unit.function_count() >= 1);
    }

    #[test]
    fn cannot_add_after_build() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        let result = unit.add_source("test2.as", "void foo() { }");
        assert!(result.is_err());
    }

    #[test]
    fn can_rebuild_after_clear() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        unit.clear();
        unit.add_source("test2.as", "void foo() { }").unwrap();
        unit.build().unwrap();

        assert!(unit.is_built());
    }

    #[test]
    fn build_fails_with_no_sources() {
        let mut unit = Unit::new();
        let result = unit.build();
        assert!(matches!(result, Err(BuildError::NoSources)));
    }

    #[test]
    fn build_fails_with_parse_errors() {
        let mut unit = Unit::new();
        unit.add_source("bad.as", "void main() { this is invalid }").unwrap();

        let result = unit.build();
        assert!(matches!(result, Err(BuildError::ParseErrors(_))));
    }

    #[test]
    fn hot_reload_update_source() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        // Update the source
        let changed = unit.update_source("test.as", "void main() { int x; }").unwrap();
        assert!(changed);
        assert!(unit.has_pending_changes());

        // Rebuild with the new source
        unit.rebuild().unwrap();
        assert!(!unit.has_pending_changes());
    }

    #[test]
    fn hot_reload_no_change() {
        let mut unit = Unit::new();
        let source = "void main() { }";
        unit.add_source("test.as", source).unwrap();
        unit.build().unwrap();

        // Update with same source
        let changed = unit.update_source("test.as", source).unwrap();
        assert!(!changed);
        assert!(!unit.has_pending_changes());
    }

    #[test]
    fn hot_reload_nonexistent_file() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        let result = unit.update_source("nonexistent.as", "void foo() { }");
        assert!(matches!(result, Err(UnitError::FileNotFound(_))));
    }

    #[test]
    fn hot_reload_multiple_files() {
        let mut unit = Unit::new();
        unit.add_source("file1.as", "void foo() { }").unwrap();

        // Can't add after build
        unit.build().unwrap();
        let result = unit.add_source("file2.as", "void bar() { }");
        assert!(matches!(result, Err(UnitError::AlreadyBuilt)));

        // But can update existing files
        unit.update_source("file1.as", "void foo() { int x; }").unwrap();
        unit.rebuild().unwrap();
    }

    #[test]
    fn default_unit() {
        let unit = Unit::default();
        assert!(!unit.is_built());
        assert_eq!(unit.source_count(), 0);
        assert_eq!(unit.function_count(), 0);
    }

    #[test]
    fn build_already_built() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        // Trying to build again should fail
        let result = unit.build();
        assert!(matches!(result, Err(BuildError::AlreadyBuilt)));
    }

    #[test]
    fn multi_file_not_supported() {
        let mut unit = Unit::new();
        unit.add_source("file1.as", "void foo() { }").unwrap();
        unit.add_source("file2.as", "void bar() { }").unwrap();

        let result = unit.build();
        assert!(matches!(result, Err(BuildError::MultiFileNotSupported)));
    }

    #[test]
    fn compiled_returns_module() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();

        assert!(unit.compiled().is_none());
        unit.build().unwrap();
        assert!(unit.compiled().is_some());
    }

    #[test]
    fn type_count_returns_zero() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        // Currently always returns 0 since registry is not stored
        assert_eq!(unit.type_count(), 0);
    }

    #[test]
    fn dirty_files_list() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();

        // Before build, the file should be dirty
        assert!(unit.dirty_files().contains("test.as"));

        unit.build().unwrap();

        // After build, dirty files should be cleared
        assert!(unit.dirty_files().is_empty());

        // Update and check dirty again
        unit.update_source("test.as", "void main() { int x; }").unwrap();
        assert!(unit.dirty_files().contains("test.as"));
    }

    #[test]
    fn rebuild_without_build() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();

        // Rebuild on unbuilt unit should just build
        unit.rebuild().unwrap();
        assert!(unit.is_built());
    }

    #[test]
    fn rebuild_no_changes() {
        let mut unit = Unit::new();
        unit.add_source("test.as", "void main() { }").unwrap();
        unit.build().unwrap();

        // Clear dirty files and rebuild should be a no-op
        assert!(unit.dirty_files().is_empty());
        unit.rebuild().unwrap();
        assert!(unit.is_built());
    }

    #[test]
    #[ignore = "requires semantic analysis in compiler"]
    fn compilation_error() {
        let mut unit = Unit::new();
        // Valid syntax but semantic error (undefined variable)
        unit.add_source("test.as", "void main() { x = 1; }").unwrap();

        let result = unit.build();
        assert!(matches!(result, Err(BuildError::CompilationErrors(_))));
    }

    #[test]
    fn unit_error_display() {
        let err = UnitError::AlreadyBuilt;
        assert!(err.to_string().contains("already been built"));

        let err = UnitError::FileNotFound("test.as".to_string());
        assert!(err.to_string().contains("test.as"));
    }

    #[test]
    fn build_error_display() {
        let err = BuildError::NoSources;
        assert!(err.to_string().contains("No sources"));

        let err = BuildError::AlreadyBuilt;
        assert!(err.to_string().contains("already been built"));

        let err = BuildError::MultiFileNotSupported;
        assert!(err.to_string().contains("not yet implemented"));
    }

    #[test]
    fn build_error_into_errors_parse() {
        use angelscript_core::{ParseErrorKind, Span};

        let parse_err = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 5, 3),
            "expected ';'".to_string(),
        );
        let err = BuildError::ParseErrors(vec![
            ("test.as".to_string(), vec![parse_err]),
        ]);

        let errors = err.into_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_parse());
    }

    #[test]
    fn build_error_into_errors_compilation() {
        use angelscript_core::Span;

        let comp_err = CompilationError::UnknownType {
            name: "Foo".to_string(),
            span: Span::new(1, 1, 3),
        };
        let err = BuildError::CompilationErrors(vec![comp_err]);

        let errors = err.into_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_compilation());
    }

    #[test]
    fn build_error_into_errors_empty() {
        let err = BuildError::NoSources;
        let errors = err.into_errors();
        assert!(errors.is_empty());

        let err = BuildError::AlreadyBuilt;
        let errors = err.into_errors();
        assert!(errors.is_empty());

        let err = BuildError::MultiFileNotSupported;
        let errors = err.into_errors();
        assert!(errors.is_empty());
    }

    #[test]
    fn build_error_first_error() {
        use angelscript_core::{ParseErrorKind, Span};

        let parse_err1 = ParseError::new(
            ParseErrorKind::ExpectedToken,
            Span::new(1, 5, 3),
            "first".to_string(),
        );
        let parse_err2 = ParseError::new(
            ParseErrorKind::UnexpectedToken,
            Span::new(2, 10, 5),
            "second".to_string(),
        );
        let err = BuildError::ParseErrors(vec![
            ("test.as".to_string(), vec![parse_err1, parse_err2]),
        ]);

        let first = err.first_error();
        assert!(first.is_some());
        assert!(first.unwrap().is_parse());

        // Empty errors return None
        let err = BuildError::NoSources;
        assert!(err.first_error().is_none());
    }
}
