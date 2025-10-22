use crate::compiler::bytecode::{BytecodeModule, Instruction, ScriptValue};
use crate::vm::memory::*;
use std::sync::{Arc, RwLock};

/// Virtual Machine for executing AngelScript bytecode
pub struct VM {
    /// Bytecode module being executed
    module: BytecodeModule,

    /// Call stack (function call frames)
    call_stack: Vec<StackFrame>,

    /// Value stack (for passing arguments and temporary values)
    value_stack: Vec<ScriptValue>,

    /// Object heap (all allocated objects)
    heap: ObjectHeap,

    /// Global variables
    globals: Vec<ScriptValue>,

    /// Type registry
    type_registry: Arc<RwLock<TypeRegistry>>,

    /// Object register (for 'this' pointer and temp objects)
    object_register: Option<u64>,

    /// Value register (for comparisons and temp values)
    value_register: ScriptValue,

    /// Instruction pointer
    ip: u32,

    /// Execution state
    state: VMState,

    /// Init list buffer (for building initialization lists)
    init_list_buffer: Option<Vec<ScriptValue>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VMState {
    Running,
    Suspended,
    Finished,
    Error(String),
}

/// Stack frame for a function call
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Function being executed
    function_id: u32,

    /// Return address (instruction pointer to return to)
    return_address: u32,

    /// Local variables (indexed by var number)
    locals: Vec<ScriptValue>,

    /// Frame pointer (index in call stack)
    frame_pointer: usize,
}

impl StackFrame {
    pub fn new(
        function_id: u32,
        return_address: u32,
        local_count: u32,
        frame_pointer: usize,
    ) -> Self {
        Self {
            function_id,
            return_address,
            locals: vec![ScriptValue::Void; local_count as usize],
            frame_pointer,
        }
    }

    pub fn get_local(&self, index: u32) -> &ScriptValue {
        &self.locals[index as usize]
    }

    pub fn set_local(&mut self, index: u32, value: ScriptValue) {
        self.locals[index as usize] = value;
    }
}

impl VM {
    /// Create a new VM with a bytecode module
    pub fn new(module: BytecodeModule, type_registry: Arc<RwLock<TypeRegistry>>) -> Self {
        let heap = ObjectHeap::new(type_registry.clone());
        let global_count = module.globals.len();

        Self {
            module,
            call_stack: Vec::new(),
            value_stack: Vec::new(),
            heap,
            globals: vec![ScriptValue::Void; global_count],
            type_registry,
            object_register: None,
            value_register: ScriptValue::Void,
            ip: 0,
            state: VMState::Running,
            init_list_buffer: None,
        }
    }

    /// Execute a function by name
    pub fn execute_function(&mut self, name: &str) -> Result<ScriptValue, String> {
        let func = self
            .module
            .find_function(name)
            .ok_or_else(|| format!("Function '{}' not found", name))?;

        // Create initial stack frame
        let frame = StackFrame::new(
            0, // function_id (we'd need to track this)
            0, // return address (no return for entry point)
            func.local_count,
            0, // frame pointer
        );

        self.call_stack.push(frame);
        self.ip = func.address;
        self.state = VMState::Running;

        // Execute until finished
        while self.state == VMState::Running {
            self.execute_instruction()?;
        }

        match &self.state {
            VMState::Finished => Ok(self.value_register.clone()),
            VMState::Error(msg) => Err(msg.clone()),
            _ => Err("Unexpected VM state".to_string()),
        }
    }

