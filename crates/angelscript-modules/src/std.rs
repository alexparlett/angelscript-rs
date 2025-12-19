//! Standard utility functions for AngelScript.
//!
//! Provides basic I/O and exception functions.

use crate::ScriptString;
use angelscript_core::{CallContext, native_error::NativeError};
use angelscript_registry::Module;

// =============================================================================
// EXCEPTION FUNCTIONS
// =============================================================================

/// Throw an exception with the given message.
///
/// This raises an exception that will be caught by the nearest try-catch block,
/// or will terminate script execution if uncaught.
///
/// Usage: `throw("Something went wrong")`
///
/// Note: The actual exception raising is handled by the VM through CallContext.
/// This function signature is for registration purposes.
#[angelscript_macros::function(generic, name = "throw")]
#[param(type = ScriptString, in)]
pub fn as_throw(_ctx: &mut CallContext) -> Result<(), NativeError> {
    // VM implementation will:
    // 1. Get the message from the stack via ctx
    // 2. Store the exception message in VM state
    // 3. Set exception state flag
    // 4. Return control to VM which will unwind to nearest TryBegin handler
    todo!()
}

/// Get information about the current exception.
///
/// This is typically called inside a catch block to retrieve the exception message.
/// Returns an empty string if no exception is active.
///
/// Usage: `string msg = getExceptionInfo();`
///
/// Note: The actual exception info retrieval is handled by the VM through CallContext.
#[angelscript_macros::function(generic, name = "getExceptionInfo")]
#[returns(type = ScriptString)]
pub fn as_get_exception_info(_ctx: &mut CallContext) -> Result<(), NativeError> {
    // VM implementation will:
    // 1. Get the current exception message from VM state
    // 2. Push a ScriptString onto the stack via ctx
    todo!()
}

// =============================================================================
// OUTPUT FUNCTIONS
// =============================================================================

/// Print formatted string to stdout without newline.
/// Usage: `print("Hello {}", name)`
#[angelscript_macros::function(generic, name = "print")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_print(_ctx: &mut CallContext) -> Result<(), NativeError> {
    todo!()
}

/// Print formatted string to stdout with newline.
/// Usage: `println("Hello {}", name)`
#[angelscript_macros::function(generic, name = "println")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_println(_ctx: &mut CallContext) -> Result<(), NativeError> {
    todo!()
}

/// Print formatted string to stderr without newline.
/// Usage: `eprint("Error: {}", msg)`
#[angelscript_macros::function(generic, name = "eprint")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_eprint(_ctx: &mut CallContext) -> Result<(), NativeError> {
    todo!()
}

/// Print formatted string to stderr with newline.
/// Usage: `eprintln("Error: {}", msg)`
#[angelscript_macros::function(generic, name = "eprintln")]
#[param(type = ScriptString, in)]
#[param(variable, in, variadic)]
pub fn as_eprintln(_ctx: &mut CallContext) -> Result<(), NativeError> {
    todo!()
}

// =============================================================================
// MODULE CREATION
// =============================================================================

/// Creates the std module with utility functions.
pub fn module() -> Module {
    Module::new()
        .function(as_throw)
        .function(as_get_exception_info)
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
        assert_eq!(m.functions.len(), 6); // throw, getExceptionInfo, print, println, eprint, eprintln
    }
}
