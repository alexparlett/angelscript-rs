//! Bytecode emitter for the AngelScript compiler.
//!
//! The [`BytecodeEmitter`] provides a high-level API for generating bytecode,
//! handling constants, jumps, and loop control flow.
//!
//! The emitter owns all compilation output:
//! - `ConstantPool` - shared across all functions for deduplication
//! - `compiled_functions` - completed function bytecode
//! - `global_inits` - global variable initializer bytecode
//!
//! Use `start_chunk()` / `finish_chunk()` for each function, with nested calls
//! for lambdas (chunks are stacked).
//!
//! # Example
//!
//! ```ignore
//! use angelscript_compiler::emit::BytecodeEmitter;
//!
//! let mut emitter = BytecodeEmitter::new();
//!
//! emitter.start_chunk();
//! emitter.set_line(1);
//! emitter.emit_int(42);
//! emitter.emit_int(10);
//! emitter.emit(OpCode::AddI64);
//! emitter.finish_function(func_hash, "myFunc".to_string());
//!
//! let (constants, functions, inits) = emitter.decompose();
//! ```

mod jumps;

use angelscript_core::TypeHash;

use crate::bytecode::{BytecodeChunk, Constant, ConstantPool, OpCode};
use jumps::JumpManager;

// ============================================================================
// Compilation Output Types
// ============================================================================

/// A compiled function with its bytecode.
#[derive(Debug)]
pub struct CompiledFunctionEntry {
    /// Function hash (for linking).
    pub hash: TypeHash,
    /// Function name (for debugging).
    pub name: String,
    /// Compiled bytecode.
    pub bytecode: BytecodeChunk,
}

/// A global variable initializer.
#[derive(Debug)]
pub struct GlobalInitEntry {
    /// Global variable hash.
    pub hash: TypeHash,
    /// Variable name.
    pub name: String,
    /// Initializer bytecode (evaluates expression and stores to global).
    pub bytecode: BytecodeChunk,
}

// ============================================================================
// BytecodeEmitter
// ============================================================================

/// Emits bytecode instructions and owns all compilation output.
///
/// The emitter maintains:
/// - A shared constant pool for all functions (deduplication)
/// - A stack of bytecode chunks (for nested lambda compilation)
/// - Completed functions and global initializers
///
/// Use `start_chunk()` before compiling each function and `finish_function()`
/// or `finish_chunk()` when done. For lambdas, chunks are stacked - the parent
/// function's chunk is preserved while the lambda is compiled.
pub struct BytecodeEmitter {
    /// Owned constant pool (shared across all functions)
    constants: ConstantPool,

    /// Stack of bytecode chunks - top is current, others are paused (for lambdas)
    chunk_stack: Vec<BytecodeChunk>,

    /// Completed functions with their bytecode
    compiled_functions: Vec<CompiledFunctionEntry>,

    /// Global variable initializers
    global_inits: Vec<GlobalInitEntry>,

    /// Jump management for control flow
    jumps: JumpManager,

