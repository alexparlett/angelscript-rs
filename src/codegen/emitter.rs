//! Bytecode emitter for generating instructions during compilation.
//!
//! This module provides the `BytecodeEmitter` which is used during function
//! compilation to generate bytecode instructions.

use crate::codegen::ir::Instruction;

/// Bytecode emitter for generating instructions during compilation.
///
/// This structure tracks the current bytecode stream and provides methods
/// for emitting instructions.
#[derive(Debug, Clone)]
pub struct BytecodeEmitter {
    /// Generated instructions
    instructions: Vec<Instruction>,

    /// String constant table
    string_constants: Vec<String>,

    /// Next available stack offset
    next_stack_offset: u32,

    /// Stack of loop start/end positions for break/continue
    loop_stack: Vec<LoopContext>,
}

/// Context for tracking loop positions for break/continue.
#[derive(Debug, Clone)]
struct LoopContext {
    /// Position to jump to for continue (loop start)
    continue_target: usize,
    /// Positions that need to be patched with break target (loop end)
    break_positions: Vec<usize>,
}

impl BytecodeEmitter {
    /// Creates a new bytecode emitter.
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            string_constants: Vec::new(),
            next_stack_offset: 0,
            loop_stack: Vec::new(),
        }
    }

    /// Emits an instruction and returns its position.
    pub fn emit(&mut self, instruction: Instruction) -> usize {
        let pos = self.instructions.len();
        self.instructions.push(instruction);
        pos
    }

    /// Gets the current instruction position.
    ///
    /// This is useful for forward jumps where you need to know where you are.
    pub fn current_position(&self) -> usize {
        self.instructions.len()
    }

    /// Patches a jump instruction at the given position with the correct offset.
    ///
    /// # Parameters
    ///
    /// - `position`: The position of the jump instruction to patch
    /// - `target`: The target position to jump to
    ///
    /// # Panics
    ///
    /// Panics if the instruction at `position` is not a jump instruction.
    pub fn patch_jump(&mut self, position: usize, target: usize) {
        let offset = (target as i32) - (position as i32) - 1;
        match &mut self.instructions[position] {
            Instruction::Jump(off) => *off = offset,
            Instruction::JumpIfTrue(off) => *off = offset,
            Instruction::JumpIfFalse(off) => *off = offset,
            _ => panic!("Attempted to patch non-jump instruction"),
        }
    }

    /// Adds a string constant and returns its index.
    pub fn add_string_constant(&mut self, s: String) -> u32 {
        // Check if the string already exists
        if let Some(idx) = self.string_constants.iter().position(|sc| sc == &s) {
            return idx as u32;
        }

        // Add new string
        let idx = self.string_constants.len() as u32;
        self.string_constants.push(s);
        idx
    }

    /// Gets the next available stack offset.
    pub fn next_stack_offset(&self) -> u32 {
        self.next_stack_offset
    }

    /// Allocates a stack slot and returns its offset.
    pub fn allocate_stack_slot(&mut self) -> u32 {
        let offset = self.next_stack_offset;
        self.next_stack_offset += 1;
        offset
    }

    /// Enters a loop context.
    ///
    /// Should be called when starting to compile a loop.
    /// The continue_target is the position to jump to for continue statements.
    pub fn enter_loop(&mut self, continue_target: usize) {
        self.loop_stack.push(LoopContext {
            continue_target,
            break_positions: Vec::new(),
        });
    }

    /// Exits a loop context and patches all break statements.
    ///
    /// Should be called after compiling a loop.
    /// The break_target is the position to jump to for break statements.
    pub fn exit_loop(&mut self, break_target: usize) {
        if let Some(loop_ctx) = self.loop_stack.pop() {
            // Patch all break statements to jump to the break target
            for pos in loop_ctx.break_positions {
                self.patch_jump(pos, break_target);
            }
        }
    }

    /// Emits a continue instruction.
    ///
    /// Jumps to the current loop's continue target.
    ///
    /// # Returns
    ///
    /// `Some(position)` if we're in a loop, `None` otherwise.
    pub fn emit_continue(&mut self) -> Option<usize> {
        if let Some(loop_ctx) = self.loop_stack.last() {
            let current_pos = self.current_position();
            let offset = (loop_ctx.continue_target as i32) - (current_pos as i32) - 1;
            Some(self.emit(Instruction::Jump(offset)))
        } else {
            None
        }
    }

    /// Emits a break instruction (placeholder that will be patched later).
    ///
    /// # Returns
    ///
    /// `Some(position)` if we're in a loop, `None` otherwise.
    pub fn emit_break(&mut self) -> Option<usize> {
        if self.loop_stack.is_empty() {
            return None;
        }

        let pos = self.emit(Instruction::Jump(0)); // Placeholder offset
        self.loop_stack.last_mut().unwrap().break_positions.push(pos);
        Some(pos)
    }

    /// Checks if we're currently inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loop_stack.is_empty()
    }

    /// Finishes bytecode generation and returns the completed bytecode.
    pub fn finish(self) -> CompiledBytecode {
        CompiledBytecode {
            instructions: self.instructions,
            string_constants: self.string_constants,
        }
    }

    /// Gets a reference to the generated instructions (for testing/debugging).
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled bytecode ready for execution.
#[derive(Debug, Clone)]
pub struct CompiledBytecode {
    /// The bytecode instructions
    pub instructions: Vec<Instruction>,

