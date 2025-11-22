use crate::compiler::bytecode::BytecodeModule;
use crate::compiler::compiler::Compiler;
use crate::core::script_function::ScriptFunction;
use crate::core::script_type::ScriptType;
use crate::core::type_registry::TypeRegistry;
use crate::parser::script_builder::{IncludeCallback, PragmaCallback, ScriptBuilder};
use std::sync::{Arc, RwLock};

pub struct ScriptModule {
    pub name: String,
    pub bytecode: Option<BytecodeModule>,
    pub sources: Vec<SourceSection>,
    pub state: ModuleState,

    registry: Arc<RwLock<TypeRegistry>>,
    script_builder: ScriptBuilder,
}

#[derive(Clone)]
pub struct SourceSection {
    pub name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModuleState {
    Empty,
    Building,
    Built,
    Failed,
}

impl ScriptModule {
    pub fn new(name: String, registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            name,
            bytecode: None,
            sources: Vec::new(),
            state: ModuleState::Empty,
            registry,
            script_builder: ScriptBuilder::new(),
        }
    }

    pub fn add_ref(&self) -> i32 {
        1
    }

    pub fn release(&self) -> i32 {
        0
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn add_script_section(&mut self, name: &str, code: &str) -> Result<i32, String> {
        match self.state {
            ModuleState::Building => {
                return Err("Cannot add sections while building".to_string());
            }
            ModuleState::Built => {
                self.state = ModuleState::Empty;
                self.bytecode = None;
                self.sources.clear();
            }
            _ => {}
        }

        self.sources.push(SourceSection {
            name: name.to_string(),
        });

        self.script_builder.add_section(name, code);

        Ok(0)
    }

    pub fn build(&mut self) -> i32 {
        match self.build_internal() {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    fn build_internal(&mut self) -> Result<(), Vec<String>> {
        match self.state {
            ModuleState::Built => return Ok(()),
            ModuleState::Building => {
                return Err(vec!["Module is already being built".to_string()]);
            }
            _ => {}
        }

        self.state = ModuleState::Building;

        if self.sources.is_empty() {
            self.state = ModuleState::Failed;
            return Err(vec!["Module has no source code".to_string()]);
        }

        let ast = match self.script_builder.build() {
            Ok(ast) => ast,
            Err(e) => {
                self.state = ModuleState::Failed;
                return Err(vec![format!("Parse error: {}", e)]);
            }
        };

        let compiler = Compiler::new(Arc::clone(&self.registry));
        let bytecode = match compiler.compile(ast) {
            Ok(bc) => bc,
            Err(e) => {
                self.state = ModuleState::Failed;
                return Err(vec![format!("Compilation error: {:?}", e)]);
            }
        };

        self.bytecode = Some(bytecode);
        self.state = ModuleState::Built;

        self.script_builder.clear();

        Ok(())
    }

    pub fn discard(&mut self) {
        self.sources.clear();
        self.bytecode = None;
        self.state = ModuleState::Empty;
        self.script_builder.clear();
    }

    pub fn get_function_count(&self) -> u32 {
        self.bytecode
            .as_ref()
            .map(|bc| bc.function_addresses.len() as u32)
            .unwrap_or(0)
    }

    pub fn get_function_by_index(&self, index: u32) -> Option<ScriptFunction> {
        let bytecode = self.bytecode.as_ref()?;

        let func_id = bytecode
            .function_addresses
            .keys()
            .nth(index as usize)
            .copied()?;

        Some(ScriptFunction::new(func_id, Arc::clone(&self.registry)))
    }

    pub fn get_function_by_name(&self, name: &str) -> Option<ScriptFunction> {
        let registry = self.registry.read().unwrap();
        let func_info = registry.find_function(name, &[])?;

        Some(ScriptFunction::new(
            func_info.function_id,
            Arc::clone(&self.registry),
        ))
    }

    pub fn get_function_by_decl(&self, _decl: &str) -> Option<ScriptFunction> {
        None
    }

    pub fn get_global_var_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry.get_all_globals().len() as u32
    }

    pub fn get_global_var_index_by_name(&self, name: &str) -> Option<i32> {
        let registry = self.registry.read().unwrap();
        registry.get_global(name).map(|g| g.address as i32)
    }

    pub fn get_global_var_index_by_decl(&self, _decl: &str) -> Option<i32> {
        None
    }

    pub fn get_global_var(
        &self,
        index: u32,
        name: &mut Option<String>,
        namespace_out: &mut Option<String>,
        type_id: &mut i32,
        is_const: &mut bool,
    ) -> Result<i32, String> {
        let registry = self.registry.read().unwrap();
        let globals = registry.get_all_globals();

        let global = globals
            .get(index as usize)
            .ok_or("Global index out of bounds")?;

        *name = Some(global.name.clone());
        *namespace_out = None;
        *type_id = global.type_id as i32;
        *is_const = global.is_const;

        Ok(0)
    }

    pub fn get_object_type_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_all_types()
            .iter()
            .filter(|t| t.kind == crate::core::types::TypeKind::Class)
            .count() as u32
    }

    pub fn get_object_type_by_index(&self, index: u32) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        let type_id = registry
            .get_all_types()
            .iter()
            .filter(|t| t.kind == crate::core::types::TypeKind::Class)
            .nth(index as usize)?
            .type_id;

        Some(ScriptType::new(type_id, Arc::clone(&self.registry)))
    }

