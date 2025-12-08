//! Standard utility functions for AngelScript.
//!
//! Provides basic I/O and utility functions like `print`, `println`, etc.
//!
//! TODO: Implement macro-based registration for global functions.
//! For now, this is a placeholder module.

use angelscript_registry::Module;

/// Creates the std module with utility functions.
pub fn module() -> Module {
    Module::new()
    // TODO: Add std functions once we have a simpler API for global function registration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_module_is_root() {
        let m = module();
        assert!(m.namespace.is_empty());
    }
}
