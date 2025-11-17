use crate::compiler::bytecode::BytecodeModule;
use crate::core::script_function::ScriptFunction;
use crate::core::script_object::ScriptObject;
use crate::core::type_registry::TypeRegistry;
use crate::core::types::{FunctionId, ScriptValue};
use crate::vm::memory::ObjectHeap;
use crate::vm::vm::VM;
use std::any::Any;
use std::sync::{Arc, RwLock};

/// asIScriptContext - Execution context for running scripts
pub struct ScriptContext {
    vm: VM,
    state: ExecutionState,
    prepared_function: Option<FunctionId>,
    exception_string: String,
    heap: Arc<RwLock<ObjectHeap>>,
    registry: Arc<RwLock<TypeRegistry>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    Uninitialized,
    Prepared,
    Executing,
    Finished,
    Aborted,
    Exception,
    Suspended,
}

impl ScriptContext {
    pub(crate) fn new(registry: Arc<RwLock<TypeRegistry>>, bytecode: BytecodeModule) -> Self {
        let heap = Arc::new(RwLock::new(ObjectHeap::new(Arc::clone(&registry))));
        let vm = VM::new(bytecode, Arc::clone(&registry));

        Self {
            vm,
            state: ExecutionState::Uninitialized,
            prepared_function: None,
            exception_string: String::new(),
            heap,
            registry,
        }
    }

    pub fn add_ref(&self) -> i32 {
        1
    }

    pub fn release(&self) -> i32 {
        0
    }

    pub fn prepare(&mut self, func: &ScriptFunction) -> Result<i32, String> {
        if self.state == ExecutionState::Executing || self.state == ExecutionState::Suspended {
            return Err("Context is active".to_string());
        }

        self.prepared_function = Some(func.get_id() as FunctionId);
        self.state = ExecutionState::Prepared;
        self.vm.clear_stacks();
        self.exception_string.clear();

        Ok(0)
    }

    pub fn unprepare(&mut self) -> Result<i32, String> {
        if self.state == ExecutionState::Executing || self.state == ExecutionState::Suspended {
            return Err("Context is active".to_string());
        }

        self.prepared_function = None;
        self.state = ExecutionState::Uninitialized;
        self.vm.clear_stacks();
        Ok(0)
    }

    pub fn execute(&mut self) -> Result<ExecutionState, String> {
        if self.state != ExecutionState::Prepared && self.state != ExecutionState::Suspended {
            return Err("Context not prepared".to_string());
        }

        let func_id = self.prepared_function.ok_or("No function prepared")?;

        self.state = ExecutionState::Executing;

        match self.vm.execute_function_by_id(func_id) {
            Ok(_) => {
                self.state = ExecutionState::Finished;
                Ok(ExecutionState::Finished)
            }
            Err(e) => {
                self.exception_string = e.clone();
                self.state = ExecutionState::Exception;
                Err(e)
            }
        }
    }

    pub fn abort(&mut self) -> Result<i32, String> {
        self.state = ExecutionState::Aborted;
        Ok(0)
    }

    pub fn suspend(&mut self) -> Result<i32, String> {
        self.state = ExecutionState::Suspended;
        Ok(0)
    }

    pub fn get_state(&self) -> ExecutionState {
        self.state
    }

    pub fn push_state(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn pop_state(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn is_nested(&self, nested_call_stack_size: &mut u32) -> bool {
        *nested_call_stack_size = self.vm.get_call_stack_size();
        *nested_call_stack_size > 1
    }

    pub fn set_arg_byte(&mut self, _arg: u32, value: u8) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt8(value));
        Ok(0)
    }

