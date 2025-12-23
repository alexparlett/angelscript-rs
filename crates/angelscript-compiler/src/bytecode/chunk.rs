//! Bytecode chunk for compiled functions.
//!
//! A `BytecodeChunk` contains the compiled bytecode for a single function,
//! along with line number information for debugging.

use super::OpCode;

/// A chunk of compiled bytecode for a single function.
///
/// Constants are stored at module level in a `ConstantPool`, not per-function.
/// This allows deduplication of constants across functions.
#[derive(Debug, Clone, Default)]
pub struct BytecodeChunk {
    /// The bytecode instructions.
    code: Vec<u8>,
    /// Line numbers for debugging (parallel to code).
    /// Each entry corresponds to a byte in `code`.
    lines: Vec<u32>,
}

impl BytecodeChunk {
    /// Create a new empty bytecode chunk.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a bytecode chunk with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            code: Vec::with_capacity(capacity),
            lines: Vec::with_capacity(capacity),
        }
    }

    /// Write an opcode.
    pub fn write_op(&mut self, op: OpCode, line: u32) {
        self.code.push(op as u8);
        self.lines.push(line);
    }

    /// Write a byte operand.
    pub fn write_byte(&mut self, byte: u8, line: u32) {
        self.code.push(byte);
        self.lines.push(line);
    }

    /// Write a 16-bit operand (big-endian).
    pub fn write_u16(&mut self, value: u16, line: u32) {
        self.code.push((value >> 8) as u8);
        self.lines.push(line);
        self.code.push(value as u8);
        self.lines.push(line);
    }

    /// Write a 32-bit operand (big-endian).
    pub fn write_u32(&mut self, value: u32, line: u32) {
        self.code.push((value >> 24) as u8);
        self.lines.push(line);
        self.code.push((value >> 16) as u8);
        self.lines.push(line);
        self.code.push((value >> 8) as u8);
        self.lines.push(line);
        self.code.push(value as u8);
        self.lines.push(line);
    }

    /// Write a 64-bit operand (big-endian).
    pub fn write_u64(&mut self, value: u64, line: u32) {
        self.code.push((value >> 56) as u8);
        self.lines.push(line);
        self.code.push((value >> 48) as u8);
        self.lines.push(line);
        self.code.push((value >> 40) as u8);
        self.lines.push(line);
        self.code.push((value >> 32) as u8);
        self.lines.push(line);
        self.code.push((value >> 24) as u8);
        self.lines.push(line);
        self.code.push((value >> 16) as u8);
        self.lines.push(line);
        self.code.push((value >> 8) as u8);
        self.lines.push(line);
        self.code.push(value as u8);
        self.lines.push(line);
    }

    /// Get current code offset (for jump patching).
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// Emit a jump instruction and return the offset to patch later.
    ///
    /// The jump offset is initialized to 0xFFFF as a placeholder.
    pub fn emit_jump(&mut self, op: OpCode, line: u32) -> usize {
        self.write_op(op, line);
        let offset = self.code.len();
        self.write_u16(0xFFFF, line); // Placeholder
        offset
    }

    /// Patch a jump instruction at the given offset to jump to current position.
    ///
    /// # Panics
    ///
    /// Panics if the jump distance exceeds u16::MAX.
    pub fn patch_jump(&mut self, offset: usize) {
        let jump_distance = self.code.len() - offset - 2;
        assert!(
            jump_distance <= u16::MAX as usize,
            "jump distance {} exceeds u16::MAX",
            jump_distance
        );
        self.code[offset] = (jump_distance >> 8) as u8;
        self.code[offset + 1] = jump_distance as u8;
    }

    /// Emit a loop instruction that jumps back to the given offset.
    pub fn emit_loop(&mut self, loop_start: usize, line: u32) {
        self.write_op(OpCode::Loop, line);

        // +2 for the operand bytes we're about to write
        let offset = self.code.len() - loop_start + 2;
        assert!(
            offset <= u16::MAX as usize,
            "loop offset {} exceeds u16::MAX",
            offset
        );
        self.write_u16(offset as u16, line);
    }

    /// Get the bytecode.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Get the line numbers.
    pub fn lines(&self) -> &[u32] {
        &self.lines
    }

    /// Get the line number for a given offset.
    pub fn line_at(&self, offset: usize) -> Option<u32> {
        self.lines.get(offset).copied()
    }

    /// Get the length of the bytecode.
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Check if the chunk is empty.
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// Read a byte at the given offset.
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        self.code.get(offset).copied()
    }

    /// Read a u16 at the given offset (big-endian).
    pub fn read_u16(&self, offset: usize) -> Option<u16> {
        if offset + 1 < self.code.len() {
            Some(((self.code[offset] as u16) << 8) | (self.code[offset + 1] as u16))
        } else {
            None
        }
    }

    /// Read a u64 at the given offset (big-endian).
    pub fn read_u64(&self, offset: usize) -> Option<u64> {
        if offset + 7 < self.code.len() {
            Some(
                ((self.code[offset] as u64) << 56)
                    | ((self.code[offset + 1] as u64) << 48)
                    | ((self.code[offset + 2] as u64) << 40)
                    | ((self.code[offset + 3] as u64) << 32)
                    | ((self.code[offset + 4] as u64) << 24)
                    | ((self.code[offset + 5] as u64) << 16)
                    | ((self.code[offset + 6] as u64) << 8)
                    | (self.code[offset + 7] as u64),
            )
        } else {
            None
        }
    }

    /// Read an opcode at the given offset.
    pub fn read_op(&self, offset: usize) -> Option<OpCode> {
        self.code.get(offset).and_then(|&b| OpCode::from_u8(b))
    }

    /// Extract all opcodes from the chunk, skipping operands.
    ///
    /// This is useful for testing bytecode sequences without worrying about
    /// specific operand values or instruction offsets.
    pub fn opcodes(&self) -> Vec<OpCode> {
        let mut ops = Vec::new();
        let mut offset = 0;

        while offset < self.code.len() {
            if let Some(op) = self.read_op(offset) {
                ops.push(op);
                offset += 1 + op.operand_size();
            } else {
                // Invalid opcode, skip one byte
                offset += 1;
            }
        }

        ops
    }

    /// Check if this chunk contains exactly the given opcode sequence.
    ///
    /// This ignores operand values, only checking the opcodes themselves.
    /// Panics with a descriptive message if the sequences don't match.
    #[track_caller]
    pub fn assert_opcodes(&self, expected: &[OpCode]) {
        let actual = self.opcodes();
        assert_eq!(
            actual,
            expected,
            "Bytecode mismatch.\nExpected: {:?}\nActual:   {:?}",
            expected.iter().map(|op| op.name()).collect::<Vec<_>>(),
            actual.iter().map(|op| op.name()).collect::<Vec<_>>(),
        );
    }

    /// Check if this chunk contains the given opcodes (in order, but not necessarily contiguous).
    ///
    /// Useful for verifying key opcodes are present without checking every instruction.
    #[track_caller]
    pub fn assert_contains_opcodes(&self, expected: &[OpCode]) {
        let actual = self.opcodes();
        let mut expected_iter = expected.iter().peekable();

        for op in &actual {
            if expected_iter.peek() == Some(&op) {
                expected_iter.next();
            }
        }

        if expected_iter.peek().is_some() {
            let remaining: Vec<_> = expected_iter.map(|op| op.name()).collect();
            panic!(
                "Missing opcodes in sequence.\nExpected to find: {:?}\nActual bytecode:  {:?}",
                remaining,
                actual.iter().map(|op| op.name()).collect::<Vec<_>>(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chunk_is_empty() {
        let chunk = BytecodeChunk::new();
        assert!(chunk.is_empty());
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn write_op() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(42, 1);

        assert_eq!(chunk.len(), 2);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(42));
        assert_eq!(chunk.line_at(0), Some(1));
        assert_eq!(chunk.line_at(1), Some(1));
    }

    #[test]
    fn write_u16() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_u16(0x1234, 5);

        assert_eq!(chunk.len(), 2);
        assert_eq!(chunk.read_u16(0), Some(0x1234));
        assert_eq!(chunk.line_at(0), Some(5));
        assert_eq!(chunk.line_at(1), Some(5));
    }

    #[test]
    fn emit_and_patch_jump() {
        let mut chunk = BytecodeChunk::new();

        // Emit some code
        chunk.write_op(OpCode::PushTrue, 1);

        // Emit jump (will be patched later)
        let jump_offset = chunk.emit_jump(OpCode::JumpIfFalse, 2);

        // Emit more code
        chunk.write_op(OpCode::PushOne, 3);
        chunk.write_op(OpCode::PushZero, 3);

        // Patch the jump to skip to here
        chunk.patch_jump(jump_offset);

        // The jump should skip over PushOne and PushZero (2 bytes)
        assert_eq!(chunk.read_u16(jump_offset), Some(2));
    }

    #[test]
    fn emit_loop() {
        let mut chunk = BytecodeChunk::new();

        let loop_start = chunk.current_offset();
        chunk.write_op(OpCode::PushOne, 1);
        chunk.write_op(OpCode::Pop, 1);

        chunk.emit_loop(loop_start, 2);

        // Loop instruction + 2 byte offset
        assert_eq!(chunk.len(), 5);
        assert_eq!(chunk.read_op(2), Some(OpCode::Loop));
        // Offset should jump back 5 bytes (2 for body + 3 for loop instruction)
        assert_eq!(chunk.read_u16(3), Some(5));
    }

    #[test]
    fn read_byte_out_of_bounds() {
        let chunk = BytecodeChunk::new();
        assert_eq!(chunk.read_byte(0), None);
    }

    #[test]
    fn opcodes_extraction() {
        let mut chunk = BytecodeChunk::new();

        // Constant (1 byte operand) + Add (no operand) + SetLocal (1 byte operand)
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1); // constant index
        chunk.write_op(OpCode::Add, 1);
        chunk.write_op(OpCode::SetLocal, 1);
        chunk.write_byte(0, 1); // slot

        let ops = chunk.opcodes();
        assert_eq!(ops, vec![OpCode::Constant, OpCode::Add, OpCode::SetLocal]);
    }

    #[test]
    fn opcodes_with_wide_operands() {
        let mut chunk = BytecodeChunk::new();

        // Call has 3-byte operand (u16 + u8)
        chunk.write_op(OpCode::Call, 1);
        chunk.write_u16(0x1234, 1); // function hash index
        chunk.write_byte(2, 1); // arg count
        chunk.write_op(OpCode::Return, 1);

        let ops = chunk.opcodes();
        assert_eq!(ops, vec![OpCode::Call, OpCode::Return]);
    }

    #[test]
    fn assert_opcodes_success() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1);
        chunk.write_op(OpCode::SetLocal, 1);
        chunk.write_byte(0, 1);

        // Should not panic
        chunk.assert_opcodes(&[OpCode::Constant, OpCode::SetLocal]);
    }

    #[test]
    #[should_panic(expected = "Bytecode mismatch")]
    fn assert_opcodes_failure() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1);

        // Should panic - wrong opcode
        chunk.assert_opcodes(&[OpCode::GetLocal]);
    }

    #[test]
    fn assert_contains_opcodes_success() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::GetLocal, 1);
        chunk.write_byte(0, 1);
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1);
        chunk.write_op(OpCode::Add, 1);
        chunk.write_op(OpCode::SetLocal, 1);
        chunk.write_byte(0, 1);

        // Should find these in order (not contiguous)
        chunk.assert_contains_opcodes(&[OpCode::GetLocal, OpCode::Add, OpCode::SetLocal]);
    }

    #[test]
    #[should_panic(expected = "Missing opcodes")]
    fn assert_contains_opcodes_failure() {
        let mut chunk = BytecodeChunk::new();
        chunk.write_op(OpCode::Constant, 1);
        chunk.write_byte(0, 1);

        // Should panic - Sub not present
        chunk.assert_contains_opcodes(&[OpCode::Constant, OpCode::Sub]);
    }
}
