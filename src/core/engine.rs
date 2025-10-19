use std::any::TypeId as StdTypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::core::context::Context;
use crate::core::module::Module;

/// The main script engine that manages the entire AngelScript system
pub struct ScriptEngine {
    inner: Arc<RwLock<EngineInner>>,
}

pub struct EngineInner {
    /// Registered object types
    pub object_types: HashMap<String, ObjectTypeInfo>,
    /// Registered interface types
    pub interface_types: HashMap<String, InterfaceTypeInfo>,
    /// Registered enum types
    pub enum_types: HashMap<String, EnumTypeInfo>,
    /// Registered global functions
    pub global_functions: HashMap<String, FunctionInfo>,
    /// Registered global properties
    pub global_properties: HashMap<String, PropertyInfo>,
    /// Type ID counter
    pub next_type_id: u32,
    /// Modules
    pub modules: HashMap<String, Box<Module>>,
    /// Map Rust TypeId to script type ID
    rust_type_map: HashMap<StdTypeId, u32>,
}

#[derive(Debug, Clone)]
pub struct ObjectTypeInfo {
    pub type_id: u32,
    pub name: String,
    pub size: usize,
    pub flags: TypeFlags,
    pub properties: Vec<PropertyInfo>,
    pub methods: Vec<MethodInfo>,
    pub behaviours: Vec<BehaviourInfo>,
    pub rust_type_id: Option<StdTypeId>,
}

#[derive(Debug, Clone)]
pub struct InterfaceTypeInfo {
    pub type_id: u32,
    pub name: String,
    pub methods: Vec<MethodInfo>,
}

#[derive(Debug, Clone)]
pub struct EnumTypeInfo {
    pub type_id: u32,
    pub name: String,
    pub values: Vec<(String, i32)>,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub type_id: u32,
    pub offset: usize,
    pub access: AccessSpecifier,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<ParamInfo>,
    pub access: AccessSpecifier,
    pub is_const: bool,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: Option<String>,
    pub type_id: u32,
    pub flags: ParamFlags,
}

#[derive(Debug, Clone)]
pub struct BehaviourInfo {
    pub behaviour_type: BehaviourType,
    pub function_id: u32,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub return_type_id: u32,
    pub params: Vec<ParamInfo>,
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
    Destruct,
    Factory,
    AddRef,
    Release,
    GetRefCount,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TypeFlags: u32 {
        const VALUE_TYPE = 0x0001;
        const REF_TYPE = 0x0002;
        const GC_TYPE = 0x0004;
        const POD_TYPE = 0x0008;
        const HANDLE = 0x0010;
        const TEMPLATE = 0x0020;
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
        let mut engine = Self {
            inner: Arc::new(RwLock::new(EngineInner {
                object_types: HashMap::new(),
                interface_types: HashMap::new(),
                enum_types: HashMap::new(),
                global_functions: HashMap::new(),
                global_properties: HashMap::new(),
                next_type_id: 100,
                modules: HashMap::new(),
                rust_type_map: HashMap::new(),
            })),
        };

        engine.register_primitive_types();
        engine
    }