    /// String constant table
    pub string_constants: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_emitter_is_empty() {
        let emitter = BytecodeEmitter::new();
        assert_eq!(emitter.instructions().len(), 0);
        assert_eq!(emitter.next_stack_offset(), 0);
    }

    #[test]
    fn emit_instruction_adds_to_list() {
        let mut emitter = BytecodeEmitter::new();
        emitter.emit(Instruction::PushInt(42));
        emitter.emit(Instruction::PushInt(10));

        assert_eq!(emitter.instructions().len(), 2);
        assert_eq!(emitter.instructions()[0], Instruction::PushInt(42));
        assert_eq!(emitter.instructions()[1], Instruction::PushInt(10));
    }

    #[test]
    fn emit_returns_position() {
        let mut emitter = BytecodeEmitter::new();
        let pos1 = emitter.emit(Instruction::Nop);
        let pos2 = emitter.emit(Instruction::Nop);
        let pos3 = emitter.emit(Instruction::Nop);

        assert_eq!(pos1, 0);
        assert_eq!(pos2, 1);
        assert_eq!(pos3, 2);
    }

    #[test]
    fn current_position_tracks_correctly() {
        let mut emitter = BytecodeEmitter::new();
        assert_eq!(emitter.current_position(), 0);

        emitter.emit(Instruction::Nop);
        assert_eq!(emitter.current_position(), 1);

        emitter.emit(Instruction::Nop);
        assert_eq!(emitter.current_position(), 2);
    }

    #[test]
    fn patch_jump_updates_offset() {
        let mut emitter = BytecodeEmitter::new();
        let jump_pos = emitter.emit(Instruction::Jump(0)); // Placeholder
        emitter.emit(Instruction::Nop);
        emitter.emit(Instruction::Nop);
        let target = emitter.current_position();

        emitter.patch_jump(jump_pos, target);

        // Jump from position 0 to position 3: offset = 3 - 0 - 1 = 2
        assert_eq!(emitter.instructions()[jump_pos], Instruction::Jump(2));
    }

    #[test]
    fn patch_jump_if_false_works() {
        let mut emitter = BytecodeEmitter::new();
        let jump_pos = emitter.emit(Instruction::JumpIfFalse(0));
        emitter.emit(Instruction::Nop);
        let target = emitter.current_position();

        emitter.patch_jump(jump_pos, target);

        // Jump from position 0 to position 2: offset = 2 - 0 - 1 = 1
        assert_eq!(
            emitter.instructions()[jump_pos],
            Instruction::JumpIfFalse(1)
        );
    }

    #[test]
    fn add_string_constant_deduplicates() {
        let mut emitter = BytecodeEmitter::new();

        let idx1 = emitter.add_string_constant("hello".to_string());
        let idx2 = emitter.add_string_constant("world".to_string());
        let idx3 = emitter.add_string_constant("hello".to_string()); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first "hello"
        assert_eq!(emitter.string_constants.len(), 2);
    }

