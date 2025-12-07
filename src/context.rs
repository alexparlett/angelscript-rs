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

use angelscript_core::AngelScriptError;
use angelscript_ffi::{FfiRegistry, FfiRegistryBuilder, RegistrationError};
use angelscript_module::Module;
use angelscript_modules::default_modules;
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
/// Context is not Debug because FfiRegistryBuilder contains function pointers
pub struct Context<'app> {
    /// Installed native modules
    modules: Vec<Module<'app>>,
    /// Builder for FFI registry (consumed on seal)
    builder: Option<FfiRegistryBuilder>,
    /// Sealed FFI registry (available after seal)
    ffi_registry: Option<Arc<FfiRegistry>>,
}

impl<'app> Context<'app> {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            builder: Some(FfiRegistryBuilder::new()),
            ffi_registry: None,
        }
    }

    /// Create a context with default modules pre-installed and sealed.
    ///
    /// Default modules include:
    /// - `std` - I/O functions (print, println, eprint, eprintln)
    /// - `string` - String type and parsing/formatting functions
    /// - `math` - Math constants and functions (sin, cos, sqrt, PI, etc.)
    /// - `array` - Array template type (array<T>)
    /// - `dictionary` - Dictionary template type (dictionary)
    ///
    /// The context is NOT sealed - call `seal()` when done adding modules.
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
    /// # Errors
    ///
    /// Returns `ContextError::AlreadySealed` if the context has already been sealed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut ctx = Context::new();
    ///
    /// let mut math = Module::new(&["math"]);
    /// // ... register math functions ...
    ///
    /// ctx.install(math)?;
    /// ctx.seal()?;
    /// ```
    pub fn install(&mut self, module: Module<'app>) -> Result<(), ContextError> {
        // Check if already sealed
        if self.ffi_registry.is_some() {
            return Err(ContextError::AlreadySealed);
        }

        // Install module into builder
        // Builder should always exist if not sealed, but handle gracefully
        let builder = match self.builder.as_mut() {
            Some(b) => b,
            None => return Err(ContextError::AlreadySealed),
        };
        module.install_into(builder)?;

        // Keep module for reference
        self.modules.push(module);
        Ok(())
    }

    /// Seal the context, building the immutable FFI registry.
    ///
    /// After sealing, no more modules can be installed, but `create_unit()` can be called.
    /// This resolves all type references and builds the shared FFI registry.
    ///
    /// Calling `seal()` multiple times is safe - subsequent calls are no-ops.
    ///
    /// # Errors
    ///
    /// Returns an error if the FFI registry fails to build (e.g., unresolved types).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut ctx = Context::new();
    /// ctx.install(my_module)?;
    /// ctx.seal()?;
    ///
    /// let ctx = Arc::new(ctx);
    /// let unit = ctx.create_unit();
    /// ```
    pub fn seal(&mut self) -> Result<(), ContextError> {
        // Already sealed - no-op
        if self.ffi_registry.is_some() {
            return Ok(());
        }

        // Take builder and build the registry
        let builder = self.builder.take().unwrap_or_default();
        let registry = builder.build().map_err(ContextError::RegistryBuildFailed)?;
        self.ffi_registry = Some(Arc::new(registry));

        Ok(())
    }

    /// Check if the context has been sealed.
    pub fn is_sealed(&self) -> bool {
        self.ffi_registry.is_some()
    }

    /// Get the FFI registry (available after sealing).
    pub fn ffi_registry(&self) -> Option<&Arc<FfiRegistry>> {
        self.ffi_registry.as_ref()
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
    /// types, and global properties through the sealed FFI registry.
    ///
    /// # Errors
    ///
    /// Returns `ContextError::NotSealed` if the context has not been sealed.
    /// Call `seal()` first, or use `with_default_modules()` which auto-seals.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    ///
    /// let ctx = Arc::new(Context::with_default_modules()?);
    /// let mut unit = ctx.create_unit()?;
    ///
    /// unit.add_source("main.as", "void main() { }")?;
    /// unit.build()?;
    /// ```
    pub fn create_unit(self: &Arc<Self>) -> Result<Unit<'app>, ContextError> {
        if !self.is_sealed() {
            return Err(ContextError::NotSealed);
        }
        Ok(Unit::with_context(Arc::clone(self)))
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
    ModuleBuildFailed(#[from] RegistrationError),

    /// Context is already sealed - cannot install modules
    #[error("context is already sealed - cannot install modules after seal() or create_unit()")]
    AlreadySealed,

    /// Context is not sealed - must call seal() before creating units
    #[error("context is not sealed - call seal() before create_unit()")]
    NotSealed,

    /// Failed to build FFI registry
    #[error("failed to build FFI registry: {0:?}")]
    RegistryBuildFailed(Vec<RegistrationError>),
}

impl ContextError {
    /// Convert to a vector of `AngelScriptError`.
    ///
    /// This extracts the underlying registration errors, enabling unified
    /// error handling with the top-level `AngelScriptError` type.
    ///
    /// For variants that don't contain underlying errors (ModuleNotFound,
    /// ApplyFailed, AlreadySealed, NotSealed), this returns an empty vector.
    pub fn into_errors(self) -> Vec<AngelScriptError> {
        match self {
            ContextError::ModuleBuildFailed(err) => vec![AngelScriptError::from(err)],
            ContextError::RegistryBuildFailed(errors) => {
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
            ContextError::ModuleBuildFailed(err) => Some(AngelScriptError::from(err.clone())),
            ContextError::RegistryBuildFailed(errors) => {
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
        let mut ctx = Context::<'static>::new();
        ctx.seal().unwrap();
        let ctx = Arc::new(ctx);
        let unit = ctx.create_unit().unwrap();

        assert!(!unit.is_built());
    }

    #[test]
    fn context_create_unit_unsealed_returns_error() {
        let ctx = Arc::new(Context::<'static>::new());
        let result = ctx.create_unit();
        assert!(matches!(result, Err(ContextError::NotSealed)));
    }

    #[test]
    fn context_seal() {
        let mut ctx = Context::<'static>::new();
        assert!(!ctx.is_sealed());

        ctx.seal().unwrap();
        assert!(ctx.is_sealed());
        assert!(ctx.ffi_registry().is_some());
    }

    #[test]
    fn context_seal_idempotent() {
        let mut ctx = Context::<'static>::new();
        ctx.seal().unwrap();
        ctx.seal().unwrap(); // Should be no-op
        assert!(ctx.is_sealed());
    }

    #[test]
    fn context_install_after_seal_fails() {
        let mut ctx = Context::<'static>::new();
        ctx.seal().unwrap();

        let result = ctx.install(Module::new(&["test"]));
        assert!(matches!(result, Err(ContextError::AlreadySealed)));
    }

    #[test]
    fn context_with_default_modules_not_sealed() {
        let ctx = Context::<'static>::with_default_modules().unwrap();
        assert!(!ctx.is_sealed());
        assert!(ctx.ffi_registry().is_none()); // Not sealed yet
    }

    #[test]
    fn context_with_default_modules_can_seal() {
        let mut ctx = Context::<'static>::with_default_modules().unwrap();
        ctx.seal().unwrap();
        assert!(ctx.is_sealed());
        assert!(ctx.ffi_registry().is_some());
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
    fn context_error_display() {
        let err = ContextError::ModuleNotFound("foo".to_string());
        assert!(err.to_string().contains("module not found"));

        let err = ContextError::ApplyFailed("some error".to_string());
        assert!(err.to_string().contains("failed to apply"));

        let err = ContextError::AlreadySealed;
        assert!(err.to_string().contains("already sealed"));
    }

    #[test]
    fn context_error_into_errors_module_build() {
        let err = ContextError::ModuleBuildFailed(
            RegistrationError::TypeNotFound("Foo".to_string())
        );

        let errors = err.into_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].is_registration());
    }

    #[test]
    fn context_error_into_errors_registry_build() {
        let err = ContextError::RegistryBuildFailed(vec![
            RegistrationError::TypeNotFound("Foo".to_string()),
            RegistrationError::DuplicateType("Bar".to_string()),
        ]);

        let errors = err.into_errors();
        assert_eq!(errors.len(), 2);
        assert!(errors[0].is_registration());
        assert!(errors[1].is_registration());
    }

    #[test]
    fn context_error_into_errors_empty() {
        let err = ContextError::ModuleNotFound("foo".to_string());
        let errors = err.into_errors();
        assert!(errors.is_empty());

        let err = ContextError::AlreadySealed;
        let errors = err.into_errors();
        assert!(errors.is_empty());

        let err = ContextError::NotSealed;
        let errors = err.into_errors();
        assert!(errors.is_empty());
    }

    #[test]
    fn context_error_first_error() {
        let err = ContextError::RegistryBuildFailed(vec![
            RegistrationError::TypeNotFound("First".to_string()),
            RegistrationError::DuplicateType("Second".to_string()),
        ]);

        let first = err.first_error();
        assert!(first.is_some());
        assert!(first.unwrap().is_registration());

        // Empty errors return None
        let err = ContextError::AlreadySealed;
        assert!(err.first_error().is_none());
    }
}
