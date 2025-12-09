//! Bytecode types for the AngelScript compiler.
//!
//! This module contains the core bytecode types:
//!
//! - [`OpCode`] - The instruction set for the VM
//! - [`BytecodeChunk`] - Compiled bytecode for a function
//! - [`Constant`] and [`ConstantPool`] - Module-level constant storage

mod chunk;
mod constant;
mod opcode;

pub use chunk::BytecodeChunk;
pub use constant::{Constant, ConstantPool};
pub use opcode::OpCode;