    /// Current source line for debug info
    current_line: u32,
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeEmitter {
    /// Create a new bytecode emitter.
    pub fn new() -> Self {
        Self {
            constants: ConstantPool::new(),
            chunk_stack: Vec::new(),
            compiled_functions: Vec::new(),
            global_inits: Vec::new(),
            jumps: JumpManager::new(),
            current_line: 1,
        }
    }

    // ========================================================================
    // Chunk Lifecycle
    // ========================================================================

    /// Start a new function's bytecode chunk.
    ///
    /// Call this before compiling a function body. For nested lambdas,
    /// this pushes a new chunk onto the stack (parent chunk is preserved).
    pub fn start_chunk(&mut self) {
        self.chunk_stack.push(BytecodeChunk::new());
        self.jumps = JumpManager::new();
    }

    /// Finish the current chunk and return it.
    ///
    /// Use this for custom handling (e.g., lambdas that need special processing).
    /// For normal functions, prefer `finish_function()`.
    pub fn finish_chunk(&mut self) -> BytecodeChunk {
        self.chunk_stack
            .pop()
            .expect("No chunk to finish - call start_chunk() first")
    }

    /// Finish the current chunk and register it as a compiled function.
    pub fn finish_function(&mut self, hash: TypeHash, name: String) {
        let bytecode = self.finish_chunk();
        self.compiled_functions.push(CompiledFunctionEntry {
            hash,
            name,
            bytecode,
        });
    }

    /// Finish the current chunk and register it as a global initializer.
    pub fn finish_global_init(&mut self, hash: TypeHash, name: String) {
        let bytecode = self.finish_chunk();
        self.global_inits.push(GlobalInitEntry {
            hash,
            name,
            bytecode,
        });
    }

    /// Get the current bytecode chunk (for emit operations).
    fn current_chunk(&mut self) -> &mut BytecodeChunk {
        self.chunk_stack
            .last_mut()
            .expect("No active chunk - call start_chunk() first")
    }

    /// Get an immutable reference to the current chunk.
    fn current_chunk_ref(&self) -> &BytecodeChunk {
        self.chunk_stack
            .last()
            .expect("No active chunk - call start_chunk() first")
    }

    /// Consume the emitter and return all compilation output.
    ///
    /// Returns (constants, compiled_functions, global_inits).
    pub fn decompose(
        self,
    ) -> (
        ConstantPool,
        Vec<CompiledFunctionEntry>,
        Vec<GlobalInitEntry>,
    ) {
        (self.constants, self.compiled_functions, self.global_inits)
    }

    /// Set current source line for debug info.
    ///
    /// All subsequent instructions will be associated with this line number.
    pub fn set_line(&mut self, line: u32) {
        self.current_line = line;
    }

    /// Get current source line.
    pub fn current_line(&self) -> u32 {
        self.current_line
    }

    // ==========================================================================
    // Basic Emission
    // ==========================================================================

    /// Emit a single opcode with no operands.
    pub fn emit(&mut self, op: OpCode) {
        let line = self.current_line;
        self.current_chunk().write_op(op, line);
    }

    /// Emit opcode with 8-bit operand.
    pub fn emit_byte(&mut self, op: OpCode, byte: u8) {
        let line = self.current_line;
        let chunk = self.current_chunk();
        chunk.write_op(op, line);
        chunk.write_byte(byte, line);
    }

    /// Emit opcode with 16-bit operand.
    pub fn emit_u16(&mut self, op: OpCode, value: u16) {
        let line = self.current_line;
        let chunk = self.current_chunk();
        chunk.write_op(op, line);
        chunk.write_u16(value, line);
    }

    /// Emit a constant load instruction.
    ///
    /// Constants are added to the shared module pool (deduplicated).
    /// Uses narrow (8-bit) or wide (16-bit) index based on pool size.
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
    ///
    /// Optimizes common cases: 0 uses `PushZero`, 1 uses `PushOne`.
    pub fn emit_int(&mut self, value: i64) {
        match value {
            0 => self.emit(OpCode::PushZero),
            1 => self.emit(OpCode::PushOne),
            _ => self.emit_constant(Constant::Int(value)),
        }
    }

    /// Emit an unsigned integer constant.
    pub fn emit_uint(&mut self, value: u64) {
        match value {
            0 => self.emit(OpCode::PushZero),
            1 => self.emit(OpCode::PushOne),
            _ => self.emit_constant(Constant::Uint(value)),
        }
    }

    /// Emit a 32-bit float constant.
    pub fn emit_f32(&mut self, value: f32) {
        self.emit_constant(Constant::Float32(value));
    }

    /// Emit a 64-bit float constant.
    pub fn emit_f64(&mut self, value: f64) {
        self.emit_constant(Constant::Float64(value));
    }

    /// Emit a string constant.
    ///
    /// NOTE: Stores raw string data in the constant pool. The actual string type
    /// is determined by `Context::string_factory()` and the factory function
    /// is called at runtime to produce the final string value.
    pub fn emit_string(&mut self, value: &str) {
        self.emit_constant(Constant::StringData(value.as_bytes().to_vec()));
    }

    /// Emit a string constant from raw bytes.
    pub fn emit_string_bytes(&mut self, bytes: Vec<u8>) {
        self.emit_constant(Constant::StringData(bytes));
    }

    /// Emit null.
    pub fn emit_null(&mut self) {
        self.emit(OpCode::PushNull);
    }

    /// Emit boolean.
    pub fn emit_bool(&mut self, value: bool) {
        self.emit(if value {
            OpCode::PushTrue
        } else {
            OpCode::PushFalse
        });
    }

    // ==========================================================================
    // Local Variables
    // ==========================================================================

    /// Emit get local variable.
    ///
    /// Uses narrow (8-bit) or wide (16-bit) slot index based on slot number.
    pub fn emit_get_local(&mut self, slot: u32) {
        if slot < 256 {
            self.emit_byte(OpCode::GetLocal, slot as u8);
        } else {
            self.emit_u16(OpCode::GetLocalWide, slot as u16);
        }
    }

    /// Emit set local variable.
    ///
    /// Uses narrow (8-bit) or wide (16-bit) slot index based on slot number.
    pub fn emit_set_local(&mut self, slot: u32) {
        if slot < 256 {
            self.emit_byte(OpCode::SetLocal, slot as u8);
        } else {
            self.emit_u16(OpCode::SetLocalWide, slot as u16);
        }
    }

    // ==========================================================================
    // Global Variables
    // ==========================================================================

    /// Emit get global variable by hash.
    pub fn emit_get_global(&mut self, global_hash: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(global_hash));
        if index < 256 {
            self.emit_byte(OpCode::GetGlobal, index as u8);
        } else {
            self.emit_u16(OpCode::GetGlobal, index as u16);
        }
    }

