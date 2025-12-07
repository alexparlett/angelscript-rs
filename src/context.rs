//! Execution context for the scripting engine.
//!
//! TODO: This module will be rebuilt with TypeRegistry in Phase 2.
//! For now, it's stubbed out to allow the crate to compile.

use std::sync::Arc;
use thiserror::Error;

use crate::unit::Unit;

/// Execution context that owns installed modules.
///
/// TODO: Will be rebuilt with TypeRegistry in Phase 2.
pub struct Context {
    // Placeholder - TypeRegistry will go here
}

impl Context {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self {}
    }

    /// Create a context with default modules pre-installed.
    ///
    /// TODO: Will register default modules when TypeRegistry is implemented.
    pub fn with_default_modules() -> Result<Self, ContextError> {
        Ok(Self::new())
    }

    /// Seal the context, building the immutable type registry.
    ///
    /// TODO: Will build TypeRegistry when implemented.
    pub fn seal(&mut self) -> Result<(), ContextError> {
        Ok(())
    }

    /// Check if the context has been sealed.
    pub fn is_sealed(&self) -> bool {
        true // Stubbed as always sealed for now
    }

    /// Create a new compilation unit from this context.
    pub fn create_unit(self: &Arc<Self>) -> Result<Unit, ContextError> {
        Ok(Unit::with_context(Arc::clone(self)))
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during context operations.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Context is already sealed - cannot install modules
    #[error("context is already sealed - cannot install modules after seal() or create_unit()")]
    AlreadySealed,

    /// Context is not sealed - must call seal() before creating units
    #[error("context is not sealed - call seal() before create_unit()")]
    NotSealed,

    /// Failed to build type registry
    #[error("failed to build type registry: {0}")]
    RegistryBuildFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_new() {
        let _ctx = Context::new();
    }

    #[test]
    fn context_default() {
        let _ctx = Context::default();
    }

    #[test]
    fn context_with_default_modules() {
        let _ctx = Context::with_default_modules().unwrap();
    }

    #[test]
    fn context_seal() {
        let mut ctx = Context::new();
        ctx.seal().unwrap();
        assert!(ctx.is_sealed());
    }

    #[test]
    fn context_create_unit() {
        let mut ctx = Context::new();
        ctx.seal().unwrap();
        let ctx = Arc::new(ctx);
        let _unit = ctx.create_unit().unwrap();
    }
}
