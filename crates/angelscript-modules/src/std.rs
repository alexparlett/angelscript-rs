//! Standard utility functions for AngelScript.
//!
//! Provides basic I/O functions with format string support.

use angelscript_core::CallContext;
use angelscript_registry::Module;
use crate::ScriptString;

// =============================================================================
// OUTPUT FUNCTIONS
// =============================================================================

/// Print formatted string to stdout without newline.
/// Usage: `print("Hello {}", name)`
#[angelscript_macros::function(generic, name = "print")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_print(_ctx: &CallContext) {
    todo!()
}

/// Print formatted string to stdout with newline.
/// Usage: `println("Hello {}", name)`
#[angelscript_macros::function(generic, name = "println")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_println(_ctx: &CallContext) {
    todo!()
}

/// Print formatted string to stderr without newline.
/// Usage: `eprint("Error: {}", msg)`
#[angelscript_macros::function(generic, name = "eprint")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_eprint(_ctx: &CallContext) {
    todo!()
}

/// Print formatted string to stderr with newline.
/// Usage: `eprintln("Error: {}", msg)`
#[angelscript_macros::function(generic, name = "eprintln")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_eprintln(_ctx: &CallContext) {
    todo!()
}

// =============================================================================
// MODULE CREATION
// =============================================================================

/// Creates the std module with utility functions.
pub fn module() -> Module {
    Module::new()
        .function(as_print)
        .function(as_println)
        .function(as_eprint)
        .function(as_eprintln)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_creates() {
        let m = module();
        assert!(m.namespace.is_empty());
        assert_eq!(m.functions.len(), 4);
    }
}