    /// Emit set global variable by hash.
    pub fn emit_set_global(&mut self, global_hash: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(global_hash));
        if index < 256 {
            self.emit_byte(OpCode::SetGlobal, index as u8);
        } else {
            self.emit_u16(OpCode::SetGlobal, index as u16);
        }
    }

    // ==========================================================================
    // Function Calls
    // ==========================================================================

    /// Emit function call.
    ///
    /// # Arguments
    /// * `func_hash` - Hash of the function to call
    /// * `arg_count` - Number of arguments on the stack
    pub fn emit_call(&mut self, func_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(func_hash));
        self.emit_u16(OpCode::Call, index as u16);
        let line = self.current_line;
        self.current_chunk().write_byte(arg_count, line);
    }

    /// Emit method call on an object.
    ///
    /// # Arguments
    /// * `method_hash` - Hash of the method to call
    /// * `arg_count` - Number of arguments on the stack (excluding `this`)
    pub fn emit_call_method(&mut self, method_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(method_hash));
        self.emit_u16(OpCode::CallMethod, index as u16);
        let line = self.current_line;
        self.current_chunk().write_byte(arg_count, line);
    }

    /// Emit virtual method call using vtable slot (for polymorphic class dispatch).
    ///
    /// # Arguments
    /// * `vtable_slot` - The vtable slot index for the method
    /// * `arg_count` - Number of arguments on the stack (excluding `this`)
    pub fn emit_call_virtual(&mut self, vtable_slot: u16, arg_count: u8) {
        self.emit_u16(OpCode::CallVirtual, vtable_slot);
        let line = self.current_line;
        self.current_chunk().write_byte(arg_count, line);
    }

    /// Emit interface method call using itable (for polymorphic interface dispatch).
    ///
    /// # Arguments
    /// * `interface_hash` - Hash of the interface type
    /// * `slot` - The itable slot index for the method
    /// * `arg_count` - Number of arguments on the stack (excluding `this`)
    pub fn emit_call_interface(&mut self, interface_hash: TypeHash, slot: u16, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(interface_hash));
        self.emit_u16(OpCode::CallInterface, index as u16);
        let line = self.current_line;
        self.current_chunk().write_u16(slot, line);
        self.current_chunk().write_byte(arg_count, line);
    }

    /// Emit return with value.
    pub fn emit_return(&mut self) {
        self.emit(OpCode::Return);
    }

    /// Emit return from void function.
    pub fn emit_return_void(&mut self) {
        self.emit(OpCode::ReturnVoid);
    }

    // ==========================================================================
    // Jumps and Control Flow
    // ==========================================================================

    /// Emit a forward jump (target unknown).
    ///
    /// Returns a label that must be patched later with [`patch_jump`].
    pub fn emit_jump(&mut self, op: OpCode) -> JumpLabel {
        self.emit(op);
        let offset = self.current_chunk_ref().current_offset();
        let line = self.current_line;
        self.current_chunk().write_u16(0xFFFF, line); // Placeholder
        JumpLabel(offset)
    }

    /// Patch a forward jump to the current position.
    pub fn patch_jump(&mut self, label: JumpLabel) {
        self.current_chunk().patch_jump(label.0);
    }

    /// Emit a backward jump (for loops).
    ///
    /// # Arguments
    /// * `target` - The bytecode offset to jump back to
    pub fn emit_loop(&mut self, target: usize) {
        let line = self.current_line;
        self.current_chunk().emit_loop(target, line);
    }

    /// Get current bytecode offset.
    ///
    /// Used to mark loop targets before emitting loop body.
    pub fn current_offset(&self) -> usize {
        self.current_chunk_ref().current_offset()
    }

    // ==========================================================================
    // Loop Control (Break/Continue)
    // ==========================================================================

    /// Enter a loop context.
    ///
    /// Call this at the start of a loop, before emitting the loop body.
    ///
    /// # Arguments
    /// * `continue_target` - The bytecode offset for continue statements
    pub fn enter_loop(&mut self, continue_target: usize) {
        self.jumps.enter_loop(continue_target);
    }

    /// Enter a loop context with deferred continue target.
    ///
    /// Use this for do-while loops where the continue target (condition)
    /// isn't known until after the body is compiled. Call `set_continue_target`
    /// later to set the target.
    pub fn enter_loop_deferred(&mut self) {
        self.jumps.enter_loop_deferred();
    }

    /// Exit a loop context.
    ///
    /// Patches all break jumps to the current position.
    /// Call this after the loop body and any backward jump.
    pub fn exit_loop(&mut self) {
        let break_labels = self.jumps.exit_loop();
        for label in break_labels {
            self.patch_jump(label);
        }
    }

    /// Emit a break statement.
    ///
    /// Returns an error if not inside a breakable context (loop or switch).
    pub fn emit_break(&mut self) -> Result<(), BreakError> {
        if !self.jumps.in_breakable() {
            return Err(BreakError::NotInBreakable);
        }
        let label = self.emit_jump(OpCode::Jump);
        self.jumps.add_break(label);
        Ok(())
    }

    /// Emit a continue statement.
    ///
    /// Returns an error if not inside a loop.
    /// For do-while loops with deferred continue target, emits a forward jump
    /// that will be patched later.
    pub fn emit_continue(&mut self) -> Result<(), BreakError> {
        match self.jumps.continue_target()? {
            Some(target) => {
                // Continue target is known - emit backward jump
                self.emit_loop(target);
            }
            None => {
                // Continue target is deferred (do-while) - emit forward jump
                let label = self.emit_jump(OpCode::Jump);
                self.jumps.add_continue(label);
            }
        }
        Ok(())
    }

    /// Check if currently inside a loop.
    pub fn in_loop(&self) -> bool {
        self.jumps.in_loop()
    }

    /// Get current loop nesting depth.
    pub fn loop_depth(&self) -> usize {
        self.jumps.loop_depth()
    }

    // ==========================================================================
    // Switch Control
    // ==========================================================================

    /// Enter a switch context.
    ///
    /// Call this at the start of a switch statement.
    /// Switch statements support break but not continue.
    pub fn enter_switch(&mut self) {
        self.jumps.enter_switch();
    }

    /// Exit a switch context.
    ///
    /// Patches all break jumps to the current position.
    /// Call this after the switch body.
    pub fn exit_switch(&mut self) {
        let break_labels = self.jumps.exit_switch();
        for label in break_labels {
            self.patch_jump(label);
        }
    }

    /// Check if currently inside a switch.
    pub fn in_switch(&self) -> bool {
        self.jumps.in_switch()
    }

    /// Check if currently inside any breakable context (loop or switch).
    pub fn in_breakable(&self) -> bool {
        self.jumps.in_breakable()
    }

    /// Update the continue target for the innermost loop.
    ///
    /// This is useful for `for` loops where the continue target is
    /// the update expression, not the condition.
    ///
    /// For do-while loops, this also patches any deferred continue jumps.
    pub fn set_continue_target(&mut self, target: usize) {
        let pending_labels = self.jumps.set_continue_target(target);
        for label in pending_labels {
            self.patch_jump(label);
        }
    }

    /// Get current breakable nesting depth (loops and switches).
    pub fn breakable_depth(&self) -> usize {
        self.jumps.breakable_depth()
    }

    // ==========================================================================
    // Object Operations
    // ==========================================================================

    /// Emit object creation with constructor call.
    ///
    /// # Arguments
    /// * `type_hash` - Hash of the type to instantiate
    /// * `ctor_hash` - Hash of the constructor to call
    /// * `arg_count` - Number of constructor arguments
    pub fn emit_new(&mut self, type_hash: TypeHash, ctor_hash: TypeHash, arg_count: u8) {
        let type_index = self.constants.add(Constant::TypeHash(type_hash));
        let ctor_index = self.constants.add(Constant::TypeHash(ctor_hash));
        self.emit_u16(OpCode::New, type_index as u16);
        let line = self.current_line;
        let chunk = self.current_chunk();
        chunk.write_u16(ctor_index as u16, line);
        chunk.write_byte(arg_count, line);
    }

    /// Emit factory function call for object creation.
    ///
    /// # Arguments
    /// * `factory_hash` - Hash of the factory function
    /// * `arg_count` - Number of factory arguments
    pub fn emit_new_factory(&mut self, factory_hash: TypeHash, arg_count: u8) {
        let index = self.constants.add(Constant::TypeHash(factory_hash));
        self.emit_u16(OpCode::NewFactory, index as u16);
        let line = self.current_line;
        self.current_chunk().write_byte(arg_count, line);
    }

    /// Emit field access.
    pub fn emit_get_field(&mut self, field_index: u16) {
        self.emit_u16(OpCode::GetField, field_index);
    }

    /// Emit field assignment.
    pub fn emit_set_field(&mut self, field_index: u16) {
        self.emit_u16(OpCode::SetField, field_index);
    }

    /// Emit `this` reference.
    pub fn emit_get_this(&mut self) {
        self.emit(OpCode::GetThis);
    }

    // ==========================================================================
    // Type Operations
    // ==========================================================================

    /// Emit type conversion opcode.
    ///
    /// Use this for built-in conversions like `I32toI64`, `F32toF64`, etc.
    pub fn emit_conversion(&mut self, op: OpCode) {
        self.emit(op);
    }

    /// Emit explicit type cast.
    ///
    /// May fail at runtime if the cast is not valid.
    pub fn emit_cast(&mut self, target_type: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(target_type));
        self.emit_u16(OpCode::Cast, index as u16);
    }

    /// Emit instanceof check.
    ///
    /// Pushes a boolean result indicating if the value is an instance of the type.
    pub fn emit_instanceof(&mut self, type_hash: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(type_hash));
        self.emit_u16(OpCode::InstanceOf, index as u16);
    }

    // ==========================================================================
    // Stack Operations
    // ==========================================================================

    /// Emit pop (discard top of stack).
    pub fn emit_pop(&mut self) {
        self.emit(OpCode::Pop);
    }

    /// Emit pop N values.
    pub fn emit_pop_n(&mut self, count: u8) {
        self.emit_byte(OpCode::PopN, count);
    }

    /// Emit duplicate top of stack.
    pub fn emit_dup(&mut self) {
        self.emit(OpCode::Dup);
    }

    /// Emit pick (copy value at offset from top to top of stack).
    ///
    /// Offset 0 = duplicate top (like Dup), offset 1 = copy second from top, etc.
    pub fn emit_pick(&mut self, offset: u8) {
        self.emit_byte(OpCode::Pick, offset);
    }

    // ==========================================================================
    // Reference Counting
    // ==========================================================================

    /// Emit add reference count.
    ///
    /// The VM will inspect the value on the stack and call the appropriate
    /// addref behavior based on the runtime type of the object.
    pub fn emit_add_ref(&mut self) {
        let line = self.current_line;
        let chunk = self.current_chunk();
        chunk.write_op(OpCode::AddRef, line);
    }

    /// Emit release reference count.
    ///
    /// The VM will inspect the value on the stack and call the appropriate
    /// release behavior based on the runtime type of the object.
    pub fn emit_release(&mut self) {
        let line = self.current_line;
        let chunk = self.current_chunk();
        chunk.write_op(OpCode::Release, line);
    }

    // ==========================================================================
    // Function Pointers
    // ==========================================================================

    /// Emit function pointer creation.
    pub fn emit_func_ptr(&mut self, func_hash: TypeHash) {
        let index = self.constants.add(Constant::TypeHash(func_hash));
        self.emit_u16(OpCode::FuncPtr, index as u16);
    }

    /// Emit call through function pointer.
    pub fn emit_call_func_ptr(&mut self, arg_count: u8) {
        self.emit_byte(OpCode::CallFuncPtr, arg_count);
    }

    // ==========================================================================
    // Init Lists
    // ==========================================================================

    /// Emit begin init list.
    pub fn emit_init_list_begin(&mut self, size: u16) {
        self.emit_u16(OpCode::InitListBegin, size);
    }

    /// Emit end init list.
    pub fn emit_init_list_end(&mut self) {
        self.emit(OpCode::InitListEnd);
    }

    // ==========================================================================
    // Inspection
    // ==========================================================================

    /// Get current chunk size (for debugging).
    pub fn code_size(&self) -> usize {
        self.current_chunk_ref().len()
    }

    /// Get a reference to the current bytecode chunk.
    ///
    /// Useful for inspection during compilation, e.g., return path checking.
    pub fn chunk(&self) -> &BytecodeChunk {
        self.current_chunk_ref()
    }

    // ==========================================================================
    // Accessors
    // ==========================================================================

    /// Get a reference to the compiled functions.
    pub fn compiled_functions(&self) -> &[CompiledFunctionEntry] {
        &self.compiled_functions
    }

    /// Get a reference to the constant pool.
    pub fn constants(&self) -> &ConstantPool {
        &self.constants
    }

    /// Get a reference to the global initializers.
    pub fn global_inits(&self) -> &[GlobalInitEntry] {
        &self.global_inits
    }
}

