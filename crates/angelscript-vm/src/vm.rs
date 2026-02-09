//! Virtual machine — executes compiled AngelScript bytecode.
//!
//! The VM uses a register-based design with a dword stack. Each function call
//! gets a stack frame at a fixed offset, and local variables / temporaries are
//! addressed as offsets from the frame pointer.
//!
//! Key registers:
//! - `register`: 64-bit general register (used for return values, comparisons, etc.)
//! - `program_counter`: current position in the bytecode buffer
//! - `frame_pointer`: base of the current stack frame
//! - `stack_pointer`: top of the stack

use angelscript_core::opcode::OpCode;
use angelscript_core::{FunctionId, QualifiedName, Value};

use crate::module::{Module, NativeCallContext};

/// Maximum stack size in dwords.
const MAX_STACK_SIZE: usize = 64 * 1024; // 256 KB

/// A call frame on the call stack.
#[derive(Debug, Clone)]
struct CallFrame {
    /// The function being executed.
    function_id: FunctionId,
    /// Program counter to resume at after return.
    return_pc: usize,
    /// Frame pointer of the caller.
    return_fp: usize,
    /// Stack pointer of the caller.
    return_sp: usize,
}

/// VM execution status.
#[derive(Debug, Clone, PartialEq)]
pub enum VmStatus {
    /// Ready to execute.
    Ready,
    /// Currently executing.
    Running,
    /// Execution suspended (cooperative yield).
    Suspended,
    /// Execution completed with a return value.
    Finished(Value),
    /// Execution failed with an error.
    Error(String),
}

/// The bytecode virtual machine.
pub struct Vm {
    /// The dword stack.
    stack: Vec<u32>,
    /// Stack pointer (next free dword).
    sp: usize,
    /// Frame pointer (base of current stack frame).
    fp: usize,
    /// Program counter (dword offset into current function's bytecode).
    pc: usize,
    /// 64-bit general-purpose register.
    register: u64,
    /// Call stack.
    call_stack: Vec<CallFrame>,
    /// Current function being executed.
    current_function: Option<FunctionId>,
    /// VM status.
    status: VmStatus,
}

impl Vm {
    pub fn new() -> Self {
        Vm {
            stack: vec![0; MAX_STACK_SIZE],
            sp: 0,
            fp: 0,
            pc: 0,
            register: 0,
            call_stack: Vec::new(),
            current_function: None,
            status: VmStatus::Ready,
        }
    }

    /// Get the current VM status.
    pub fn status(&self) -> &VmStatus {
        &self.status
    }

    /// Call a function by name and return the result.
    pub fn call(
        &mut self,
        module: &Module,
        name: &QualifiedName,
        args: &[Value],
    ) -> Result<Value, String> {
        let func_id = module
            .find_function(name)
            .ok_or_else(|| format!("function '{}' not found", name))?;

        self.call_by_id(module, func_id, args)
    }

    /// Call a function by ID and return the result.
    pub fn call_by_id(
        &mut self,
        module: &Module,
        func_id: FunctionId,
        args: &[Value],
    ) -> Result<Value, String> {
        // Reset state.
        self.sp = 0;
        self.fp = 0;
        self.pc = 0;
        self.register = 0;
        self.call_stack.clear();
        self.status = VmStatus::Running;

        // Push arguments onto the stack frame.
        // Arguments are at the start of the frame, indexed by offset.
        for arg in args {
            match arg {
                Value::I64(v) => {
                    self.stack_write_qw(self.sp, *v as u64);
                    self.sp += 2;
                }
                Value::U64(v) => {
                    self.stack_write_qw(self.sp, *v);
                    self.sp += 2;
                }
                Value::F64(v) => {
                    let bits = v.to_bits();
                    self.stack_write_qw(self.sp, bits);
                    self.sp += 2;
                }
                _ => {
                    self.stack_write_dw(self.sp, arg.to_dword());
                    self.sp += 1;
                }
            }
        }

        // Check if this is a native function.
        if module.get_native_function(func_id).is_some() {
            return self.call_native(module, func_id);
        }

        // Set up the script function.
        let func = module
            .get_function(func_id)
            .ok_or_else(|| format!("function id {} not found", func_id))?;

        // Reserve stack space for locals + temporaries.
        let frame_size = func.stack_size as usize;
        if self.sp + frame_size > MAX_STACK_SIZE {
            return Err("stack overflow".to_string());
        }
        // Zero-init the frame beyond args.
        let args_size = self.sp;
        for i in args_size..args_size + frame_size.saturating_sub(args_size) {
            self.stack[i] = 0;
        }
        self.sp = self.sp.max(frame_size);
        self.current_function = Some(func_id);

        // Execute.
        self.execute(module)
    }