    /// Register built-in primitive types
    fn register_primitive_types(&mut self) {
        self.register_object_type_raw("void", 0, TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("bool", std::mem::size_of::<bool>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("int8", std::mem::size_of::<i8>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("int16", std::mem::size_of::<i16>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("int32", std::mem::size_of::<i32>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("int64", std::mem::size_of::<i64>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("int", std::mem::size_of::<i32>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("uint8", std::mem::size_of::<u8>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("uint16", std::mem::size_of::<u16>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("uint32", std::mem::size_of::<u32>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("uint64", std::mem::size_of::<u64>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("uint", std::mem::size_of::<u32>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("float", std::mem::size_of::<f32>(), TypeFlags::POD_TYPE)
            .unwrap();
        self.register_object_type_raw("double", std::mem::size_of::<f64>(), TypeFlags::POD_TYPE)
            .unwrap();
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
        let size = std::mem::size_of::<T>();
        let rust_type_id = StdTypeId::of::<T>();

        let mut inner = self.inner.write().unwrap();

        if inner.object_types.contains_key(name) {
            return Err(format!("Type '{}' already registered", name));
        }

        let type_id = inner.next_type_id;
        inner.next_type_id += 1;

        inner.rust_type_map.insert(rust_type_id, type_id);

        inner.object_types.insert(
            name.to_string(),
            ObjectTypeInfo {
                type_id,
                name: name.to_string(),
                size,
                flags,
                properties: Vec::new(),
                methods: Vec::new(),
                behaviours: Vec::new(),
                rust_type_id: Some(rust_type_id),
            },
        );

        Ok(type_id)
    }

    /// Register an object type with explicit size (for opaque types or special cases)
    pub fn register_object_type_raw(
        &mut self,
        name: &str,
        size: usize,
        flags: TypeFlags,
    ) -> Result<u32, String> {
        let mut inner = self.inner.write().unwrap();

        if inner.object_types.contains_key(name) {
            return Err(format!("Type '{}' already registered", name));
        }

        let type_id = inner.next_type_id;
        inner.next_type_id += 1;

        inner.object_types.insert(
            name.to_string(),
            ObjectTypeInfo {
                type_id,
                name: name.to_string(),
                size,
                flags,
                properties: Vec::new(),
                methods: Vec::new(),
                behaviours: Vec::new(),
                rust_type_id: None,
            },
        );

        Ok(type_id)
    }

    /// Register an interface type
    pub fn register_interface_type(&mut self, name: &str) -> Result<u32, String> {
        let mut inner = self.inner.write().unwrap();

        if inner.interface_types.contains_key(name) {
            return Err(format!("Interface '{}' already registered", name));
        }

        let type_id = inner.next_type_id;
        inner.next_type_id += 1;

        inner.interface_types.insert(
            name.to_string(),
            InterfaceTypeInfo {
                type_id,
                name: name.to_string(),
                methods: Vec::new(),
            },
        );

        Ok(type_id)
    }

    /// Register an enum type
    pub fn register_enum_type(&mut self, name: &str) -> Result<u32, String> {
        let mut inner = self.inner.write().unwrap();

        if inner.enum_types.contains_key(name) {
            return Err(format!("Enum '{}' already registered", name));
        }

        let type_id = inner.next_type_id;
        inner.next_type_id += 1;

        inner.enum_types.insert(
            name.to_string(),
            EnumTypeInfo {
                type_id,
                name: name.to_string(),
                values: Vec::new(),
            },
        );

        Ok(type_id)
    }

    /// Register an enum value
    pub fn register_enum_value(
        &mut self,
        enum_name: &str,
        value_name: &str,
        value: i32,
    ) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();

        let enum_info = inner
            .enum_types
            .get_mut(enum_name)
            .ok_or_else(|| format!("Enum '{}' not found", enum_name))?;

        enum_info.values.push((value_name.to_string(), value));
        Ok(())
    }

    /// Register an object method
    pub fn register_object_method(
        &mut self,
        type_name: &str,
        declaration: &str,
    ) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();

        let obj_type = inner
            .object_types
            .get_mut(type_name)
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        // TODO: Parse declaration properly
        obj_type.methods.push(MethodInfo {
            name: "method".to_string(),
            return_type_id: 0,
            params: Vec::new(),
            access: AccessSpecifier::Public,
            is_const: false,
        });

        Ok(())
    }

    /// Register a global function
    pub fn register_global_function(&mut self, declaration: &str) -> Result<(), String> {
        let mut inner = self.inner.write().unwrap();

        // TODO: Parse declaration properly
        inner.global_functions.insert(
            "function".to_string(),
            FunctionInfo {
                name: "function".to_string(),
                return_type_id: 0,
                params: Vec::new(),
            },
        );

        Ok(())
    }

    // pub fn register_global_property<T: PropertyValue + Clone + 'static>(
    //     &self,
    //     name: &str,
    //     initial_value: T
    // ) -> crate::core::types::Result<GlobalPropertyHandle<T>> {
    //     let mut inner = self.inner.write().unwrap();
    //
    //     let boxed_value = Box::new(initial_value.clone());
    //     let id = inner.next_property_id.fetch_add(1, Ordering::SeqCst);
    //
    //     let property = Arc::new(GlobalProperty::new_with_id(
    //         name.to_string(),
    //         boxed_value,
    //         id
    //     ));
    //     property.add_ref();
    //
    //     inner.global_properties.insert(name.to_string(), Arc::clone(&property));
    //
    //     Ok(GlobalPropertyHandle::new(property))
    // }

    /// Get a module by name
    pub fn get_module(&mut self, name: &str, flag: GetModuleFlag) -> Option<&mut Module> {
        let mut inner = self.inner.write().unwrap();

        match flag {
            GetModuleFlag::OnlyIfExists => inner.modules.get_mut(name).map(|b| b.as_mut()),

            GetModuleFlag::CreateIfNotExists => {
                if !inner.modules.contains_key(name) {
                    let module =
                        Box::new(Module::new(name.to_string(), Arc::downgrade(&self.inner)));
                    inner.modules.insert(name.to_string(), module);
                }
                inner.modules.get_mut(name).map(|b| b.as_mut())
            }

            GetModuleFlag::AlwaysCreate => {
                if let Some(existing) = inner.modules.get_mut(name) {
                    existing.discard();
                }

                let module = Box::new(Module::new(name.to_string(), Arc::downgrade(&self.inner)));
                inner.modules.insert(name.to_string(), module);
                inner.modules.get_mut(name).map(|b| b.as_mut())
            }
        }
    }

    /// Discard a module
    pub fn discard_module(&mut self, name: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.modules.remove(name);
    }

    /// Create a new execution context
    pub fn create_context(&self) -> Context {
        Context::new()
    }
}

impl EngineInner {
    /// Look up a type by name
    pub fn get_type_id(&self, name: &str) -> Option<u32> {
        if let Some(obj_type) = self.object_types.get(name) {
            return Some(obj_type.type_id);
        }
        if let Some(iface_type) = self.interface_types.get(name) {
            return Some(iface_type.type_id);
        }
        if let Some(enum_type) = self.enum_types.get(name) {
            return Some(enum_type.type_id);
        }
        None
    }

    /// Get object type info
    pub fn get_object_type(&self, name: &str) -> Option<&ObjectTypeInfo> {
        self.object_types.get(name)
    }

    /// Get interface type info
    pub fn get_interface_type(&self, name: &str) -> Option<&InterfaceTypeInfo> {
        self.interface_types.get(name)
    }

    /// Get enum type info
    pub fn get_enum_type(&self, name: &str) -> Option<&EnumTypeInfo> {
        self.enum_types.get(name)
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
