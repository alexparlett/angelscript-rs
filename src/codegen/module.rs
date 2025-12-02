//! Compiled module containing executable bytecode.
//!
//! This is the final output of code generation and the input to the VM.

use crate::codegen::CompiledBytecode;
use crate::semantic::types::type_def::FunctionId;
use crate::semantic::SemanticError;
use rustc_hash::FxHashMap;

/// Result of compiling all functions in a script.
///
/// This is the executable module that the VM will use to run the script.
#[derive(Debug)]
pub struct CompiledModule {
    /// Map of FunctionId to compiled bytecode (includes both regular functions and lambdas)
    pub functions: FxHashMap<FunctionId, CompiledBytecode>,

    /// All errors encountered during compilation
    pub errors: Vec<SemanticError>,
}
