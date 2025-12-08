//! AngelScript Compiler
//!
//! A clean 2-pass compiler implementation for AngelScript.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Register all types and functions with complete signatures
//! - **Pass 2 (Compilation)**: Type check function bodies and generate bytecode
//!
//! ## Modules
//!
//! - [`types`]: Core type definitions (TypeHash, DataType, TypeDef, FunctionDef, ExprInfo, TypeBehaviors)
//! - [`registry`]: Script type and function registry
//! - [`context`]: Compilation context with name resolution
//! - [`passes`]: Compiler passes (registration and compilation)

use angelscript_core::CompilationError;
use angelscript_parser::ast::Script;

/// A compiled module containing bytecode and metadata.
#[derive(Debug, Default)]
pub struct CompiledModule {
    /// Compiled functions
    pub functions: Vec<CompiledFunction>,
}

/// A compiled function.
#[derive(Debug)]
pub struct CompiledFunction {
    pub name: String,
}

/// Result of compilation.
pub struct CompilationResult {
    /// The compiled module
    pub module: CompiledModule,
    /// Any errors that occurred
    pub errors: Vec<CompilationError>,
}

impl CompilationResult {
    /// Check if compilation succeeded (no errors).
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// The main compiler entry point.
///
/// TODO: This will take a TypeRegistry once Phase 2 is complete.
pub struct Compiler;

impl Compiler {
    /// Compile a script.
    ///
    /// TODO: Will take `&TypeRegistry` parameter once Phase 2 is complete.
    pub fn compile(script: &Script<'_>) -> CompilationResult {
        use angelscript_parser::ast::Item;

        // TODO: Implement actual compilation
        // For now, return a stub module with function names from the AST
        let functions = script
            .items()
            .iter()
            .filter_map(|item| {
                if let Item::Function(f) = item {
                    Some(CompiledFunction {
                        name: f.name.to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        CompilationResult {
            module: CompiledModule { functions },
            errors: vec![],
        }
    }
}
