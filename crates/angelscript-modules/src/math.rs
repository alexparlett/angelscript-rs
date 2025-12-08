//! Math module providing constants and functions.
//!
//! All items are in the `math` namespace, e.g., `math::PI()`, `math::sin(x)`.
//!
//! TODO: Implement macro-based registration for global functions.
//! For now, this is a placeholder module.

use angelscript_registry::Module;

/// Creates the math module with constants and functions.
///
/// Everything is in the `math` namespace, accessible as `math::sin(x)`, `math::PI()`, etc.
pub fn module() -> Module {
    Module::in_namespace(&["math"])
    // TODO: Add math functions once we have a simpler API for global function registration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_module_in_namespace() {
        let m = module();
        assert_eq!(m.qualified_namespace(), "math");
    }
}