    pub fn set_arg_word(&mut self, _arg: u32, value: u16) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt16(value));
        Ok(0)
    }

    pub fn set_arg_dword(&mut self, _arg: u32, value: u32) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt32(value));
        Ok(0)
    }

    pub fn set_arg_qword(&mut self, _arg: u32, value: u64) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt64(value));
        Ok(0)
    }

    pub fn set_arg_float(&mut self, _arg: u32, value: f32) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Float(value));
        Ok(0)
    }

    pub fn set_arg_double(&mut self, _arg: u32, value: f64) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Double(value));
        Ok(0)
    }

    pub fn set_arg_address(
        &mut self,
        _arg: u32,
        addr: Arc<RwLock<Box<dyn Any + Send + Sync>>>,
    ) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Dynamic(addr));
        Ok(0)
    }

    pub fn set_arg_object(&mut self, _arg: u32, obj: &ScriptObject) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm
            .push_arg(ScriptValue::ObjectHandle(obj.get_handle()));
        Ok(0)
    }

    pub fn get_return_byte(&self) -> u8 {
        self.vm.get_return_byte()
    }

    pub fn get_return_word(&self) -> u16 {
        self.vm.get_return_word()
    }

    pub fn get_return_dword(&self) -> u32 {
        self.vm.get_return_dword()
    }

    pub fn get_return_qword(&self) -> u64 {
        self.vm.get_return_qword()
    }

    pub fn get_return_float(&self) -> f32 {
        self.vm.get_return_float()
    }

    pub fn get_return_double(&self) -> f64 {
        self.vm.get_return_double()
    }

    pub fn get_return_object(&self) -> Option<ScriptObject> {
        self.vm.get_return_object().map(|handle| {
            ScriptObject::new(handle, Arc::clone(&self.heap), Arc::clone(&self.registry))
        })
    }

    pub fn get_return_address(&self) -> Option<Arc<RwLock<Box<dyn Any + Send + Sync>>>> {
        self.vm.get_return_address()
    }

    pub fn set_object(&mut self, obj: &ScriptObject) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.set_object_register(Some(obj.get_handle()));
        Ok(0)
    }

    pub fn get_this_pointer(&self, _type_id: u32) -> Option<ScriptObject> {
        self.vm.get_object_register().map(|handle| {
            ScriptObject::new(handle, Arc::clone(&self.heap), Arc::clone(&self.registry))
        })
    }

    pub fn get_this_type_id(&self) -> i32 {
        if let Some(handle) = self.vm.get_object_register() {
            let heap = self.heap.read().unwrap();
            heap.get_object(handle)
                .map(|obj| obj.type_id() as i32)
                .unwrap_or(0)
        } else {
            0
        }
    }

    pub fn set_exception(&mut self, description: &str) -> Result<i32, String> {
        self.exception_string = description.to_string();
        self.state = ExecutionState::Exception;
        Ok(0)
    }

    pub fn get_exception_string(&self) -> &str {
        &self.exception_string
    }

    pub fn clear_exception_callback(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn set_line_callback(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn clear_line_callback(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn get_call_stack_size(&self) -> u32 {
        self.vm.get_call_stack_size()
    }

    pub fn get_function(&self, stack_level: u32) -> Option<ScriptFunction> {
        if stack_level == 0 {
            self.prepared_function
                .map(|func_id| ScriptFunction::new(func_id, Arc::clone(&self.registry)))
        } else {
            None
        }
    }

    pub fn get_line_number(&self, _stack_level: u32, _column: &mut i32) -> i32 {
        0
    }

    pub fn get_var_count(&self, _stack_level: u32) -> u32 {
        0
    }

    pub fn get_var(
        &self,
        _var_index: u32,
        _stack_level: u32,
        _name: &mut Option<String>,
        _type_id: &mut i32,
    ) -> Result<i32, String> {
        Err("Not implemented".to_string())
    }

    pub fn get_var_decl(
        &self,
        _var_index: u32,
        _stack_level: u32,
        _include_namespace: bool,
    ) -> Option<String> {
        None
    }

    pub fn get_address_of_var(
        &mut self,
        var_index: u32,
        stack_level: u32,
    ) -> Option<&mut ScriptValue> {
        self.vm.get_address_of_var(var_index, stack_level)
    }

    pub fn get_this_pointer_at_stack_level(&self, _stack_level: u32) -> Option<ScriptObject> {
        None
    }
}