    pub fn get_type_id_by_decl(&self, _decl: &str) -> Option<i32> {
        None
    }

    pub fn get_type_info_by_name(&self, name: &str) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        let type_id = registry.lookup_type(name, &[])?;

        Some(ScriptType::new(type_id, Arc::clone(&self.registry)))
    }

    pub fn get_type_info_by_decl(&self, _decl: &str) -> Option<ScriptType> {
        None
    }

    pub fn get_enum_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry
            .get_all_types()
            .iter()
            .filter(|t| t.kind == crate::core::types::TypeKind::Enum)
            .count() as u32
    }

    pub fn get_enum_by_index(&self, index: u32) -> Option<ScriptType> {
        let registry = self.registry.read().unwrap();
        let type_id = registry
            .get_all_types()
            .iter()
            .filter(|t| t.kind == crate::core::types::TypeKind::Enum)
            .nth(index as usize)?
            .type_id;

        Some(ScriptType::new(type_id, Arc::clone(&self.registry)))
    }

    pub fn get_typedef_count(&self) -> u32 {
        0
    }

    pub fn get_typedef_by_index(&self, _index: u32) -> Option<ScriptType> {
        None
    }

    pub fn get_imported_function_count(&self) -> u32 {
        0
    }

    pub fn get_imported_function_index_by_decl(&self, _decl: &str) -> Option<i32> {
        None
    }

    pub fn remove_function(&mut self, _func: &ScriptFunction) -> Result<i32, String> {
        Err("Not supported".to_string())
    }

    pub fn reset_global_vars(&mut self, _func: Option<&ScriptFunction>) -> Result<i32, String> {
        Ok(0)
    }

    pub fn rebind_imported_functions(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn bind_imported_function(
        &mut self,
        _import_index: u32,
        _func: &ScriptFunction,
    ) -> Result<i32, String> {
        Err("Not supported".to_string())
    }

    pub fn unbind_imported_function(&mut self, _import_index: u32) -> Result<i32, String> {
        Err("Not supported".to_string())
    }

    pub fn bind_all_imported_functions(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn unbind_all_imported_functions(&mut self) -> Result<i32, String> {
        Ok(0)
    }

    pub fn set_default_namespace(&mut self, _namespace: &str) -> Result<i32, String> {
        Ok(0)
    }

    pub fn get_default_namespace(&self) -> String {
        String::new()
    }

    pub fn set_access_mask(&mut self, _access_mask: u32) -> Result<i32, String> {
        Ok(0)
    }

    pub fn is_built(&self) -> bool {
        self.state == ModuleState::Built
    }

    pub fn define_word(&mut self, word: String) {
        self.script_builder.define_word(word);
    }

    pub fn is_defined(&self, word: &str) -> bool {
        self.script_builder.is_defined(word)
    }

    pub fn set_include_callback<C: IncludeCallback + 'static>(&mut self, callback: C) {
        self.script_builder.set_include_callback(callback);
    }

    pub fn set_pragma_callback<C: PragmaCallback + 'static>(&mut self, callback: C) {
        self.script_builder.set_pragma_callback(callback);
    }
}
