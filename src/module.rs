//! High-level script module API.
//!
//! This module provides the main user-facing API for working with AngelScript.
//! Users register source files, build the module, and execute functions.
//!
//! # Example
//!
//! ```ignore
//! use angelscript::ScriptModule;
//!
//! let mut module = ScriptModule::new();
//!
//! // Add source files
//! module.add_source("player.as", r#"
//!     class Player {
//!         int health;
//!         Player(int h) { health = h; }
//!     }
//! "#)?;
//!
//! module.add_source("main.as", r#"
//!     void main() {
//!         Player p = Player(100);
//!     }
//! "#)?;
//!
//! // Build the module (parse + compile all sources)
//! module.build()?;
//!
//! // Execute functions
//! module.call("main", &[])?;
//! ```

use crate::semantic::{Compiler, CompiledModule, SemanticError};
use crate::{parse_lenient, ParseError};
use bumpalo::Bump;
use std::collections::{HashMap, HashSet};

/// A compiled script module ready for execution.
///
/// This is the main entry point for working with AngelScript. Users:
/// 1. Create a module with `ScriptModule::new()`
/// 2. Add source files with `add_source()`
/// 3. Build the module with `build()`
/// 4. Execute functions with `call()`
///
/// All parsing and compilation happens internally during `build()`.
#[derive(Default)]
pub struct ScriptModule {
    /// Source files to compile (filename → source code)
    sources: HashMap<String, String>,

    /// Hash of each source file (for change detection)
    source_hashes: HashMap<String, u64>,

    /// Files marked as dirty (need recompilation)
    dirty_files: HashSet<String>,

    /// Memory arena for AST allocation (created during build)
    arena: Option<Bump>,

    /// Compiled module (available after build)
    compiled: Option<CompiledModule>,

    /// Type registry (available after build)
    /// TODO: Cannot store Registry with lifetimes here - need to redesign module API
    // registry: Option<Registry>,

    /// Whether the module has been built
    is_built: bool,
}

impl ScriptModule {
    /// Create a new empty script module.
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            source_hashes: HashMap::new(),
            dirty_files: HashSet::new(),
            arena: None,
            compiled: None,
            // registry: None,
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

