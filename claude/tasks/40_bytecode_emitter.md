# Task 40: Bytecode Emitter

## Overview

Implement the bytecode emitter that generates instructions, manages jump targets, and handles break/continue for loops.

## Goals

1. Emit individual opcodes and operands
2. Manage forward jumps with patching
3. Handle backward jumps for loops
4. Support break/continue statements
5. Track source line numbers for debugging

## Dependencies

- Task 31: Compiler Foundation (BytecodeChunk, OpCode)

## Files to Create

```
crates/angelscript-compiler/src/
├── emit/
│   ├── mod.rs             # BytecodeEmitter
│   └── jumps.rs           # Jump management
└── lib.rs
```

## Detailed Implementation

### BytecodeEmitter (emit/mod.rs)

```rust
use angelscript_core::TypeHash;

use crate::bytecode::{BytecodeChunk, Constant, ConstantPool, OpCode};

mod jumps;
use jumps::JumpManager;

/// Emits bytecode instructions.
/// Uses a shared module-level constant pool for deduplication.
pub struct BytecodeEmitter<'pool> {
    /// The bytecode chunk being built (per-function)
    chunk: BytecodeChunk,

    /// Shared module-level constant pool (deduplicated)
    constants: &'pool mut ConstantPool,

    /// Jump management for control flow
    jumps: JumpManager,

    /// Current source line for debug info
    current_line: u32,
}

impl<'pool> BytecodeEmitter<'pool> {
    pub fn new(constants: &'pool mut ConstantPool) -> Self {
        Self {
            chunk: BytecodeChunk::new(),
            constants,
            jumps: JumpManager::new(),
            current_line: 1,
        }
    }

    /// Set current source line for debug info.
    pub fn set_line(&mut self, line: u32) {
        self.current_line = line;
    }

    // ==========================================================================
    // Basic Emission
    // ==========================================================================

    /// Emit a single opcode.
    pub fn emit(&mut self, op: OpCode) {
        self.chunk.write_op(op, self.current_line);
    }

    /// Emit opcode with 8-bit operand.
    pub fn emit_byte(&mut self, op: OpCode, byte: u8) {
        self.chunk.write_op(op, self.current_line);
        self.chunk.write_byte(byte, self.current_line);
    }

    /// Emit opcode with 16-bit operand.
    pub fn emit_u16(&mut self, op: OpCode, value: u16) {
        self.chunk.write_op(op, self.current_line);
        self.chunk.write_u16(value, self.current_line);
    }

    /// Emit a constant load instruction.
    /// Constants are added to the shared module pool (deduplicated).
    pub fn emit_constant(&mut self, constant: Constant) {
        let index = self.constants.add(constant);
        if index < 256 {
            self.emit_byte(OpCode::Constant, index as u8);
        } else {
            self.emit_u16(OpCode::ConstantWide, index as u16);
        }
    }

    // ==========================================================================
    // Constants
    // ==========================================================================

    /// Emit an integer constant.
    pub fn emit_int(&mut self, value: i64) {
        match value {
            0 => self.emit(OpCode::PushZero),
            1 => self.emit(OpCode::PushOne),
            _ => self.emit_constant(Constant::Int(value)),
        }
    }

    /// Emit a float constant.
    pub fn emit_float(&mut self, value: f64) {
        self.emit_constant(Constant::Float64(value));
    }

    /// Emit a string constant.
    /// NOTE: Stores RAW string data in the constant pool. The actual string type
    /// is determined by Context::default_string_factory and the factory function
    /// is called by the compiler to produce the final string value.
    pub fn emit_string(&mut self, value: String) {
        self.emit_constant(Constant::String(value));
    }

    /// Emit null.
    pub fn emit_null(&mut self) {
        self.emit(OpCode::PushNull);
    }

    /// Emit boolean.
    pub fn emit_bool(&mut self, value: bool) {
        self.emit(if value { OpCode::PushTrue } else { OpCode::PushFalse });
    }

    // ==========================================================================
    // Local Variables
    // ==========================================================================

    /// Emit get local variable.
    pub fn emit_get_local(&mut self, slot: u32) {
        if slot < 256 {
            self.emit_byte(OpCode::GetLocal, slot as u8);
        } else {
            self.emit_u16(OpCode::GetLocalWide, slot as u16);
        }
    }

    /// Emit set local variable.
    pub fn emit_set_local(&mut self, slot: u32) {
        if slot < 256 {
            self.emit_byte(OpCode::SetLocal, slot as u8);
        } else {
            self.emit_u16(OpCode::SetLocalWide, slot as u16);
        }
    }

    // ==========================================================================
    // Function Calls
    // ==========================================================================

    /// Emit function call.
    pub fn emit_call(&mut self, func_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(func_hash));
        self.emit_u16(OpCode::Call, index as u16);
        self.chunk.write_byte(arg_count, self.current_line);
    }

    /// Emit method call.
    pub fn emit_call_method(&mut self, method_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(method_hash));
        self.emit_u16(OpCode::CallMethod, index as u16);
        self.chunk.write_byte(arg_count, self.current_line);
    }

    /// Emit virtual method call (interface dispatch).
    pub fn emit_call_virtual(&mut self, method_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(method_hash));
        self.emit_u16(OpCode::CallVirtual, index as u16);
        self.chunk.write_byte(arg_count, self.current_line);
    }

    // ==========================================================================
    // Jumps and Control Flow
    // ==========================================================================

    /// Emit a forward jump (target unknown).
    /// Returns a label that must be patched later.
    pub fn emit_jump(&mut self, op: OpCode) -> JumpLabel {
        self.emit(op);
        let offset = self.chunk.current_offset();
        self.chunk.write_u16(0xFFFF, self.current_line);  // Placeholder
        JumpLabel(offset)
    }

    /// Patch a forward jump to current position.
    pub fn patch_jump(&mut self, label: JumpLabel) {
        self.chunk.patch_jump(label.0);
    }

    /// Emit a backward jump (for loops).
    pub fn emit_loop(&mut self, target: usize) {
        self.emit(OpCode::Loop);
        let offset = self.chunk.current_offset() - target + 2;
        self.chunk.write_u16(offset as u16, self.current_line);
    }

    /// Get current bytecode offset (for loop targets).
    pub fn current_offset(&self) -> usize {
        self.chunk.current_offset()
    }

    // ==========================================================================
    // Loop Control (Break/Continue)
    // ==========================================================================

    /// Enter a loop context.
    pub fn enter_loop(&mut self, continue_target: usize) {
        self.jumps.enter_loop(continue_target);
    }

    /// Exit a loop context, patching all break jumps.
    pub fn exit_loop(&mut self) {
        let break_labels = self.jumps.exit_loop();
        for label in break_labels {
            self.patch_jump(label);
        }
    }

    /// Emit a break statement.
    pub fn emit_break(&mut self) -> Result<(), BreakError> {
        if !self.jumps.in_loop() {
            return Err(BreakError::NotInLoop);
        }
        let label = self.emit_jump(OpCode::Jump);
        self.jumps.add_break(label);
        Ok(())
    }

    /// Emit a continue statement.
    pub fn emit_continue(&mut self) -> Result<(), BreakError> {
        let target = self.jumps.continue_target()?;
        self.emit_loop(target);
        Ok(())
    }

    // ==========================================================================
    // Object Operations
    // ==========================================================================

    /// Emit object creation.
    pub fn emit_new(&mut self, type_hash: TypeHash, ctor_hash: TypeHash, arg_count: u8) {
        let type_index = self.constants.add(Constant::TypeHash(type_hash));
        let ctor_index = self.constants.add(Constant::TypeHash(ctor_hash));
        self.emit_u16(OpCode::New, type_index as u16);
        self.chunk.write_u16(ctor_index as u16, self.current_line);
        self.chunk.write_byte(arg_count, self.current_line);
    }

    /// Emit field access.
    pub fn emit_get_field(&mut self, field_index: u16) {
        self.emit_u16(OpCode::GetField, field_index);
    }

    /// Emit field assignment.
    pub fn emit_set_field(&mut self, field_index: u16) {
        self.emit_u16(OpCode::SetField, field_index);
    }

    // ==========================================================================
    // Type Operations
    // ==========================================================================

    /// Emit type conversion.
    pub fn emit_conversion(&mut self, op: OpCode) {
        self.emit(op);
    }

    /// Emit type cast.
    pub fn emit_cast(&mut self, target_type: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(target_type));
        self.emit_u16(OpCode::Cast, index as u16);
    }

    /// Emit instanceof check.
    pub fn emit_instanceof(&mut self, type_hash: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(type_hash));
        self.emit_u16(OpCode::InstanceOf, index as u16);
    }

    // ==========================================================================
    // Finalization
    // ==========================================================================

    /// Finish and return the bytecode chunk.
    pub fn finish(self) -> BytecodeChunk {
        self.chunk
    }

    /// Get current chunk size (for debugging).
    pub fn code_size(&self) -> usize {
        self.chunk.code.len()
    }
}

/// A label for a forward jump that needs patching.
#[derive(Debug, Clone, Copy)]
pub struct JumpLabel(usize);

/// Error from break/continue.
#[derive(Debug)]
pub enum BreakError {
    NotInLoop,
}

// Note: BytecodeEmitter cannot implement Default because it requires
// a &mut ConstantPool reference. Create with BytecodeEmitter::new(constants).
```

