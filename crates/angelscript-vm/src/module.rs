//! Compiled module — the unit of execution for the VM.
//!
//! A `Module` holds all the compiled functions from a script, together with
//! any global variables and constant data. The VM loads a module and executes
//! functions within it.

use std::collections::HashMap;

use angelscript_core::opcode::ByteCode;
use angelscript_core::{FunctionId, QualifiedName};

/// A compiled function ready for execution.
#[derive(Debug)]
pub struct ScriptFunction {
    /// The function ID.
    pub id: FunctionId,
    /// The function's qualified name.
    pub name: QualifiedName,
    /// Compiled bytecode.
    pub bytecode: ByteCode,
    /// Stack frame size in dwords (local variables + temporaries).
    pub stack_size: i16,
    /// Number of parameters.
    pub param_count: u16,
}

/// A native (host-registered) function.
pub struct NativeFunction {
    /// The function ID.
    pub id: FunctionId,
    /// The function's qualified name.
    pub name: QualifiedName,
    /// The native callback.
    pub callback: Box<dyn Fn(&mut NativeCallContext) -> Result<(), String>>,
}

impl std::fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFunction")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}

/// Context passed to native function callbacks.
pub struct NativeCallContext {
    /// Arguments passed to the native function (read from the stack).
    pub args: Vec<u32>,
    /// Return value — the native sets this.
    pub return_value: u64,
    /// Whether a 64-bit return value should be used.
    pub return_is_64bit: bool,
}

impl NativeCallContext {
    pub fn new(args: Vec<u32>) -> Self {
        NativeCallContext {
            args,
            return_value: 0,
            return_is_64bit: false,
        }
    }

    /// Get a 32-bit integer argument.
    pub fn arg_i32(&self, index: usize) -> i32 {
        self.args.get(index).copied().unwrap_or(0) as i32
    }

    /// Get a 32-bit float argument.
    pub fn arg_f32(&self, index: usize) -> f32 {
        f32::from_bits(self.args.get(index).copied().unwrap_or(0))
    }

    /// Set a 32-bit integer return value.
    pub fn set_return_i32(&mut self, val: i32) {
        self.return_value = val as u32 as u64;
        self.return_is_64bit = false;
    }

    /// Set a 32-bit float return value.
    pub fn set_return_f32(&mut self, val: f32) {
        self.return_value = val.to_bits() as u64;
        self.return_is_64bit = false;
    }

    /// Set a 64-bit integer return value.
    pub fn set_return_i64(&mut self, val: i64) {
        self.return_value = val as u64;
        self.return_is_64bit = true;
    }

    /// Set a 64-bit float return value.
    pub fn set_return_f64(&mut self, val: f64) {
        self.return_value = val.to_bits();
        self.return_is_64bit = true;
    }
}

/// A compiled module containing all functions from a script.
#[derive(Debug)]
pub struct Module {
    /// Script functions indexed by FunctionId.
    pub functions: HashMap<FunctionId, ScriptFunction>,
    /// Native functions indexed by FunctionId.
    pub native_functions: HashMap<FunctionId, NativeFunction>,
    /// Lookup from qualified name to function ID.
    pub name_to_id: HashMap<QualifiedName, FunctionId>,
    /// Global variable storage (dword array).
    pub globals: Vec<u32>,
}

impl Module {
    pub fn new() -> Self {
        Module {
            functions: HashMap::new(),
            native_functions: HashMap::new(),
            name_to_id: HashMap::new(),
            globals: Vec::new(),
        }
    }

    /// Add a script function to the module.
    pub fn add_function(&mut self, func: ScriptFunction) {
        self.name_to_id.insert(func.name.clone(), func.id);
        self.functions.insert(func.id, func);
    }

    /// Add a native function to the module.
    pub fn add_native_function(&mut self, func: NativeFunction) {
        self.name_to_id.insert(func.name.clone(), func.id);
        self.native_functions.insert(func.id, func);
    }

    /// Look up a function ID by qualified name.
    pub fn find_function(&self, name: &QualifiedName) -> Option<FunctionId> {
        self.name_to_id.get(name).copied()
    }

    /// Get a script function by ID.
    pub fn get_function(&self, id: FunctionId) -> Option<&ScriptFunction> {
        self.functions.get(&id)
    }

    /// Get a native function by ID.
    pub fn get_native_function(&self, id: FunctionId) -> Option<&NativeFunction> {
        self.native_functions.get(&id)
    }
}

impl Default for Module {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::opcode::ByteCode;

    #[test]
    fn module_add_and_find() {
        let mut module = Module::new();
        let name = QualifiedName::global("main");
        let id = FunctionId(1);
        module.add_function(ScriptFunction {
            id,
            name: name.clone(),
            bytecode: ByteCode::new(),
            stack_size: 4,
            param_count: 0,
        });

        assert_eq!(module.find_function(&name), Some(id));
        assert!(module.get_function(id).is_some());
    }

    #[test]
    fn native_call_context() {
        let mut ctx = NativeCallContext::new(vec![42, 10]);
        assert_eq!(ctx.arg_i32(0), 42);
        assert_eq!(ctx.arg_i32(1), 10);

        ctx.set_return_i32(52);
        assert_eq!(ctx.return_value, 52);
        assert!(!ctx.return_is_64bit);
    }
}
