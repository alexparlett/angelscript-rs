//! Type Registry
//!
//! This module provides the central registry for all types in the AngelScript engine.
//! It stores type information, function signatures, and global variables.
//!
//! Note: Native function implementations (callables) are NOT stored here.
//! They are stored in SystemFunctionRegistry (in the callfunc module).
//! This registry only stores metadata and FunctionIds which can be used
//! to look up the actual callables.

use crate::core::engine_properties::EngineProperty;
use crate::core::span::Span;
use crate::core::types::{
    AccessSpecifier, BehaviourType, FunctionId, ModuleId, ScriptValue, TYPE_BOOL, TYPE_DOUBLE,
    TYPE_FLOAT, TYPE_INT8, TYPE_INT16, TYPE_INT32, TYPE_INT64, TYPE_STRING, TYPE_UINT8,
    TYPE_UINT16, TYPE_UINT32, TYPE_UINT64, TYPE_VOID, TypeFlags, TypeId, TypeKind,
    TypeRegistration,
};
use crate::parser::ast::Expr;
use std::any::TypeId as StdTypeId;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TypeRegistry {
    types: HashMap<TypeId, Arc<TypeInfo>>,
    types_by_name: HashMap<String, TypeId>,
    functions: HashMap<FunctionId, Arc<FunctionInfo>>,
    globals: HashMap<String, Arc<GlobalInfo>>,

    properties: HashMap<EngineProperty, usize>,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub name: String,
    pub namespace: Vec<String>,
    pub kind: TypeKind,
    pub flags: TypeFlags,
    pub registration: TypeRegistration,

    pub properties: Vec<PropertyInfo>,
    pub methods: HashMap<String, Vec<MethodSignature>>,

    pub base_type: Option<TypeId>,
    pub interfaces: Vec<TypeId>,

    /// Behaviours map BehaviourType -> FunctionId
    /// The actual callables are stored in SystemFunctionRegistry
    pub behaviours: HashMap<BehaviourType, FunctionId>,

    /// The Rust TypeId for application-registered types
    pub rust_type_id: Option<StdTypeId>,

    pub vtable: Vec<VTableEntry>,

    pub definition_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub type_id: TypeId,
    pub offset: Option<usize>,
    pub access: AccessSpecifier,
    pub flags: PropertyFlags,

    /// FunctionId of the getter (if any)
    /// The actual callable is stored in SystemFunctionRegistry
    pub getter: Option<FunctionId>,
    
    /// FunctionId of the setter (if any)
    /// The actual callable is stored in SystemFunctionRegistry
    pub setter: Option<FunctionId>,

    pub definition_span: Option<Span>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PropertyFlags: u32 {
        const CONST = 0x01;
        const PRIVATE = 0x02;
        const PROTECTED = 0x04;
        const PUBLIC = 0x08;
        const VIRTUAL = 0x10;
        const INHERITED = 0x20;
    }
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub function_id: FunctionId,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub function_id: FunctionId,
    pub name: String,
    pub full_name: String,
    pub namespace: Vec<String>,

    pub return_type: TypeId,
    pub return_flags: ReturnFlags,
    pub parameters: Vec<ParameterInfo>,

    pub kind: FunctionKind,
    pub flags: FunctionFlags,

    pub owner_type: Option<TypeId>,
    pub vtable_index: Option<usize>,

    pub implementation: FunctionImpl,

    pub definition_span: Option<Span>,

    pub locals: Vec<LocalVarInfo>,

    pub bytecode_address: Option<u32>,
    pub local_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    Global,
    Method { is_const: bool },
    Constructor,
    Destructor,
    Operator(OperatorType),
    Conversion,
    Lambda,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorType {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Equals,
    Compare,
    Index,
    Call,
    Assign,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FunctionFlags: u32 {
        const CONST = 0x01;
        const VIRTUAL = 0x02;
        const FINAL = 0x04;
        const OVERRIDE = 0x08;
        const ABSTRACT = 0x10;
        const PRIVATE = 0x20;
        const PROTECTED = 0x40;
        const PUBLIC = 0x80;
        const EXPLICIT = 0x100;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReturnFlags: u32 {
        const REF = 0x01;
        const CONST = 0x02;
        const AUTO_HANDLE = 0x04;
    }
}

#[derive(Debug, Clone)]
pub enum FunctionImpl {
    /// Native function - the actual callable is stored in SystemFunctionRegistry
    /// system_id is the FunctionId used to look up the callable
    Native {
        system_id: u32,
    },
    /// Script function - bytecode is stored in the module
    Script {
        bytecode_offset: u32,
        module_id: ModuleId,
    },
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: Option<String>,
    pub type_id: TypeId,
    pub flags: ParameterFlags,
    pub default_expr: Option<Arc<Expr>>,
    pub definition_span: Option<Span>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ParameterFlags: u32 {
        const IN = 0x01;
        const OUT = 0x02;
        const INOUT = 0x04;
        const CONST = 0x08;
        const AUTO_HANDLE = 0x010;
    }
}

#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_const: bool,
    pub is_param: bool,
    pub index: usize,

    pub definition_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct GlobalInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_const: bool,
    pub address: u32,

    pub definition_span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct VTableEntry {
    pub method_name: String,
    pub function_id: FunctionId,
    pub override_of: Option<FunctionId>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            types: HashMap::new(),
            types_by_name: HashMap::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            properties: HashMap::new(),
        };

        registry.init_properties();
        registry.register_primitives();

        registry
    }

    fn init_properties(&mut self) {
        for prop in [
            EngineProperty::AllowUnsafeReferences,
            EngineProperty::OptimizeBytecode,
            EngineProperty::BuildWithoutLineCues,
            EngineProperty::IncludeDebugInfo,
            EngineProperty::TrackLocalScopes,
            EngineProperty::StoreDocComments,
            EngineProperty::MaxCallStackSize,
            EngineProperty::InitContextStackSize,
            EngineProperty::UseCharacterLiterals,
            EngineProperty::AllowMultilineStrings,
            EngineProperty::DisallowEmptyListElements,
            EngineProperty::DisallowValueAssignForRefType,
            EngineProperty::AlwaysImplDefaultConstruct,
            EngineProperty::CompilerWarnings,
            EngineProperty::DisallowGlobalVars,
            EngineProperty::RequireEnumScope,
        ] {
            self.properties.insert(prop, prop.default_value());
        }
    }

    fn register_primitives(&mut self) {
        let primitives = [
            ("void", TYPE_VOID),
            ("bool", TYPE_BOOL),
            ("int8", TYPE_INT8),
            ("int16", TYPE_INT16),
            ("int", TYPE_INT32),
            ("int64", TYPE_INT64),
            ("uint8", TYPE_UINT8),
            ("uint16", TYPE_UINT16),
            ("uint", TYPE_UINT32),
            ("uint64", TYPE_UINT64),
            ("float", TYPE_FLOAT),
            ("double", TYPE_DOUBLE),
            ("string", TYPE_STRING),
        ];

        for (name, type_id) in primitives {
            let type_info = TypeInfo {
                type_id,
                name: name.to_string(),
                namespace: Vec::new(),
                kind: TypeKind::Primitive,
                flags: if name == "string" {
                    TypeFlags::VALUE_TYPE
                } else {
                    TypeFlags::VALUE_TYPE | TypeFlags::POD_TYPE
                },
                registration: TypeRegistration::Application,

                properties: Vec::new(),
                methods: HashMap::new(),
                base_type: None,
                interfaces: Vec::new(),
                behaviours: HashMap::new(),

                rust_type_id: None,

                vtable: Vec::new(),

                definition_span: None,
            };

            self.types_by_name.insert(name.to_string(), type_id);
            self.types.insert(type_id, Arc::new(type_info));
        }
    }

    pub fn set_property(&mut self, property: EngineProperty, value: usize) {
        self.properties.insert(property, value);
    }

    pub fn get_property(&self, property: EngineProperty) -> usize {
        self.properties
            .get(&property)
            .copied()
            .unwrap_or_else(|| property.default_value())
    }

    pub fn register_type(&mut self, type_info: TypeInfo) -> Result<TypeId, String> {
        let type_id = type_info.type_id;
        let name = type_info.name.clone();

        if self.types_by_name.contains_key(&name) {
            return Err(format!("Type '{}' already registered", name));
        }

        self.types_by_name.insert(name, type_id);
        self.types.insert(type_id, Arc::new(type_info));

        Ok(type_id)
    }

    pub fn get_type(&self, type_id: TypeId) -> Option<Arc<TypeInfo>> {
        self.types.get(&type_id).cloned()
    }

    pub fn lookup_type(&self, name: &str, namespace: &[String]) -> Option<TypeId> {
        if let Some(&type_id) = self.types_by_name.get(name) {
            return Some(type_id);
        }

        for i in (0..=namespace.len()).rev() {
            let qualified_name = if i == 0 {
                name.to_string()
            } else {
                format!("{}::{}", namespace[..i].join("::"), name)
            };

            if let Some(&type_id) = self.types_by_name.get(&qualified_name) {
                return Some(type_id);
            }
        }

        None
    }

    pub fn add_property(&mut self, type_id: TypeId, property: PropertyInfo) -> Result<(), String> {
        let type_info = self
            .types
            .get_mut(&type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let type_info = Arc::make_mut(type_info);
        type_info.properties.push(property);

        Ok(())
    }

    pub fn add_method(
        &mut self,
        type_id: TypeId,
        method_name: String,
        function_id: FunctionId,
    ) -> Result<(), String> {
        let type_info = self
            .types
            .get_mut(&type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let type_info = Arc::make_mut(type_info);
        type_info
            .methods
            .entry(method_name)
            .or_insert_with(Vec::new)
            .push(MethodSignature { function_id });

        Ok(())
    }

    pub fn add_behaviour(
        &mut self,
        type_id: TypeId,
        behaviour: BehaviourType,
        function_id: FunctionId,
    ) -> Result<(), String> {
        let type_info = self
            .types
            .get_mut(&type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let type_info = Arc::make_mut(type_info);
        type_info.behaviours.insert(behaviour, function_id);

        Ok(())
    }

    pub fn register_function(&mut self, func_info: FunctionInfo) -> Result<FunctionId, String> {
        let function_id = func_info.function_id;

        if self.functions.contains_key(&function_id) {
            return Err(format!("Function {} already registered", function_id));
        }

        self.functions.insert(function_id, Arc::new(func_info));
        Ok(function_id)
    }

    pub fn resolve_typedef(&self, type_id: TypeId) -> TypeId {
        let mut current_id = type_id;
        let mut visited = std::collections::HashSet::new();

        // Follow typedef chain (with cycle detection)
        while let Some(type_info) = self.get_type(current_id) {
            if !visited.insert(current_id) {
                // Cycle detected!
                return type_id;
            }

            if type_info.kind == TypeKind::Typedef {
                if let Some(aliased_id) = type_info.base_type {
                    current_id = aliased_id;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        current_id
    }

    pub fn get_function(&self, function_id: FunctionId) -> Option<Arc<FunctionInfo>> {
        self.functions.get(&function_id).cloned()
    }

    pub fn get_functions_by_name(&self, name: &str) -> Vec<Arc<FunctionInfo>> {
        self.functions
            .values()
            .filter(|f| f.name == name)
            .cloned()
            .collect()
    }

    pub fn get_methods_by_name(
        &self,
        type_id: TypeId,
        method_name: &str,
    ) -> Vec<Arc<FunctionInfo>> {
        let type_info = match self.get_type(type_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let method_sigs = match type_info.get_method(method_name) {
            Some(m) => m,
            None => return Vec::new(),
        };

        method_sigs
            .iter()
            .filter_map(|sig| self.get_function(sig.function_id))
            .collect()
    }

    pub fn find_function(&self, name: &str, namespace: &[String]) -> Option<Arc<FunctionInfo>> {
        self.functions
            .values()
            .find(|f| {
                if f.namespace == namespace && f.name == name {
                    return true;
                }

                if f.full_name == name {
                    return true;
                }

                false
            })
            .cloned()
    }

    pub fn update_function_address(
        &mut self,
        function_id: FunctionId,
        address: u32,
    ) -> Result<(), String> {
        let func = self
            .functions
            .get_mut(&function_id)
            .ok_or_else(|| format!("Function {} not found", function_id))?;

        let func = Arc::make_mut(func);
        func.bytecode_address = Some(address);

        Ok(())
    }

    pub fn update_function_locals(
        &mut self,
        function_id: FunctionId,
        locals: Vec<LocalVarInfo>,
    ) -> Result<(), String> {
        let func = self
            .functions
            .get_mut(&function_id)
            .ok_or_else(|| format!("Function {} not found", function_id))?;

        let func = Arc::make_mut(func);
        func.local_count = locals.len() as u32;
        func.locals = locals;

        Ok(())
    }

    pub fn register_global(&mut self, global_info: GlobalInfo) -> Result<(), String> {
        let name = global_info.name.clone();

        if self.globals.contains_key(&name) {
            return Err(format!("Global '{}' already registered", name));
        }

        self.globals.insert(name, Arc::new(global_info));
        Ok(())
    }

    pub fn get_global(&self, name: &str) -> Option<Arc<GlobalInfo>> {
        self.globals.get(name).cloned()
    }

    pub fn get_next_global_address(&self) -> u32 {
        self.globals.len() as u32
    }

    pub fn get_type_count(&self) -> u32 {
        self.types.len() as u32
    }

    pub fn get_type_by_index(&self, index: u32) -> Option<TypeId> {
        self.types.keys().nth(index as usize).copied()
    }

    pub fn get_all_types(&self) -> Vec<Arc<TypeInfo>> {
        self.types.values().cloned().collect()
    }

    pub fn get_all_functions(&self) -> Vec<Arc<FunctionInfo>> {
        self.functions.values().cloned().collect()
    }

    pub fn get_all_globals(&self) -> Vec<Arc<GlobalInfo>> {
        self.globals.values().cloned().collect()
    }

    pub fn update_vtable(
        &mut self,
        type_id: TypeId,
        vtable: Vec<VTableEntry>,
    ) -> Result<(), String> {
        let type_info = self
            .types
            .get_mut(&type_id)
            .ok_or_else(|| format!("Type {} not found", type_id))?;

        let type_info = Arc::make_mut(type_info);
        type_info.vtable = vtable;

        Ok(())
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInfo {
    pub fn is_value_type(&self) -> bool {
        self.flags.contains(TypeFlags::VALUE_TYPE)
    }

    pub fn is_ref_type(&self) -> bool {
        self.flags.contains(TypeFlags::REF_TYPE)
    }

    pub fn is_pod(&self) -> bool {
        self.flags.contains(TypeFlags::POD_TYPE)
    }

    pub fn can_be_handle(&self) -> bool {
        !self.flags.contains(TypeFlags::NOHANDLE)
    }

    pub fn can_be_inherited(&self) -> bool {
        !self.flags.contains(TypeFlags::NOINHERIT)
    }

    pub fn is_abstract(&self) -> bool {
        self.flags.contains(TypeFlags::ABSTRACT)
    }

    pub fn get_property(&self, name: &str) -> Option<&PropertyInfo> {
        self.properties.iter().find(|p| p.name == name)
    }

    pub fn get_method(&self, name: &str) -> Option<&Vec<MethodSignature>> {
        self.methods.get(name)
    }

    pub fn get_behaviour(&self, behaviour: BehaviourType) -> Option<FunctionId> {
        self.behaviours.get(&behaviour).copied()
    }
}