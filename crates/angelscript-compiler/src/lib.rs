// Allow approximate PI values in tests - we use 3.14/3.5 as convenient test float values
#![cfg_attr(test, allow(clippy::approx_constant))]

//! AngelScript Compiler
//!
//! A clean 2-pass compiler implementation for AngelScript.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Register all types and functions with complete signatures
//! - **Pass 1b (Type Completion)**: Resolve inheritance and copy inherited members
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
//! - [`function_compiler`]: Single function compilation
//! - [`overload`]: Overload resolution for function calls
//! - [`return_checker`]: Return path verification
//! - [`scope`]: Local scope management for function compilation
//! - [`stmt`]: Statement compiler for control flow and declarations
//! - [`type_resolver`]: Type resolution from AST to semantic types

pub mod bytecode;
pub mod context;
pub mod conversion;
pub mod emit;
pub mod expr;
mod expr_info;
pub mod field_init;
pub mod function_compiler;
pub mod operators;
pub mod overload;
pub mod passes;
pub mod return_checker;
pub mod scope;
pub mod stmt;
pub mod template;
pub mod type_resolver;

pub use context::CompilationContext;
pub use conversion::{
    Conversion, ConversionKind, find_cast, find_conversion, find_handle_conversion,
    find_primitive_conversion,
};
pub use emit::{BreakError, BytecodeEmitter, JumpLabel};
pub use expr::ExprCompiler;
pub use expr_info::{ExprInfo, ValueSource};
pub use function_compiler::FunctionCompiler;
pub use overload::{OverloadMatch, resolve_overload};
pub use passes::{
    CompilationOutput, CompilationPass, CompiledFunctionEntry, GlobalInitEntry, RegistrationOutput,
    RegistrationPass,
};
pub use return_checker::ReturnChecker;
pub use scope::{CapturedVar, LocalScope, LocalVar, VarLookup};
pub use stmt::StmtCompiler;
pub use type_resolver::TypeResolver;

// Re-export CompilationError from core for convenience
pub use angelscript_core::CompilationError;

use angelscript_core::{TypeHash, UnitId};
use angelscript_parser::ast::Script;
use angelscript_registry::SymbolRegistry;

/// A compiled module containing bytecode and metadata.
#[derive(Debug, Default)]
pub struct CompiledModule {
    /// Compiled functions.
    pub functions: Vec<CompiledFunction>,
    /// Global variable initializers (in declaration order).
    pub global_inits: Vec<CompiledFunction>,
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
/// Orchestrates the multi-pass compilation:
/// 1. Registration pass - collect type and function signatures
/// 2. Type completion pass - resolve inheritance, copy members
/// 3. Compilation pass - generate bytecode for function bodies
pub struct Compiler<'a> {
    /// Global registry with FFI types and shared types.
    global_registry: &'a SymbolRegistry,
    /// Unit ID for this compilation.
    unit_id: UnitId,
    /// String type hash from string factory (for string literal compilation).
    string_type_hash: Option<TypeHash>,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler with a global registry.
    ///
    /// # Arguments
    ///
    /// * `global_registry` - Registry containing FFI types and shared types
    /// * `unit_id` - Unique identifier for this compilation unit
    /// * `string_type_hash` - Type hash for string literals (from StringFactory)
    pub fn new(
        global_registry: &'a SymbolRegistry,
        unit_id: UnitId,
        string_type_hash: Option<TypeHash>,
    ) -> Self {
        Self {
            global_registry,
            unit_id,
            string_type_hash,
        }
    }

    /// Compile a script.
    ///
    /// Runs all compilation passes and returns the compiled module.
    pub fn compile(&self, script: &Script<'_>) -> CompilationResult {
        let mut ctx = CompilationContext::new(self.global_registry);

        // Set string type hash if configured (enables string literal compilation)
        if let Some(hash) = self.string_type_hash {
            ctx.set_string_type(hash);
        }

        let mut all_errors = Vec::new();

        // Pass 1: Registration
        let reg_pass = RegistrationPass::new(&mut ctx, self.unit_id);
        let reg_output = reg_pass.run(script);
        all_errors.extend(reg_output.errors);

        // Pass 1b: Type Completion
        let completion_pass = passes::TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            self.global_registry,
            reg_output.pending_resolutions,
        );
        let completion_output = completion_pass.run();
        all_errors.extend(completion_output.errors);

        // Pass 2: Compilation
        let compile_pass = CompilationPass::new(&mut ctx, self.unit_id);
        let (compile_output, constants) = compile_pass.run(script);
        all_errors.extend(compile_output.errors);

        // Build result
        let functions = compile_output
            .functions
            .into_iter()
            .map(|f| CompiledFunction {
                name: f.name,
                bytecode: f.bytecode,
            })
            .collect();

        let global_inits = compile_output
            .global_inits
            .into_iter()
            .map(|g| CompiledFunction {
                name: g.name,
                bytecode: g.bytecode,
            })
            .collect();

        CompilationResult {
            module: CompiledModule {
                functions,
                global_inits,
                constants,
            },
            errors: all_errors,
        }
    }
}
