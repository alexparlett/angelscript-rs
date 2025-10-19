use crate::compiler::bytecode::Value;
use crate::core::module::Module;
use crate::vm::VM;

/// Script execution context
///
/// A context represents a single execution thread. It can be reused
/// for multiple function calls to avoid allocation overhead.
pub struct Context {
    /// The VM that executes bytecode
    vm: VM,

    /// Currently prepared function
    prepared_function: Option<PreparedFunction>,

    /// Execution state
    state: ContextState,
}

struct PreparedFunction {
    /// Pointer to the module (user must ensure it stays alive)
    module_ptr: *const Module,
    function_id: u32,
    function_address: u32,
    param_count: u8,
    arguments: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextState {
    /// Context is ready to prepare a new function
    Unprepared,

    /// Context has a function prepared and ready to execute
    Prepared,

    /// Context is currently executing
    Executing,

    /// Execution finished successfully
    Finished,

    /// Execution was aborted
    Aborted,

    /// An exception occurred during execution
    Exception,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionResult {
    /// Execution completed successfully
    Finished,

    /// Execution was suspended (for coroutines)
    Suspended,

    /// Execution was aborted
    Aborted,

    /// An exception occurred
    Exception,
}

impl Context {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            vm: VM::new(),
            prepared_function: None,
            state: ContextState::Unprepared,
        }
    }

    /// Prepare a function for execution
    ///
    /// # Safety
    /// The module must remain valid for the lifetime of this context
    /// or until unprepare() is called.
    pub fn prepare(&mut self, module: &Module, function_name: &str) -> Result<(), String> {
        // Check state
        if self.state == ContextState::Executing {
            return Err("Cannot prepare while executing".to_string());
        }

        if !module.is_built() {
            return Err("Module is not compiled".to_string());
        }

        let bytecode = module.bytecode.as_ref().ok_or("Module has no bytecode")?;

        let function = bytecode
            .functions
            .iter()
            .find(|f| f.name == function_name)
            .ok_or_else(|| format!("Function '{}' not found", function_name))?;

        let function_id = bytecode
            .functions
            .iter()
            .position(|f| f.name == function_name)
            .unwrap() as u32;

        // Store pointer to module
        self.prepared_function = Some(PreparedFunction {
            module_ptr: module as *const Module,
            function_id,
            function_address: function.address,
            param_count: function.param_count,
            arguments: Vec::new(),
        });

        self.state = ContextState::Prepared;

        Ok(())
    }

    /// Set an argument (generic)
    pub fn set_arg(&mut self, index: u32, value: Value) -> Result<(), String> {
        let prepared = self
            .prepared_function
            .as_mut()
            .ok_or("No function prepared")?;

        if index >= prepared.param_count as u32 {
            return Err(format!(
                "Argument index {} out of range (function has {} parameters)",
                index, prepared.param_count
            ));
        }

        // Ensure we have enough space
        while prepared.arguments.len() <= index as usize {
            prepared.arguments.push(Value::Void);
        }

        prepared.arguments[index as usize] = value;
        Ok(())
    }

    /// Set a 32-bit integer argument
    pub fn set_arg_dword(&mut self, index: u32, value: i32) -> Result<(), String> {
        self.set_arg(index, Value::Int32(value))
    }

    /// Set a 64-bit integer argument
    pub fn set_arg_qword(&mut self, index: u32, value: i64) -> Result<(), String> {
        self.set_arg(index, Value::Int64(value))
    }

    /// Set a float argument
    pub fn set_arg_float(&mut self, index: u32, value: f32) -> Result<(), String> {
        self.set_arg(index, Value::Float(value))
    }

    /// Set a double argument
    pub fn set_arg_double(&mut self, index: u32, value: f64) -> Result<(), String> {
        self.set_arg(index, Value::Double(value))
    }

    /// Set a byte argument
    pub fn set_arg_byte(&mut self, index: u32, value: u8) -> Result<(), String> {
        self.set_arg(index, Value::UInt8(value))
    }

    /// Set a word (16-bit) argument
    pub fn set_arg_word(&mut self, index: u32, value: u16) -> Result<(), String> {
        self.set_arg(index, Value::UInt16(value))
    }

    /// Set an object handle argument
    pub fn set_arg_object(&mut self, index: u32, handle: u32) -> Result<(), String> {
        self.set_arg(index, Value::Handle(handle))
    }

    /// Execute the prepared function
    pub fn execute(&mut self) -> Result<ExecutionResult, String> {
        let prepared = self
            .prepared_function
            .as_ref()
            .ok_or("No function prepared")?;

        if self.state != ContextState::Prepared {
            return Err(format!(
                "Context not in prepared state (current state: {:?})",
                self.state
            ));
        }

        self.state = ContextState::Executing;

        // Safety: User must ensure module is still valid
        let module = unsafe { &*prepared.module_ptr };
        let bytecode = module.bytecode.as_ref().ok_or("Module has no bytecode")?;

        // Execute
        let result =
            self.vm
                .execute_with_args(bytecode, prepared.function_address, &prepared.arguments);

        match result {
            Ok(_) => {
                self.state = ContextState::Finished;
                Ok(ExecutionResult::Finished)
            }
            Err(e) => {
                self.state = ContextState::Exception;
                Err(e)
            }
        }
    }

    /// Get the return value as a 32-bit integer
    pub fn get_return_dword(&self) -> Result<i32, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::Int32(v) => Ok(*v),
            Value::UInt32(v) => Ok(*v as i32),
            _ => Err("Return value is not a dword".to_string()),
        }
    }

    /// Get the return value as a 64-bit integer
    pub fn get_return_qword(&self) -> Result<i64, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::Int64(v) => Ok(*v),
            Value::UInt64(v) => Ok(*v as i64),
            _ => Err("Return value is not a qword".to_string()),
        }
    }

    /// Get the return value as a float
    pub fn get_return_float(&self) -> Result<f32, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::Float(v) => Ok(*v),
            _ => Err("Return value is not a float".to_string()),
        }
    }

    /// Get the return value as a double
    pub fn get_return_double(&self) -> Result<f64, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::Double(v) => Ok(*v),
            _ => Err("Return value is not a double".to_string()),
        }
    }

    /// Get the return value as a byte
    pub fn get_return_byte(&self) -> Result<u8, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::UInt8(v) => Ok(*v),
            Value::Int8(v) => Ok(*v as u8),
            _ => Err("Return value is not a byte".to_string()),
        }
    }

    /// Get the return value as a word (16-bit)
    pub fn get_return_word(&self) -> Result<u16, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        match self.vm.get_return_value() {
            Value::UInt16(v) => Ok(*v),
            Value::Int16(v) => Ok(*v as u16),
            _ => Err("Return value is not a word".to_string()),
        }
    }

    /// Get the return value (generic)
    pub fn get_return_value(&self) -> Result<&Value, String> {
        if self.state != ContextState::Finished {
            return Err("Execution did not finish successfully".to_string());
        }

        Ok(self.vm.get_return_value())
    }

    /// Get the current execution state
    pub fn get_state(&self) -> ContextState {
        self.state
    }

    /// Abort execution
    pub fn abort(&mut self) {
        if self.state == ContextState::Executing {
            self.vm.abort();
            self.state = ContextState::Aborted;
        }
    }

    /// Unprepare the context (reset for reuse)
    pub fn unprepare(&mut self) {
        self.prepared_function = None;
        self.state = ContextState::Unprepared;
        self.vm.reset();
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}
