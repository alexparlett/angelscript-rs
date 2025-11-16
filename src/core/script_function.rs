use crate::core::script_module::ScriptModule;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct ScriptFunction {
    id: u32,
    func_type: FuncType,
    module: Option<Arc<RwLock<ScriptModule>>>,
    system_func_id: Option<u32>,
}

impl ScriptFunction {
    pub(crate) fn new_script(id: u32, module: Arc<RwLock<ScriptModule>>) -> Self {
        Self {
            id,
            func_type: FuncType::Script,
            module: Some(module),
            system_func_id: None,
        }
    }

    pub(crate) fn new_system(id: u32) -> Self {
        Self {
            id,
            func_type: FuncType::System,
            module: None,
            system_func_id: Some(id),
        }
    }

    pub fn add_ref(&self) -> i32 {
        1
    }

    pub fn release(&self) -> i32 {
        0
    }

    pub fn get_id(&self) -> i32 {
        self.id as i32
    }

    pub fn get_func_type(&self) -> FuncType {
        self.func_type
    }

    pub fn get_module_name(&self) -> Option<String> {
        self.module.as_ref().map(|m| {
            let module = m.read().unwrap();
            module.name.clone()
        })
    }

    pub fn get_module(&self) -> Option<Arc<RwLock<ScriptModule>>> {
        self.module.clone()
    }

    pub fn get_script_section_name(&self) -> Option<String> {
        None
    }

    pub fn get_config_group(&self) -> Option<String> {
        None
    }

    pub fn get_access_mask(&self) -> u32 {
        0xFFFFFFFF
    }

    pub fn get_name(&self) -> String {
        match self.func_type {
            FuncType::Script => {
                if let Some(module) = &self.module {
                    let m = module.read().unwrap();
                    if let Some(bytecode) = &m.bytecode {
                        bytecode
                            .functions
                            .get(self.id as usize)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| String::from("<unknown>"))
                    } else {
                        String::from("<unknown>")
                    }
                } else {
                    String::from("<unknown>")
                }
            }
            FuncType::System => String::from("<system>"),
            _ => String::from("<unknown>"),
        }
    }

    pub fn get_namespace(&self) -> String {
        String::new()
    }

    pub fn get_declaration(
        &self,
        include_object_name: bool,
        include_namespace: bool,
        include_param_names: bool,
    ) -> String {
        self.get_name()
    }

    pub fn is_read_only(&self) -> bool {
        false
    }

    pub fn is_private(&self) -> bool {
        false
    }

    pub fn is_protected(&self) -> bool {
        false
    }

    pub fn is_final(&self) -> bool {
        false
    }

    pub fn is_override(&self) -> bool {
        false
    }

    pub fn is_shared(&self) -> bool {
        self.func_type == FuncType::System
    }

    pub fn is_explicit(&self) -> bool {
        false
    }

    pub fn is_property(&self) -> bool {
        false
    }

    pub fn get_param_count(&self) -> u32 {
        match self.func_type {
            FuncType::Script => {
                if let Some(module) = &self.module {
                    let m = module.read().unwrap();
                    if let Some(bytecode) = &m.bytecode {
                        bytecode
                            .functions
                            .get(self.id as usize)
                            .map(|f| f.param_count as u32)
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            FuncType::System => 0,
            _ => 0,
        }
    }

    pub fn get_param(
        &self,
        index: u32,
        type_id: &mut i32,
        flags: &mut u32,
        name: &mut Option<String>,
        default_arg: &mut Option<String>,
    ) -> Result<i32, String> {
        *type_id = 0;
        *flags = 0;
        *name = None;
        *default_arg = None;
        Ok(0)
    }

    pub fn get_return_type_id(&self, flags: &mut u32) -> i32 {
        *flags = 0;
        match self.func_type {
            FuncType::Script => {
                if let Some(module) = &self.module {
                    let m = module.read().unwrap();
                    if let Some(bytecode) = &m.bytecode {
                        bytecode
                            .functions
                            .get(self.id as usize)
                            .map(|f| f.return_type as i32)
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            FuncType::System => 0,
            _ => 0,
        }
    }

    pub fn get_type_id(&self) -> i32 {
        0
    }

    pub fn is_compatible_with_type_id(&self, type_id: i32) -> bool {
        false
    }

    pub fn get_delegate_object(&self) -> Option<u64> {
        None
    }

    pub fn get_delegate_function(&self) -> Option<ScriptFunction> {
        None
    }

    pub fn get_delegate_object_type(&self) -> i32 {
        0
    }

    pub fn get_var_count(&self) -> u32 {
        0
    }

    pub fn get_var(
        &self,
        index: u32,
        name: &mut Option<String>,
        type_id: &mut i32,
    ) -> Result<i32, String> {
        *name = None;
        *type_id = 0;
        Err("Not supported".to_string())
    }

    pub fn get_var_decl(&self, index: u32, include_namespace: bool) -> Option<String> {
        None
    }

    pub fn find_next_line_with_code(&self, line: i32) -> i32 {
        -1
    }

    pub fn get_declared_at(
        &self,
        script_section: &mut Option<String>,
        row: &mut i32,
        col: &mut i32,
    ) -> Result<i32, String> {
        *script_section = None;
        *row = 0;
        *col = 0;
        Err("Not supported".to_string())
    }

    pub fn set_user_data(
        &mut self,
        data: *mut std::ffi::c_void,
        type_id: u32,
    ) -> Result<i32, String> {
        Ok(0)
    }

    pub fn get_user_data(&self, type_id: u32) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    pub(crate) fn is_system(&self) -> bool {
        self.func_type == FuncType::System
    }

    pub(crate) fn get_system_id(&self) -> Option<u32> {
        self.system_func_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncType {
    Dummy,
    System,
    Script,
    Interface,
    Virtual,
    FuncDef,
    Imported,
    Delegate,
}
