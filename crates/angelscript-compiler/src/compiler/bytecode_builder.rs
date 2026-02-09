//! Bytecode builder with label-based jump resolution.
//!
//! During compilation, jump targets use labels rather than absolute offsets.
//! Labels are resolved to concrete offsets when `finalize()` is called.

use angelscript_core::opcode::{ByteCode, Instruction, OpCode};

/// A label ID for jump targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

/// Bytecode builder that accumulates instructions and resolves labels.
#[derive(Debug)]
pub struct BytecodeBuilder {
    instructions: Vec<BuilderEntry>,
    next_label: u32,
}

/// An entry in the builder: either an instruction or a label.
#[derive(Debug, Clone)]
enum BuilderEntry {
    Instruction(Instruction),
    Label(Label),
}

impl BytecodeBuilder {
    pub fn new() -> Self {
        BytecodeBuilder {
            instructions: Vec::new(),
            next_label: 0,
        }
    }

    /// Allocate a new label.
    pub fn new_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }

    /// Place a label at the current position.
    pub fn place_label(&mut self, label: Label) {
        self.instructions.push(BuilderEntry::Label(label));
    }

    /// Emit an instruction.
    pub fn emit(&mut self, inst: Instruction) {
        self.instructions.push(BuilderEntry::Instruction(inst));
    }

    /// Emit a no-argument instruction.
    pub fn emit_op(&mut self, op: OpCode) {
        self.emit(Instruction::no_arg(op));
    }

    /// Emit a single-word-argument instruction.
    pub fn emit_w(&mut self, op: OpCode, w0: i16) {
        self.emit(Instruction::with_w(op, w0));
    }

    /// Emit a two-word-argument instruction.
    pub fn emit_ww(&mut self, op: OpCode, w0: i16, w1: i16) {
        self.emit(Instruction::with_ww(op, w0, w1));
    }

    /// Emit a three-word-argument instruction.
    pub fn emit_www(&mut self, op: OpCode, w0: i16, w1: i16, w2: i16) {
        self.emit(Instruction::with_www(op, w0, w1, w2));
    }

    /// Emit a dword-argument instruction.
    pub fn emit_dw(&mut self, op: OpCode, dw: u32) {
        self.emit(Instruction::with_dw(op, dw));
    }

    /// Emit a word + dword-argument instruction.
    pub fn emit_w_dw(&mut self, op: OpCode, w0: i16, dw: u32) {
        self.emit(Instruction::with_w_dw(op, w0, dw));
    }

    /// Emit a qword-argument instruction.
    pub fn emit_qw(&mut self, op: OpCode, qw: u64) {
        self.emit(Instruction::with_qw(op, qw));
    }

    /// Emit a jump instruction targeting a label.
    /// The label offset will be resolved during finalization.
    pub fn emit_jump(&mut self, op: OpCode, label: Label) {
        // Store the label ID in the dword arg; we'll patch it in finalize().
        self.emit(Instruction::with_dw(op, label.0));
    }

    /// Get the current instruction count (before finalization).
    pub fn instruction_count(&self) -> usize {
        self.instructions
            .iter()
            .filter(|e| matches!(e, BuilderEntry::Instruction(_)))
            .count()
    }

    /// Finalize: resolve labels to concrete byte offsets, produce instructions.
    pub fn finalize(self) -> Vec<Instruction> {
        // First pass: compute label positions in dword offsets.
        let mut label_positions = std::collections::HashMap::new();
        let mut offset: u32 = 0;

        for entry in &self.instructions {
            match entry {
                BuilderEntry::Label(label) => {
                    label_positions.insert(*label, offset);
                }
                BuilderEntry::Instruction(inst) => {
                    offset += inst.op.size_dwords();
                }
            }
        }

        // Second pass: emit instructions, patching jump targets.
        let mut result = Vec::new();
        let mut current_offset: u32 = 0;

        for entry in self.instructions {
            match entry {
                BuilderEntry::Label(_) => { /* skip labels in output */ }
                BuilderEntry::Instruction(mut inst) => {
                    let inst_size = inst.op.size_dwords();

                    // Patch jump instructions.
                    if is_jump_op(inst.op) {
                        let label = Label(inst.dw_arg);
                        if let Some(&target_offset) = label_positions.get(&label) {
                            // Compute relative offset from the *next* instruction.
                            let next_offset = current_offset + inst_size;
                            let relative = target_offset as i32 - next_offset as i32;
                            inst.dw_arg = relative as u32;
                        }
                    }

                    current_offset += inst_size;
                    result.push(inst);
                }
            }
        }

        result
    }

    /// Finalize and encode into a `ByteCode` buffer.
    pub fn finalize_to_bytecode(self) -> ByteCode {
        let instructions = self.finalize();
        let mut bc = ByteCode::new();

        for inst in &instructions {
            encode_instruction(inst, &mut bc.data);
        }

        bc
    }
}

