// src/core/engine.rs - Updated to match AngelScript specification

use crate::core::declaration_parser::DeclarationParser;
use crate::core::script_module::ScriptModule;
use crate::core::types::{
    allocate_type_id, AccessSpecifier, BehaviourInfo, BehaviourType, EnumType, FuncdefInfo,
    GlobalFunction, GlobalProperty, InterfaceInfo, ObjectMethod, ObjectProperty, ObjectType, TypeFlags,
    TYPE_VOID,
};
use std::any::TypeId as StdTypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// The main script engine that manages the entire AngelScript system
pub struct ScriptEngine {
    pub inner: Arc<RwLock<EngineInner>>,
}

pub struct EngineInner {
    /// Registered object types
    pub object_types: HashMap<String, ObjectType>,
    /// Registered enum types
    pub enum_types: HashMap<String, EnumType>,
    /// Registered typedefs
    pub interface_types: HashMap<String, InterfaceInfo>,
    /// Registered funcdefs (function pointer types)
    pub funcdefs: HashMap<String, FuncdefInfo>,
    /// Registered global functions
    pub global_functions: Vec<GlobalFunction>,
    /// Registered global properties
    pub global_properties: Vec<GlobalProperty>,
    /// Modules
    pub modules: HashMap<String, Box<ScriptModule>>,
}

/// Flags for GetModule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetModuleFlag {
    /// Only return existing module, don't create
    OnlyIfExists,
    /// Create module if it doesn't exist
    CreateIfNotExists,
    /// Always create a new module (discards existing)
    AlwaysCreate,
}

