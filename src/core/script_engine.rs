use crate::core::engine_properties::EngineProperty;
use crate::core::script_module::ScriptModule;
use crate::core::type_registry::{
    FunctionFlags, FunctionImpl, FunctionInfo, FunctionKind, GlobalInfo, ParameterInfo,
    PropertyFlags, PropertyInfo, TypeInfo, TypeRegistry,
};
use crate::core::types::{
    allocate_function_id, allocate_type_id, AccessSpecifier, BehaviourType, FunctionId, TypeFlags, TypeId,
    TypeKind, TypeRegistration, TYPE_VOID,
};
use crate::parser::declaration_parser::DeclarationParser;
use std::any::TypeId as StdTypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ScriptEngine {
    pub registry: Arc<RwLock<TypeRegistry>>,
    modules: HashMap<String, Box<ScriptModule>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetModuleFlag {
    OnlyIfExists,
    CreateIfNotExists,
    AlwaysCreate,
}

impl ScriptEngine {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(TypeRegistry::new())),
            modules: HashMap::new(),
        }
    }

    pub fn set_engine_property(
        &mut self,
        property: EngineProperty,
        value: usize,
    ) -> Result<(), String> {
        let mut registry = self.registry.write().unwrap();
        registry.set_property(property, value);
        Ok(())
    }

    pub fn get_engine_property(&self, property: EngineProperty) -> usize {
        let registry = self.registry.read().unwrap();
        registry.get_property(property)
    }

    pub fn register_object_type<T: 'static>(
        &mut self,
        name: &str,
        flags: TypeFlags,
    ) -> Result<u32, String> {
        let rust_type_id = StdTypeId::of::<T>();

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: name.to_string(),
            namespace: Vec::new(),
            kind: TypeKind::Class,
            flags,
            registration: TypeRegistration::Application,

            properties: Vec::new(),
            methods: HashMap::new(),

            base_type: None,
            interfaces: Vec::new(),

            behaviours: HashMap::new(),

            rust_type_id: Some(rust_type_id),
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),

            vtable: Vec::new(),

            definition_span: None,
        };

        let mut registry = self.registry.write().unwrap();
        registry.register_type(type_info)
    }

    pub fn register_object_property(
        &mut self,
        type_name: &str,
        declaration: &str,
    ) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_property_declaration(declaration)?;

        let mut registry = self.registry.write().unwrap();

        let type_id = registry
            .lookup_type(type_name, &[])
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        registry.add_property(
            type_id,
            PropertyInfo {
                name: sig.name,
                type_id: sig.type_id,
                offset: None,
                access: AccessSpecifier::Public,
                flags: if sig.is_const {
                    PropertyFlags::PUBLIC | PropertyFlags::CONST
                } else {
                    PropertyFlags::PUBLIC
                },
                getter: None,
                setter: None,
                definition_span: None,
            },
        )
    }

    pub fn register_object_method(
        &mut self,
        type_name: &str,
        declaration: &str,
    ) -> Result<FunctionId, String> {
        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_function_declaration(declaration)?;

        let mut registry = self.registry.write().unwrap();

        let type_id = registry
            .lookup_type(type_name, &[])
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        let function_id = allocate_function_id();

        let func_info = FunctionInfo {
            function_id,
            name: sig.name.clone(),
            full_name: format!("{}::{}", type_name, sig.name),
            namespace: Vec::new(),

            return_type: sig.return_type_id,
            return_is_ref: sig.is_ref,
            parameters: sig
                .parameters
                .into_iter()
                .map(|p| ParameterInfo {
                    name: p.name,
                    type_id: p.type_id,
                    flags: p.flags,
                    default_expr: p.default_expr,
                    definition_span: p.definition_span,
                })
                .collect(),

            kind: FunctionKind::Method {
                is_const: sig.is_const,
            },
            flags: if sig.is_const {
                FunctionFlags::PUBLIC | FunctionFlags::CONST
            } else {
                FunctionFlags::PUBLIC
            },

            owner_type: Some(type_id),
            vtable_index: None,

            implementation: FunctionImpl::Native {
                system_id: function_id,
            },

            definition_span: None,

            locals: Vec::new(),

            bytecode_address: None,
            local_count: 0,
        };

        registry.add_method(type_id, sig.name, function_id)?;
        registry.register_function(func_info)
    }

    pub fn register_object_behaviour(
        &mut self,
        type_name: &str,
        behaviour: BehaviourType,
        declaration: &str,
    ) -> Result<FunctionId, String> {
        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_behaviour_declaration(declaration)?;

        let mut registry = self.registry.write().unwrap();

        let type_id = registry
            .lookup_type(type_name, &[])
            .ok_or_else(|| format!("Type '{}' not found", type_name))?;

        let function_id = allocate_function_id();

        let behaviour_name = format!("{:?}", behaviour);

        let func_info = FunctionInfo {
            function_id,
            name: behaviour_name.clone(),
            full_name: format!("{}::{}", type_name, behaviour_name),
            namespace: Vec::new(),

            return_type: match behaviour {
                BehaviourType::Construct | BehaviourType::ListFactory => type_id,
                _ => TYPE_VOID,
            },
            return_is_ref: sig.is_ref,
            parameters: sig
                .parameters
                .into_iter()
                .map(|p| ParameterInfo {
                    name: p.name,
                    type_id: p.type_id,
                    flags: p.flags,
                    default_expr: p.default_expr,
                    definition_span: p.definition_span,
                })
                .collect(),

            kind: match behaviour {
                BehaviourType::Construct => FunctionKind::Constructor,
                BehaviourType::Destruct => FunctionKind::Destructor,
                _ => FunctionKind::Method { is_const: false },
            },
            flags: FunctionFlags::PUBLIC,

            owner_type: Some(type_id),
            vtable_index: None,

            implementation: FunctionImpl::Native {
                system_id: function_id,
            },

            definition_span: None,

            locals: Vec::new(),

            bytecode_address: None,
            local_count: 0,
        };

        registry.add_behaviour(type_id, behaviour, function_id)?;
        registry.register_function(func_info)
    }

    pub fn register_global_function(&mut self, declaration: &str) -> Result<FunctionId, String> {
        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_function_declaration(declaration)?;

        let function_id = allocate_function_id();

        let func_info = FunctionInfo {
            function_id,
            name: sig.name.clone(),
            full_name: sig.name,
            namespace: Vec::new(),

            return_type: sig.return_type_id,
            return_is_ref: sig.is_ref,
            parameters: sig
                .parameters
                .into_iter()
                .map(|p| ParameterInfo {
                    name: p.name,
                    type_id: p.type_id,
                    flags: p.flags,
                    default_expr: p.default_expr,
                    definition_span: p.definition_span,
                })
                .collect(),

            kind: FunctionKind::Global,
            flags: FunctionFlags::PUBLIC,

            owner_type: None,
            vtable_index: None,

            implementation: FunctionImpl::Native {
                system_id: function_id,
            },

            definition_span: None,

            locals: Vec::new(),

            bytecode_address: None,
            local_count: 0,
        };

        let mut registry = self.registry.write().unwrap();
        registry.register_function(func_info)
    }

    pub fn register_funcdef(&mut self, declaration: &str) -> Result<TypeId, String> {
        let decl = declaration
            .trim()
            .strip_prefix("funcdef")
            .unwrap_or(declaration)
            .trim();

        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_function_declaration(decl)?;

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: sig.name,
            namespace: Vec::new(),
            kind: TypeKind::Funcdef,
            flags: TypeFlags::FUNCDEF,
            registration: TypeRegistration::Application,

            properties: Vec::new(),
            methods: HashMap::new(),
            base_type: None,
            interfaces: Vec::new(),
            behaviours: HashMap::new(),

            rust_type_id: None,
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),

            vtable: Vec::new(),

            definition_span: None,
        };

        let mut registry = self.registry.write().unwrap();

        registry.register_type(type_info)
    }

    pub fn register_global_property(&mut self, declaration: &str) -> Result<(), String> {
        let parser = DeclarationParser::new(Arc::clone(&self.registry));
        let sig = parser.parse_property_declaration(declaration)?;

        let mut registry = self.registry.write().unwrap();
        let address = registry.get_next_global_address();

        let global_info = GlobalInfo {
            name: sig.name,
            type_id: sig.type_id,
            is_const: sig.is_const,
            address,
            definition_span: None,
        };

        registry.register_global(global_info)
    }

    pub fn register_enum(
        &mut self,
        name: &str,
        values: Vec<(&str, i32)>,
    ) -> Result<TypeId, String> {
        let mut registry = self.registry.write().unwrap();

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: name.to_string(),
            namespace: Vec::new(),
            kind: TypeKind::Enum,
            flags: TypeFlags::ENUM | TypeFlags::VALUE_TYPE,
            registration: TypeRegistration::Application,

            properties: values
                .into_iter()
                .map(|(name, value)| PropertyInfo {
                    name: name.to_string(),
                    type_id: crate::core::types::TYPE_INT32,
                    offset: Some(value as usize),
                    access: AccessSpecifier::Public,
                    flags: PropertyFlags::PUBLIC | PropertyFlags::CONST,
                    getter: None,
                    setter: None,
                    definition_span: None,
                })
                .collect(),

            methods: HashMap::new(),
            base_type: None,
            interfaces: vec![],
            behaviours: HashMap::new(),

            rust_type_id: None,
            rust_accessors: HashMap::new(),
            rust_methods: HashMap::new(),

            vtable: vec![],

            definition_span: None,
        };

        registry.register_type(type_info)
    }

    pub fn get_module(&mut self, name: &str, flag: GetModuleFlag) -> Option<&mut ScriptModule> {
        match flag {
            GetModuleFlag::OnlyIfExists => self.modules.get_mut(name).map(|m| m.as_mut()),
            GetModuleFlag::CreateIfNotExists => {
                if !self.modules.contains_key(name) {
                    let module = ScriptModule::new(name.to_string(), Arc::clone(&self.registry));
                    self.modules.insert(name.to_string(), Box::new(module));
                }
                self.modules.get_mut(name).map(|m| m.as_mut())
            }
            GetModuleFlag::AlwaysCreate => {
                let module = ScriptModule::new(name.to_string(), Arc::clone(&self.registry));
                self.modules.insert(name.to_string(), Box::new(module));
                self.modules.get_mut(name).map(|m| m.as_mut())
            }
        }
    }

    pub fn discard_module(&mut self, name: &str) {
        self.modules.remove(name);
    }

    pub fn get_type_count(&self) -> u32 {
        let registry = self.registry.read().unwrap();
        registry.get_type_count()
    }

    pub fn get_type_by_index(&self, index: u32) -> Option<TypeId> {
        let registry = self.registry.read().unwrap();
        registry.get_type_by_index(index)
    }

    pub fn get_type_info_by_id(&self, type_id: TypeId) -> Option<Arc<TypeInfo>> {
        let registry = self.registry.read().unwrap();
        registry.get_type(type_id)
    }

    pub fn get_type_info_by_name(&self, name: &str) -> Option<Arc<TypeInfo>> {
        let registry = self.registry.read().unwrap();
        let type_id = registry.lookup_type(name, &[])?;
        registry.get_type(type_id)
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