    /// Execute a single instruction
    fn execute_instruction(&mut self) -> Result<(), String> {
        if self.ip as usize >= self.module.instructions.len() {
            return Err("Instruction pointer out of bounds".to_string());
        }

        let instruction = self.module.instructions[self.ip as usize].clone();
        self.ip += 1;

        match instruction {
            // ==================== OBJECT MANAGEMENT ====================
            Instruction::Alloc { type_id, func_id } => {
                self.execute_alloc(type_id, func_id)?;
            }

            Instruction::Free { var, func_id } => {
                self.execute_free(var, func_id)?;
            }

            Instruction::LoadObj { var } => {
                let value = self.current_frame().get_local(var).clone();
                if let ScriptValue::ObjectHandle(handle) = value {
                    self.object_register = Some(handle);
                } else {
                    return Err("LoadObj: variable is not an object handle".to_string());
                }
            }

            Instruction::StoreObj { var } => {
                let handle = self
                    .object_register
                    .ok_or("StoreObj: no object in register")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::ObjectHandle(handle));
                self.object_register = None;
            }

            Instruction::RefCpy { dst, src } => {
                let value = self.current_frame().get_local(src).clone();
                if let ScriptValue::ObjectHandle(handle) = value {
                    self.heap.add_ref(handle);
                }
                self.current_frame_mut().set_local(dst, value);
            }

            Instruction::TypeId { type_id } => {
                self.value_stack.push(ScriptValue::UInt32(type_id));
            }

            Instruction::Cast { type_id } => {
                let value = self.value_stack.pop().ok_or("Cast: value stack empty")?;

                if let ScriptValue::ObjectHandle(handle) = value {
                    let object = self
                        .heap
                        .get_object(handle)
                        .ok_or("Cast: invalid object handle")?;

                    if self.can_cast(object.type_id(), type_id) {
                        self.object_register = Some(handle);
                    } else {
                        self.object_register = None;
                    }
                } else {
                    self.object_register = None;
                }
            }

            Instruction::FuncPtr { func_id } => {
                self.value_stack.push(ScriptValue::UInt32(func_id));
            }

            // ==================== PROPERTY ACCESS (HASHMAP-BASED) ====================
            Instruction::GetProperty {
                obj_var,
                prop_name_id,
                dst_var,
            } => {
                self.execute_get_property(obj_var, prop_name_id, dst_var)?;
            }

            Instruction::SetProperty {
                obj_var,
                prop_name_id,
                src_var,
            } => {
                self.execute_set_property(obj_var, prop_name_id, src_var)?;
            }

            Instruction::GetThisProperty {
                prop_name_id,
                dst_var,
            } => {
                self.execute_get_this_property(prop_name_id, dst_var)?;
            }

            Instruction::SetThisProperty {
                prop_name_id,
                src_var,
            } => {
                self.execute_set_this_property(prop_name_id, src_var)?;
            }

            // ==================== MATH INSTRUCTIONS ====================
            Instruction::NEGi { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("NEGi: not an int32")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(-value));
            }