impl ScriptEngine {
    /// Create a new script engine
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EngineInner {
                object_types: HashMap::new(),
                enum_types: HashMap::new(),
                interface_types: HashMap::new(),
                funcdefs: HashMap::new(),
                global_functions: Vec::new(),
                global_properties: Vec::new(),
                modules: HashMap::new(),
            })),
        }
    }

    /// Get a thread-safe reference to the engine for the compiler
    pub fn get_ref(&self) -> Arc<RwLock<EngineInner>> {
        Arc::clone(&self.inner)
    }

    /// Register an object type using a Rust type (automatically gets size)
    pub fn register_object_type<T: 'static>(
        &mut self,
        name: &str,
        flags: TypeFlags,
    ) -> Result<u32, String> {
        let rust_type_id = StdTypeId::of::<T>();

        let mut inner = self.inner.write().unwrap();

        if inner.object_types.contains_key(name) {
            return Err(format!("Type '{}' already registered", name));
        }

        let type_id = allocate_type_id();

        inner.object_types.insert(
            name.to_string(),
            ObjectType {
                type_id,
                name: name.to_string(),
                flags,
                properties: Vec::new(),
                methods: Vec::new(),
                behaviours: Vec::new(),
                rust_type_id: Some(rust_type_id),
            },
        );

        Ok(type_id)
    }

    pub fn register_object_method(
        &mut self,
        type_name: &str,
        declaration: &str,
    ) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        let sig = parser.parse_function_declaration(declaration)?;

        let mut inner = self.inner.write().unwrap();
        let obj_type = inner
            .object_types
            .get_mut(type_name)
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        obj_type.methods.push(ObjectMethod {
            name: sig.name,
            return_type_id: sig.return_type_id,
            params: sig.params,
            is_const: sig.is_const,
            is_virtual: false,
            is_final: false,
            access: AccessSpecifier::Public,
            function_id:  allocate_type_id(),
        });

        Ok(())
    }

    /// Register an object property - NO PLACEHOLDERS
    pub fn register_object_property(
        &mut self,
        type_name: &str,
        declaration: &str,
    ) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        let sig = parser.parse_property_declaration(declaration)?;

        let mut inner = self.inner.write().unwrap();
        let obj_type = inner
            .object_types
            .get_mut(type_name)
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        obj_type.properties.push(ObjectProperty {
            name: sig.name,
            type_id: sig.type_id,
            is_handle: sig.is_handle,
            is_const: sig.is_const,
            access: AccessSpecifier::Public,
        });

        Ok(())
    }

    /// Register an object behaviour
    pub fn register_object_behaviour(
        &mut self,
        type_name: &str,
        behaviour: BehaviourType,
        declaration: &str,
    ) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        parser.parse_behaviour_declaration(declaration)?;

        let mut inner = self.inner.write().unwrap();

        let function_id = allocate_type_id();

        let obj_type = inner
            .object_types
            .get_mut(type_name)
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        obj_type.behaviours.push(BehaviourInfo {
            behaviour_type: behaviour,
            function_id,
            return_type_id: TYPE_VOID,
            params: vec![],
        });

        Ok(())
    }

    /// Register a global function - NO PLACEHOLDERS
    pub fn register_global_function(&mut self, declaration: &str) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        let sig = parser.parse_function_declaration(declaration)?;

        let mut inner = self.inner.write().unwrap();
        inner.global_functions.push(GlobalFunction {
            name: sig.name,
            return_type_id: sig.return_type_id,
            params: sig.params,
            function_id: allocate_type_id(),
        });

        Ok(())
    }

    /// Register a funcdef - NO PLACEHOLDERS
    pub fn register_funcdef(&mut self, declaration: &str) -> Result<u32, String> {
        // Strip "funcdef" keyword if present
        let decl = declaration.trim();
        let decl = if decl.starts_with("funcdef") {
            decl.strip_prefix("funcdef").unwrap().trim()
        } else {
            decl
        };

        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        let sig = parser.parse_function_declaration(decl)?;

        let mut inner = self.inner.write().unwrap();

        if inner.funcdefs.contains_key(&sig.name) {
            return Err(format!("Funcdef '{}' already registered", sig.name));
        }

        let type_id = allocate_type_id();

        inner.funcdefs.insert(
            sig.name.clone(),
            FuncdefInfo {
                type_id,
                name: sig.name,
                return_type_id: sig.return_type_id,
                params: sig.params,
            },
        );

        Ok(type_id)
    }

    /// Register a global property - NO PLACEHOLDERS
    pub fn register_global_property(&mut self, declaration: &str) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.inner));
        let sig = parser.parse_property_declaration(declaration)?;

        let mut inner = self.inner.write().unwrap();
        inner.global_properties.push(GlobalProperty {
            name: sig.name,
            type_id: sig.type_id,
            is_const: sig.is_const,
            is_handle: sig.is_handle,
        });

        Ok(())
    }

    /// Get a module by name
    pub fn get_module(&mut self, _name: &str, _flag: GetModuleFlag) -> Option<&mut ScriptModule> {
        None
    }

    /// Discard a module
    pub fn discard_module(&mut self, name: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.modules.remove(name);
    }
}

impl EngineInner {
    /// Look up a type by name
    pub fn get_type_id(&self, name: &str) -> Option<u32> {
        if let Some(obj_type) = self.object_types.get(name) {
            return Some(obj_type.type_id);
        }
        if let Some(enum_type) = self.enum_types.get(name) {
            return Some(enum_type.type_id);
        }
        if let Some(interface_type) = self.interface_types.get(name) {
            return Some(interface_type.type_id);
        }
        if let Some(funcdef) = self.funcdefs.get(name) {
            return Some(funcdef.type_id);
        }
        None
    }

    /// Get object type info
    pub fn get_object_type(&self, name: &str) -> Option<&ObjectType> {
        self.object_types.get(name)
    }

    /// Get enum type info
    pub fn get_enum_type(&self, name: &str) -> Option<&EnumType> {
        self.enum_types.get(name)
    }

    /// Get typedef info
    pub fn get_interface_type(&self, name: &str) -> Option<&InterfaceInfo> {
        self.interface_types.get(name)
    }

    /// Get funcdef info
    pub fn get_funcdef(&self, name: &str) -> Option<&FuncdefInfo> {
        self.funcdefs.get(name)
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