    /// Add a source file to the module.
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
    /// Returns an error if the module has already been built.
    /// Use `update_source()` to change a file after building, or `clear()` to rebuild from scratch.
    pub fn add_source(&mut self, filename: impl Into<String>, source: impl Into<String>) -> Result<(), ModuleError> {
        if self.is_built {
            return Err(ModuleError::AlreadyBuilt);
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
    /// Returns an error if the file doesn't exist in the module.
    pub fn update_source(&mut self, filename: impl AsRef<str>, source: impl Into<String>) -> Result<bool, ModuleError> {
        let filename = filename.as_ref();

        if !self.sources.contains_key(filename) {
            return Err(ModuleError::FileNotFound(filename.to_string()));
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

        // Create arena for AST allocation
        let arena = Bump::new();

        // Parse all sources
        let (scripts, all_parse_errors) = {
            #[cfg(feature = "profiling")]
            profiling::scope!("parsing");

            let mut all_parse_errors = Vec::new();
            let mut scripts = Vec::new();

            for (filename, source) in &self.sources {
                let (script, parse_errors) = parse_lenient(source, &arena);

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

        // Compile the script(s)
        let compilation_result = {
            #[cfg(feature = "profiling")]
            profiling::scope!("compilation");

            if scripts.len() == 1 {
                // Single file - use existing Compiler
                Compiler::compile(&scripts[0].1)
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
        // self.registry = Some(compilation_result.registry);  // TODO: Cannot store Registry with lifetimes
        self.arena = Some(arena);
        self.is_built = true;
        self.dirty_files.clear();

        Ok(())
    }

    /// Check if the module has been built.
    pub fn is_built(&self) -> bool {
        self.is_built
    }

    /// Get the compiled module (available after build).
    pub fn compiled(&self) -> Option<&CompiledModule> {
        self.compiled.as_ref()
    }

    /// Get the type registry (available after build).
    // TODO: Cannot return Registry with lifetimes - need to redesign module API
    // pub fn registry(&self) -> Option<&Registry> {
    //     self.registry.as_ref()
    // }

    /// Clear the module and reset to empty state.
    ///
    /// This allows you to reuse the module for a different set of sources.
    pub fn clear(&mut self) {
        self.sources.clear();
        self.source_hashes.clear();
        self.dirty_files.clear();
        self.arena = None;
        self.compiled = None;
        // self.registry = None;  // TODO: Cannot store Registry with lifetimes
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


/// Errors that can occur when adding sources or managing the module.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModuleError {
    /// The module has already been built
    #[error("Module has already been built. Use update_source() for hot reloading or clear() to rebuild.")]
    AlreadyBuilt,

    /// File not found in module
    #[error("File '{0}' not found in module")]
    FileNotFound(String),
}

/// Errors that can occur during module building.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// No sources have been added
    #[error("No sources added to module")]
    NoSources,

    /// Module has already been built
    #[error("Module has already been built")]
    AlreadyBuilt,

    /// Parse errors occurred
    #[error("Parse errors in {} file(s)", .0.len())]
    ParseErrors(Vec<(String, Vec<ParseError>)>),

    /// Compilation errors occurred
    #[error("Compilation errors: {0:?}")]
    CompilationErrors(Vec<SemanticError>),

    /// Multi-file compilation not yet supported
    #[error("Multi-file compilation not yet implemented")]
    MultiFileNotSupported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_module() {
        let module = ScriptModule::new();
        assert!(!module.is_built());
        assert_eq!(module.source_count(), 0);
    }

    #[test]
    fn add_source() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        assert_eq!(module.source_count(), 1);
    }

    #[test]
    fn build_simple_module() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();

        module.build().unwrap();

        assert!(module.is_built());
        assert!(module.function_count() >= 1);
    }

    #[test]
    fn cannot_add_after_build() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        let result = module.add_source("test2.as", "void foo() { }");
        assert!(result.is_err());
    }

    #[test]
    fn can_rebuild_after_clear() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        module.clear();
        module.add_source("test2.as", "void foo() { }").unwrap();
        module.build().unwrap();

        assert!(module.is_built());
    }

    #[test]
    fn build_fails_with_no_sources() {
        let mut module = ScriptModule::new();
        let result = module.build();
        assert!(matches!(result, Err(BuildError::NoSources)));
    }

    #[test]
    fn build_fails_with_parse_errors() {
        let mut module = ScriptModule::new();
        module.add_source("bad.as", "void main() { this is invalid }").unwrap();

        let result = module.build();
        assert!(matches!(result, Err(BuildError::ParseErrors(_))));
    }

    #[test]
    fn hot_reload_update_source() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        // Update the source
        let changed = module.update_source("test.as", "void main() { int x; }").unwrap();
        assert!(changed);
        assert!(module.has_pending_changes());

        // Rebuild with the new source
        module.rebuild().unwrap();
        assert!(!module.has_pending_changes());
    }

    #[test]
    fn hot_reload_no_change() {
        let mut module = ScriptModule::new();
        let source = "void main() { }";
        module.add_source("test.as", source).unwrap();
        module.build().unwrap();

        // Update with same source
        let changed = module.update_source("test.as", source).unwrap();
        assert!(!changed);
        assert!(!module.has_pending_changes());
    }

    #[test]
    fn hot_reload_nonexistent_file() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        let result = module.update_source("nonexistent.as", "void foo() { }");
        assert!(matches!(result, Err(ModuleError::FileNotFound(_))));
    }

    #[test]
    fn hot_reload_multiple_files() {
        let mut module = ScriptModule::new();
        module.add_source("file1.as", "void foo() { }").unwrap();

        // Can't add after build
        module.build().unwrap();
        let result = module.add_source("file2.as", "void bar() { }");
        assert!(matches!(result, Err(ModuleError::AlreadyBuilt)));

        // But can update existing files
        module.update_source("file1.as", "void foo() { int x; }").unwrap();
        module.rebuild().unwrap();
    }

    #[test]
    fn default_module() {
        let module = ScriptModule::default();
        assert!(!module.is_built());
        assert_eq!(module.source_count(), 0);
        assert_eq!(module.function_count(), 0);
    }

    #[test]
    fn build_already_built() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        // Trying to build again should fail
        let result = module.build();
        assert!(matches!(result, Err(BuildError::AlreadyBuilt)));
    }

    #[test]
    fn multi_file_not_supported() {
        let mut module = ScriptModule::new();
        module.add_source("file1.as", "void foo() { }").unwrap();
        module.add_source("file2.as", "void bar() { }").unwrap();

        let result = module.build();
        assert!(matches!(result, Err(BuildError::MultiFileNotSupported)));
    }

    #[test]
    fn compiled_returns_module() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();

        assert!(module.compiled().is_none());
        module.build().unwrap();
        assert!(module.compiled().is_some());
    }

    #[test]
    fn type_count_returns_zero() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        // Currently always returns 0 since registry is not stored
        assert_eq!(module.type_count(), 0);
    }

    #[test]
    fn dirty_files_list() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();

        // Before build, the file should be dirty
        assert!(module.dirty_files().contains("test.as"));

        module.build().unwrap();

        // After build, dirty files should be cleared
        assert!(module.dirty_files().is_empty());

        // Update and check dirty again
        module.update_source("test.as", "void main() { int x; }").unwrap();
        assert!(module.dirty_files().contains("test.as"));
    }

    #[test]
    fn rebuild_without_build() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();

        // Rebuild on unbuilt module should just build
        module.rebuild().unwrap();
        assert!(module.is_built());
    }

    #[test]
    fn rebuild_no_changes() {
        let mut module = ScriptModule::new();
        module.add_source("test.as", "void main() { }").unwrap();
        module.build().unwrap();

        // Clear dirty files and rebuild should be a no-op
        assert!(module.dirty_files().is_empty());
        module.rebuild().unwrap();
        assert!(module.is_built());
    }

    #[test]
    fn compilation_error() {
        let mut module = ScriptModule::new();
        // Valid syntax but semantic error (undefined variable)
        module.add_source("test.as", "void main() { x = 1; }").unwrap();

        let result = module.build();
        assert!(matches!(result, Err(BuildError::CompilationErrors(_))));
    }

    #[test]
    fn module_error_display() {
        let err = ModuleError::AlreadyBuilt;
        assert!(err.to_string().contains("already been built"));

        let err = ModuleError::FileNotFound("test.as".to_string());
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
}