            Instruction::NEGf { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("NEGf: not a float")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(-value));
            }

            Instruction::NEGd { var } => {
                let frame = self.current_frame_mut();
                if let ScriptValue::Double(value) = frame.get_local(var) {
                    frame.set_local(var, ScriptValue::Double(-value));
                } else {
                    return Err("NEGd: not a double".to_string());
                }
            }

            Instruction::NEGi64 { var } => {
                let frame = self.current_frame_mut();
                if let ScriptValue::Int64(value) = frame.get_local(var) {
                    frame.set_local(var, ScriptValue::Int64(-value));
                } else {
                    return Err("NEGi64: not an int64".to_string());
                }
            }

            Instruction::ADDi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("ADDi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("ADDi: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val + b_val));
            }

            Instruction::SUBi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("SUBi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("SUBi: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val - b_val));
            }

            Instruction::MULi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("MULi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("MULi: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val * b_val));
            }

            Instruction::DIVi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("DIVi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("DIVi: operand b not int32")?;
                if b_val == 0 {
                    return Err("Division by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val / b_val));
            }

            Instruction::MODi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("MODi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("MODi: operand b not int32")?;
                if b_val == 0 {
                    return Err("Modulo by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val % b_val));
            }

            Instruction::POWi { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("POWi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("POWi: operand b not int32")?;
                let result = (a_val as f64).powi(b_val);
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(result as i32));
            }

            // Float operations
            Instruction::ADDf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("ADDf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("ADDf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val + b_val));
            }

            Instruction::SUBf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("SUBf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("SUBf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val - b_val));
            }

            Instruction::MULf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("MULf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("MULf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val * b_val));
            }

            Instruction::DIVf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("DIVf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("DIVf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val / b_val));
            }

            Instruction::MODf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("MODf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("MODf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val % b_val));
            }

            Instruction::POWf { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("POWf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("POWf: operand b not float")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Float(a_val.powf(b_val)));
            }

            // Double operations
            Instruction::ADDd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("ADDd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("ADDd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val + b_val));
            }

            Instruction::SUBd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("SUBd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("SUBd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val - b_val));
            }

            Instruction::MULd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("MULd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("MULd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val * b_val));
            }

            Instruction::DIVd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("DIVd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("DIVd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val / b_val));
            }

            Instruction::MODd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("MODd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("MODd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val % b_val));
            }

            Instruction::POWd { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("POWd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("POWd: operand b not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val.powf(b_val)));
            }

            Instruction::POWdi { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("POWdi: operand a not double".to_string()),
                };
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("POWdi: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Double(a_val.powi(b_val)));
            }

            // Int64 operations (similar pattern - abbreviated for space)
            Instruction::ADDi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("ADDi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("ADDi64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val + b_val));
            }

            // ... (other int64 operations follow same pattern)

            // Math with immediate values
            Instruction::ADDIi { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("ADDIi: variable not int32")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value + imm));
            }

            Instruction::SUBIi { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("SUBIi: variable not int32")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value - imm));
            }

            Instruction::MULIi { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("MULIi: variable not int32")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value * imm));
            }

            Instruction::ADDIf { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("ADDIf: variable not float")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value + imm));
            }

            Instruction::SUBIf { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("SUBIf: variable not float")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value - imm));
            }

            Instruction::MULIf { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("MULIf: variable not float")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value * imm));
            }

            // ==================== BITWISE INSTRUCTIONS ====================
            Instruction::NOT { var } => {
                let value = self.current_frame().get_local(var).is_truthy();
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Bool(!value));
            }

            Instruction::BNOT { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("BNOT: variable not int32")?;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(!value));
            }

            Instruction::BNOT64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BNOT64: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(!value));
            }

            Instruction::BAND { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("BAND: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("BAND: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val & b_val));
            }

            Instruction::BOR { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("BOR: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("BOR: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val | b_val));
            }

            Instruction::BXOR { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("BXOR: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("BXOR: operand b not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(a_val ^ b_val));
            }

            // ... (other bitwise operations follow same pattern)
            Instruction::BSLL { dst, val, shift } => {
                let val_val = self
                    .current_frame()
                    .get_local(val)
                    .as_i32()
                    .ok_or("BSLL: val not int32")?;
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSLL: shift not int32")? as u32;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(val_val << shift_val));
            }

            Instruction::BSRL { dst, val, shift } => {
                let val_val = self
                    .current_frame()
                    .get_local(val)
                    .as_i32()
                    .ok_or("BSRL: val not int32")? as u32;
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSRL: shift not int32")? as u32;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt32(val_val >> shift_val));
            }

            Instruction::BSRA { dst, val, shift } => {
                let val_val = self
                    .current_frame()
                    .get_local(val)
                    .as_i32()
                    .ok_or("BSRA: val not int32")?;
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSRA: shift not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int32(val_val >> shift_val));
            }

            // ... (64-bit shifts follow same pattern)

            // ==================== COMPARISON INSTRUCTIONS ====================
            Instruction::CMPi { a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("CMPi: operand a not int32")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("CMPi: operand b not int32")?;
                self.value_register = ScriptValue::Int32(a_val.cmp(&b_val) as i32);
            }

            Instruction::CMPu { a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("CMPu: operand a not uint32")? as u32;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("CMPu: operand b not uint32")? as u32;
                self.value_register = ScriptValue::Int32(a_val.cmp(&b_val) as i32);
            }

            Instruction::CMPf { a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_f32()
                    .ok_or("CMPf: operand a not float")?;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_f32()
                    .ok_or("CMPf: operand b not float")?;
                let result = if a_val < b_val {
                    -1
                } else if a_val > b_val {
                    1
                } else {
                    0
                };
                self.value_register = ScriptValue::Int32(result);
            }

            // ... (other comparison operations)
            Instruction::CMPIi { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("CMPIi: variable not int32")?;
                self.value_register = ScriptValue::Int32(value.cmp(&imm) as i32);
            }

            // ==================== TEST INSTRUCTIONS ====================
            Instruction::TZ => {
                let is_zero = match &self.value_register {
                    ScriptValue::Int32(v) => *v == 0,
                    _ => false,
                };
                self.value_register = ScriptValue::Bool(is_zero);
            }

            Instruction::TNZ => {
                let is_not_zero = match &self.value_register {
                    ScriptValue::Int32(v) => *v != 0,
                    _ => true,
                };
                self.value_register = ScriptValue::Bool(is_not_zero);
            }

            Instruction::TS => {
                let is_negative = match &self.value_register {
                    ScriptValue::Int32(v) => *v < 0,
                    _ => false,
                };
                self.value_register = ScriptValue::Bool(is_negative);
            }

            Instruction::TNS => {
                let is_not_negative = match &self.value_register {
                    ScriptValue::Int32(v) => *v >= 0,
                    _ => true,
                };
                self.value_register = ScriptValue::Bool(is_not_negative);
            }

            Instruction::TP => {
                let is_positive = match &self.value_register {
                    ScriptValue::Int32(v) => *v > 0,
                    _ => false,
                };
                self.value_register = ScriptValue::Bool(is_positive);
            }

            Instruction::TNP => {
                let is_not_positive = match &self.value_register {
                    ScriptValue::Int32(v) => *v <= 0,
                    _ => true,
                };
                self.value_register = ScriptValue::Bool(is_not_positive);
            }

            // ==================== TYPE CONVERSION INSTRUCTIONS ====================
            // (Abbreviated - full implementation in complete file)
            Instruction::iTOf { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("iTOf: variable not int32")? as f32;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::fTOi { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("fTOi: variable not float")? as i32;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            // ... (other conversions follow same pattern)

            // ==================== INCREMENT/DECREMENT INSTRUCTIONS ====================
            Instruction::INCi { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("INCi: variable not int32")?
                    .wrapping_add(1);
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::DECi { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("DECi: variable not int32")?
                    .wrapping_sub(1);
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            // ... (other inc/dec operations)

            // ==================== FLOW CONTROL INSTRUCTIONS ====================
            Instruction::CALL { func_id } => {
                self.execute_call(func_id)?;
            }

            Instruction::CALLINTF { func_id } => {
                self.execute_call(func_id)?;
            }

            Instruction::CallPtr => {
                let func_id = match self.value_stack.pop() {
                    Some(ScriptValue::UInt32(id)) => id,
                    _ => return Err("CallPtr: no function pointer on stack".to_string()),
                };
                self.execute_call(func_id)?;
            }

            Instruction::RET { stack_size: _ } => {
                if self.call_stack.len() <= 1 {
                    self.state = VMState::Finished;
                } else {
                    let frame = self.call_stack.pop().unwrap();
                    self.ip = frame.return_address;
                }
            }

            Instruction::JMP { offset } => {
                self.ip = (self.ip as i32 + offset) as u32;
            }

            Instruction::JZ { offset } => {
                if !self.value_register.is_truthy() {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            Instruction::JNZ { offset } => {
                if self.value_register.is_truthy() {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            // ... (other jumps)
            Instruction::CALLSYS { sys_func_id } => {
                self.execute_system_call(sys_func_id)?;
            }

            Instruction::SUSPEND => {
                self.state = VMState::Suspended;
            }

            // ==================== VARIABLE OPERATIONS (SIMPLIFIED) ====================
            Instruction::SetV { var, value } => {
                self.current_frame_mut().set_local(var, value);
            }

            Instruction::CpyV { dst, src } => {
                let value = self.current_frame().get_local(src).clone();
                self.current_frame_mut().set_local(dst, value);
            }

            Instruction::ClrV { var } => {
                self.current_frame_mut().set_local(var, ScriptValue::Null);
            }

            Instruction::CpyVtoR { var } => {
                self.value_register = self.current_frame().get_local(var).clone();
            }

            Instruction::CpyRtoV { var } => {
                let cloned_value = self.value_register.clone();
                self.current_frame_mut().set_local(var, cloned_value);
            }

            // ==================== STACK OPERATIONS (SIMPLIFIED) ====================
            Instruction::PshC { value } => {
                self.value_stack.push(value);
            }

            Instruction::PshV { var } => {
                let value = self.current_frame().get_local(var).clone();
                self.value_stack.push(value);
            }

            Instruction::PshNull => {
                self.value_stack.push(ScriptValue::Null);
            }

            Instruction::Pop => {
                self.value_stack.pop();
            }

            Instruction::PopR => {
                self.value_register = self.value_stack.pop().unwrap_or(ScriptValue::Void);
            }

            Instruction::PshR => {
                self.value_stack.push(self.value_register.clone());
            }

            Instruction::Swap => {
                let len = self.value_stack.len();
                if len >= 2 {
                    self.value_stack.swap(len - 1, len - 2);
                }
            }

            // ==================== GLOBAL VARIABLE OPERATIONS (SIMPLIFIED) ====================
            Instruction::CpyVtoG { global_id, var } => {
                let value = self.current_frame().get_local(var).clone();
                self.globals[global_id as usize] = value;
            }

            Instruction::CpyGtoV { var, global_id } => {
                let value = self.globals[global_id as usize].clone();
                self.current_frame_mut().set_local(var, value);
            }

            Instruction::SetG { global_id, value } => {
                self.globals[global_id as usize] = value;
            }

            Instruction::PshG { global_id } => {
                let value = self.globals[global_id as usize].clone();
                self.value_stack.push(value);
            }

            Instruction::LdG { global_id } => {
                self.value_register = self.globals[global_id as usize].clone();
            }

            // ==================== VALIDATION ====================
            Instruction::ChkNull { var } => {
                let value = self.current_frame().get_local(var);
                if matches!(value, ScriptValue::Null) {
                    return Err("Null pointer access".to_string());
                }
            }

            Instruction::ChkNullS => {
                if let Some(value) = self.value_stack.last() {
                    if matches!(value, ScriptValue::Null) {
                        return Err("Null pointer access".to_string());
                    }
                }
            }

            // ==================== STRING MANAGEMENT ====================
            Instruction::Str { str_id } => {
                let string = self
                    .module
                    .get_string(str_id)
                    .ok_or("Invalid string ID")?
                    .to_string();
                self.value_register = ScriptValue::String(string);
            }

            // ==================== INITIALIZATION LIST MANAGEMENT ====================
            Instruction::BeginInitList => {
                self.init_list_buffer = Some(Vec::new());
            }

            Instruction::AddToInitList => {
                // Pop value from value stack and add to init list buffer
                if let Some(value) = self.value_stack.pop() {
                    if let Some(buffer) = &mut self.init_list_buffer {
                        buffer.push(value);
                    }
                }
            }

            Instruction::EndInitList {
                element_type: _,
                count: _,
            } => {
                if let Some(buffer) = self.init_list_buffer.take() {
                    self.value_stack.push(ScriptValue::InitList(buffer));
                }
            }

            // ==================== UTILITY ====================
            Instruction::Nop => {}

            Instruction::Halt => {
                self.state = VMState::Finished;
            }

            _ => {
                return Err(format!("Unimplemented instruction: {:?}", instruction));
            }
        }

        Ok(())
    }

    // ==================== HELPER METHODS ====================

    fn current_frame(&self) -> &StackFrame {
        self.call_stack.last().expect("No stack frame")
    }

    fn current_frame_mut(&mut self) -> &mut StackFrame {
        self.call_stack.last_mut().expect("No stack frame")
    }

    fn execute_alloc(&mut self, type_id: u32, func_id: u32) -> Result<(), String> {
        let handle = self.heap.allocate_script(type_id)?;
        self.object_register = Some(handle);

        if func_id != 0 {
            self.execute_call(func_id)?;
        }

        Ok(())
    }

    fn execute_free(&mut self, var: u32, func_id: u32) -> Result<(), String> {
        let handle = self
            .current_frame()
            .get_local(var)
            .as_object_handle()
            .ok_or("Free: variable is not an object handle")?;

        if func_id != 0 {
            self.object_register = Some(handle);
            self.execute_call(func_id)?;
        }

        self.heap.release_object(handle);
        self.current_frame_mut().set_local(var, ScriptValue::Null);

        Ok(())
    }

    fn execute_get_property(
        &mut self,
        obj_var: u32,
        prop_name_id: u32,
        dst_var: u32,
    ) -> Result<(), String> {
        let obj_handle = self
            .current_frame()
            .get_local(obj_var)
            .as_object_handle()
            .ok_or("GetProperty: not an object handle")?;

        let object = self
            .heap
            .get_object(obj_handle)
            .ok_or("GetProperty: invalid object handle")?;

        let prop_name = self
            .module
            .get_property_name(prop_name_id)
            .ok_or("GetProperty: invalid property name")?;

        let value = {
            let type_registry = self.type_registry.read().unwrap();
            let type_info = type_registry
                .get_type(object.type_id())
                .ok_or("GetProperty: type not found")?;

            if type_info.flags.contains(TypeFlags::RUST_TYPE) {
                if let Some(accessor) = type_info.rust_accessors.get(prop_name) {
                    if let Some(getter) = &accessor.getter {
                        getter(object)
                    } else {
                        return Err(format!("Property '{}' has no getter", prop_name));
                    }
                } else {
                    return Err(format!("Property '{}' not found on Rust type", prop_name));
                }
            } else {
                object
                    .get_property(prop_name)
                    .ok_or(format!("Property '{}' not found", prop_name))?
                    .clone()
            }
        };

        self.current_frame_mut().set_local(dst_var, value);
        Ok(())
    }

    fn execute_set_property(
        &mut self,
        obj_var: u32,
        prop_name_id: u32,
        src_var: u32,
    ) -> Result<(), String> {
        let obj_handle = self
            .current_frame()
            .get_local(obj_var)
            .as_object_handle()
            .ok_or("SetProperty: not an object handle")?;

        let value = self.current_frame().get_local(src_var).clone();

        let prop_name = self
            .module
            .get_property_name(prop_name_id)
            .ok_or("SetProperty: invalid property name")?;

        let type_registry = self.type_registry.read().unwrap();
        let object = self
            .heap
            .get_object(obj_handle)
            .ok_or("SetProperty: invalid object handle")?;
        let type_info = type_registry
            .get_type(object.type_id())
            .ok_or("SetProperty: type not found")?;

        if type_info.flags.contains(TypeFlags::RUST_TYPE) {
            if let Some(accessor) = type_info.rust_accessors.get(prop_name) {
                if let Some(setter) = &accessor.setter {
                    drop(type_registry);
                    let object_mut = self
                        .heap
                        .get_object_mut(obj_handle)
                        .ok_or("SetProperty: invalid object handle")?;
                    setter(object_mut, value);
                } else {
                    return Err(format!("Property '{}' is read-only", prop_name));
                }
            } else {
                return Err(format!("Property '{}' not found on Rust type", prop_name));
            }
        } else {
            drop(type_registry);
            let object_mut = self
                .heap
                .get_object_mut(obj_handle)
                .ok_or("SetProperty: invalid object handle")?;
            object_mut.set_property(prop_name, value);
        }

        Ok(())
    }

    fn execute_get_this_property(&mut self, prop_name_id: u32, dst_var: u32) -> Result<(), String> {
        let this_handle = self
            .object_register
            .ok_or("GetThisProperty: no 'this' object")?;

        let object = self
            .heap
            .get_object(this_handle)
            .ok_or("GetThisProperty: invalid object handle")?;

        let prop_name = self
            .module
            .get_property_name(prop_name_id)
            .ok_or("GetThisProperty: invalid property name")?;

        let value = {
            let type_registry = self.type_registry.read().unwrap();
            let type_info = type_registry
                .get_type(object.type_id())
                .ok_or("GetThisProperty: type not found")?;

            if type_info.flags.contains(TypeFlags::RUST_TYPE) {
                if let Some(accessor) = type_info.rust_accessors.get(prop_name) {
                    if let Some(getter) = &accessor.getter {
                        getter(object)
                    } else {
                        return Err(format!("Property '{}' has no getter", prop_name));
                    }
                } else {
                    return Err(format!("Property '{}' not found on Rust type", prop_name));
                }
            } else {
                object
                    .get_property(prop_name)
                    .ok_or(format!("Property '{}' not found", prop_name))?
                    .clone()
            }
        };

        self.current_frame_mut().set_local(dst_var, value);
        Ok(())
    }

    fn execute_set_this_property(&mut self, prop_name_id: u32, src_var: u32) -> Result<(), String> {
        let this_handle = self
            .object_register
            .ok_or("SetThisProperty: no 'this' object")?;

        let value = self.current_frame().get_local(src_var).clone();

        let prop_name = self
            .module
            .get_property_name(prop_name_id)
            .ok_or("SetThisProperty: invalid property name")?;

        let type_registry = self.type_registry.read().unwrap();
        let object = self
            .heap
            .get_object(this_handle)
            .ok_or("SetThisProperty: invalid object handle")?;
        let type_info = type_registry
            .get_type(object.type_id())
            .ok_or("SetThisProperty: type not found")?;

        if type_info.flags.contains(TypeFlags::RUST_TYPE) {
            if let Some(accessor) = type_info.rust_accessors.get(prop_name) {
                if let Some(setter) = &accessor.setter {
                    drop(type_registry);
                    let object_mut = self
                        .heap
                        .get_object_mut(this_handle)
                        .ok_or("SetThisProperty: invalid object handle")?;
                    setter(object_mut, value);
                } else {
                    return Err(format!("Property '{}' is read-only", prop_name));
                }
            } else {
                return Err(format!("Property '{}' not found on Rust type", prop_name));
            }
        } else {
            drop(type_registry);
            let object_mut = self
                .heap
                .get_object_mut(this_handle)
                .ok_or("SetThisProperty: invalid object handle")?;
            object_mut.set_property(prop_name, value);
        }

        Ok(())
    }

    fn execute_call(&mut self, func_id: u32) -> Result<(), String> {
        let func = self
            .module
            .functions
            .get(func_id as usize)
            .ok_or("Call: invalid function ID")?;

        let mut frame = StackFrame::new(func_id, self.ip, func.local_count, self.call_stack.len());

        // Pop arguments from value stack into locals
        for i in (0..func.param_count).rev() {
            if let Some(arg) = self.value_stack.pop() {
                frame.locals[i as usize] = arg;
            }
        }

        self.call_stack.push(frame);
        self.ip = func.address;

        Ok(())
    }

    fn execute_system_call(&mut self, _sys_func_id: u32) -> Result<(), String> {
        Err("System calls not yet implemented".to_string())
    }

    fn can_cast(&self, from_type: u32, to_type: u32) -> bool {
        from_type == to_type
    }
}