impl Default for BytecodeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if an opcode is a jump instruction.
fn is_jump_op(op: OpCode) -> bool {
    matches!(
        op,
        OpCode::JMP
            | OpCode::JZ
            | OpCode::JNZ
            | OpCode::JS
            | OpCode::JNS
            | OpCode::JP
            | OpCode::JNP
            | OpCode::JLowZ
            | OpCode::JLowNZ
    )
}

/// Encode a single instruction into the bytecode buffer.
fn encode_instruction(inst: &Instruction, buf: &mut Vec<u32>) {
    let size = inst.op.size_dwords();
    match size {
        1 => {
            // Pack: opcode in bits 0-7, w_arg0 in bits 8-15, w_arg1 in bits 16-31.
            // w_arg0 is truncated to 8 bits (sufficient for stack offsets < 256).
            let word = (inst.op as u32)
                | (((inst.w_arg0 as u8) as u32) << 8)
                | ((inst.w_arg1 as u16 as u32) << 16);
            buf.push(word);
        }
        2 => {
            // First word: opcode + w_arg0.
            let word = (inst.op as u32) | ((inst.w_arg0 as u16 as u32) << 16);
            buf.push(word);
            buf.push(inst.dw_arg);
        }
        3 => {
            // First word: opcode + w_arg0.
            let word = (inst.op as u32) | ((inst.w_arg0 as u16 as u32) << 16);
            buf.push(word);
            buf.push(inst.qw_arg as u32);
            buf.push((inst.qw_arg >> 32) as u32);
        }
        _ => unreachable!("invalid instruction size"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_instructions() {
        let mut builder = BytecodeBuilder::new();
        builder.emit_dw(OpCode::PshC4, 42);
        builder.emit_dw(OpCode::PshC4, 10);
        builder.emit_ww(OpCode::ADDi, 0, 1);
        builder.emit_op(OpCode::RET);

        let instructions = builder.finalize();
        assert_eq!(instructions.len(), 4);
        assert_eq!(instructions[0].op, OpCode::PshC4);
        assert_eq!(instructions[0].dw_arg, 42);
        assert_eq!(instructions[3].op, OpCode::RET);
    }

    #[test]
    fn label_jump_resolution() {
        let mut builder = BytecodeBuilder::new();
        let loop_start = builder.new_label();
        let loop_end = builder.new_label();

        builder.place_label(loop_start);
        // Instruction at offset 0: PshC4 (2 dwords)
        builder.emit_dw(OpCode::PshC4, 0);
        // Jump to loop_end at offset 2: JZ (2 dwords)
        builder.emit_jump(OpCode::JZ, loop_end);
        // Jump back to loop_start at offset 4: JMP (2 dwords)
        builder.emit_jump(OpCode::JMP, loop_start);
        builder.place_label(loop_end);
        builder.emit_op(OpCode::RET);

        let instructions = builder.finalize();
        assert_eq!(instructions.len(), 4);

        // JZ at offset 2 (size 2), targets offset 6 (loop_end).
        // Relative from next (offset 4): 6 - 4 = 2
        assert_eq!(instructions[1].dw_arg, 2u32);

        // JMP at offset 4 (size 2), targets offset 0 (loop_start).
        // Relative from next (offset 6): 0 - 6 = -6
        assert_eq!(instructions[2].dw_arg, (-6i32) as u32);
    }

    #[test]
    fn finalize_to_bytecode() {
        let mut builder = BytecodeBuilder::new();
        builder.emit_dw(OpCode::PshC4, 100);
        builder.emit_op(OpCode::RET);

        let bc = builder.finalize_to_bytecode();
        assert!(!bc.is_empty());
        // PshC4 = 2 dwords + RET = 1 dword = 3 total
        assert_eq!(bc.len(), 3);
    }

    #[test]
    fn instruction_count() {
        let mut builder = BytecodeBuilder::new();
        let label = builder.new_label();
        builder.emit_op(OpCode::RET);
        builder.place_label(label);
        builder.emit_op(OpCode::RET);

        assert_eq!(builder.instruction_count(), 2);
    }
}
