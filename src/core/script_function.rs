use crate::core::script_type::ScriptType;
use crate::core::type_registry::{
    FunctionFlags, FunctionImpl, FunctionKind, ParameterFlags, TypeRegistry,
};
use crate::core::types::FunctionId;
use std::sync::{Arc, RwLock};

pub struct ScriptFunction {
    function_id: FunctionId,
    registry: Arc<RwLock<TypeRegistry>>,
}

impl ScriptFunction {
    pub(crate) fn new(function_id: FunctionId, registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            function_id,
            registry,
        }
    }

    pub fn add_ref(&self) -> i32 {
        1
    }

    pub fn release(&self) -> i32 {
        0
    }

    pub fn get_id(&self) -> i32 {
        self.function_id as i32
    }

    pub fn get_func_type(&self) -> FuncType {
        let registry = self.registry.read().unwrap();
        if let Some(func_info) = registry.get_function(self.function_id) {
            match func_info.kind {
                FunctionKind::Global => FuncType::Script,
                FunctionKind::Method { .. } => FuncType::Script,
                FunctionKind::Constructor => FuncType::Script,
                FunctionKind::Destructor => FuncType::Script,
                FunctionKind::Lambda => FuncType::Script,
                FunctionKind::Operator(_) => FuncType::Script,
                FunctionKind::Conversion => FuncType::Script,
            }
        } else {
            FuncType::Dummy
        }
    }

    pub fn get_module_name(&self) -> Option<String> {
        None
    }

    pub fn get_script_section_name(&self) -> Option<String> {
        let registry = self.registry.read().unwrap();
        let func = registry.get_function(self.function_id)?;

        func.definition_span
            .as_ref()
            .map(|span| span.source_name.to_string())
    }

    pub fn get_config_group(&self) -> Option<String> {
        None
    }

    pub fn get_access_mask(&self) -> u32 {
        0xFFFFFFFF
    }

    pub fn get_object_type(&self) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        let func_info = registry.get_function(self.function_id)?;

        func_info
            .owner_type
            .map(|type_id| ScriptType::new(type_id, Arc::clone(&self.registry)))
    }

    pub fn get_name(&self) -> String {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.name.clone())
            .unwrap_or_default()
    }

    pub fn get_namespace(&self) -> String {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.namespace.join("::"))
            .unwrap_or_default()
    }

    pub fn get_declaration(
        &self,
        include_object_name: bool,
        include_namespace: bool,
        include_param_names: bool,
    ) -> String {
        let registry = self.registry.read().unwrap();
        let func = match registry.get_function(self.function_id) {
            Some(f) => f,
            None => return String::new(),
        };

        let mut decl = String::new();

        if let Some(ret_type) = registry.get_type(func.return_type) {
            if func.flags.contains(FunctionFlags::CONST)
                && matches!(func.kind, FunctionKind::Method { .. })
            {
            } else {
                decl.push_str(&ret_type.name);
                decl.push(' ');
            }
        }

        if include_namespace && !func.namespace.is_empty() {
            decl.push_str(&func.namespace.join("::"));
            decl.push_str("::");
        }

        if include_object_name {
            if let Some(owner_id) = func.owner_type {
                if let Some(owner_type) = registry.get_type(owner_id) {
                    decl.push_str(&owner_type.name);
                    decl.push_str("::");
                }
            }
        }

        decl.push_str(&func.name);

        decl.push('(');
        for (i, param) in func.parameters.iter().enumerate() {
            if i > 0 {
                decl.push_str(", ");
            }

            if let Some(param_type) = registry.get_type(param.type_id) {
                if param.flags.contains(ParameterFlags::CONST) {
                    decl.push_str("const ");
                }
                decl.push_str(&param_type.name);
            }

            if param.flags.contains(ParameterFlags::INOUT) {
                decl.push_str(" &inout");
            } else if param.flags.contains(ParameterFlags::OUT) {
                decl.push_str(" &out");
            } else if param.flags.contains(ParameterFlags::IN) {
                decl.push_str(" &in");
            }

            if include_param_names {
                if let Some(name) = &param.name {
                    decl.push(' ');
                    decl.push_str(name);
                }
            }
        }
        decl.push(')');

        if func.flags.contains(FunctionFlags::CONST) {
            decl.push_str(" const");
        }

        decl
    }

    pub fn is_read_only(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::CONST))
            .unwrap_or(false)
    }

    pub fn is_private(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::PRIVATE))
            .unwrap_or(false)
    }

    pub fn is_protected(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::PROTECTED))
            .unwrap_or(false)
    }

    pub fn is_final(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::FINAL))
            .unwrap_or(false)
    }

    pub fn is_override(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::OVERRIDE))
            .unwrap_or(false)
    }

    pub fn is_shared(&self) -> bool {
        false
    }

    pub fn is_explicit(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.flags.contains(FunctionFlags::EXPLICIT))
            .unwrap_or(false)
    }

    pub fn is_property(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.name.starts_with("get_") || f.name.starts_with("set_"))
            .unwrap_or(false)
    }

    pub fn get_param_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.parameters.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_param(
        &self,
        index: u32,
        type_id: &mut i32,
        flags: &mut u32,
        name: &mut Option<String>,
        default_arg: &mut Option<String>,
    ) -> Result<i32, String> {
        let registry = self.registry.read().unwrap();
        let func = registry
            .get_function(self.function_id)
            .ok_or("Function not found")?;

        let param = func
            .parameters
            .get(index as usize)
            .ok_or("Parameter index out of bounds")?;

        *type_id = param.type_id as i32;
        *flags = param.flags.bits();
        *name = param.name.clone();
        *default_arg = None;

        Ok(0)
    }

    pub fn get_return_type_id(&self, flags: &mut u32) -> i32 {
        let registry = self.registry.read().unwrap();
        if let Some(func) = registry.get_function(self.function_id) {
            *flags = 0;
            func.return_type as i32
        } else {
            *flags = 0;
            0
        }
    }

    pub fn get_type_id(&self) -> i32 {
        0
    }

    pub fn is_compatible_with_type_id(&self, _type_id: i32) -> bool {
        false
    }

    pub fn get_delegate_object(&self) -> Option<super::script_object::ScriptObject> {
        None
    }

    pub fn get_delegate_function(&self) -> Option<ScriptFunction> {
        None
    }

    pub fn get_delegate_object_type(&self) -> Option<ScriptType> {
        None
    }

    pub fn get_var_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.locals.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_var(
        &self,
        index: u32,
        name: &mut Option<String>,
        type_id: &mut i32,
    ) -> Result<i32, String> {
        let registry = self.registry.read().unwrap();
        let func = registry
            .get_function(self.function_id)
            .ok_or("Function not found")?;

        let local = func
            .locals
            .get(index as usize)
            .ok_or("Variable index out of bounds")?;

        *name = Some(local.name.clone());
        *type_id = local.type_id as i32;

        Ok(0)
    }

    pub fn get_var_decl(&self, index: u32, include_namespace: bool) -> Option<String> {
        let registry = self.registry.read().unwrap();
        let func = registry.get_function(self.function_id)?;
        let local = func.locals.get(index as usize)?;

        let var_type = registry.get_type(local.type_id)?;
        let mut decl = String::new();

        if local.is_const {
            decl.push_str("const ");
        }

        if include_namespace && !var_type.namespace.is_empty() {
            decl.push_str(&var_type.namespace.join("::"));
            decl.push_str("::");
        }

        decl.push_str(&var_type.name);
        decl.push(' ');
        decl.push_str(&local.name);

        Some(decl)
    }

    pub fn find_next_line_with_code(&self, _line: i32) -> i32 {
        -1
    }

    pub fn get_declared_at(
        &self,
        script_section: &mut Option<String>,
        row: &mut i32,
        col: &mut i32,
    ) -> Result<i32, String> {
        let registry = self.registry.read().unwrap();
        let func = registry
            .get_function(self.function_id)
            .ok_or("Function not found")?;

        if let Some(span) = &func.definition_span {
            *script_section = Some(span.source_name.to_string());
            *row = span.start_line as i32;
            *col = span.start_column as i32;
            Ok(0)
        } else {
            *script_section = None;
            *row = 0;
            *col = 0;
            Err("No debug information available".to_string())
        }
    }

    pub(crate) fn is_native(&self) -> bool {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| matches!(f.implementation, FunctionImpl::Native { .. }))
            .unwrap_or(false)
    }

    pub(crate) fn get_bytecode_address(&self) -> Option<u32> {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .and_then(|f| f.bytecode_address)
    }

    pub(crate) fn get_local_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.local_count)
            .unwrap_or(0)
    }

    pub(crate) fn get_param_count_internal(&self) -> usize {
        let registry = self.registry.read().unwrap();
        registry
            .get_function(self.function_id)
            .map(|f| f.parameters.len())
            .unwrap_or(0)
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