    #[test]
    fn allocate_stack_slot_increments() {
        let mut emitter = BytecodeEmitter::new();

        let slot1 = emitter.allocate_stack_slot();
        let slot2 = emitter.allocate_stack_slot();
        let slot3 = emitter.allocate_stack_slot();

        assert_eq!(slot1, 0);
        assert_eq!(slot2, 1);
        assert_eq!(slot3, 2);
        assert_eq!(emitter.next_stack_offset(), 3);
    }

    #[test]
    fn loop_context_tracking() {
        let mut emitter = BytecodeEmitter::new();
        assert!(!emitter.in_loop());

        let loop_start = emitter.current_position();
        emitter.enter_loop(loop_start);
        assert!(emitter.in_loop());

        emitter.exit_loop(10);
        assert!(!emitter.in_loop());
    }

    #[test]
    fn emit_continue_in_loop() {
        let mut emitter = BytecodeEmitter::new();
        let loop_start = emitter.current_position();
        emitter.enter_loop(loop_start);

        emitter.emit(Instruction::Nop);
        let continue_pos = emitter.emit_continue();

        assert!(continue_pos.is_some());
        // Continue from position 1 to position 0: offset = 0 - 1 - 1 = -2
        assert_eq!(emitter.instructions()[1], Instruction::Jump(-2));

        emitter.exit_loop(10);
    }

    #[test]
    fn emit_continue_outside_loop() {
        let mut emitter = BytecodeEmitter::new();
        let result = emitter.emit_continue();
        assert!(result.is_none());
    }

    #[test]
    fn emit_break_in_loop() {
        let mut emitter = BytecodeEmitter::new();
        let loop_start = emitter.current_position();
        emitter.enter_loop(loop_start);

        let break_pos = emitter.emit_break();
        assert!(break_pos.is_some());

        emitter.emit(Instruction::Nop);
        let loop_end = emitter.current_position();
        emitter.exit_loop(loop_end);

        // Break should now jump to position 2
        // From position 0 to position 2: offset = 2 - 0 - 1 = 1
        assert_eq!(emitter.instructions()[0], Instruction::Jump(1));
    }

    #[test]
    fn emit_break_outside_loop() {
        let mut emitter = BytecodeEmitter::new();
        let result = emitter.emit_break();
        assert!(result.is_none());
    }

    #[test]
    fn multiple_breaks_in_loop() {
        let mut emitter = BytecodeEmitter::new();
        let loop_start = emitter.current_position();
        emitter.enter_loop(loop_start);

        let break1 = emitter.emit_break().unwrap();
        emitter.emit(Instruction::Nop);
        let break2 = emitter.emit_break().unwrap();
        emitter.emit(Instruction::Nop);

        let loop_end = emitter.current_position();
        emitter.exit_loop(loop_end);

        // Both breaks should jump to position 4 (loop_end)
        // break1 at pos 0: offset = 4 - 0 - 1 = 3
        // break2 at pos 2: offset = 4 - 2 - 1 = 1
        assert_eq!(emitter.instructions()[break1], Instruction::Jump(3));
        assert_eq!(emitter.instructions()[break2], Instruction::Jump(1));
    }

    #[test]
    fn nested_loops() {
        let mut emitter = BytecodeEmitter::new();

        // Outer loop
        let outer_start = emitter.current_position();
        emitter.enter_loop(outer_start);

        // Inner loop
        let inner_start = emitter.current_position();
        emitter.enter_loop(inner_start);

        let inner_break = emitter.emit_break().unwrap();
        let inner_end = emitter.current_position();
        emitter.exit_loop(inner_end);

        let outer_break = emitter.emit_break().unwrap();
        let outer_end = emitter.current_position();
        emitter.exit_loop(outer_end);

        // Inner break jumps to position 1, outer break jumps to position 2
        assert_eq!(emitter.instructions()[inner_break], Instruction::Jump(0));
        assert_eq!(emitter.instructions()[outer_break], Instruction::Jump(0));
    }

    #[test]
    fn finish_returns_bytecode() {
        let mut emitter = BytecodeEmitter::new();
        emitter.emit(Instruction::PushInt(42));
        emitter.emit(Instruction::Return);
        emitter.add_string_constant("test".to_string());

        let bytecode = emitter.finish();
        assert_eq!(bytecode.instructions.len(), 2);
        assert_eq!(bytecode.string_constants.len(), 1);
        assert_eq!(bytecode.string_constants[0], "test");
    }
}
