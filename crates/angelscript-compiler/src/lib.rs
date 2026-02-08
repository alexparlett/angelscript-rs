//! AngelScript Compiler
//!
//! This crate defines the compiler interface and bytecode types for AngelScript.
//! The compilation logic is not yet implemented.

pub mod bytecode;

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
pub struct Compiler<'a> {
    /// Global registry with FFI types and shared types.
    _global_registry: &'a SymbolRegistry,
    /// Unit ID for this compilation.
    _unit_id: UnitId,
    /// String type hash from string factory (for string literal compilation).
    _string_type_hash: Option<TypeHash>,
}

impl<'a> Compiler<'a> {
    /// Create a new compiler with a global registry.
    pub fn new(
        global_registry: &'a SymbolRegistry,
        unit_id: UnitId,
        string_type_hash: Option<TypeHash>,
    ) -> Self {
        Self {
            _global_registry: global_registry,
            _unit_id: unit_id,
            _string_type_hash: string_type_hash,
        }
    }

    /// Compile a script.
    ///
    /// Currently a stub that returns an empty module with no errors.
    pub fn compile(&self, _script: &Script<'_>) -> CompilationResult {
        CompilationResult {
            module: CompiledModule::default(),
            errors: Vec::new(),
        }
    }
}
