use crate::compiler::bytecode::{BytecodeModule, Instruction};
use crate::core::type_registry::TypeRegistry;
use crate::core::types::{FunctionId, ScriptValue, TypeId, TypeRegistration};
use crate::vm::memory::*;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct VM {
    registry: Arc<RwLock<TypeRegistry>>,
    module: BytecodeModule,
    call_stack: Vec<StackFrame>,
    value_stack: Vec<ScriptValue>,
    heap: ObjectHeap,
    globals: Vec<ScriptValue>,
    object_register: Option<u64>,
    value_register: ScriptValue,
    ip: u32,
    state: VMState,
    init_list_buffer: Option<Vec<ScriptValue>>,
    system_functions: HashMap<FunctionId, SystemFunction>,
}

type SystemFunction =
    Arc<dyn Fn(&mut VM, &[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync>;

#[derive(Debug, Clone, PartialEq)]
pub enum VMState {
    Running,
    Suspended,
    Finished,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    function_id: FunctionId,
    return_address: u32,
    locals: Vec<ScriptValue>,
    frame_pointer: usize,
}

impl StackFrame {
    pub fn new(
        function_id: FunctionId,
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
    pub fn new(module: BytecodeModule, registry: Arc<RwLock<TypeRegistry>>) -> Self {
        let heap = ObjectHeap::new(Arc::clone(&registry));
        let global_count = registry.read().unwrap().get_all_globals().len();

        Self {
            registry,
            module,
            call_stack: Vec::new(),
            value_stack: Vec::new(),
            heap,
            globals: vec![ScriptValue::Void; global_count],
            object_register: None,
            value_register: ScriptValue::Void,
            ip: 0,
            state: VMState::Running,
            init_list_buffer: None,
            system_functions: HashMap::new(),
        }
    }

    pub fn load_module(&mut self, bytecode: BytecodeModule) {
        self.module = bytecode;
        self.ip = 0;
    }

    pub fn clear_stacks(&mut self) {
        self.value_stack.clear();
        self.call_stack.clear();
        self.object_register = None;
        self.value_register = ScriptValue::Void;
    }

    pub fn push_arg(&mut self, value: ScriptValue) {
        self.value_stack.push(value);
    }

    pub fn execute_function_by_id(&mut self, func_id: FunctionId) -> Result<(), String> {
        let func_info = self
            .registry
            .read()
            .unwrap()
            .get_function(func_id)
            .ok_or("Invalid function ID")?;

        let local_count = func_info.local_count;
        let param_count = func_info.parameters.len();
        let bytecode_address = func_info
            .bytecode_address
            .ok_or("Function has no bytecode address")?;

        drop(func_info);

        let mut frame = StackFrame::new(func_id, 0, local_count, 0);

        for i in (0..param_count).rev() {
            if let Some(arg) = self.value_stack.pop() {
                frame.locals[i] = arg;
            }
        }

        self.call_stack.push(frame);
        self.ip = bytecode_address;
        self.state = VMState::Running;

        while self.state == VMState::Running {
            self.execute_instruction()?;
        }

        Ok(())
    }

    pub fn call_system_function_direct(&mut self, sys_func_id: FunctionId) -> Result<(), String> {
        let mut args = Vec::new();
        while let Some(arg) = self.value_stack.pop() {
            args.push(arg);
        }
        args.reverse();

        self.execute_system_call(sys_func_id)?;

        Ok(())
    }

    pub fn get_return_byte(&self) -> u8 {
        match &self.value_register {
            ScriptValue::UInt8(v) => *v,
            ScriptValue::Int8(v) => *v as u8,
            _ => 0,
        }
    }

    pub fn get_return_word(&self) -> u16 {
        match &self.value_register {
            ScriptValue::UInt16(v) => *v,
            ScriptValue::Int16(v) => *v as u16,
            _ => 0,
        }
    }

    pub fn get_return_dword(&self) -> u32 {
        match &self.value_register {
            ScriptValue::UInt32(v) => *v,
            ScriptValue::Int32(v) => *v as u32,
            _ => 0,
        }
    }

    pub fn get_return_qword(&self) -> u64 {
        match &self.value_register {
            ScriptValue::UInt64(v) => *v,
            ScriptValue::Int64(v) => *v as u64,
            _ => 0,
        }
    }

    pub fn get_return_float(&self) -> f32 {
        match &self.value_register {
            ScriptValue::Float(v) => *v,
            _ => 0.0,
        }
    }

    pub fn get_return_double(&self) -> f64 {
        match &self.value_register {
            ScriptValue::Double(v) => *v,
            _ => 0.0,
        }
    }

    pub fn get_return_object(&self) -> Option<u64> {
        match &self.value_register {
            ScriptValue::ObjectHandle(handle) => Some(*handle),
            _ => None,
        }
    }

    pub fn get_return_address(&self) -> Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>> {
        match &self.value_register {
            ScriptValue::Dynamic(d) => Some(d.clone()),
            _ => None,
        }
    }

    pub fn set_object_register(&mut self, handle: Option<u64>) {
        self.object_register = handle;
    }

    pub fn get_object_register(&self) -> Option<u64> {
        self.object_register
    }

    pub fn get_call_stack_size(&self) -> u32 {
        self.call_stack.len() as u32
    }

    pub fn get_address_of_var(
        &mut self,
        var_index: u32,
        stack_level: u32,
    ) -> Option<&mut ScriptValue> {
        if stack_level >= self.call_stack.len() as u32 {
            return None;
        }

        let frame_idx = self.call_stack.len() - 1 - stack_level as usize;
        self.call_stack
            .get_mut(frame_idx)
            .and_then(|frame| frame.locals.get_mut(var_index as usize))
    }

    fn execute_instruction(&mut self) -> Result<(), String> {
        if self.ip as usize >= self.module.instructions.len() {
            return Err("Instruction pointer out of bounds".to_string());
        }

        let instruction = self.module.instructions[self.ip as usize].clone();
        self.ip += 1;

        match instruction {
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

            Instruction::ChkRef { var } => {
                let handle = self
                    .current_frame()
                    .get_local(var)
                    .as_object_handle()
                    .ok_or("ChkRef: variable is not an object handle")?;

                if handle == 0 {
                    return Err("Null reference".to_string());
                }

                if self.heap.get_object(handle).is_none() {
                    return Err("Invalid object reference".to_string());
                }
            }

            Instruction::ChkRefS => {
                let value = self
                    .value_stack
                    .last()
                    .ok_or("ChkRefS: value stack is empty")?;

                let handle = value
                    .as_object_handle()
                    .ok_or("ChkRefS: top of stack is not an object handle")?;

                if handle == 0 {
                    return Err("Null reference on stack".to_string());
                }

                if self.heap.get_object(handle).is_none() {
                    return Err("Invalid object reference on stack".to_string());
                }
            }

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
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("NEGd: not a double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(-value));
            }

            Instruction::NEGi64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("NEGi64: not an int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(-value));
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

            Instruction::DIVu { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("DIVu: operand a not uint32")? as u32;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("DIVu: operand b not uint32")? as u32;
                if b_val == 0 {
                    return Err("Division by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt32(a_val / b_val));
            }

            Instruction::MODu { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("MODu: operand a not uint32")? as u32;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("MODu: operand b not uint32")? as u32;
                if b_val == 0 {
                    return Err("Modulo by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt32(a_val % b_val));
            }

            Instruction::POWu { dst, a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_i32()
                    .ok_or("POWu: operand a not uint32")? as u32;
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_i32()
                    .ok_or("POWu: operand b not uint32")? as u32;
                let result = (a_val as f64).powi(b_val as i32) as u32;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt32(result));
            }

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

            Instruction::SUBi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("SUBi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("SUBi64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val - b_val));
            }

            Instruction::MULi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("MULi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("MULi64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val * b_val));
            }

            Instruction::DIVi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("DIVi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("DIVi64: operand b not int64".to_string()),
                };
                if b_val == 0 {
                    return Err("Division by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val / b_val));
            }

            Instruction::MODi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("MODi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("MODi64: operand b not int64".to_string()),
                };
                if b_val == 0 {
                    return Err("Modulo by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val % b_val));
            }

            Instruction::POWi64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("POWi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("POWi64: operand b not int64".to_string()),
                };
                let result = (a_val as f64).powi(b_val as i32) as i64;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(result));
            }

            Instruction::DIVu64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("DIVu64: operand a not uint64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("DIVu64: operand b not uint64".to_string()),
                };
                if b_val == 0 {
                    return Err("Division by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt64(a_val / b_val));
            }

            Instruction::MODu64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("MODu64: operand a not uint64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("MODu64: operand b not uint64".to_string()),
                };
                if b_val == 0 {
                    return Err("Modulo by zero".to_string());
                }
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt64(a_val % b_val));
            }

            Instruction::POWu64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("POWu64: operand a not uint64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("POWu64: operand b not uint64".to_string()),
                };
                let result = (a_val as f64).powi(b_val as i32) as u64;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt64(result));
            }

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

            Instruction::BAND64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BAND64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BAND64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val & b_val));
            }

            Instruction::BOR64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BOR64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BOR64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val | b_val));
            }

            Instruction::BXOR64 { dst, a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BXOR64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BXOR64: operand b not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(a_val ^ b_val));
            }

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

            Instruction::BSLL64 { dst, val, shift } => {
                let val_val = match self.current_frame().get_local(val) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BSLL64: val not int64".to_string()),
                };
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSLL64: shift not int32")? as u32;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(val_val << shift_val));
            }

            Instruction::BSRL64 { dst, val, shift } => {
                let val_val = match self.current_frame().get_local(val) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("BSRL64: val not uint64".to_string()),
                };
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSRL64: shift not int32")? as u32;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::UInt64(val_val >> shift_val));
            }

            Instruction::BSRA64 { dst, val, shift } => {
                let val_val = match self.current_frame().get_local(val) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("BSRA64: val not int64".to_string()),
                };
                let shift_val = self
                    .current_frame()
                    .get_local(shift)
                    .as_i32()
                    .ok_or("BSRA64: shift not int32")?;
                self.current_frame_mut()
                    .set_local(dst, ScriptValue::Int64(val_val >> shift_val));
            }

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

            Instruction::CMPd { a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("CMPd: operand a not double".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Double(v) => *v,
                    _ => return Err("CMPd: operand b not double".to_string()),
                };
                let result = if a_val < b_val {
                    -1
                } else if a_val > b_val {
                    1
                } else {
                    0
                };
                self.value_register = ScriptValue::Int32(result);
            }

            Instruction::CMPi64 { a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("CMPi64: operand a not int64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::Int64(v) => *v,
                    _ => return Err("CMPi64: operand b not int64".to_string()),
                };
                self.value_register = ScriptValue::Int32(a_val.cmp(&b_val) as i32);
            }

            Instruction::CMPu64 { a, b } => {
                let a_val = match self.current_frame().get_local(a) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("CMPu64: operand a not uint64".to_string()),
                };
                let b_val = match self.current_frame().get_local(b) {
                    ScriptValue::UInt64(v) => *v,
                    _ => return Err("CMPu64: operand b not uint64".to_string()),
                };
                self.value_register = ScriptValue::Int32(a_val.cmp(&b_val) as i32);
            }

            Instruction::CmpPtr { a, b } => {
                let a_val = self
                    .current_frame()
                    .get_local(a)
                    .as_object_handle()
                    .unwrap_or(0);
                let b_val = self
                    .current_frame()
                    .get_local(b)
                    .as_object_handle()
                    .unwrap_or(0);
                self.value_register = ScriptValue::Int32(a_val.cmp(&b_val) as i32);
            }

            Instruction::CMPIi { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("CMPIi: variable not int32")?;
                self.value_register = ScriptValue::Int32(value.cmp(&imm) as i32);
            }

            Instruction::CMPIu { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("CMPIu: variable not uint32")? as u32;
                self.value_register = ScriptValue::Int32(value.cmp(&imm) as i32);
            }

            Instruction::CMPIf { var, imm } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("CMPIf: variable not float")?;
                let result = if value < imm {
                    -1
                } else if value > imm {
                    1
                } else {
                    0
                };
                self.value_register = ScriptValue::Int32(result);
            }

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

            Instruction::iTOb { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("iTOb: variable not int32")? as i8;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int8(value));
            }

            Instruction::iTOw { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("iTOw: variable not int32")? as i16;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int16(value));
            }

            Instruction::sbTOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int8(v) => *v as i32,
                    _ => return Err("sbTOi: variable not int8".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::swTOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int16(v) => *v as i32,
                    _ => return Err("swTOi: variable not int16".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::ubTOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::UInt8(v) => *v as i32,
                    _ => return Err("ubTOi: variable not uint8".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::uwTOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::UInt16(v) => *v as i32,
                    _ => return Err("uwTOi: variable not uint16".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

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

            Instruction::uTOf { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("uTOf: variable not uint32")? as u32 as f32;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::fTOu { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("fTOu: variable not float")? as u32;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::UInt32(value));
            }

            Instruction::dTOi64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v as i64,
                    _ => return Err("dTOi64: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::dTOu64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v as u64,
                    _ => return Err("dTOu64: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::UInt64(value));
            }

            Instruction::i64TOd { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => *v as f64,
                    _ => return Err("i64TOd: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::u64TOd { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::UInt64(v) => *v as f64,
                    _ => return Err("u64TOd: variable not uint64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::dTOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v as i32,
                    _ => return Err("dTOi: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::dTOu { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v as u32,
                    _ => return Err("dTOu: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::UInt32(value));
            }

            Instruction::dTOf { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => *v as f32,
                    _ => return Err("dTOf: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::iTOd { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("iTOd: variable not int32")? as f64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::uTOd { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("uTOd: variable not uint32")? as u32 as f64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::fTOd { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("fTOd: variable not float")? as f64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::i64TOi { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => *v as i32,
                    _ => return Err("i64TOi: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int32(value));
            }

            Instruction::i64TOf { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => *v as f32,
                    _ => return Err("i64TOf: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::u64TOf { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::UInt64(v) => *v as f32,
                    _ => return Err("u64TOf: variable not uint64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::uTOi64 { var } => {
                let value =
                    self.current_frame()
                        .get_local(var)
                        .as_i32()
                        .ok_or("uTOi64: variable not uint32")? as u32 as i64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::iTOi64 { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_i32()
                    .ok_or("iTOi64: variable not int32")? as i64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::fTOi64 { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("fTOi64: variable not float")? as i64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::fTOu64 { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("fTOu64: variable not float")? as u64;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::UInt64(value));
            }

            Instruction::INCi8 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int8(v) => v.wrapping_add(1),
                    _ => return Err("INCi8: variable not int8".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int8(value));
            }

            Instruction::DECi8 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int8(v) => v.wrapping_sub(1),
                    _ => return Err("DECi8: variable not int8".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int8(value));
            }

            Instruction::INCi16 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int16(v) => v.wrapping_add(1),
                    _ => return Err("INCi16: variable not int16".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int16(value));
            }

            Instruction::DECi16 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int16(v) => v.wrapping_sub(1),
                    _ => return Err("DECi16: variable not int16".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int16(value));
            }

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

            Instruction::INCi64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => v.wrapping_add(1),
                    _ => return Err("INCi64: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::DECi64 { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Int64(v) => v.wrapping_sub(1),
                    _ => return Err("DECi64: variable not int64".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Int64(value));
            }

            Instruction::INCf { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("INCf: variable not float")?
                    + 1.0;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::DECf { var } => {
                let value = self
                    .current_frame()
                    .get_local(var)
                    .as_f32()
                    .ok_or("DECf: variable not float")?
                    - 1.0;
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Float(value));
            }

            Instruction::INCd { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => v + 1.0,
                    _ => return Err("INCd: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::DECd { var } => {
                let value = match self.current_frame().get_local(var) {
                    ScriptValue::Double(v) => v - 1.0,
                    _ => return Err("DECd: variable not double".to_string()),
                };
                self.current_frame_mut()
                    .set_local(var, ScriptValue::Double(value));
            }

            Instruction::CALL { func_id } => {
                self.execute_call(func_id)?;
            }

            Instruction::CALLINTF { func_id } => {
                self.execute_call(func_id)?;
            }

            Instruction::CALLSYS { sys_func_id } => {
                self.execute_system_call(sys_func_id)?;
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

            Instruction::JS { offset } => {
                let is_negative = match &self.value_register {
                    ScriptValue::Int32(v) => *v < 0,
                    _ => false,
                };
                if is_negative {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            Instruction::JNS { offset } => {
                let is_not_negative = match &self.value_register {
                    ScriptValue::Int32(v) => *v >= 0,
                    _ => true,
                };
                if is_not_negative {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            Instruction::JP { offset } => {
                let is_positive = match &self.value_register {
                    ScriptValue::Int32(v) => *v > 0,
                    _ => false,
                };
                if is_positive {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            Instruction::JNP { offset } => {
                let is_not_positive = match &self.value_register {
                    ScriptValue::Int32(v) => *v <= 0,
                    _ => true,
                };
                if is_not_positive {
                    self.ip = (self.ip as i32 + offset) as u32;
                }
            }

            Instruction::JMPP { offset } => {
                self.ip = offset;
            }

            Instruction::SUSPEND => {
                self.state = VMState::Suspended;
            }

            Instruction::Halt => {
                self.state = VMState::Finished;
            }

            Instruction::SetV { var, value } => {
                self.current_frame_mut().set_local(var, value);
            }

            Instruction::CpyV { dst, src } => {
                let value = self.current_frame().get_local(src).clone();
                self.current_frame_mut().set_local(dst, value);
            }

            Instruction::COPY { dst, src } => {
                self.execute_copy(dst, src)?;
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

            Instruction::Str { str_id } => {
                let string = self
                    .module
                    .get_string(str_id)
                    .ok_or("Invalid string ID")?
                    .to_string();
                self.value_register = ScriptValue::String(string);
            }

            Instruction::BeginInitList => {
                self.init_list_buffer = Some(Vec::new());
            }

            Instruction::AddToInitList => {
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

            Instruction::Nop => {}
        }

        Ok(())
    }

    fn current_frame(&self) -> &StackFrame {
        self.call_stack.last().expect("No stack frame")
    }

    fn current_frame_mut(&mut self) -> &mut StackFrame {
        self.call_stack.last_mut().expect("No stack frame")
    }

    fn execute_alloc(&mut self, type_id: TypeId, func_id: FunctionId) -> Result<(), String> {
        let type_registry = self.registry.read().unwrap();
        let type_info = type_registry
            .get_type(type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let is_rust_type = type_info.registration == TypeRegistration::Application;
        drop(type_registry);

        if is_rust_type && func_id != 0 {
            self.execute_system_call(func_id)?;

            if let ScriptValue::ObjectHandle(handle) = self.value_register {
                self.object_register = Some(handle);
            } else {
                return Err("Factory behaviour must return object handle".to_string());
            }
        } else {
            let handle = self.heap.allocate_script(type_id)?;
            self.object_register = Some(handle);

            if func_id != 0 {
                self.execute_call(func_id)?;
            }
        }

        Ok(())
    }

    fn execute_free(&mut self, var: u32, func_id: FunctionId) -> Result<(), String> {
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

    fn execute_copy(&mut self, dst: u32, src: u32) -> Result<(), String> {
        let value = self.current_frame().get_local(src).clone();

        if let ScriptValue::ObjectHandle(src_handle) = value {
            let (type_id, is_rust_type, is_value_type, has_op_assign) = {
                if let Some(src_object) = self.heap.get_object(src_handle) {
                    let type_registry = self.registry.read().unwrap();
                    if let Some(type_info) = type_registry.get_type(src_object.type_id()) {
                        let type_id = src_object.type_id();
                        let is_rust = type_info.registration == TypeRegistration::Application;
                        let is_value = type_info.is_value_type();
                        let has_assign = type_info.rust_methods.contains_key("opAssign");

                        (type_id, is_rust, is_value, has_assign)
                    } else {
                        (0, false, false, false)
                    }
                } else {
                    return Err("COPY: invalid source handle".to_string());
                }
            };

            if is_rust_type && has_op_assign {
                let dst_handle = self
                    .current_frame()
                    .get_local(dst)
                    .as_object_handle()
                    .ok_or("COPY: destination not an object handle")?;

                let op_assign_fn = {
                    let type_registry = self.registry.read().unwrap();
                    let type_info = type_registry.get_type(type_id).unwrap();
                    type_info
                        .rust_methods
                        .get("opAssign")
                        .unwrap()
                        .function
                        .clone()
                };

                if let Some(dst_object) = self.heap.get_object_mut(dst_handle) {
                    let args = vec![ScriptValue::ObjectHandle(src_handle)];
                    op_assign_fn(dst_object, &args);
                }

                return Ok(());
            }

            if is_value_type {
                let properties_to_copy = {
                    if let Some(src_object) = self.heap.get_object(src_handle) {
                        src_object.properties().clone()
                    } else {
                        return Err("COPY: invalid source handle".to_string());
                    }
                };

                let new_handle = self
                    .heap
                    .allocate_script(type_id)
                    .map_err(|e| e.to_string())?;

                if let Some(new_object) = self.heap.get_object_mut(new_handle) {
                    for (prop_name, prop_value) in properties_to_copy {
                        new_object.set_property(&prop_name, prop_value);
                    }
                }

                self.current_frame_mut()
                    .set_local(dst, ScriptValue::ObjectHandle(new_handle));
                return Ok(());
            }
        }

        self.current_frame_mut().set_local(dst, value);
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
            let type_registry = self.registry.read().unwrap();
            let type_info = type_registry
                .get_type(object.type_id())
                .ok_or("GetProperty: type not found")?;

            if type_info.registration == TypeRegistration::Application {
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

        let type_registry = self.registry.read().unwrap();
        let object = self
            .heap
            .get_object(obj_handle)
            .ok_or("SetProperty: invalid object handle")?;
        let type_info = type_registry
            .get_type(object.type_id())
            .ok_or("SetProperty: type not found")?;

        if type_info.registration == TypeRegistration::Application {
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
            let type_registry = self.registry.read().unwrap();
            let type_info = type_registry
                .get_type(object.type_id())
                .ok_or("GetThisProperty: type not found")?;

            if type_info.registration == TypeRegistration::Application {
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

        let type_registry = self.registry.read().unwrap();
        let object = self
            .heap
            .get_object(this_handle)
            .ok_or("SetThisProperty: invalid object handle")?;
        let type_info = type_registry
            .get_type(object.type_id())
            .ok_or("SetThisProperty: type not found")?;

        if type_info.registration == TypeRegistration::Application {
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

    pub(crate) fn execute_call(&mut self, func_id: FunctionId) -> Result<(), String> {
        let func_address = self
            .module
            .get_function_address(func_id)
            .ok_or_else(|| format!("Function {} has no bytecode address", func_id))?;

        let func_info = self
            .registry
            .read()
            .unwrap()
            .get_function(func_id)
            .ok_or("Call: invalid function ID")?;

        let local_count = func_info.local_count;
        let param_count = func_info.parameters.len();

        drop(func_info);

        let mut frame = StackFrame::new(func_id, self.ip, local_count, self.call_stack.len());

        for i in (0..param_count).rev() {
            if let Some(arg) = self.value_stack.pop() {
                frame.locals[i] = arg;
            }
        }

        self.call_stack.push(frame);
        self.ip = func_address;

        Ok(())
    }

    pub(crate) fn execute_system_call(&mut self, sys_func_id: FunctionId) -> Result<(), String> {
        let sys_func = self
            .system_functions
            .get(&sys_func_id)
            .ok_or_else(|| format!("System function {} not registered", sys_func_id))?
            .clone();

        let mut args = Vec::new();
        while let Some(arg) = self.value_stack.pop() {
            args.push(arg);
            if args.len() > 100 {
                break;
            }
        }
        args.reverse();

        let result = sys_func(self, &args)?;

        self.value_register = result;

        Ok(())
    }

    fn can_cast(&self, from_type: TypeId, to_type: TypeId) -> bool {
        from_type == to_type
    }

    pub fn collect_garbage(&mut self) {
        let mut roots = Vec::new();

        for global in &self.globals {
            if let ScriptValue::ObjectHandle(handle) = global {
                roots.push(*handle);
            }
        }

        for frame in &self.call_stack {
            for local in &frame.locals {
                if let ScriptValue::ObjectHandle(handle) = local {
                    roots.push(*handle);
                }
            }
        }

        for value in &self.value_stack {
            if let ScriptValue::ObjectHandle(handle) = value {
                roots.push(*handle);
            }
        }

        if let Some(handle) = self.object_register {
            roots.push(handle);
        }
        if let ScriptValue::ObjectHandle(handle) = &self.value_register {
            roots.push(*handle);
        }

        self.heap.collect_garbage(&roots);
    }

    pub fn maybe_collect_garbage(&mut self) {
        if self.heap.object_count() > 1000 {
            self.collect_garbage();
        }
    }

    pub fn print_stats(&self) {
        println!("VM Statistics:");
        println!("  Objects allocated: {}", self.heap.object_count());
        println!("  Call stack depth: {}", self.call_stack.len());
        println!("  Value stack size: {}", self.value_stack.len());
    }
}
