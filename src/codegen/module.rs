//! Compiled module containing executable bytecode.
//!
//! This is the final output of code generation and the input to the VM.

use crate::codegen::CompiledBytecode;
use crate::semantic::SemanticError;
use crate::types::TypeHash;
use rustc_hash::FxHashMap;

/// Result of compiling all functions in a script.
///
/// This is the executable module that the VM will use to run the script.
#[derive(Debug)]
pub struct CompiledModule {
    /// Map of TypeHash to compiled bytecode (includes both regular functions and lambdas)
    pub functions: FxHashMap<TypeHash, CompiledBytecode>,

    /// All errors encountered during compilation
    pub errors: Vec<SemanticError>,
}
