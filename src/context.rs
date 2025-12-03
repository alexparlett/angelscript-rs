//! Execution context for the scripting engine.
//!
//! A `Context` owns installed modules and provides the execution environment
//! for scripts.
//!
//! # Example
//!
//! ```ignore
//! use angelscript::{Context, Module};
//! use std::sync::Arc;
//!
//! // Create a context with default modules (string, array, dict, math, std)
//! let ctx = Arc::new(Context::with_default_modules().unwrap());
//!
//! // Create a compilation unit from the context
//! let mut unit = ctx.create_unit();
//! unit.add_source("main.as", "void main() { print(\"hello\"); }").unwrap();
//! unit.build().unwrap();
//! ```

use std::sync::Arc;
use thiserror::Error;

use crate::module::{FfiModuleError, Module};
use crate::modules::default_modules;
use crate::unit::Unit;

/// Execution context that owns installed modules.
///
/// The Context is the top-level container that:
/// - Owns all installed `Module`s (native registrations)
/// - Provides factory method for creating `Unit`s (compilation units)
/// - Will eventually own the VM and manage execution
///
/// # Lifetime
///
/// The `'app` lifetime parameter ensures that global property references
/// in installed modules remain valid for the lifetime of the Context.
#[derive(Debug)]
pub struct Context<'app> {
    /// Installed native modules
    modules: Vec<Module<'app>>,
}

impl<'app> Context<'app> {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    /// Create a context with default modules pre-installed.
    ///
    /// Default modules include:
    /// - `std` - I/O functions (print, println, eprint, eprintln)
    /// - `string` - String type and parsing/formatting functions
    /// - `math` - Math constants and functions (sin, cos, sqrt, PI, etc.)
    /// - `array` - Array template type (array<T>)
    /// - `dictionary` - Dictionary template type (dictionary)
    ///
    /// # Errors
    ///
    /// Returns an error if any default module fails to build.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        let mut ctx = Self::new();
        for module in default_modules().map_err(ContextError::ModuleBuildFailed)? {
            ctx.install(module)?;
        }
        Ok(ctx)
    }

    /// Install a module into the context.
    ///
    /// The module's functions, types, and global properties become available
    /// to scripts compiled against this context.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut ctx = Context::new();
    ///
    /// let mut math = Module::new(&["math"]);
    /// // ... register math functions ...
    ///
    /// ctx.install(math);
    /// ```
    pub fn install(&mut self, module: Module<'app>) -> Result<(), ContextError> {
        // Multiple modules can share the same namespace - they just contribute
        // different items (functions, types, etc.) to that namespace
        self.modules.push(module);
        Ok(())
    }

    /// Get the installed modules.
    pub fn modules(&self) -> &[Module<'app>] {
        &self.modules
    }

    /// Get a module by namespace.
    pub fn get_module(&self, namespace: &[&str]) -> Option<&Module<'app>> {
        let namespace: Vec<String> = namespace.iter().map(|s| (*s).to_string()).collect();
        self.modules.iter().find(|m| m.namespace() == namespace.as_slice())
    }

    /// Create a new compilation unit from this context.
    ///
    /// The unit will have access to all installed modules' functions,
    /// types, and global properties.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    ///
    /// let ctx = Arc::new(Context::with_default_modules()?);
    /// let mut unit = ctx.create_unit();
    ///
    /// unit.add_source("main.as", "void main() { }")?;
    /// unit.build()?;
    /// ```
    pub fn create_unit(self: &Arc<Self>) -> Unit<'app> {
        Unit::with_context(Arc::clone(self))
    }

    /// Get the total number of installed modules.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Get the total number of registered items across all modules.
    pub fn total_item_count(&self) -> usize {
        self.modules.iter().map(|m| m.item_count()).sum()
    }
}

impl Default for Context<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during context operations.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Module not found
    #[error("module not found: '{0}'")]
    ModuleNotFound(String),

    /// Failed to apply module to registry
    #[error("failed to apply module: {0}")]
    ApplyFailed(String),

    /// Failed to build a default module
    #[error("failed to build module: {0}")]
    ModuleBuildFailed(#[from] FfiModuleError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_new() {
        let ctx = Context::<'static>::new();
        assert_eq!(ctx.module_count(), 0);
    }

    #[test]
    fn context_default() {
        let ctx = Context::<'static>::default();
        assert_eq!(ctx.module_count(), 0);
    }

    #[test]
    fn context_with_default_modules() {
        let ctx = Context::<'static>::with_default_modules().unwrap();
        // Should have 5 default modules: std, string, math, array, dictionary
        assert_eq!(ctx.module_count(), 5);
    }

    #[test]
    fn context_install_module() {
        let mut ctx = Context::new();
        let module = Module::new(&["math"]);

        ctx.install(module).unwrap();

        assert_eq!(ctx.module_count(), 1);
    }

    #[test]
    fn context_install_multiple_modules() {
        let mut ctx = Context::new();

        ctx.install(Module::new(&["math"])).unwrap();
        ctx.install(Module::new(&["io"])).unwrap();
        ctx.install(Module::root()).unwrap();

        assert_eq!(ctx.module_count(), 3);
    }

    #[test]
    fn context_install_same_namespace_allowed() {
        // Multiple modules can contribute to the same namespace
        let mut ctx = Context::new();

        ctx.install(Module::new(&["math"])).unwrap();
        ctx.install(Module::new(&["math"])).unwrap();

        assert_eq!(ctx.module_count(), 2);
    }

    #[test]
    fn context_get_module() {
        let mut ctx = Context::new();
        ctx.install(Module::new(&["math"])).unwrap();
        ctx.install(Module::new(&["io"])).unwrap();

        let math = ctx.get_module(&["math"]);
        assert!(math.is_some());
        assert_eq!(math.unwrap().namespace(), &["math"]);

        let io = ctx.get_module(&["io"]);
        assert!(io.is_some());

        let nonexistent = ctx.get_module(&["nonexistent"]);
        assert!(nonexistent.is_none());
    }

    #[test]
    fn context_get_root_module() {
        let mut ctx = Context::new();
        ctx.install(Module::root()).unwrap();

        let root = ctx.get_module(&[]);
        assert!(root.is_some());
        assert!(root.unwrap().is_root());
    }

    #[test]
    fn context_modules() {
        let mut ctx = Context::new();
        ctx.install(Module::new(&["math"])).unwrap();
        ctx.install(Module::new(&["io"])).unwrap();

        let modules = ctx.modules();
        assert_eq!(modules.len(), 2);
    }

    #[test]
    fn context_create_unit() {
        let ctx = Arc::new(Context::<'static>::new());
        let unit = ctx.create_unit();

        assert!(!unit.is_built());
    }

    #[test]
    fn context_total_item_count() {
        let mut value1: i32 = 0;
        let mut value2: i32 = 0;

        let mut ctx = Context::new();

        let mut math = Module::new(&["math"]);
        math.register_global_property("int x", &mut value1).unwrap();

        let mut io = Module::new(&["io"]);
        io.register_global_property("int y", &mut value2).unwrap();

        ctx.install(math).unwrap();
        ctx.install(io).unwrap();

        assert_eq!(ctx.total_item_count(), 2);
    }

    #[test]
    fn context_debug() {
        let ctx = Context::<'static>::new();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("Context"));
    }

    #[test]
    fn context_error_display() {
        let err = ContextError::ModuleNotFound("foo".to_string());
        assert!(err.to_string().contains("module not found"));

        let err = ContextError::ApplyFailed("some error".to_string());
        assert!(err.to_string().contains("failed to apply"));
    }
}