/// A label for a forward jump that needs patching.
#[derive(Debug, Clone, Copy)]
pub struct JumpLabel(pub(crate) usize);

impl JumpLabel {
    /// Get the bytecode offset this label points to.
    pub fn offset(&self) -> usize {
        self.0
    }
}

/// Error from break/continue statements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakError {
    /// Continue used outside of a loop (switches don't support continue).
    NotInLoop,
    /// Break used outside of a breakable context (loop or switch).
    NotInBreakable,
}

impl std::fmt::Display for BreakError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakError::NotInLoop => write!(f, "continue statement not inside a loop"),
            BreakError::NotInBreakable => {
                write!(f, "break statement not inside a loop or switch")
            }
        }
    }
}

impl std::error::Error for BreakError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_constant() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_int(42);
        let chunk = emitter.finish_chunk();

        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(0)); // Index 0

        let (constants, _, _) = emitter.decompose();
        assert_eq!(constants.get(0), Some(&Constant::Int(42)));
    }

    #[test]
    fn emit_special_ints() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_int(0);
        emitter.emit_int(1);
        let chunk = emitter.finish_chunk();

        assert_eq!(chunk.read_op(0), Some(OpCode::PushZero));
        assert_eq!(chunk.read_op(1), Some(OpCode::PushOne));

        let (constants, _, _) = emitter.decompose();
        assert!(constants.is_empty()); // No constants added
    }

    #[test]
    fn emit_special_uints() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_uint(0);
        emitter.emit_uint(1);
        let chunk = emitter.finish_chunk();

        assert_eq!(chunk.read_op(0), Some(OpCode::PushZero));
        assert_eq!(chunk.read_op(1), Some(OpCode::PushOne));
    }

    #[test]
    fn constant_deduplication() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_string("hello");
        emitter.emit_string("hello"); // Same string
        emitter.finish_chunk();

        let (constants, _, _) = emitter.decompose();
        // Only one constant stored due to deduplication
        assert_eq!(constants.len(), 1);
    }

    #[test]
    fn jump_and_patch() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let label = emitter.emit_jump(OpCode::JumpIfFalse);
        emitter.emit(OpCode::PushTrue);
        emitter.patch_jump(label);
        emitter.emit(OpCode::PushFalse);

        let chunk = emitter.finish_chunk();

        // JumpIfFalse (1) + offset (2) + PushTrue (1) = 4 bytes before PushFalse
        assert_eq!(chunk.read_op(0), Some(OpCode::JumpIfFalse));
        // Jump offset should be 1 (skip PushTrue)
        assert_eq!(chunk.read_u16(1), Some(1));
        assert_eq!(chunk.read_op(3), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(4), Some(OpCode::PushFalse));
    }

    #[test]
    fn loop_break_continue() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        emitter.emit(OpCode::PushTrue);
        emitter.emit_break().unwrap();
        emitter.emit(OpCode::PushFalse);
        emitter.emit_continue().unwrap();

        emitter.exit_loop();

        let chunk = emitter.finish_chunk();

        // Verify break jumps past the loop
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::Jump)); // break
        // After exit_loop, break should be patched to end
    }

    #[test]
    fn break_outside_breakable() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let result = emitter.emit_break();
        assert!(matches!(result, Err(BreakError::NotInBreakable)));
    }

    #[test]
    fn continue_outside_loop() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let result = emitter.emit_continue();
        assert!(matches!(result, Err(BreakError::NotInLoop)));
    }

    #[test]
    fn nested_loops() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let outer_start = emitter.current_offset();
        emitter.enter_loop(outer_start);
        assert_eq!(emitter.loop_depth(), 1);

        let inner_start = emitter.current_offset();
        emitter.enter_loop(inner_start);
        assert_eq!(emitter.loop_depth(), 2);

        emitter.emit_break().unwrap(); // Breaks inner loop

        emitter.exit_loop(); // Exit inner
        assert_eq!(emitter.loop_depth(), 1);

        emitter.exit_loop(); // Exit outer
        assert_eq!(emitter.loop_depth(), 0);
    }

    #[test]
    fn emit_function_call() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let func_hash = TypeHash::from_name("myFunc");
        emitter.emit_call(func_hash, 3);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::Call));
        assert_eq!(chunk.read_byte(3), Some(3)); // arg count
    }

    #[test]
    fn emit_new_object() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let type_hash = TypeHash::from_name("MyClass");
        let ctor_hash = TypeHash::from_name("MyClass::MyClass");
        emitter.emit_new(type_hash, ctor_hash, 2);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::New));
    }

    #[test]
    fn emit_locals() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_get_local(5);
        emitter.emit_set_local(10);
        emitter.emit_get_local(300); // Wide
        emitter.emit_set_local(500); // Wide

        let chunk = emitter.finish_chunk();

        assert_eq!(chunk.read_op(0), Some(OpCode::GetLocal));
        assert_eq!(chunk.read_byte(1), Some(5));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(3), Some(10));
        assert_eq!(chunk.read_op(4), Some(OpCode::GetLocalWide));
        assert_eq!(chunk.read_u16(5), Some(300));
        assert_eq!(chunk.read_op(7), Some(OpCode::SetLocalWide));
        assert_eq!(chunk.read_u16(8), Some(500));
    }

    #[test]
    fn emit_booleans() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_bool(true);
        emitter.emit_bool(false);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::PushFalse));
    }

    #[test]
    fn emit_null() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_null();

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushNull));
    }

    #[test]
    fn emit_floats() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_f32(1.5);
        emitter.emit_f64(2.5);
        emitter.finish_chunk();

        let (constants, _, _) = emitter.decompose();
        assert_eq!(constants.len(), 2);
        assert!(matches!(constants.get(0), Some(Constant::Float32(v)) if (*v - 1.5).abs() < 0.001));
        assert!(matches!(constants.get(1), Some(Constant::Float64(v)) if (*v - 2.5).abs() < 0.001));
    }

    #[test]
    fn emit_string() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_string("hello");
        emitter.finish_chunk();

        let (constants, _, _) = emitter.decompose();
        assert_eq!(constants.len(), 1);
        assert_eq!(
            constants.get(0),
            Some(&Constant::StringData(b"hello".to_vec()))
        );
    }

    #[test]
    fn line_tracking() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.set_line(10);
        emitter.emit(OpCode::PushTrue);
        emitter.set_line(20);
        emitter.emit(OpCode::PushFalse);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.line_at(0), Some(10));
        assert_eq!(chunk.line_at(1), Some(20));
    }

    #[test]
    fn emit_stack_ops() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_pop();
        emitter.emit_pop_n(5);
        emitter.emit_dup();

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(1), Some(OpCode::PopN));
        assert_eq!(chunk.read_byte(2), Some(5));
        assert_eq!(chunk.read_op(3), Some(OpCode::Dup));
    }

    #[test]
    fn emit_type_operations() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let type_hash = TypeHash::from_name("MyClass");
        emitter.emit_conversion(OpCode::I32toI64);
        emitter.emit_cast(type_hash);
        emitter.emit_instanceof(type_hash);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::I32toI64));
        assert_eq!(chunk.read_op(1), Some(OpCode::Cast));
        // Cast uses same constant as instanceof due to deduplication
    }

    #[test]
    fn emit_field_access() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_get_field(5);
        emitter.emit_set_field(10);
        emitter.emit_get_this();

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::GetField));
        assert_eq!(chunk.read_u16(1), Some(5));
        assert_eq!(chunk.read_op(3), Some(OpCode::SetField));
        assert_eq!(chunk.read_u16(4), Some(10));
        assert_eq!(chunk.read_op(6), Some(OpCode::GetThis));
    }

    #[test]
    fn emit_returns() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_return();
        emitter.emit_return_void();

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::Return));
        assert_eq!(chunk.read_op(1), Some(OpCode::ReturnVoid));
    }

    #[test]
    fn emit_ref_counting() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        emitter.emit_add_ref();
        emitter.emit_release();

        let chunk = emitter.finish_chunk();

        // AddRef at offset 0 (no operands)
        assert_eq!(chunk.read_op(0), Some(OpCode::AddRef));

        // Release at offset 1 (no operands)
        assert_eq!(chunk.read_op(1), Some(OpCode::Release));
    }

    #[test]
    fn emit_func_ptr_ops() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let func_hash = TypeHash::from_name("callback");
        emitter.emit_func_ptr(func_hash);
        emitter.emit_call_func_ptr(2);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::FuncPtr));
        // After FuncPtr opcode and u16 index
        assert_eq!(chunk.read_op(3), Some(OpCode::CallFuncPtr));
        assert_eq!(chunk.read_byte(4), Some(2));
    }

    #[test]
    fn emit_init_list() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_init_list_begin(10);
        emitter.emit_init_list_end();

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::InitListBegin));
        assert_eq!(chunk.read_u16(1), Some(10));
        assert_eq!(chunk.read_op(3), Some(OpCode::InitListEnd));
    }

    #[test]
    fn wide_constant_index() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        // Fill up constant pool to force wide index
        for i in 0..256 {
            emitter.emit_int(i + 1000); // Use unique values
        }
        emitter.emit_int(999); // Should use ConstantWide

        let chunk = emitter.finish_chunk();
        // Find the last ConstantWide
        assert_eq!(chunk.read_op(chunk.len() - 3), Some(OpCode::ConstantWide));
        assert_eq!(chunk.read_u16(chunk.len() - 2), Some(256)); // Index 256
    }

    #[test]
    fn code_size() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        assert_eq!(emitter.code_size(), 0);
        emitter.emit(OpCode::PushTrue);
        assert_eq!(emitter.code_size(), 1);
        emitter.emit_byte(OpCode::GetLocal, 5);
        assert_eq!(emitter.code_size(), 3);
    }

    #[test]
    fn emit_globals() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let global_hash = TypeHash::from_name("g_counter");
        emitter.emit_get_global(global_hash);
        emitter.emit_set_global(global_hash);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::GetGlobal));
        assert_eq!(chunk.read_op(2), Some(OpCode::SetGlobal));

        let (constants, _, _) = emitter.decompose();
        // Same hash should be deduplicated
        assert_eq!(constants.len(), 1);
    }

    #[test]
    fn emit_method_calls() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let method_hash = TypeHash::from_name("MyClass::getValue");
        emitter.emit_call_method(method_hash, 0);

        // emit_call_virtual now takes vtable slot, not method hash
        emitter.emit_call_virtual(5, 2); // vtable slot 5, 2 args

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::CallMethod));
        assert_eq!(chunk.read_byte(3), Some(0)); // arg count
        assert_eq!(chunk.read_op(4), Some(OpCode::CallVirtual));
        assert_eq!(chunk.read_byte(7), Some(2)); // arg count
    }

    #[test]
    fn emit_interface_call() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let iface_hash = TypeHash::from_name("IMyInterface");
        emitter.emit_call_interface(iface_hash, 3, 1); // interface, slot 3, 1 arg

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::CallInterface));
    }

    #[test]
    fn emit_new_factory() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let factory_hash = TypeHash::from_name("createWidget");
        emitter.emit_new_factory(factory_hash, 3);

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::NewFactory));
        assert_eq!(chunk.read_byte(3), Some(3)); // arg count
    }

    #[test]
    fn emit_string_bytes() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit_string_bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        emitter.finish_chunk();

        let (constants, _, _) = emitter.decompose();
        assert_eq!(constants.len(), 1);
        assert_eq!(
            constants.get(0),
            Some(&Constant::StringData(vec![0xDE, 0xAD, 0xBE, 0xEF]))
        );
    }

    #[test]
    fn current_line_getter() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        assert_eq!(emitter.current_line(), 1); // Default
        emitter.set_line(42);
        assert_eq!(emitter.current_line(), 42);
    }

    #[test]
    fn in_loop_check() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        assert!(!emitter.in_loop());
        emitter.enter_loop(0);
        assert!(emitter.in_loop());
        emitter.exit_loop();
        assert!(!emitter.in_loop());
    }

    #[test]
    fn jump_label_offset() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.emit(OpCode::PushTrue); // offset 0
        let label = emitter.emit_jump(OpCode::Jump); // offset 1

        assert_eq!(label.offset(), 2); // Points to placeholder u16
    }

    // ==========================================================================
    // Switch Tests
    // ==========================================================================

    #[test]
    fn switch_break() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.enter_switch();
        assert!(emitter.in_switch());
        assert!(emitter.in_breakable());
        assert!(!emitter.in_loop());

        emitter.emit(OpCode::PushOne);
        emitter.emit_break().unwrap(); // break works in switch
        emitter.emit(OpCode::PushZero);

        emitter.exit_switch();
        assert!(!emitter.in_switch());

        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushOne));
        assert_eq!(chunk.read_op(1), Some(OpCode::Jump)); // break
    }

    #[test]
    fn switch_continue_error() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.enter_switch();

        // Continue should error in switch (no enclosing loop)
        let result = emitter.emit_continue();
        assert!(matches!(result, Err(BreakError::NotInLoop)));
    }

    #[test]
    fn switch_inside_loop() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        emitter.enter_switch();
        assert!(emitter.in_loop());
        assert!(emitter.in_switch());
        assert_eq!(emitter.loop_depth(), 1);
        assert_eq!(emitter.breakable_depth(), 2);

        // Break targets switch
        emitter.emit_break().unwrap();
        // Continue skips switch, targets loop
        emitter.emit_continue().unwrap();

        emitter.exit_switch();
        emitter.exit_loop();
    }

    #[test]
    fn loop_inside_switch() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        emitter.enter_switch();

        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);
        assert!(emitter.in_loop());
        assert!(emitter.in_switch());
        assert_eq!(emitter.loop_depth(), 1);
        assert_eq!(emitter.breakable_depth(), 2);

        emitter.exit_loop();
        assert!(!emitter.in_loop());
        assert!(emitter.in_switch());

        emitter.exit_switch();
    }

    #[test]
    fn set_continue_target() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);

        // Emit some instructions to advance the offset
        emitter.emit(OpCode::PushTrue);
        emitter.emit(OpCode::Pop);

        // Update continue target to a later point
        let new_target = emitter.current_offset();
        emitter.set_continue_target(new_target);

        // Emit more instructions
        emitter.emit(OpCode::PushFalse);

        // Continue should jump to the updated target (new_target)
        emitter.emit_continue().unwrap();

        emitter.exit_loop();
    }

    #[test]
    fn set_continue_target_skips_switch() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        let loop_start = emitter.current_offset();
        emitter.enter_loop(loop_start);
        emitter.enter_switch();

        // Emit some instructions
        emitter.emit(OpCode::PushTrue);

        // Update continue target - should update the loop, not the switch
        let new_target = emitter.current_offset();
        emitter.set_continue_target(new_target);

        emitter.exit_switch();

        // Emit more instructions after switch
        emitter.emit(OpCode::PushFalse);

        // Continue should jump to the updated target
        emitter.emit_continue().unwrap();
        emitter.exit_loop();
    }

    #[test]
    fn breakable_depth() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();
        assert_eq!(emitter.breakable_depth(), 0);

        emitter.enter_loop(0);
        assert_eq!(emitter.breakable_depth(), 1);

        emitter.enter_switch();
        assert_eq!(emitter.breakable_depth(), 2);

        emitter.enter_loop(0);
        assert_eq!(emitter.breakable_depth(), 3);

        emitter.exit_loop();
        assert_eq!(emitter.breakable_depth(), 2);

        emitter.exit_switch();
        assert_eq!(emitter.breakable_depth(), 1);

        emitter.exit_loop();
        assert_eq!(emitter.breakable_depth(), 0);
    }

    // ==========================================================================
    // Lifecycle Tests
    // ==========================================================================

    #[test]
    fn finish_function_registers_entry() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let func_hash = TypeHash::from_name("myFunc");
        emitter.start_chunk();
        emitter.emit(OpCode::PushTrue);
        emitter.emit(OpCode::Return);
        emitter.finish_function(func_hash, "myFunc".to_string());

        let (_, functions, _) = emitter.decompose();
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "myFunc");
        assert_eq!(functions[0].hash, func_hash);
        assert_eq!(functions[0].bytecode.read_op(0), Some(OpCode::PushTrue));
    }

    #[test]
    fn finish_global_init_registers_entry() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let global_hash = TypeHash::from_name("g_value");
        emitter.start_chunk();
        emitter.emit_int(42);
        emitter.finish_global_init(global_hash, "g_value".to_string());

        let (_, _, inits) = emitter.decompose();
        assert_eq!(inits.len(), 1);
        assert_eq!(inits[0].name, "g_value");
        assert_eq!(inits[0].hash, global_hash);
    }

    #[test]
    fn nested_chunks_for_lambdas() {
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Start outer function
        emitter.start_chunk();
        emitter.emit(OpCode::PushTrue);

        // Start inner lambda
        emitter.start_chunk();
        emitter.emit(OpCode::PushFalse);
        emitter.emit(OpCode::Return);
        let lambda_hash = TypeHash::from_name("$lambda_0");
        emitter.finish_function(lambda_hash, "$lambda_0".to_string());

        // Continue outer function
        emitter.emit(OpCode::Return);
        let func_hash = TypeHash::from_name("outerFunc");
        emitter.finish_function(func_hash, "outerFunc".to_string());

        let (_, functions, _) = emitter.decompose();
        assert_eq!(functions.len(), 2);

        // Lambda should be first (finished first)
        assert_eq!(functions[0].name, "$lambda_0");
        assert_eq!(functions[0].bytecode.read_op(0), Some(OpCode::PushFalse));

        // Outer function second
        assert_eq!(functions[1].name, "outerFunc");
        assert_eq!(functions[1].bytecode.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(functions[1].bytecode.read_op(1), Some(OpCode::Return));
    }
}