    /// Execute the current function until it returns or errors.
    fn execute(&mut self, module: &Module) -> Result<Value, String> {
        loop {
            let func_id = match self.current_function {
                Some(id) => id,
                None => {
                    self.status = VmStatus::Error("no current function".to_string());
                    return Err("no current function".to_string());
                }
            };

            let func = match module.get_function(func_id) {
                Some(f) => f,
                None => {
                    self.status = VmStatus::Error(format!("function {} not found", func_id));
                    return Err(format!("function {} not found", func_id));
                }
            };

            // Check bounds.
            if self.pc >= func.bytecode.data.len() {
                // Past end of bytecode — implicit return.
                if let Some(frame) = self.call_stack.pop() {
                    self.pc = frame.return_pc;
                    self.fp = frame.return_fp;
                    self.sp = frame.return_sp;
                    self.current_function = Some(frame.function_id);
                    continue;
                } else {
                    // Top-level return.
                    let result = self.register_to_value();
                    self.status = VmStatus::Finished(result.clone());
                    return Ok(result);
                }
            }

            // Decode instruction.
            let word0 = func.bytecode.data[self.pc];
            let op_byte = (word0 & 0xFF) as u8;
            let op = match decode_opcode(op_byte) {
                Some(op) => op,
                None => {
                    let msg = format!("unknown opcode: {}", op_byte);
                    self.status = VmStatus::Error(msg.clone());
                    return Err(msg);
                }
            };

            // Extract inline word arguments from word0.
            // For 1-dword instructions: opcode[7:0] | w_arg0[15:8] | w_arg1[31:16]
            // w_arg0 is 8 bits only in 1-dword format (enough for variable offsets < 256).
            let w_arg0: i16;
            let w_arg1: i16;
            if op.size_dwords() == 1 {
                // 1-dword: w_arg0 is bits 8-15 (8-bit), w_arg1 is bits 16-31 (16-bit).
                w_arg0 = ((word0 >> 8) & 0xFF) as u8 as i8 as i16;
                w_arg1 = ((word0 >> 16) as u16) as i16;
            } else {
                // 2+ dword: w_arg0 is bits 16-31 of word0.
                w_arg0 = ((word0 >> 16) as u16) as i16;
                w_arg1 = 0;
            }

            // For 2-dword instructions, the second word is a dword arg.
            let dw_arg = if op.size_dwords() >= 2 && self.pc + 1 < func.bytecode.data.len() {
                func.bytecode.data[self.pc + 1]
            } else {
                0
            };

            // For 3-dword instructions (SetV8, PshC8, ALLOC), read qword.
            let qw_arg = if op.size_dwords() >= 3 && self.pc + 2 < func.bytecode.data.len() {
                let lo = func.bytecode.data[self.pc + 1] as u64;
                let hi = func.bytecode.data[self.pc + 2] as u64;
                lo | (hi << 32)
            } else {
                0
            };

            // For 2-dword instructions, w_arg0 is in the upper 16 bits of word0.
            let w_arg0_2dw = ((word0 >> 16) as u16) as i16;

            // Advance PC past this instruction.
            let inst_size = op.size_dwords() as usize;
            self.pc += inst_size;

            // Dispatch.
            match op {
                // ── Set operations ──────────────────────────────────────
                OpCode::SetV4 => {
                    // SetV4 is a 2-dword instruction: opcode+w_arg0 | dw_arg
                    // w_arg0 (in upper 16 bits of word0) = variable offset, dw_arg = value
                    let offset = w_arg0_2dw as usize;
                    self.stack_write_dw(self.fp + offset, dw_arg);
                }
                OpCode::SetV8 => {
                    // SetV8 is a 3-dword instruction: opcode+w_arg0 | qw_lo | qw_hi
                    let offset = w_arg0_2dw as usize;
                    self.stack_write_qw(self.fp + offset, qw_arg);
                }

                // ── Copy operations ─────────────────────────────────────
                OpCode::CpyVtoV4 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let val = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, val);
                }
                OpCode::CpyVtoV8 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let val = self.stack_read_qw(self.fp + src);
                    self.stack_write_qw(self.fp + dst, val);
                }
                OpCode::CpyVtoR4 => {
                    let src = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + src);
                    self.register = val as u64;
                }
                OpCode::CpyVtoR8 => {
                    let src = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + src);
                    self.register = val;
                }
                OpCode::CpyRtoV4 => {
                    let dst = w_arg0 as usize;
                    self.stack_write_dw(self.fp + dst, self.register as u32);
                }
                OpCode::CpyRtoV8 => {
                    let dst = w_arg0 as usize;
                    self.stack_write_qw(self.fp + dst, self.register);
                }

                // ── Integer arithmetic ──────────────────────────────────
                OpCode::ADDi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src) as i32;
                    self.stack_write_dw(self.fp + dst, a.wrapping_add(b) as u32);
                }
                OpCode::SUBi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src) as i32;
                    self.stack_write_dw(self.fp + dst, a.wrapping_sub(b) as u32);
                }
                OpCode::MULi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src) as i32;
                    self.stack_write_dw(self.fp + dst, a.wrapping_mul(b) as u32);
                }
                OpCode::DIVi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src) as i32;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_dw(self.fp + dst, a.wrapping_div(b) as u32);
                }
                OpCode::MODi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src) as i32;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_dw(self.fp + dst, a.wrapping_rem(b) as u32);
                }

                // ── Float arithmetic ────────────────────────────────────
                OpCode::ADDf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + src));
                    self.stack_write_dw(self.fp + dst, (a + b).to_bits());
                }
                OpCode::SUBf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + src));
                    self.stack_write_dw(self.fp + dst, (a - b).to_bits());
                }
                OpCode::MULf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + src));
                    self.stack_write_dw(self.fp + dst, (a * b).to_bits());
                }
                OpCode::DIVf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + src));
                    if b == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_dw(self.fp + dst, (a / b).to_bits());
                }
                OpCode::MODf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + src));
                    if b == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_dw(self.fp + dst, (a % b).to_bits());
                }

                // ── Double arithmetic ───────────────────────────────────
                OpCode::ADDd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + src));
                    self.stack_write_qw(self.fp + dst, (a + b).to_bits());
                }
                OpCode::SUBd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + src));
                    self.stack_write_qw(self.fp + dst, (a - b).to_bits());
                }
                OpCode::MULd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + src));
                    self.stack_write_qw(self.fp + dst, (a * b).to_bits());
                }
                OpCode::DIVd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + src));
                    if b == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_qw(self.fp + dst, (a / b).to_bits());
                }
                OpCode::MODd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + src));
                    if b == 0.0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_qw(self.fp + dst, (a % b).to_bits());
                }

                // ── 64-bit integer arithmetic ───────────────────────────
                OpCode::ADDi64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as i64;
                    self.stack_write_qw(self.fp + dst, a.wrapping_add(b) as u64);
                }
                OpCode::SUBi64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as i64;
                    self.stack_write_qw(self.fp + dst, a.wrapping_sub(b) as u64);
                }
                OpCode::MULi64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as i64;
                    self.stack_write_qw(self.fp + dst, a.wrapping_mul(b) as u64);
                }
                OpCode::DIVi64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as i64;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_qw(self.fp + dst, a.wrapping_div(b) as u64);
                }
                OpCode::MODi64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as i64;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    self.stack_write_qw(self.fp + dst, a.wrapping_rem(b) as u64);
                }

                // ── Negate ──────────────────────────────────────────────
                OpCode::NEGi => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_dw(self.fp + offset, val.wrapping_neg() as u32);
                }
                OpCode::NEGf => {
                    let offset = w_arg0 as usize;
                    let val = f32::from_bits(self.stack_read_dw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, (-val).to_bits());
                }
                OpCode::NEGd => {
                    let offset = w_arg0 as usize;
                    let val = f64::from_bits(self.stack_read_qw(self.fp + offset));
                    self.stack_write_qw(self.fp + offset, (-val).to_bits());
                }
                OpCode::NEGi64 => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + offset) as i64;
                    self.stack_write_qw(self.fp + offset, val.wrapping_neg() as u64);
                }

                // ── Increment / Decrement ───────────────────────────────
                OpCode::IncVi => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_dw(self.fp + offset, val.wrapping_add(1) as u32);
                }
                OpCode::DecVi => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_dw(self.fp + offset, val.wrapping_sub(1) as u32);
                }

                // ── Bitwise operations ──────────────────────────────────
                OpCode::BNOT => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset);
                    self.stack_write_dw(self.fp + offset, !val);
                }
                OpCode::BAND => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst);
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a & b);
                }
                OpCode::BOR => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst);
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a | b);
                }
                OpCode::BXOR => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst);
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a ^ b);
                }
                OpCode::BSLL => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst);
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a.wrapping_shl(b));
                }
                OpCode::BSRL => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst);
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a.wrapping_shr(b));
                }
                OpCode::BSRA => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + dst) as i32;
                    let b = self.stack_read_dw(self.fp + src);
                    self.stack_write_dw(self.fp + dst, a.wrapping_shr(b) as u32);
                }

                // ── 64-bit bitwise ──────────────────────────────────────
                OpCode::BNOT64 => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + offset);
                    self.stack_write_qw(self.fp + offset, !val);
                }
                OpCode::BAND64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst);
                    let b = self.stack_read_qw(self.fp + src);
                    self.stack_write_qw(self.fp + dst, a & b);
                }
                OpCode::BOR64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst);
                    let b = self.stack_read_qw(self.fp + src);
                    self.stack_write_qw(self.fp + dst, a | b);
                }
                OpCode::BXOR64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst);
                    let b = self.stack_read_qw(self.fp + src);
                    self.stack_write_qw(self.fp + dst, a ^ b);
                }
                OpCode::BSLL64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst);
                    let b = self.stack_read_qw(self.fp + src) as u32;
                    self.stack_write_qw(self.fp + dst, a.wrapping_shl(b));
                }
                OpCode::BSRL64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst);
                    let b = self.stack_read_qw(self.fp + src) as u32;
                    self.stack_write_qw(self.fp + dst, a.wrapping_shr(b));
                }
                OpCode::BSRA64 => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + dst) as i64;
                    let b = self.stack_read_qw(self.fp + src) as u32;
                    self.stack_write_qw(self.fp + dst, a.wrapping_shr(b) as u64);
                }

                // ── Comparisons ─────────────────────────────────────────
                OpCode::CMPi => {
                    let a_off = w_arg0 as usize;
                    let b_off = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + a_off) as i32;
                    let b = self.stack_read_dw(self.fp + b_off) as i32;
                    let result = if a < b {
                        -1i32
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    self.register = result as u32 as u64;
                }
                OpCode::CMPu => {
                    let a_off = w_arg0 as usize;
                    let b_off = w_arg1 as usize;
                    let a = self.stack_read_dw(self.fp + a_off);
                    let b = self.stack_read_dw(self.fp + b_off);
                    let result = if a < b {
                        -1i32
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    self.register = result as u32 as u64;
                }
                OpCode::CMPf => {
                    let a_off = w_arg0 as usize;
                    let b_off = w_arg1 as usize;
                    let a = f32::from_bits(self.stack_read_dw(self.fp + a_off));
                    let b = f32::from_bits(self.stack_read_dw(self.fp + b_off));
                    let result = if a < b {
                        -1i32
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    self.register = result as u32 as u64;
                }
                OpCode::CMPd => {
                    let a_off = w_arg0 as usize;
                    let b_off = w_arg1 as usize;
                    let a = f64::from_bits(self.stack_read_qw(self.fp + a_off));
                    let b = f64::from_bits(self.stack_read_qw(self.fp + b_off));
                    let result = if a < b {
                        -1i32
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    self.register = result as u32 as u64;
                }
                OpCode::CMPi64 => {
                    let a_off = w_arg0 as usize;
                    let b_off = w_arg1 as usize;
                    let a = self.stack_read_qw(self.fp + a_off) as i64;
                    let b = self.stack_read_qw(self.fp + b_off) as i64;
                    let result = if a < b {
                        -1i32
                    } else if a > b {
                        1
                    } else {
                        0
                    };
                    self.register = result as u32 as u64;
                }

                // ── Logic / conditional ─────────────────────────────────
                OpCode::NOT => {
                    let val = self.register as u32;
                    self.register = if val == 0 { 1 } else { 0 };
                }
                OpCode::TZ => {
                    let val = self.register as i32;
                    self.register = if val == 0 { 1 } else { 0 };
                }
                OpCode::TNZ => {
                    let val = self.register as i32;
                    self.register = if val != 0 { 1 } else { 0 };
                }
                OpCode::TS => {
                    let val = self.register as i32;
                    self.register = if val < 0 { 1 } else { 0 };
                }
                OpCode::TNS => {
                    let val = self.register as i32;
                    self.register = if val >= 0 { 1 } else { 0 };
                }
                OpCode::TP => {
                    let val = self.register as i32;
                    self.register = if val > 0 { 1 } else { 0 };
                }
                OpCode::TNP => {
                    let val = self.register as i32;
                    self.register = if val <= 0 { 1 } else { 0 };
                }

                // ── Control flow ────────────────────────────────────────
                OpCode::JMP => {
                    let offset = dw_arg as i32;
                    self.pc = (self.pc as i32 + offset) as usize;
                }
                OpCode::JZ => {
                    let offset = dw_arg as i32;
                    if self.register as u32 == 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                OpCode::JNZ => {
                    let offset = dw_arg as i32;
                    if self.register as u32 != 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                OpCode::JS => {
                    let offset = dw_arg as i32;
                    if (self.register as i32) < 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                OpCode::JNS => {
                    let offset = dw_arg as i32;
                    if (self.register as i32) >= 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                OpCode::JP => {
                    let offset = dw_arg as i32;
                    if (self.register as i32) > 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }
                OpCode::JNP => {
                    let offset = dw_arg as i32;
                    if (self.register as i32) <= 0 {
                        self.pc = (self.pc as i32 + offset) as usize;
                    }
                }

                OpCode::CALL => {
                    let target_func_id = FunctionId(dw_arg);

                    // Check for native function.
                    if module.get_native_function(target_func_id).is_some() {
                        let result = self.call_native(module, target_func_id)?;
                        // Put the return value in the register.
                        match result {
                            Value::I32(v) => self.register = v as u32 as u64,
                            Value::I64(v) => self.register = v as u64,
                            Value::F32(v) => self.register = v.to_bits() as u64,
                            Value::F64(v) => self.register = v.to_bits(),
                            Value::Bool(v) => self.register = if v { 1 } else { 0 },
                            _ => self.register = 0,
                        }
                        continue;
                    }

                    // Script function call.
                    let callee = module.get_function(target_func_id).ok_or_else(|| {
                        format!("call target function {} not found", target_func_id)
                    })?;

                    // Save current state.
                    self.call_stack.push(CallFrame {
                        function_id: func_id,
                        return_pc: self.pc,
                        return_fp: self.fp,
                        return_sp: self.sp,
                    });

                    // Set up new frame.
                    let new_fp = self.sp;
                    let frame_size = callee.stack_size as usize;
                    if new_fp + frame_size > MAX_STACK_SIZE {
                        return Err("stack overflow".to_string());
                    }

                    // Zero-init the new frame.
                    for i in new_fp..new_fp + frame_size {
                        self.stack[i] = 0;
                    }

                    self.fp = new_fp;
                    self.sp = new_fp + frame_size;
                    self.pc = 0;
                    self.current_function = Some(target_func_id);
                }

                OpCode::RET => {
                    if let Some(frame) = self.call_stack.pop() {
                        self.pc = frame.return_pc;
                        self.fp = frame.return_fp;
                        self.sp = frame.return_sp;
                        self.current_function = Some(frame.function_id);
                    } else {
                        // Top-level return.
                        let result = self.register_to_value();
                        self.status = VmStatus::Finished(result.clone());
                        return Ok(result);
                    }
                }

                // ── Type conversions ────────────────────────────────────
                OpCode::iTOf => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_dw(self.fp + offset, (val as f32).to_bits());
                }
                OpCode::iTOd => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_qw(self.fp + offset, (val as f64).to_bits());
                }
                OpCode::fTOi => {
                    let offset = w_arg0 as usize;
                    let val = f32::from_bits(self.stack_read_dw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, (val as i32) as u32);
                }
                OpCode::fTOd => {
                    let offset = w_arg0 as usize;
                    let val = f32::from_bits(self.stack_read_dw(self.fp + offset));
                    self.stack_write_qw(self.fp + offset, (val as f64).to_bits());
                }
                OpCode::dTOi => {
                    let offset = w_arg0 as usize;
                    let val = f64::from_bits(self.stack_read_qw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, (val as i32) as u32);
                }
                OpCode::dTOf => {
                    let offset = w_arg0 as usize;
                    let val = f64::from_bits(self.stack_read_qw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, (val as f32).to_bits());
                }
                OpCode::iTOi64 => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset) as i32;
                    self.stack_write_qw(self.fp + offset, val as i64 as u64);
                }
                OpCode::i64TOi => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + offset) as i64;
                    self.stack_write_dw(self.fp + offset, val as i32 as u32);
                }
                OpCode::i64TOf => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + offset) as i64;
                    self.stack_write_dw(self.fp + offset, (val as f32).to_bits());
                }
                OpCode::i64TOd => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_qw(self.fp + offset) as i64;
                    self.stack_write_qw(self.fp + offset, (val as f64).to_bits());
                }
                OpCode::fTOi64 => {
                    let offset = w_arg0 as usize;
                    let val = f32::from_bits(self.stack_read_dw(self.fp + offset));
                    self.stack_write_qw(self.fp + offset, val as i64 as u64);
                }
                OpCode::dTOi64 => {
                    let offset = w_arg0 as usize;
                    let val = f64::from_bits(self.stack_read_qw(self.fp + offset));
                    self.stack_write_qw(self.fp + offset, val as i64 as u64);
                }
                OpCode::uTOi64 => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset);
                    self.stack_write_qw(self.fp + offset, val as u64);
                }
                OpCode::uTOf => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset);
                    self.stack_write_dw(self.fp + offset, (val as f32).to_bits());
                }
                OpCode::uTOd => {
                    let offset = w_arg0 as usize;
                    let val = self.stack_read_dw(self.fp + offset);
                    self.stack_write_qw(self.fp + offset, (val as f64).to_bits());
                }
                OpCode::fTOu => {
                    let offset = w_arg0 as usize;
                    let val = f32::from_bits(self.stack_read_dw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, val as u32);
                }
                OpCode::dTOu => {
                    let offset = w_arg0 as usize;
                    let val = f64::from_bits(self.stack_read_qw(self.fp + offset));
                    self.stack_write_dw(self.fp + offset, val as u32);
                }

                // ── Power operations ────────────────────────────────────
                OpCode::POWi => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let base = self.stack_read_dw(self.fp + dst) as i32;
                    let exp = self.stack_read_dw(self.fp + src) as i32;
                    let result = if exp >= 0 {
                        (base as f64).powi(exp) as i32
                    } else {
                        0
                    };
                    self.stack_write_dw(self.fp + dst, result as u32);
                }
                OpCode::POWf => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let base = f32::from_bits(self.stack_read_dw(self.fp + dst));
                    let exp = f32::from_bits(self.stack_read_dw(self.fp + src));
                    self.stack_write_dw(self.fp + dst, base.powf(exp).to_bits());
                }
                OpCode::POWd => {
                    let dst = w_arg0 as usize;
                    let src = w_arg1 as usize;
                    let base = f64::from_bits(self.stack_read_qw(self.fp + dst));
                    let exp = f64::from_bits(self.stack_read_qw(self.fp + src));
                    self.stack_write_qw(self.fp + dst, base.powf(exp).to_bits());
                }

                // ── No-op / metadata instructions ───────────────────────
                OpCode::SUSPEND
                | OpCode::LINE
                | OpCode::LABEL
                | OpCode::VarDecl
                | OpCode::Block
                | OpCode::ObjInfo => {
                    // No-op in execution.
                }

                // ── TryBlock ────────────────────────────────────────────
                OpCode::TryBlock => {
                    // For now, skip past the try-catch marker.
                    // Full exception handling would record the catch handler offset.
                }

                // ── Fallback for unimplemented opcodes ──────────────────
                _ => {
                    return Err(format!("unimplemented opcode: {:?}", op));
                }
            }
        }
    }

    /// Call a native function.
    fn call_native(&mut self, module: &Module, func_id: FunctionId) -> Result<Value, String> {
        let native = module
            .get_native_function(func_id)
            .ok_or_else(|| format!("native function {} not found", func_id))?;

        // Collect args from the current stack frame.
        let args: Vec<u32> = (0..self.sp.saturating_sub(self.fp))
            .map(|i| self.stack_read_dw(self.fp + i))
            .collect();

        let mut ctx = NativeCallContext::new(args);
        (native.callback)(&mut ctx)?;

        if ctx.return_is_64bit {
            self.register = ctx.return_value;
            Ok(Value::I64(ctx.return_value as i64))
        } else {
            self.register = ctx.return_value;
            Ok(Value::I32(ctx.return_value as i32))
        }
    }

    /// Convert the register to a Value (used for return values).
    fn register_to_value(&self) -> Value {
        // The register is 64 bits. For a basic return, we use the lower 32 bits as i32.
        // The caller should interpret based on the function's return type.
        Value::I32(self.register as i32)
    }

    // ── Stack helpers ───────────────────────────────────────────────────

    fn stack_read_dw(&self, offset: usize) -> u32 {
        if offset < self.stack.len() {
            self.stack[offset]
        } else {
            0
        }
    }

    fn stack_write_dw(&mut self, offset: usize, val: u32) {
        if offset < self.stack.len() {
            self.stack[offset] = val;
        }
    }

    fn stack_read_qw(&self, offset: usize) -> u64 {
        if offset + 1 < self.stack.len() {
            let lo = self.stack[offset] as u64;
            let hi = self.stack[offset + 1] as u64;
            lo | (hi << 32)
        } else {
            0
        }
    }

    fn stack_write_qw(&mut self, offset: usize, val: u64) {
        if offset + 1 < self.stack.len() {
            self.stack[offset] = val as u32;
            self.stack[offset + 1] = (val >> 32) as u32;
        }
    }
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode an opcode byte to an `OpCode` enum value.
fn decode_opcode(byte: u8) -> Option<OpCode> {
    // We rely on the repr(u8) ordering matching exactly.
    // Since OpCode is repr(u8), we can transmute carefully.
    // The total number of opcodes defines the valid range.
    if byte <= OpCode::LABEL as u8 {
        // SAFETY: OpCode is repr(u8) with contiguous values 0..=LABEL.
        Some(unsafe { std::mem::transmute::<u8, OpCode>(byte) })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ScriptFunction;
    use angelscript_core::opcode::ByteCode;

    /// Helper to build a module with a single function from raw bytecode dwords.
    fn single_function_module(name: &str, bytecode_data: Vec<u32>, stack_size: i16) -> Module {
        let mut module = Module::new();
        let func_name = QualifiedName::global(name);
        let id = FunctionId(1);
        module.add_function(ScriptFunction {
            id,
            name: func_name,
            bytecode: ByteCode {
                data: bytecode_data,
            },
            stack_size,
            param_count: 0,
        });
        module
    }

    /// Encode a 1-dword instruction (no dword arg).
    fn encode_1dw(op: OpCode, w0: i16, w1: i16) -> u32 {
        (op as u32) | ((w0 as u16 as u32) << 8) | ((w1 as u16 as u32) << 16)
    }

    /// Encode word0 of a 2-dword instruction.
    fn encode_2dw_word0(op: OpCode, w0: i16) -> u32 {
        (op as u32) | ((w0 as u16 as u32) << 16)
    }

    #[test]
    fn test_return_constant() {
        // int answer() { return 42; }
        // SetV4 v0 = 42
        // CpyVtoR4 v0
        // RET
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            42,                                 // SetV4 v0, 42
            encode_1dw(OpCode::CpyVtoR4, 0, 0), // CpyVtoR4 v0
            encode_1dw(OpCode::RET, 0, 0),      // RET
        ];
        let module = single_function_module("answer", bc, 1);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("answer"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_add_two_ints() {
        // int add() { int a = 10; int b = 20; return a + b; }
        // Stack: v0=a(offset 0), v1=b(offset 1), v2=temp(offset 2)
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            10, // SetV4 v0, 10
            encode_2dw_word0(OpCode::SetV4, 1),
            20, // SetV4 v1, 20
            // Copy v0 to v2, then ADDi v2, v1 (result in v2)
            encode_1dw(OpCode::CpyVtoV4, 2, 0), // CpyVtoV4 v2, v0
            encode_1dw(OpCode::ADDi, 2, 1),     // ADDi v2, v1
            encode_1dw(OpCode::CpyVtoR4, 2, 0), // CpyVtoR4 v2
            encode_1dw(OpCode::RET, 0, 0),      // RET
        ];
        let module = single_function_module("add", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("add"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(30));
    }

    #[test]
    fn test_subtraction() {
        // Stack: v0=a(0), v1=b(1), v2=temp(2)
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            50,
            encode_2dw_word0(OpCode::SetV4, 1),
            30,
            encode_1dw(OpCode::CpyVtoV4, 2, 0),
            encode_1dw(OpCode::SUBi, 2, 1),
            encode_1dw(OpCode::CpyVtoR4, 2, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("sub", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("sub"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(20));
    }

    #[test]
    fn test_multiplication() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            6,
            encode_2dw_word0(OpCode::SetV4, 1),
            7,
            encode_1dw(OpCode::CpyVtoV4, 2, 0),
            encode_1dw(OpCode::MULi, 2, 1),
            encode_1dw(OpCode::CpyVtoR4, 2, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("mul", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("mul"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_division() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            100,
            encode_2dw_word0(OpCode::SetV4, 1),
            4,
            encode_1dw(OpCode::CpyVtoV4, 2, 0),
            encode_1dw(OpCode::DIVi, 2, 1),
            encode_1dw(OpCode::CpyVtoR4, 2, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("div", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("div"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(25));
    }

    #[test]
    fn test_division_by_zero() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            10,
            encode_2dw_word0(OpCode::SetV4, 1),
            0,
            encode_1dw(OpCode::CpyVtoV4, 2, 0),
            encode_1dw(OpCode::DIVi, 2, 1),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("div", bc, 3);
        let mut vm = Vm::new();
        let result = vm.call(&module, &QualifiedName::global("div"), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("division by zero"));
    }

    #[test]
    fn test_negation() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            42,
            encode_1dw(OpCode::NEGi, 0, 0),
            encode_1dw(OpCode::CpyVtoR4, 0, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("neg", bc, 1);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("neg"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(-42));
    }

    #[test]
    fn test_conditional_jump() {
        // if (10 == 10) return 1; else return 0;
        // SetV4 v0=10, SetV4 v1=10
        // CMPi v0, v1 -> register = 0 (equal)
        // JZ skip_else (relative offset +2 dwords = past next JMP + RET? Let's use simple flow)
        // Actually let's do: CMP, JNZ to else, set 1 + RET, else: set 0 + RET

        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            10, // v0=10
            encode_2dw_word0(OpCode::SetV4, 1),
            10,                             // v1=10
            encode_1dw(OpCode::CMPi, 0, 1), // CMP v0,v1 -> reg=0 (equal)
            // JNZ +6 (skip to else: SetV4 0 + CpyVtoR4 + RET = 2+1+1=4 dwords)
            encode_2dw_word0(OpCode::JNZ, 0),
            4u32.wrapping_neg(), // Will actually use positive offset
        ];

        // Let me build this more carefully:
        // offset 0: SetV4 v0 10     (2 dw)
        // offset 2: SetV4 v1 10     (2 dw)
        // offset 4: CMPi v0 v1      (1 dw) -> reg=0
        // offset 5: JNZ +5          (2 dw) -> jump to offset 12 if not zero
        // offset 7: SetV4 v2 1      (2 dw) -> then branch
        // offset 9: CpyVtoR4 v2     (1 dw)
        // offset 10: RET             (1 dw)
        // offset 11 (else):
        //   but JNZ's offset is from offset 7 (after the instruction), so +5 would be offset 12.
        //   Let's not overcomplicate. Let me use simpler logic:
        //   return 1; (since equal)

        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            10, // offset 0: v0=10
            encode_2dw_word0(OpCode::SetV4, 1),
            10,                             // offset 2: v1=10
            encode_1dw(OpCode::CMPi, 0, 1), // offset 4: CMP -> reg=0
            encode_2dw_word0(OpCode::JNZ, 0),
            4, // offset 5: JNZ +4 -> goto 11
            // then: return 1
            encode_2dw_word0(OpCode::SetV4, 2),
            1,                                  // offset 7: v2=1
            encode_1dw(OpCode::CpyVtoR4, 2, 0), // offset 9: reg=v2
            encode_1dw(OpCode::RET, 0, 0),      // offset 10: return
            // else: return 0
            encode_2dw_word0(OpCode::SetV4, 2),
            0,                                  // offset 11: v2=0
            encode_1dw(OpCode::CpyVtoR4, 2, 0), // offset 13: reg=v2
            encode_1dw(OpCode::RET, 0, 0),      // offset 14: return
        ];
        let module = single_function_module("cond", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("cond"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(1)); // 10 == 10, so then branch
    }

    #[test]
    fn test_loop_sum() {
        // Sum 1+2+3+4+5 using a loop.
        // v0 = sum (offset 0)
        // v1 = i (offset 1)
        // v2 = temp (offset 2)

        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            0, // offset 0: sum=0
            encode_2dw_word0(OpCode::SetV4, 1),
            1, // offset 2: i=1
            // Loop start (offset 4):
            encode_2dw_word0(OpCode::SetV4, 2),
            6,                              // offset 4: temp=6 (limit)
            encode_1dw(OpCode::CMPi, 1, 2), // offset 6: CMP i, 6
            // JS = jump if negative (i < 6)
            // JNS = jump if not negative (i >= 6) -> exit loop
            encode_2dw_word0(OpCode::JNS, 0),
            4, // offset 7: JNS +4 -> goto 13 (exit)
            // loop body: sum += i
            encode_1dw(OpCode::ADDi, 0, 1),  // offset 9: sum += i
            encode_1dw(OpCode::IncVi, 1, 0), // offset 10: i++
            encode_2dw_word0(OpCode::JMP, 0),
            (-10i32 as u32), // offset 11: JMP -10 -> goto 3...
                             // Actually let me compute: JMP offset is from after the instruction.
                             // After JMP at offset 11 (size 2), next instruction would be at offset 13.
                             // We want to jump to offset 4. So relative = 4 - 13 = -9.
                             // Let me redo:
        ];

        // Redo with correct relative offsets:
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            0, // offset 0: sum=0
            encode_2dw_word0(OpCode::SetV4, 1),
            1, // offset 2: i=1
            // Loop start at offset 4:
            encode_2dw_word0(OpCode::SetV4, 2),
            6,                              // offset 4: temp=6
            encode_1dw(OpCode::CMPi, 1, 2), // offset 6: CMP i, 6 -> reg = i<6? -1 : i==6? 0 : 1
            encode_2dw_word0(OpCode::JNS, 0),
            4, // offset 7: JNS +4 -> skip to offset 13 (i >= 6)
            // Loop body (offset 9):
            encode_1dw(OpCode::ADDi, 0, 1),  // offset 9: sum += i
            encode_1dw(OpCode::IncVi, 1, 0), // offset 10: i++
            encode_2dw_word0(OpCode::JMP, 0),
            (-9i32 as u32), // offset 11: JMP -9 -> goto 4
            // Loop exit (offset 13):
            encode_1dw(OpCode::CpyVtoR4, 0, 0), // offset 13: reg = sum
            encode_1dw(OpCode::RET, 0, 0),      // offset 14: return
        ];
        let module = single_function_module("sum", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("sum"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(15)); // 1+2+3+4+5 = 15
    }

    #[test]
    fn test_float_arithmetic() {
        // float calc: 1.5 * 3.0 = 4.5
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            1.5f32.to_bits(),
            encode_2dw_word0(OpCode::SetV4, 1),
            3.0f32.to_bits(),
            encode_1dw(OpCode::CpyVtoV4, 2, 0),
            encode_1dw(OpCode::MULf, 2, 1),
            encode_1dw(OpCode::CpyVtoR4, 2, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("calc", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("calc"), &[])
            .unwrap();
        let val = f32::from_bits(result.to_dword());
        assert!((val - 4.5).abs() < 0.001);
    }

    #[test]
    fn test_double_arithmetic() {
        // double calc: 2.5 + 3.5 = 6.0
        let bc = vec![
            encode_2dw_word0(OpCode::SetV8, 0),
            2.5f64.to_bits() as u32,
            (2.5f64.to_bits() >> 32) as u32,
            encode_2dw_word0(OpCode::SetV8, 2),
            3.5f64.to_bits() as u32,
            (3.5f64.to_bits() >> 32) as u32,
            encode_1dw(OpCode::CpyVtoV8, 4, 0),
            encode_1dw(OpCode::ADDd, 4, 2),
            encode_1dw(OpCode::CpyVtoR8, 4, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("calc", bc, 6);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("calc"), &[])
            .unwrap();
        // Register has the raw bits; extract as f64.
        let val = f64::from_bits(vm.register);
        assert!((val - 6.0).abs() < 0.001);
    }

    #[test]
    fn test_logical_not() {
        // NOT: register = !0 = 1
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            0,
            encode_1dw(OpCode::CpyVtoR4, 0, 0), // reg = 0
            encode_1dw(OpCode::NOT, 0, 0),      // reg = 1
            // Store reg back for return.
            encode_1dw(OpCode::CpyRtoV4, 0, 0),
            encode_1dw(OpCode::CpyVtoR4, 0, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("not", bc, 1);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("not"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(1));
    }

    #[test]
    fn test_bitwise_operations() {
        // v0 = 0xFF00, v1 = 0x0FF0
        // AND: 0x0F00, OR: 0xFFF0, XOR: 0xF0F0
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            0xFF00,
            encode_2dw_word0(OpCode::SetV4, 1),
            0x0FF0,
            // Test AND
            encode_1dw(OpCode::CpyVtoV4, 2, 0), // v2 = v0
            encode_1dw(OpCode::BAND, 2, 1),     // v2 &= v1
            encode_1dw(OpCode::CpyVtoR4, 2, 0), // reg = v2 (0x0F00)
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("band", bc, 3);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("band"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(0x0F00));
    }

    #[test]
    fn test_conversion_itof() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            42,
            encode_1dw(OpCode::iTOf, 0, 0), // v0 = 42 as float
            encode_1dw(OpCode::CpyVtoR4, 0, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("conv", bc, 1);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("conv"), &[])
            .unwrap();
        let val = f32::from_bits(result.to_dword());
        assert!((val - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_parameters() {
        // Function that takes two i32 params and returns their sum.
        // Params are at v0 and v1 in the stack frame.
        let bc = vec![
            encode_1dw(OpCode::CpyVtoV4, 2, 0), // v2 = v0 (first param)
            encode_1dw(OpCode::ADDi, 2, 1),     // v2 += v1 (second param)
            encode_1dw(OpCode::CpyVtoR4, 2, 0), // reg = v2
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let mut module = Module::new();
        let func_name = QualifiedName::global("add");
        let id = FunctionId(1);
        module.add_function(ScriptFunction {
            id,
            name: func_name.clone(),
            bytecode: ByteCode { data: bc },
            stack_size: 3,
            param_count: 2,
        });

        let mut vm = Vm::new();
        let result = vm
            .call(&module, &func_name, &[Value::I32(17), Value::I32(25)])
            .unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_native_function() {
        let mut module = Module::new();

        // Register a native function that doubles its first argument.
        let native_id = FunctionId(100);
        module.add_native_function(crate::module::NativeFunction {
            id: native_id,
            name: QualifiedName::global("double_it"),
            callback: Box::new(|ctx: &mut NativeCallContext| {
                let val = ctx.arg_i32(0);
                ctx.set_return_i32(val * 2);
                Ok(())
            }),
        });

        let mut vm = Vm::new();
        let result = vm
            .call(
                &module,
                &QualifiedName::global("double_it"),
                &[Value::I32(21)],
            )
            .unwrap();
        assert_eq!(result, Value::I32(42));
    }

    #[test]
    fn test_function_not_found() {
        let module = Module::new();
        let mut vm = Vm::new();
        let result = vm.call(&module, &QualifiedName::global("nonexistent"), &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_increment_decrement() {
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            10,
            encode_1dw(OpCode::IncVi, 0, 0), // v0++ -> 11
            encode_1dw(OpCode::IncVi, 0, 0), // v0++ -> 12
            encode_1dw(OpCode::DecVi, 0, 0), // v0-- -> 11
            encode_1dw(OpCode::CpyVtoR4, 0, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("incdec", bc, 1);
        let mut vm = Vm::new();
        let result = vm
            .call(&module, &QualifiedName::global("incdec"), &[])
            .unwrap();
        assert_eq!(result, Value::I32(11));
    }

    #[test]
    fn test_test_zero() {
        // TZ: register = 1 if value == 0
        let bc = vec![
            encode_2dw_word0(OpCode::SetV4, 0),
            0,
            encode_1dw(OpCode::CpyVtoR4, 0, 0), // reg = 0
            encode_1dw(OpCode::TZ, 0, 0),       // reg = 1 (was zero)
            encode_1dw(OpCode::CpyRtoV4, 0, 0),
            encode_1dw(OpCode::CpyVtoR4, 0, 0),
            encode_1dw(OpCode::RET, 0, 0),
        ];
        let module = single_function_module("tz", bc, 1);
        let mut vm = Vm::new();
        let result = vm.call(&module, &QualifiedName::global("tz"), &[]).unwrap();
        assert_eq!(result, Value::I32(1));
    }
}