### Jump Management (emit/jumps.rs)

```rust
use super::JumpLabel;

/// Manages jump targets for control flow.
pub struct JumpManager {
    /// Stack of loop contexts
    loops: Vec<LoopContext>,
}

/// Context for a single loop.
struct LoopContext {
    /// Target for continue statements
    continue_target: usize,
    /// Pending break jumps to patch
    break_labels: Vec<JumpLabel>,
}

impl JumpManager {
    pub fn new() -> Self {
        Self { loops: Vec::new() }
    }

    /// Enter a new loop context.
    pub fn enter_loop(&mut self, continue_target: usize) {
        self.loops.push(LoopContext {
            continue_target,
            break_labels: Vec::new(),
        });
    }

    /// Exit loop context, returning break labels to patch.
    pub fn exit_loop(&mut self) -> Vec<JumpLabel> {
        self.loops.pop()
            .map(|ctx| ctx.break_labels)
            .unwrap_or_default()
    }

    /// Check if we're inside a loop.
    pub fn in_loop(&self) -> bool {
        !self.loops.is_empty()
    }

    /// Add a break label to patch later.
    pub fn add_break(&mut self, label: JumpLabel) {
        if let Some(ctx) = self.loops.last_mut() {
            ctx.break_labels.push(label);
        }
    }

    /// Get the continue target for current loop.
    pub fn continue_target(&self) -> Result<usize, super::BreakError> {
        self.loops.last()
            .map(|ctx| ctx.continue_target)
            .ok_or(super::BreakError::NotInLoop)
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_constant() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        emitter.emit_int(42);
        let chunk = emitter.finish();

        assert_eq!(chunk.code[0], OpCode::Constant as u8);
        assert_eq!(constants.get(0), Some(&Constant::Int(42)));
    }

    #[test]
    fn emit_special_ints() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        emitter.emit_int(0);
        emitter.emit_int(1);
        let chunk = emitter.finish();

        assert_eq!(chunk.code[0], OpCode::PushZero as u8);
        assert_eq!(chunk.code[1], OpCode::PushOne as u8);
    }

    #[test]
    fn constant_deduplication() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        emitter.emit_string("hello".to_string());
        emitter.emit_string("hello".to_string());  // Same string
        let _chunk = emitter.finish();

        // Only one constant stored due to deduplication
        assert_eq!(constants.len(), 1);
    }

    #[test]
    fn jump_and_patch() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let label = emitter.emit_jump(OpCode::JumpIfFalse);
        emitter.emit(OpCode::PushTrue);
        emitter.patch_jump(label);
        emitter.emit(OpCode::PushFalse);

        let chunk = emitter.finish();
        // Jump should target the PushFalse instruction
    }

    #[test]
    fn loop_break_continue() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        emitter.emit(OpCode::PushTrue);
        emitter.emit_break().unwrap();
        emitter.emit(OpCode::PushFalse);
        emitter.emit_continue().unwrap();

        emitter.exit_loop();

        let chunk = emitter.finish();
        // Break should jump past loop
        // Continue should jump to loop_start
    }

    #[test]
    fn break_outside_loop() {
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);
        let result = emitter.emit_break();
        assert!(matches!(result, Err(BreakError::NotInLoop)));
    }
}
```

## Acceptance Criteria

- [ ] Opcodes emit correctly with operands
- [ ] Constants added to pool and referenced
- [ ] Forward jumps work with patching
- [ ] Backward jumps (loops) calculate correct offset
- [ ] Break/continue work within loops
- [ ] Break/continue error when outside loop
- [ ] Nested loops handled correctly
- [ ] Line numbers tracked for debugging
- [ ] All tests pass

## Next Phase

Task 40: Expression Compilation - Basics (literals, identifiers, binary ops)
