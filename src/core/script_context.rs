// src/api/context.rs - Thin wrapper around VM

use crate::api::function::ScriptFunction;
use crate::api::script_object::ScriptObject;
use crate::compiler::bytecode::ScriptValue;
use crate::vm::vm::VM;
use crate::vm::memory::ObjectHeap;
use std::any::Any;
use std::sync::{Arc, RwLock};
use crate::core::script_function::ScriptFunction;

pub struct ScriptContext {
    vm: VM,
    state: ExecutionState,
    prepared_function: Option<ScriptFunction>,
    exception_string: String,
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
    pub(crate) fn new(vm: VM) -> Self {
        Self {
            vm,
            state: ExecutionState::Uninitialized,
            prepared_function: None,
            exception_string: String::new(),
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

        self.prepared_function = Some(func.clone());
        self.state = ExecutionState::Prepared;
        self.vm.clear_stacks();
        self.exception_string.clear();

        // ✅ Only load bytecode for script functions
        if !func.is_system() {
            let module = func.get_module();
            let module_lock = module.read().unwrap();
            let bytecode = module_lock.bytecode.as_ref()
                                      .ok_or("Module not built")?
                .clone();

            self.vm.load_module(bytecode);
        }
        // System functions don't need bytecode loaded

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

        let func = self.prepared_function.as_ref()
                       .ok_or("No function prepared")?;

        self.state = ExecutionState::Executing;

        // ✅ Check if system or script function
        if func.is_system() {
            // ✅ System function - call directly (no bytecode execution)
            let sys_func_id = func.get_system_id()
                                  .ok_or("System function has no ID")?;

            match self.vm.execute_system_call(sys_func_id) {
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
        } else {
            // ✅ Script function - execute bytecode
            match self.vm.execute_call(func.get_id() as u32) {
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
    }

    pub fn abort(&mut self) -> Result<i32, String> {
        self.vm.abort();
        self.state = ExecutionState::Aborted;
        Ok(0)
    }

    pub fn suspend(&mut self) -> Result<i32, String> {
        self.vm.suspend();
        self.state = ExecutionState::Suspended;
        Ok(0)
    }

    pub fn get_state(&self) -> ExecutionState {
        self.state
    }

    pub fn is_nested(&self, nested_call_stack_size: &mut u32) -> bool {
        *nested_call_stack_size = self.vm.get_call_stack_size();
        *nested_call_stack_size > 1
    }

    pub fn set_arg_byte(&mut self, arg: u32, value: u8) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt8(value));
        Ok(0)
    }

    pub fn set_arg_word(&mut self, arg: u32, value: u16) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt16(value));
        Ok(0)
    }

    pub fn set_arg_dword(&mut self, arg: u32, value: u32) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt32(value));
        Ok(0)
    }

    pub fn set_arg_qword(&mut self, arg: u32, value: u64) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::UInt64(value));
        Ok(0)
    }

    pub fn set_arg_float(&mut self, arg: u32, value: f32) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Float(value));
        Ok(0)
    }

    pub fn set_arg_double(&mut self, arg: u32, value: f64) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Double(value));
        Ok(0)
    }

    pub fn set_arg_address(&mut self, arg: u32, addr: Arc<RwLock<Box<dyn Any + Send + Sync>>>) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::Dynamic(addr));
        Ok(0)
    }

    pub fn set_arg_object(&mut self, arg: u32, obj: &ScriptObject) -> Result<i32, String> {
        if self.state != ExecutionState::Prepared {
            return Err("Context not prepared".to_string());
        }
        self.vm.push_arg(ScriptValue::ObjectHandle(obj.get_handle()));
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

    pub fn get_return_object(&self, heap: Arc<RwLock<ObjectHeap>>) -> Option<ScriptObject> {
        self.vm.get_return_object(heap)
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

    pub fn get_this_pointer(&self, type_id: u32) -> Option<u64> {
        self.vm.get_object_register()
    }

    pub fn get_this_type_id(&self) -> u32 {
        0
    }

    pub fn set_exception(&mut self, description: &str) -> Result<i32, String> {
        self.exception_string = description.to_string();
        self.state = ExecutionState::Exception;
        Ok(0)
    }

    pub fn get_exception_string(&self) -> &str {
        &self.exception_string
    }

    pub fn get_call_stack_size(&self) -> u32 {
        self.vm.get_call_stack_size()
    }

    pub fn get_function(&self, stack_level: u32) -> Option<ScriptFunction> {
        if stack_level == 0 {
            self.prepared_function.clone()
        } else {
            None
        }
    }

    pub fn get_address_of_var(&mut self, var_index: u32, stack_level: u32) -> Option<&mut ScriptValue> {
        self.vm.get_address_of_var(var_index, stack_level)
    }
}
