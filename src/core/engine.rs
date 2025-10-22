// src/core/engine.rs - Updated to match AngelScript specification

use crate::core::declaration_parser::DeclarationParser;
use crate::core::module::Module;
use std::any::TypeId as StdTypeId;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
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
    /// Type ID counter
    pub next_type_id: AtomicU32,
    /// Modules
    pub modules: HashMap<String, Box<Module>>,
}

#[derive(Debug, Clone)]
pub struct ObjectType {
    pub type_id: u32,
    pub name: String,
    pub flags: TypeFlags,
    pub properties: Vec<ObjectProperty>,
    pub methods: Vec<ObjectMethod>,
    pub behaviours: Vec<BehaviourInfo>,
    pub destructor: Option<u32>,
    pub rust_type_id: Option<StdTypeId>,
}

#[derive(Debug, Clone)]
pub struct ObjectProperty {
    pub name: String,
    pub type_id: u32,
    pub is_handle: bool,
    pub is_readonly: bool,
    pub access: AccessSpecifier,
}

#[derive(Debug, Clone)]
pub struct ObjectMethod {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
    pub is_const: bool,
    pub is_virtual: bool,
    pub is_final: bool,
    pub access: AccessSpecifier,
}

#[derive(Debug, Clone)]
pub struct MethodParam {
    pub name: String,
    pub type_id: u32,
    pub is_ref: bool,
    pub is_out: bool,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct EnumType {
    pub type_id: u32,
    pub name: String,
    pub values: HashMap<String, i32>,
}

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub type_id: u32,
    pub name: String,
    pub aliased_type_id: u32,
}

#[derive(Debug, Clone)]
pub struct FuncdefInfo {
    pub type_id: u32,
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Clone)]
pub struct GlobalFunction {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<MethodParam>,
}

#[derive(Debug, Clone)]
pub struct GlobalProperty {
    pub name: String,
    pub type_id: u32,
    pub is_const: bool,
    pub is_handle: bool,
}

#[derive(Debug, Clone)]
pub struct BehaviourInfo {
    pub behaviour_type: BehaviourType,
    pub function_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSpecifier {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviourType {
    Construct,
    ListConstruct,
    Destruct,
    Factory,
    ListFactory,
    AddRef,
    Release,
    GetWeakRefFlag,
    TemplateCallback,
    GetRefCount,
    SetGCFlag,
    GetGCFlag,
    EnumRefs,
    ReleaseRefs,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeFlags: u32 {
        /// Value type (passed by value, managed by engine)
        const VALUE_TYPE = 0x00000001;
        /// Reference type (passed by reference, managed by app)
        const REF_TYPE = 0x00000002;
        /// Garbage collected type
        const GC_TYPE = 0x00000004;
        /// Plain Old Data (no special handling needed)
        const POD_TYPE = 0x00000008;
        /// Type cannot be used as handle
        const NOHANDLE = 0x00000010;
        /// Type cannot be copied (no assignment)
        const SCOPED = 0x00000020;
        /// Template type
        const TEMPLATE = 0x00000040;
        /// Type uses ASALIGN macro
        const ASHANDLE = 0x00000080;
        /// Type has constructor
        const APP_CLASS_CONSTRUCTOR = 0x00000100;
        /// Type has destructor
        const APP_CLASS_DESTRUCTOR = 0x00000200;
        /// Type has assignment operator
        const APP_CLASS_ASSIGNMENT = 0x00000400;
        /// Type has copy constructor
        const APP_CLASS_COPY_CONSTRUCTOR = 0x00000800;
        /// Type cannot be inherited from
        const NOINHERIT = 0x00001000;
        /// Type cannot be stored (local variables only)
        const NOSTORE = 0x00002000;
        /// All members are integers
        const APP_CLASS_ALLINTS = 0x00004000;
        /// All members are floats
        const APP_CLASS_ALLFLOATS = 0x00008000;
        /// Type requires 8-byte alignment
        const APP_CLASS_ALIGN8 = 0x00010000;
        /// Type doesn't use reference counting
        const NOCOUNT = 0x00020000;
        /// Shorthand for class with all features
        const APP_CLASS_CDAK = Self::APP_CLASS_CONSTRUCTOR.bits() |
                               Self::APP_CLASS_DESTRUCTOR.bits() |
                               Self::APP_CLASS_ASSIGNMENT.bits() |
                               Self::APP_CLASS_COPY_CONSTRUCTOR.bits();
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ParamFlags: u32 {
        const IN = 0x0001;
        const OUT = 0x0002;
        const INOUT = 0x0003;
        const CONST = 0x0004;
    }
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
                next_type_id: AtomicU32::new(100),
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

        let type_id = inner
            .next_type_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        inner.object_types.insert(
            name.to_string(),
            ObjectType {
                type_id,
                name: name.to_string(),
                flags,
                properties: Vec::new(),
                methods: Vec::new(),
                behaviours: Vec::new(),
                destructor: None,
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
            is_readonly: sig.is_const,
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

        let function_id = inner
            .next_type_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let obj_type = inner
            .object_types
            .get_mut(type_name)
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        if behaviour == BehaviourType::Destruct {
            obj_type.destructor = Some(function_id);
        }

        obj_type.behaviours.push(BehaviourInfo {
            behaviour_type: behaviour,
            function_id,
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

        let type_id = inner
            .next_type_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

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
    pub fn get_module(&mut self, _name: &str, _flag: GetModuleFlag) -> Option<&mut Module> {
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
