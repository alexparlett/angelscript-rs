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
//! - [`bytecode`]: Bytecode types (OpCode, BytecodeChunk, ConstantPool)
//! - [`context`]: Compilation context with namespace-aware resolution
//! - [`conversion`]: Type conversion system for type checking and overload resolution
//! - [`emit`]: High-level bytecode emitter
//! - [`expr`]: Expression compiler with bidirectional type checking
//! - [`expr_info`]: Expression type information
//! - [`overload`]: Overload resolution for function calls
//! - [`scope`]: Local scope management for function compilation
//! - [`stmt`]: Statement compiler for control flow and declarations
//! - [`type_resolver`]: Type resolution from AST to semantic types

pub mod bytecode;
pub mod context;
pub mod conversion;
pub mod emit;
pub mod expr;
mod expr_info;
pub mod operators;
pub mod overload;
pub mod passes;
pub mod scope;
pub mod stmt;
pub mod template;
pub mod type_resolver;

pub use context::{CompilationContext, Scope};
pub use conversion::{
    Conversion, ConversionKind, find_cast, find_conversion, find_handle_conversion,
    find_primitive_conversion,
};
pub use emit::{BreakError, BytecodeEmitter, JumpLabel};
pub use expr::ExprCompiler;
pub use expr_info::ExprInfo;
pub use overload::{OverloadMatch, resolve_overload};
pub use passes::{RegistrationOutput, RegistrationPass};
pub use scope::{CapturedVar, LocalScope, LocalVar, VarLookup};
pub use stmt::StmtCompiler;
pub use type_resolver::TypeResolver;

// Re-export CompilationError from core for convenience
pub use angelscript_core::CompilationError;

use angelscript_parser::ast::Script;

/// A compiled module containing bytecode and metadata.
#[derive(Debug, Default)]
pub struct CompiledModule {
    /// Compiled functions.
    pub functions: Vec<CompiledFunction>,
    /// Module-level constant pool.
    pub constants: bytecode::ConstantPool,
}

/// A compiled function.
#[derive(Debug)]
pub struct CompiledFunction {
    /// Function name.
    pub name: String,
    /// Compiled bytecode.
    pub bytecode: bytecode::BytecodeChunk,
}

/// Result of compilation.
pub struct CompilationResult {
    /// The compiled module.
    pub module: CompiledModule,
    /// Any errors that occurred.
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
/// TODO: This will take a SymbolRegistry once Phase 2 is complete.
pub struct Compiler;

impl Compiler {
    /// Compile a script.
    ///
    /// TODO: Will take `&SymbolRegistry` parameter once Phase 2 is complete.
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
                        bytecode: bytecode::BytecodeChunk::new(),
                    })
                } else {
                    None
                }
            })
            .collect();

        CompilationResult {
            module: CompiledModule {
                functions,
                constants: bytecode::ConstantPool::new(),
            },
            errors: vec![],
        }
    }
}
