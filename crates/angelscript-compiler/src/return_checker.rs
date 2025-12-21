//! Return path verification for non-void functions.
//!
//! This module provides [`ReturnChecker`] which verifies that all code paths
//! in a function return a value. This is required for non-void functions.
//!
//! # Example
//!
//! ```ignore
//! let checker = ReturnChecker::new();
//! if !checker.all_paths_return(&bytecode) {
//!     // Error: not all code paths return a value
//! }
//! ```

use crate::bytecode::{BytecodeChunk, OpCode};

/// Verifies all code paths return a value.
///
/// For non-void functions, we must ensure that every execution path
/// ends with a `Return` instruction. This is a simple bytecode analysis
/// that checks if the function ends with a return instruction.
///
/// Note: This is a simplified implementation that only checks if the
/// bytecode ends with a return. A more sophisticated implementation
/// would perform control flow graph analysis to verify all paths.
pub struct ReturnChecker;

impl ReturnChecker {
    /// Create a new return checker.
    pub fn new() -> Self {
        Self
    }

    /// Check if all code paths in the bytecode return a value.
    ///
    /// Returns `true` if the bytecode ends with a `Return` or `ReturnVoid`
    /// instruction, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `bytecode` - The compiled bytecode to check
    pub fn all_paths_return(&self, bytecode: &BytecodeChunk) -> bool {
        if bytecode.is_empty() {
            return false;
        }

        // Find the last opcode by scanning backwards
        // We need to find an actual opcode, not an operand byte
        self.ends_with_return(bytecode)
    }

    /// Check if the bytecode ends with a return instruction.
    fn ends_with_return(&self, bytecode: &BytecodeChunk) -> bool {
        // Scan the bytecode to find the last instruction
        let code = bytecode.code();
        if code.is_empty() {
            return false;
        }

        // Simple approach: scan from the beginning and track the last opcode
        let mut offset = 0;
        let mut last_op = None;

        while offset < code.len() {
            if let Some(op) = OpCode::from_u8(code[offset]) {
                last_op = Some(op);
                offset += 1;

                // Skip operands based on the opcode
                offset += Self::operand_size(op);
            } else {
                // Invalid opcode, move forward
                offset += 1;
            }
        }

        matches!(last_op, Some(OpCode::Return) | Some(OpCode::ReturnVoid))
    }

    /// Get the size of operands for an opcode.
    fn operand_size(op: OpCode) -> usize {
        match op {
            // No operands
            OpCode::PushNull
            | OpCode::PushTrue
            | OpCode::PushFalse
            | OpCode::PushZero
            | OpCode::PushOne
            | OpCode::Pop
            | OpCode::Dup
            | OpCode::GetThis
            | OpCode::Return
            | OpCode::ReturnVoid
            | OpCode::Add
            | OpCode::Sub
            | OpCode::Mul
            | OpCode::Div
            | OpCode::Mod
            | OpCode::Neg
            | OpCode::Pow
            | OpCode::BitAnd
            | OpCode::BitOr
            | OpCode::BitXor
            | OpCode::BitNot
            | OpCode::Shl
            | OpCode::Shr
            | OpCode::Ushr
            | OpCode::Eq
            | OpCode::Lt
            | OpCode::Le
            | OpCode::Gt
            | OpCode::Ge
            | OpCode::Not
            | OpCode::HandleToConst
            | OpCode::ValueToHandle
            | OpCode::InitListEnd
            | OpCode::PreInc
            | OpCode::PreDec
            | OpCode::PostInc
            | OpCode::PostDec
            | OpCode::HandleOf
            | OpCode::Swap
            | OpCode::AddRef
            | OpCode::Release
            | OpCode::TryEnd
            | OpCode::I8toI16
            | OpCode::I8toI32
            | OpCode::I8toI64
            | OpCode::I16toI32
            | OpCode::I16toI64
            | OpCode::I32toI64
            | OpCode::U8toU16
            | OpCode::U8toU32
            | OpCode::U8toU64
            | OpCode::U16toU32
            | OpCode::U16toU64
            | OpCode::U32toU64
            | OpCode::I64toI32
            | OpCode::I64toI16
            | OpCode::I64toI8
            | OpCode::I32toI16
            | OpCode::I32toI8
            | OpCode::I16toI8
            | OpCode::I32toF32
            | OpCode::I32toF64
            | OpCode::I64toF32
            | OpCode::I64toF64
            | OpCode::F32toI32
            | OpCode::F32toI64
            | OpCode::F64toI32
            | OpCode::F64toI64
            | OpCode::F32toF64
            | OpCode::F64toF32 => 0,

            // 1-byte operand
            OpCode::Constant
            | OpCode::PopN
            | OpCode::Pick
            | OpCode::GetLocal
            | OpCode::SetLocal
            | OpCode::CallFuncPtr => 1,

            // 2-byte operand (u16)
            OpCode::ConstantWide
            | OpCode::GetLocalWide
            | OpCode::SetLocalWide
            | OpCode::GetGlobal
            | OpCode::SetGlobal
            | OpCode::GetField
            | OpCode::SetField
            | OpCode::Jump
            | OpCode::JumpIfFalse
            | OpCode::JumpIfTrue
            | OpCode::Loop
            | OpCode::DerivedToBase
            | OpCode::ClassToInterface
            | OpCode::InstanceOf
            | OpCode::Cast
            | OpCode::FuncPtr
            | OpCode::InitListBegin
            | OpCode::TryBegin => 2,

            // Variable: hash (u16) + arg count (u8)
            OpCode::Call
            | OpCode::CallMethod
            | OpCode::CallVirtual
            | OpCode::New
            | OpCode::NewFactory => 3,

            // Interface call: hash (u16) + slot (u16) + arg count (u8)
            OpCode::CallInterface => 5,
        }
    }
}

impl Default for ReturnChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bytecode_does_not_return() {
        let chunk = BytecodeChunk::new();
        let checker = ReturnChecker::new();
        assert!(!checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_ending_with_return() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::PushOne, 1);
        chunk.write_op(OpCode::Return, 1);

        let checker = ReturnChecker::new();
        assert!(checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_ending_with_return_void() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::ReturnVoid, 1);

        let checker = ReturnChecker::new();
        assert!(checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_not_ending_with_return() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::PushOne, 1);
        chunk.write_op(OpCode::Pop, 1);

        let checker = ReturnChecker::new();
        assert!(!checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_with_jump_ending_with_return() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::PushTrue, 1);
        chunk.emit_jump(OpCode::JumpIfFalse, 1);
        chunk.write_op(OpCode::PushOne, 2);
        chunk.write_op(OpCode::Return, 2);

        let checker = ReturnChecker::new();
        assert!(checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_with_call_not_ending_with_return() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Call, 1);
        chunk.write_u16(0x1234, 1); // hash
        chunk.write_byte(2, 1); // arg count

        let checker = ReturnChecker::new();
        assert!(!checker.all_paths_return(&chunk));
    }

    #[test]
    fn bytecode_with_call_ending_with_return() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Call, 1);
        chunk.write_u16(0x1234, 1); // hash
        chunk.write_byte(2, 1); // arg count
        chunk.write_op(OpCode::Return, 2);

        let checker = ReturnChecker::new();
        assert!(checker.all_paths_return(&chunk));
    }
}
