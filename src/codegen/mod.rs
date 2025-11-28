//! Code generation - bytecode emission.
//!
//! This module handles the generation of bytecode from the validated AST.
//! It provides:
//! - Instruction set (IR)
//! - Bytecode emitter for generating instructions
//! - Compiled module output (for VM execution)

pub mod ir;
pub mod emitter;
pub mod module;

pub use emitter::{BytecodeEmitter, CompiledBytecode};
pub use ir::Instruction;
pub use module::CompiledModule;
