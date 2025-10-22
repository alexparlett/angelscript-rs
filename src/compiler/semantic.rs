use crate::compiler::symbol::{Symbol, SymbolKind, SymbolTable};
use crate::core::engine;
use crate::core::engine::{EngineInner, InterfaceInfo, TypeFlags};
use crate::parser::ast::*;
use engine::GlobalFunction;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
// ==================== TYPE CONSTANTS ====================

pub type TypeId = u32;

pub const TYPE_VOID: TypeId = 0;
pub const TYPE_BOOL: TypeId = 1;
pub const TYPE_INT8: TypeId = 2;
pub const TYPE_INT16: TypeId = 3;
pub const TYPE_INT32: TypeId = 4;
pub const TYPE_INT64: TypeId = 5;
pub const TYPE_UINT8: TypeId = 6;
pub const TYPE_UINT16: TypeId = 7;
pub const TYPE_UINT32: TypeId = 8;
pub const TYPE_UINT64: TypeId = 9;
pub const TYPE_FLOAT: TypeId = 10;
pub const TYPE_DOUBLE: TypeId = 11;
pub const TYPE_STRING: TypeId = 12;
pub const TYPE_AUTO: TypeId = 13;

// ==================== EXPRESSION CONTEXT ====================

/// Context information about an expression after semantic analysis
#[derive(Debug, Clone, PartialEq)]
pub struct ExprContext {
    /// The resulting type of the expression
    pub result_type: TypeId,

    /// Can this expression be assigned to? (lvalue)
    pub is_lvalue: bool,

    /// Is this a temporary value that needs cleanup?
    pub is_temporary: bool,

    /// Is this a handle (reference-counted pointer)?
    pub is_handle: bool,

    /// Is this expression const?
    pub is_const: bool,

    /// Does this need destructor call?
    pub requires_cleanup: bool,

    /// Is this a reference parameter?
    pub is_reference: bool,

    /// Can this be null? (for handles)
    pub is_nullable: bool,

    /// Source location (for error messages)
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl ExprContext {
    /// Create a simple value context
    pub fn value(result_type: TypeId) -> Self {
        Self {
            result_type,
            is_lvalue: false,
            is_temporary: true,
            is_handle: false,
            is_const: false,
            requires_cleanup: false,
            is_reference: false,
            is_nullable: false,
            location: None,
        }
    }

    /// Create an lvalue context (assignable)
    pub fn lvalue(result_type: TypeId, is_const: bool) -> Self {
        Self {
            result_type,
            is_lvalue: true,
            is_temporary: false,
            is_handle: false,
            is_const,
            requires_cleanup: false,
            is_reference: false,
            is_nullable: false,
            location: None,
        }
    }

    /// Create a handle context
    pub fn handle(result_type: TypeId, is_nullable: bool) -> Self {
        Self {
            result_type,
            is_lvalue: false,
            is_temporary: true,
            is_handle: true,
            is_const: false,
            requires_cleanup: true,
            is_reference: false,
            is_nullable,
            location: None,
        }
    }

    /// Mark as requiring cleanup
    pub fn with_cleanup(mut self) -> Self {
        self.requires_cleanup = true;
        self
    }

    /// Mark as const
    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    /// Add location information
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.location = Some(SourceLocation { line, column });
        self
    }
}

impl fmt::Display for ExprContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "type={}", self.result_type)?;
        if self.is_lvalue {
            write!(f, " lvalue")?;
        }
        if self.is_handle {
            write!(f, " handle")?;
        }
        if self.is_const {
            write!(f, " const")?;
        }
        if self.is_temporary {
            write!(f, " temp")?;
        }
        Ok(())
    }
}

// ==================== TYPE INFO ====================

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub name: String,
    pub members: HashMap<String, MemberInfo>,
    pub methods: HashMap<String, Vec<MethodInfo>>,
    pub properties: HashMap<String, PropertyInfo>,
    pub base_class: Option<TypeId>,
    pub interfaces: Vec<TypeId>,
    pub is_value_type: bool,
    pub is_ref_type: bool,
    pub has_destructor: bool,
    pub is_final: bool,
    pub can_be_handle: bool,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_private: bool,
    pub is_protected: bool,
    pub is_const: bool,
    pub is_handle: bool,
    pub has_inline_init: bool,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub return_type: TypeId,
    pub params: Vec<ParamInfo>,
    pub is_const: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_ref: bool,
    pub is_out: bool,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_handle: bool,
    pub is_readonly: bool,
    pub getter: Option<String>,
    pub setter: Option<String>,
}

// ==================== SEMANTIC ANALYZER ====================

pub struct SemanticAnalyzer {
    pub engine: Arc<RwLock<EngineInner>>,
    pub symbol_table: SymbolTable,
    pub script_types: HashMap<String, TypeInfo>,
    pub current_namespace: Vec<String>,
    pub current_class: Option<String>,
    pub current_function: Option<String>,
    pub errors: Vec<SemanticError>,
    next_type_id: TypeId,
    loop_depth: u32,
    switch_depth: u32,
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub location: Option<SourceLocation>,
}

impl SemanticError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            location: None,
        }
    }

    pub fn undefined_symbol(name: &str) -> Self {
        Self::new(format!("Undefined symbol: {}", name))
    }

    pub fn undefined_function(name: &str) -> Self {
        Self::new(format!("Undefined function: {}", name))
    }

    pub fn not_an_lvalue() -> Self {
        Self::new("Expression is not an lvalue (cannot be assigned to)".to_string())
    }

    pub fn cannot_modify_const() -> Self {
        Self::new("Cannot modify const value".to_string())
    }

    pub fn handle_assignment_requires_handles() -> Self {
        Self::new("Handle assignment (@) requires both operands to be handles".to_string())
    }

    pub fn incompatible_types(type1: TypeId, type2: TypeId) -> Self {
        Self::new(format!("Incompatible types: {} and {}", type1, type2))
    }

    pub fn already_a_handle() -> Self {
        Self::new("Expression is already a handle".to_string())
    }

    pub fn identity_requires_handles() -> Self {
        Self::new("'is' and '!is' operators require handle operands".to_string())
    }

    pub fn condition_must_be_bool() -> Self {
        Self::new("Condition must be convertible to bool".to_string())
    }

    pub fn incompatible_ternary_branches(type1: TypeId, type2: TypeId) -> Self {
        Self::new(format!(
            "Ternary operator branches have incompatible types: {} and {}",
            type1, type2
        ))
    }

    pub fn invalid_operation() -> Self {
        Self::new("Invalid operation for these types".to_string())
    }

    pub fn unknown_member(member: &str) -> Self {
        Self::new(format!("Unknown member: {}", member))
    }

    pub fn invalid_cast(from: TypeId, to: TypeId) -> Self {
        Self::new(format!("Cannot cast from type {} to {}", from, to))
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(loc) = &self.location {
            write!(f, " at line {}, column {}", loc.line, loc.column)?;
        }
        Ok(())
    }
}

impl SemanticAnalyzer {
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        let mut analyzer = Self {
            engine,
            symbol_table: SymbolTable::new(),
            script_types: HashMap::new(),
            current_namespace: Vec::new(),
            current_class: None,
            current_function: None,
            errors: Vec::new(),
            next_type_id: 100,
            loop_depth: 0,
            switch_depth: 0,
        };

        analyzer.register_primitive_types();
        analyzer
    }

    fn register_primitive_types(&mut self) {
        let primitives = vec![
            ("void", TYPE_VOID),
            ("bool", TYPE_BOOL),
            ("int8", TYPE_INT8),
            ("int16", TYPE_INT16),
            ("int", TYPE_INT32),
            ("int32", TYPE_INT32),
            ("int64", TYPE_INT64),
            ("uint8", TYPE_UINT8),
            ("uint16", TYPE_UINT16),
            ("uint", TYPE_UINT32),
            ("uint32", TYPE_UINT32),
            ("uint64", TYPE_UINT64),
            ("float", TYPE_FLOAT),
            ("double", TYPE_DOUBLE),
            ("string", TYPE_STRING),
            ("auto", TYPE_AUTO),
        ];

        for (name, type_id) in primitives {
            self.script_types.insert(
                name.to_string(),
                TypeInfo {
                    type_id,
                    name: name.to_string(),
                    members: HashMap::new(),
                    methods: HashMap::new(),
                    properties: HashMap::new(),
                    base_class: None,
                    interfaces: Vec::new(),
                    is_value_type: true,
                    is_ref_type: false,
                    has_destructor: false,
                    is_final: true,
                    can_be_handle: false,
                },
            );
        }
    }

    pub fn allocate_type_id(&mut self) -> TypeId {
        let id = self.next_type_id;
        self.next_type_id += 1;
        id
    }

    // ==================== MAIN ANALYSIS ENTRY POINT ====================

    pub fn analyze(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        self.errors.clear();

        self.import_engine_registry();

        self.collect_all_declarations(script);

        if let Err(errors) = self.validate_all_references(script) {
            return Err(errors);
        }

        if let Err(errors) = self.resolve_type_hierarchies(script) {
            return Err(errors);
        }

        if let Err(errors) = self.resolve_class_members(script) {
            return Err(errors);
        }

        if let Err(errors) = self.auto_generate_special_methods(script) {
            return Err(errors);
        }

        if let Err(errors) = self.validate_method_modifiers(script) {
            return Err(errors);
        }

        if let Err(errors) = self.validate_interface_implementations(script) {
            return Err(errors);
        }

        if let Err(errors) = self.validate_constructor_initialization(script) {
            return Err(errors);
        }

        for item in &script.items {
            self.analyze_script_node(item);
        }

        if let Err(errors) = self.final_validation(script) {
            return Err(errors);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    // ==================== PASS 0: IMPORT ENGINE REGISTRY ====================

    fn import_engine_registry(&mut self) {
        // Clone all data once - this is temporary and gets dropped after import
        // Much better than acquiring/releasing locks hundreds of times
        let (object_types, enum_types, typedefs, funcdefs, global_functions, global_properties) = {
            let engine = self.engine.read().unwrap();
            (
                engine.object_types.clone(),
                engine.enum_types.clone(),
                engine.interface_types.clone(),
                engine.funcdefs.clone(),
                engine.global_functions.clone(),
                engine.global_properties.clone(),
            )
        }; // Lock released here

        // Now process without any locks
        for (type_name, obj_type) in &object_types {
            self.import_engine_type(type_name, obj_type);
        }

        for (enum_name, enum_type) in &enum_types {
            self.import_engine_enum(enum_name, enum_type);
        }

        for (typedef_name, typedef_info) in &typedefs {
            self.import_engine_interface_def(typedef_name, typedef_info);
        }

        for func in &global_functions {
            self.import_engine_function(func);
        }

        for prop in &global_properties {
            self.import_engine_property(prop);
        }

        for (funcdef_name, funcdef_info) in &funcdefs {
            self.import_engine_funcdef(funcdef_name, funcdef_info);
        }
    }

    fn import_engine_type(&mut self, type_name: &str, obj_type: &engine::ObjectType) {
        if self.script_types.contains_key(type_name) {
            return;
        }

        let mut members = HashMap::new();
        for prop in &obj_type.properties {
            members.insert(
                prop.name.clone(),
                MemberInfo {
                    name: prop.name.clone(),
                    type_id: prop.type_id,
                    is_private: false,
                    is_protected: false,
                    is_const: prop.is_readonly,
                    is_handle: prop.is_handle,
                    has_inline_init: false,
                },
            );
        }

        let mut methods = HashMap::new();
        for method in &obj_type.methods {
            let method_info = MethodInfo {
                name: method.name.clone(),
                return_type: method.return_type_id,
                params: method
                    .params
                    .iter()
                    .map(|p| ParamInfo {
                        name: p.name.clone(),
                        type_id: p.type_id,
                        is_ref: p.is_ref,
                        is_out: p.is_out,
                    })
                    .collect(),
                is_const: method.is_const,
                is_virtual: method.is_virtual,
                is_override: false,
                is_final: method.is_final,
            };

            methods
                .entry(method.name.clone())
                .or_insert_with(Vec::new)
                .push(method_info);
        }

        let is_ref_type = obj_type.flags.contains(TypeFlags::REF_TYPE);
        let can_be_handle = is_ref_type && !obj_type.flags.contains(TypeFlags::NOHANDLE);

        let type_info = TypeInfo {
            type_id: obj_type.type_id,
            name: type_name.to_string(),
            members,
            methods,
            properties: HashMap::new(),
            base_class: None,
            interfaces: Vec::new(),
            is_value_type: obj_type.flags.contains(TypeFlags::VALUE_TYPE),
            is_ref_type,
            has_destructor: obj_type.destructor.is_some(),
            is_final: obj_type.flags.contains(TypeFlags::NOINHERIT),
            can_be_handle,
        };

        self.script_types.insert(type_name.to_string(), type_info);

        let symbol = Symbol {
            name: type_name.to_string(),
            kind: SymbolKind::Type,
            type_id: obj_type.type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table
            .insert_global(type_name.to_string(), symbol);
    }

    fn import_engine_enum(&mut self, enum_name: &str, enum_type: &engine::EnumType) {
        if self.symbol_table.lookup_global(enum_name).is_some() {
            return;
        }

        let symbol = Symbol {
            name: enum_name.to_string(),
            kind: SymbolKind::Type,
            type_id: enum_type.type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table
            .insert_global(enum_name.to_string(), symbol);

        for (value_name, _value) in &enum_type.values {
            let value_symbol = Symbol {
                name: value_name.clone(),
                kind: SymbolKind::EnumVariant,
                type_id: enum_type.type_id,
                is_const: true,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            };
            self.symbol_table
                .insert_global(value_name.clone(), value_symbol);
        }
    }

    fn import_engine_interface_def(&mut self, typedef_name: &str, typedef_info: &InterfaceInfo) {
        if self.symbol_table.lookup_global(typedef_name).is_some() {
            return;
        }

        let symbol = Symbol {
            name: typedef_name.to_string(),
            kind: SymbolKind::Type,
            type_id: typedef_info.type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table
            .insert_global(typedef_name.to_string(), symbol);
    }

    fn import_engine_function(&mut self, func: &GlobalFunction) {
        if self.symbol_table.lookup_global(&func.name).is_some() {
            return;
        }

        let symbol = Symbol {
            name: func.name.clone(),
            kind: SymbolKind::Function,
            type_id: func.return_type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table.insert_global(func.name.clone(), symbol);
    }

    fn import_engine_property(&mut self, prop: &engine::GlobalProperty) {
        if self.symbol_table.lookup_global(&prop.name).is_some() {
            return;
        }

        let symbol = Symbol {
            name: prop.name.clone(),
            kind: SymbolKind::Variable,
            type_id: prop.type_id,
            is_const: prop.is_const,
            is_handle: prop.is_handle,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table.insert_global(prop.name.clone(), symbol);
    }

    fn import_engine_funcdef(&mut self, funcdef_name: &str, funcdef_info: &engine::FuncdefInfo) {
        if self.symbol_table.lookup_global(funcdef_name).is_some() {
            return;
        }

        let symbol = Symbol {
            name: funcdef_name.to_string(),
            kind: SymbolKind::Type,
            type_id: funcdef_info.type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: vec![],
        };
        self.symbol_table
            .insert_global(funcdef_name.to_string(), symbol);
    }

    // ==================== PASS 1: COLLECT SCRIPT DECLARATIONS ====================

    fn collect_all_declarations(&mut self, script: &Script) {
        for item in &script.items {
            self.collect_declarations(item);
        }
    }

    fn collect_declarations(&mut self, node: &ScriptNode) {
        match node {
            ScriptNode::Class(class) => {
                self.register_class_declaration(class);
            }

            ScriptNode::Interface(interface) => {
                self.register_interface_declaration(interface);
            }

            ScriptNode::Enum(enum_def) => {
                self.register_enum_declaration(enum_def);
            }

            ScriptNode::Func(func) => {
                self.register_function_declaration(func);
            }

            ScriptNode::Var(var) => {
                self.register_global_variable(var);
            }

            ScriptNode::Typedef(typedef) => {
                self.register_typedef(typedef);
            }

            ScriptNode::FuncDef(funcdef) => {
                self.register_funcdef(funcdef);
            }

            ScriptNode::Namespace(ns) => {
                let saved_namespace = self.current_namespace.clone();
                self.current_namespace.extend(ns.name.clone());

                for item in &ns.items {
                    self.collect_declarations(item);
                }

                self.current_namespace = saved_namespace;
            }

            _ => {}
        }
    }

    fn register_class_declaration(&mut self, class: &Class) {
        let type_id = self.allocate_type_id();

        let symbol = Symbol {
            name: class.name.clone(),
            kind: SymbolKind::Type,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(class.name.clone(), symbol);

        let is_final = class.modifiers.contains(&"final".to_string());

        let type_info = TypeInfo {
            type_id,
            name: class.name.clone(),
            members: HashMap::new(),
            methods: HashMap::new(),
            properties: HashMap::new(),
            base_class: None,
            interfaces: Vec::new(),
            is_value_type: false,
            is_ref_type: true,
            has_destructor: false,
            is_final,
            can_be_handle: true,
        };
        self.script_types.insert(class.name.clone(), type_info);
    }

    fn register_interface_declaration(&mut self, interface: &Interface) {
        let type_id = self.allocate_type_id();

        let symbol = Symbol {
            name: interface.name.clone(),
            kind: SymbolKind::Type,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(interface.name.clone(), symbol);

        let type_info = TypeInfo {
            type_id,
            name: interface.name.clone(),
            members: HashMap::new(),
            methods: HashMap::new(),
            properties: HashMap::new(),
            base_class: None,
            interfaces: Vec::new(),
            is_value_type: false,
            is_ref_type: true,
            has_destructor: false,
            is_final: false,
            can_be_handle: true,
        };
        self.script_types.insert(interface.name.clone(), type_info);
    }

    fn register_enum_declaration(&mut self, enum_def: &Enum) {
        let type_id = self.allocate_type_id();

        let symbol = Symbol {
            name: enum_def.name.clone(),
            kind: SymbolKind::Type,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(enum_def.name.clone(), symbol);

        for variant in &enum_def.variants {
            let variant_symbol = Symbol {
                name: variant.name.clone(),
                kind: SymbolKind::EnumVariant,
                type_id,
                is_const: true,
                is_handle: false,
                is_reference: false,
                namespace: self.current_namespace.clone(),
            };
            self.symbol_table
                .insert(variant.name.clone(), variant_symbol);
        }
    }

    fn register_function_declaration(&mut self, func: &Func) {
        let type_id = self.allocate_type_id();

        let symbol = Symbol {
            name: func.name.clone(),
            kind: SymbolKind::Function,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(func.name.clone(), symbol);
    }

    fn register_global_variable(&mut self, var: &Var) {
        let type_id = self.resolve_type_from_ast(&var.var_type);

        for decl in &var.declarations {
            let symbol = Symbol {
                name: decl.name.clone(),
                kind: SymbolKind::Variable,
                type_id,
                is_const: var.var_type.is_const,
                is_handle: false,
                is_reference: false,
                namespace: self.current_namespace.clone(),
            };
            self.symbol_table.insert(decl.name.clone(), symbol);
        }
    }

    fn register_typedef(&mut self, typedef: &Typedef) {
        let base_type_id = self
            .get_type_id_by_name(&typedef.prim_type)
            .unwrap_or(TYPE_VOID);

        let symbol = Symbol {
            name: typedef.name.clone(),
            kind: SymbolKind::Type,
            type_id: base_type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(typedef.name.clone(), symbol);
    }

    fn register_funcdef(&mut self, funcdef: &FuncDef) {
        let type_id = self.allocate_type_id();

        let symbol = Symbol {
            name: funcdef.name.clone(),
            kind: SymbolKind::Type,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace: self.current_namespace.clone(),
        };
        self.symbol_table.insert(funcdef.name.clone(), symbol);
    }

    // ==================== PASS 2: VALIDATE ALL REFERENCES ====================

    fn validate_all_references(&self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            self.validate_item_references(item, &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_item_references(&self, node: &ScriptNode, errors: &mut Vec<SemanticError>) {
        match node {
            ScriptNode::Func(func) => {
                if let Some(ret_type) = &func.return_type {
                    if !self.type_exists_in_registry(ret_type) {
                        errors.push(SemanticError::new(format!(
                            "Unknown return type: {}",
                            self.extract_type_name(ret_type)
                        )));
                    }
                }

                for param in &func.params {
                    if !self.type_exists_in_registry(&param.param_type) {
                        errors.push(SemanticError::new(format!(
                            "Unknown parameter type: {}",
                            self.extract_type_name(&param.param_type)
                        )));
                    }
                }
            }

            ScriptNode::Class(class) => {
                for base in &class.extends {
                    if !self.type_exists_by_name(base) {
                        errors.push(SemanticError::new(format!("Unknown base class: {}", base)));
                    }
                }

                for member in &class.members {
                    if let ClassMember::Var(var) = member {
                        if !self.type_exists_in_registry(&var.var_type) {
                            errors.push(SemanticError::new(format!(
                                "Unknown member type: {}",
                                self.extract_type_name(&var.var_type)
                            )));
                        }
                    }
                }
            }

            ScriptNode::Interface(interface) => {
                for base in &interface.extends {
                    if !self.type_exists_by_name(base) {
                        errors.push(SemanticError::new(format!(
                            "Unknown base interface: {}",
                            base
                        )));
                    }
                }
            }

            ScriptNode::Var(var) => {
                if !self.type_exists_in_registry(&var.var_type) {
                    errors.push(SemanticError::new(format!(
                        "Unknown variable type: {}",
                        self.extract_type_name(&var.var_type)
                    )));
                }
            }

            ScriptNode::Namespace(ns) => {
                for item in &ns.items {
                    self.validate_item_references(item, errors);
                }
            }

            _ => {}
        }
    }

    fn type_exists_in_registry(&self, type_def: &Type) -> bool {
        let type_name = self.extract_type_name(type_def);
        self.type_exists_by_name(&type_name)
    }

    fn type_exists_by_name(&self, type_name: &str) -> bool {
        if self.script_types.contains_key(type_name) {
            return true;
        }

        if self.symbol_table.lookup(type_name).is_some() {
            return true;
        }

        false
    }

    fn extract_type_name(&self, type_def: &Type) -> String {
        match &type_def.datatype {
            DataType::PrimType(name) => name.clone(),
            DataType::Identifier(name) => name.clone(),
            DataType::Auto => "auto".to_string(),
            DataType::Question => "?".to_string(),
        }
    }

    // ==================== PASS 3: RESOLVE TYPE HIERARCHIES ====================

    fn resolve_type_hierarchies(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                if let Err(e) = self.validate_not_inheriting_final(class) {
                    errors.push(e);
                    continue;
                }

                if !class.extends.is_empty() {
                    let base_name = &class.extends[0];

                    if let Some(base_type_id) = self.lookup_type_id(base_name) {
                        if let Some(type_info) = self.script_types.get_mut(&class.name) {
                            type_info.base_class = Some(base_type_id);
                        }
                    } else {
                        errors.push(SemanticError::new(format!(
                            "Base class '{}' not found for class '{}'",
                            base_name, class.name
                        )));
                    }
                }

                for (i, interface_name) in class.extends.iter().enumerate() {
                    if i == 0 {
                        continue;
                    }

                    if let Some(interface_type_id) = self.lookup_type_id(interface_name) {
                        if let Some(type_info) = self.script_types.get_mut(&class.name) {
                            type_info.interfaces.push(interface_type_id);
                        }
                    } else {
                        errors.push(SemanticError::new(format!(
                            "Interface '{}' not found for class '{}'",
                            interface_name, class.name
                        )));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // ==================== PASS 4: RESOLVE CLASS MEMBERS ====================

    fn resolve_class_members(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                if let Err(e) = self.resolve_class_member_details(class) {
                    errors.push(e);
                }
            }

            if let ScriptNode::Interface(interface) = item {
                if let Err(e) = self.resolve_interface_member_details(interface) {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn resolve_class_member_details(&mut self, class: &Class) -> Result<(), SemanticError> {
        let type_info = self
            .script_types
            .get(&class.name)
            .ok_or_else(|| SemanticError::new(format!("Class '{}' not found", class.name)))?;

        for member in &class.members {
            match member {
                ClassMember::Var(var) => {
                    let member_type_id = self.resolve_type_from_ast(&var.var_type);

                    for decl in &var.declarations {
                        let has_inline_init = decl.initializer.is_some();

                        let member_info = MemberInfo {
                            name: decl.name.clone(),
                            type_id: member_type_id,
                            is_private: matches!(var.visibility, Some(Visibility::Private)),
                            is_protected: matches!(var.visibility, Some(Visibility::Protected)),
                            is_const: var.var_type.is_const,
                            is_handle: false,
                            has_inline_init,
                        };

                        if let Some(type_info) = self.script_types.get_mut(&class.name) {
                            type_info.members.insert(decl.name.clone(), member_info);
                        }
                    }
                }

                ClassMember::Func(func) => {
                    let return_type = func
                        .return_type
                        .as_ref()
                        .map(|t| self.resolve_type_from_ast(t))
                        .unwrap_or(TYPE_VOID);

                    let is_virtual = func.modifiers.contains(&"virtual".to_string())
                        || func.modifiers.contains(&"override".to_string());
                    let is_override = func.modifiers.contains(&"override".to_string());
                    let is_final = func.modifiers.contains(&"final".to_string());

                    let method_info = MethodInfo {
                        name: func.name.clone(),
                        return_type,
                        params: func
                            .params
                            .iter()
                            .map(|p| ParamInfo {
                                name: p.name.clone().unwrap_or_default(),
                                type_id: self.resolve_type_from_ast(&p.param_type),
                                is_ref: matches!(
                                    p.type_mod,
                                    Some(TypeMod::InOut) | Some(TypeMod::Out)
                                ),
                                is_out: matches!(p.type_mod, Some(TypeMod::Out)),
                            })
                            .collect(),
                        is_const: func.is_const,
                        is_virtual,
                        is_override,
                        is_final,
                    };

                    if let Some(type_info) = self.script_types.get_mut(&class.name) {
                        type_info
                            .methods
                            .entry(func.name.clone())
                            .or_insert_with(Vec::new)
                            .push(method_info);

                        if func.name.starts_with('~') {
                            type_info.has_destructor = true;
                        }
                    }
                }

                ClassMember::VirtProp(prop) => {
                    let prop_type = self.resolve_type_from_ast(&prop.prop_type);

                    let setter = prop
                        .accessors
                        .iter()
                        .find(|prop| prop.kind == AccessorKind::Set);
                    let getter = prop
                        .accessors
                        .iter()
                        .find(|prop| prop.kind == AccessorKind::Get);

                    let property_info = PropertyInfo {
                        name: prop.name.clone(),
                        type_id: prop_type,
                        is_handle: false,
                        is_readonly: setter.is_none(),
                        getter: getter.as_ref().map(|_| format!("get_{}", prop.name)),
                        setter: setter.as_ref().map(|_| format!("set_{}", prop.name)),
                    };

                    if let Some(type_info) = self.script_types.get_mut(&class.name) {
                        type_info
                            .properties
                            .insert(prop.name.clone(), property_info);
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }

    fn resolve_interface_member_details(
        &mut self,
        interface: &Interface,
    ) -> Result<(), SemanticError> {
        for member in &interface.members {
            match member {
                InterfaceMember::Method(func) => {
                    let return_type = self.resolve_type_from_ast(&func.return_type);

                    let method_info = MethodInfo {
                        name: func.name.clone(),
                        return_type,
                        params: func
                            .params
                            .iter()
                            .map(|p| ParamInfo {
                                name: p.name.clone().unwrap_or_default(),
                                type_id: self.resolve_type_from_ast(&p.param_type),
                                is_ref: matches!(
                                    p.type_mod,
                                    Some(TypeMod::InOut) | Some(TypeMod::Out)
                                ),
                                is_out: matches!(p.type_mod, Some(TypeMod::Out)),
                            })
                            .collect(),
                        is_const: func.is_const,
                        is_virtual: true,
                        is_override: false,
                        is_final: false,
                    };

                    if let Some(type_info) = self.script_types.get_mut(&interface.name) {
                        type_info
                            .methods
                            .entry(func.name.clone())
                            .or_insert_with(Vec::new)
                            .push(method_info);
                    }
                }

                InterfaceMember::VirtProp(prop) => {
                    let prop_type = self.resolve_type_from_ast(&prop.prop_type);

                    let setter = prop
                        .accessors
                        .iter()
                        .find(|prop| prop.kind == AccessorKind::Set);
                    let getter = prop
                        .accessors
                        .iter()
                        .find(|prop| prop.kind == AccessorKind::Get);

                    let property_info = PropertyInfo {
                        name: prop.name.clone(),
                        type_id: prop_type,
                        is_handle: false,
                        is_readonly: setter.is_none(),
                        getter: getter.as_ref().map(|_| format!("get_{}", prop.name)),
                        setter: setter.as_ref().map(|_| format!("set_{}", prop.name)),
                    };

                    if let Some(type_info) = self.script_types.get_mut(&interface.name) {
                        type_info
                            .properties
                            .insert(prop.name.clone(), property_info);
                    }
                }
            }
        }

        Ok(())
    }

    // ==================== PASS 4.5: AUTO-GENERATE SPECIAL METHODS ====================

    fn auto_generate_special_methods(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                if let Err(e) = self.auto_generate_class_methods(class) {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn auto_generate_class_methods(&mut self, class: &Class) -> Result<(), SemanticError> {
        let needs_default_constructor = !self.has_any_constructor(class);
        let needs_copy_constructor =
            !self.has_copy_constructor(class) && !self.is_deleted_method(class, "copy_constructor");
        let needs_destructor =
            !self.has_destructor(class) && !self.is_deleted_method(class, "destructor");
        let needs_opassign =
            !self.has_opassign(class) && !self.is_deleted_method(class, "opAssign");

        let type_info = self
            .script_types
            .get(&class.name)
            .ok_or_else(|| SemanticError::new(format!("Class '{}' not found", class.name)))?;

        let class_type_id = type_info.type_id;

        if needs_default_constructor {
            self.generate_default_constructor(&class.name, class_type_id)?;
        }

        if needs_copy_constructor {
            self.generate_copy_constructor(&class.name, class_type_id)?;
        }

        if needs_destructor {
            self.generate_destructor(&class.name, class_type_id)?;
        }

        if needs_opassign {
            self.generate_opassign(&class.name, class_type_id)?;
        }

        Ok(())
    }

    fn has_any_constructor(&self, class: &Class) -> bool {
        class.members.iter().any(|m| {
            if let ClassMember::Func(func) = m {
                func.name == class.name && func.return_type.is_none()
            } else {
                false
            }
        })
    }

    fn has_copy_constructor(&self, class: &Class) -> bool {
        class.members.iter().any(|m| {
            if let ClassMember::Func(func) = m {
                if func.name == class.name && func.return_type.is_none() && func.params.len() == 1 {
                    let param = &func.params[0];
                    if let DataType::Identifier(type_name) = &param.param_type.datatype {
                        return type_name == &class.name
                            && param.param_type.is_const
                            && !param.param_type.modifiers.is_empty();
                    }
                }
            }
            false
        })
    }

    fn has_destructor(&self, class: &Class) -> bool {
        class.members.iter().any(|m| {
            if let ClassMember::Func(func) = m {
                func.name.starts_with('~')
            } else {
                false
            }
        })
    }

    fn has_opassign(&self, class: &Class) -> bool {
        class.members.iter().any(|m| {
            if let ClassMember::Func(func) = m {
                func.name == "opAssign"
            } else {
                false
            }
        })
    }

    fn is_deleted_method(&self, class: &Class, method_type: &str) -> bool {
        class.members.iter().any(|m| {
            if let ClassMember::Func(func) = m {
                let is_target_method = match method_type {
                    "copy_constructor" => func.name == class.name && func.params.len() == 1,
                    "destructor" => func.name.starts_with('~'),
                    "opAssign" => func.name == "opAssign",
                    _ => false,
                };

                is_target_method && func.modifiers.iter().any(|m| m == "delete")
            } else {
                false
            }
        })
    }

    fn generate_default_constructor(
        &mut self,
        class_name: &str,
        _class_type_id: TypeId,
    ) -> Result<(), SemanticError> {
        let method_info = MethodInfo {
            name: class_name.to_string(),
            return_type: TYPE_VOID,
            params: vec![],
            is_const: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
        };

        let type_info = self.script_types.get_mut(class_name).unwrap();
        type_info
            .methods
            .entry(class_name.to_string())
            .or_insert_with(Vec::new)
            .push(method_info);

        Ok(())
    }

    fn generate_copy_constructor(
        &mut self,
        class_name: &str,
        class_type_id: TypeId,
    ) -> Result<(), SemanticError> {
        let method_info = MethodInfo {
            name: class_name.to_string(),
            return_type: TYPE_VOID,
            params: vec![ParamInfo {
                name: "other".to_string(),
                type_id: class_type_id,
                is_ref: true,
                is_out: false,
            }],
            is_const: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
        };

        let type_info = self.script_types.get_mut(class_name).unwrap();
        type_info
            .methods
            .entry(class_name.to_string())
            .or_insert_with(Vec::new)
            .push(method_info);

        Ok(())
    }

    fn generate_destructor(
        &mut self,
        class_name: &str,
        _class_type_id: TypeId,
    ) -> Result<(), SemanticError> {
        let destructor_name = format!("~{}", class_name);

        let method_info = MethodInfo {
            name: destructor_name.clone(),
            return_type: TYPE_VOID,
            params: vec![],
            is_const: false,
            is_virtual: true,
            is_override: false,
            is_final: false,
        };

        let type_info = self.script_types.get_mut(class_name).unwrap();
        type_info
            .methods
            .entry(destructor_name)
            .or_insert_with(Vec::new)
            .push(method_info);

        type_info.has_destructor = true;

        Ok(())
    }

    fn generate_opassign(
        &mut self,
        class_name: &str,
        class_type_id: TypeId,
    ) -> Result<(), SemanticError> {
        let method_info = MethodInfo {
            name: "opAssign".to_string(),
            return_type: class_type_id,
            params: vec![ParamInfo {
                name: "other".to_string(),
                type_id: class_type_id,
                is_ref: true,
                is_out: false,
            }],
            is_const: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
        };

        let type_info = self.script_types.get_mut(class_name).unwrap();
        type_info
            .methods
            .entry("opAssign".to_string())
            .or_insert_with(Vec::new)
            .push(method_info);

        Ok(())
    }

    // ==================== PASS 5: VALIDATE METHOD MODIFIERS ====================

    fn validate_method_modifiers(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                self.validate_class_method_modifiers(class, &mut errors);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_class_method_modifiers(&self, class: &Class, errors: &mut Vec<SemanticError>) {
        for member in &class.members {
            if let ClassMember::Func(func) = member {
                if func.modifiers.contains(&"override".to_string()) {
                    if let Err(e) = self.validate_override_modifier(class, func) {
                        errors.push(e);
                    }
                }

                if func.modifiers.contains(&"final".to_string()) {
                    if let Err(e) = self.validate_final_modifier(class, func) {
                        errors.push(e);
                    }
                }

                if let Err(e) = self.validate_not_overriding_final(class, func) {
                    errors.push(e);
                }

                if let Err(e) = self.validate_missing_override(class, func) {
                    errors.push(e);
                }
            }
        }
    }

    fn validate_override_modifier(&self, class: &Class, func: &Func) -> Result<(), SemanticError> {
        if class.extends.is_empty() {
            return Err(SemanticError::new(format!(
                "Method '{}::{}' has 'override' modifier but class has no base class",
                class.name, func.name
            )));
        }

        let base_class_name = &class.extends[0];
        let base_type_id = self.get_type_id_by_name(base_class_name).ok_or_else(|| {
            SemanticError::new(format!("Base class '{}' not found", base_class_name))
        })?;

        let base_method = self.find_method_in_hierarchy(base_type_id, &func.name);

        if base_method.is_none() {
            return Err(SemanticError::new(format!(
                "Method '{}::{}' has 'override' modifier but does not override any base class method",
                class.name, func.name
            )));
        }

        let base_method = base_method.unwrap();
        if !self.method_signatures_compatible_func(func, &base_method) {
            return Err(SemanticError::new(format!(
                "Method '{}::{}' signature does not match base class method signature",
                class.name, func.name
            )));
        }

        if !base_method.is_virtual {
            return Err(SemanticError::new(format!(
                "Method '{}::{}' cannot override non-virtual base method",
                class.name, func.name
            )));
        }

        Ok(())
    }

    fn validate_missing_override(&self, class: &Class, func: &Func) -> Result<(), SemanticError> {
        if class.extends.is_empty() {
            return Ok(());
        }

        let base_class_name = &class.extends[0];
        let base_type_id = self.get_type_id_by_name(base_class_name).ok_or_else(|| {
            SemanticError::new(format!("Base class '{}' not found", base_class_name))
        })?;

        if let Some(base_method) = self.find_method_in_hierarchy(base_type_id, &func.name) {
            if base_method.is_virtual && !func.modifiers.contains(&"override".to_string()) {
                return Err(SemanticError::new(format!(
                    "Method '{}::{}' hides base class virtual method. Use 'override' keyword or rename method",
                    class.name, func.name
                )));
            }
        }

        Ok(())
    }

    fn validate_final_modifier(&self, class: &Class, func: &Func) -> Result<(), SemanticError> {
        if func.modifiers.contains(&"override".to_string()) {
            return Ok(());
        }

        if class.extends.is_empty() {
            return Ok(());
        }

        let base_class_name = &class.extends[0];
        if let Some(base_type_id) = self.get_type_id_by_name(base_class_name) {
            if self
                .find_method_in_hierarchy(base_type_id, &func.name)
                .is_some()
            {
                return Ok(());
            }
        }

        Ok(())
    }

    fn validate_not_overriding_final(
        &self,
        class: &Class,
        func: &Func,
    ) -> Result<(), SemanticError> {
        if class.extends.is_empty() {
            return Ok(());
        }

        let base_class_name = &class.extends[0];
        let base_type_id = self.get_type_id_by_name(base_class_name).ok_or_else(|| {
            SemanticError::new(format!("Base class '{}' not found", base_class_name))
        })?;

        if let Some(base_method) = self.find_method_in_hierarchy(base_type_id, &func.name) {
            if base_method.is_final {
                return Err(SemanticError::new(format!(
                    "Cannot override final method '{}' from base class '{}'",
                    func.name, base_class_name
                )));
            }
        }

        Ok(())
    }

    fn validate_not_inheriting_final(&self, class: &Class) -> Result<(), SemanticError> {
        if class.extends.is_empty() {
            return Ok(());
        }

        let base_class_name = &class.extends[0];

        if self.is_class_final(base_class_name) {
            return Err(SemanticError::new(format!(
                "Cannot inherit from final class '{}'",
                base_class_name
            )));
        }

        Ok(())
    }

    fn is_class_final(&self, class_name: &str) -> bool {
        if let Some(type_info) = self.script_types.get(class_name) {
            return type_info.is_final;
        }

        let engine = self.engine.read().unwrap();
        if let Some(obj_type) = engine.object_types.get(class_name) {
            return obj_type.flags.contains(TypeFlags::NOINHERIT);
        }

        false
    }

    // ==================== PASS 6: VALIDATE INTERFACE IMPLEMENTATIONS ====================

    fn validate_interface_implementations(
        &self,
        script: &Script,
    ) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Iterate through script to find all classes
        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                // First item in extends is base class (if any)
                // Rest are interfaces
                for interface_name in class.extends.iter().skip(1) {
                    if let Err(e) =
                        self.validate_interface_implementation(&class.name, interface_name)
                    {
                        errors.push(e);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_interface_implementation(
        &self,
        class_name: &str,
        interface_name: &str,
    ) -> Result<(), SemanticError> {
        let interface_type_id = self.get_type_id_by_name(interface_name).ok_or_else(|| {
            SemanticError::new(format!("Interface '{}' not found", interface_name))
        })?;

        let interface_type = self
            .script_types
            .values()
            .find(|t| t.type_id == interface_type_id)
            .ok_or_else(|| {
                SemanticError::new(format!("Interface '{}' not found", interface_name))
            })?;

        let class_type = self
            .script_types
            .get(class_name)
            .ok_or_else(|| SemanticError::new(format!("Class '{}' not found", class_name)))?;

        // Validate all interface methods are implemented
        for (method_name, interface_methods) in &interface_type.methods {
            for interface_method in interface_methods {
                let class_has_method =
                    class_type
                        .methods
                        .get(method_name)
                        .map_or(false, |class_methods| {
                            class_methods
                                .iter()
                                .any(|cm| self.method_signatures_match(cm, interface_method))
                        });

                if !class_has_method {
                    return Err(SemanticError::new(format!(
                        "Class '{}' does not implement interface method '{}::{}' with signature {}",
                        class_name,
                        interface_name,
                        method_name,
                        self.format_method_signature(interface_method)
                    )));
                }
            }
        }

        // Validate all interface properties are implemented
        for (prop_name, interface_prop) in &interface_type.properties {
            match class_type.properties.get(prop_name) {
                Some(class_prop) => {
                    // Check type compatibility
                    if class_prop.type_id != interface_prop.type_id {
                        return Err(SemanticError::new(format!(
                            "Class '{}' property '{}' has type {} but interface '{}' requires type {}",
                            class_name,
                            prop_name,
                            class_prop.type_id,
                            interface_name,
                            interface_prop.type_id
                        )));
                    }

                    // Check readonly compatibility (class can be more restrictive)
                    if interface_prop.is_readonly && !class_prop.is_readonly {
                        return Err(SemanticError::new(format!(
                            "Class '{}' property '{}' must be readonly to match interface '{}'",
                            class_name, prop_name, interface_name
                        )));
                    }
                }
                None => {
                    return Err(SemanticError::new(format!(
                        "Class '{}' does not implement interface property '{}::{}'",
                        class_name, interface_name, prop_name
                    )));
                }
            }
        }

        Ok(())
    }

    fn method_signatures_match(&self, method1: &MethodInfo, method2: &MethodInfo) -> bool {
        if method1.return_type != method2.return_type {
            return false;
        }

        if method1.params.len() != method2.params.len() {
            return false;
        }

        for (p1, p2) in method1.params.iter().zip(&method2.params) {
            if p1.type_id != p2.type_id || p1.is_ref != p2.is_ref || p1.is_out != p2.is_out {
                return false;
            }
        }

        if method1.is_const != method2.is_const {
            return false;
        }

        true
    }

    fn method_signatures_compatible_func(&self, func: &Func, method: &MethodInfo) -> bool {
        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.resolve_type_from_ast(t))
            .unwrap_or(TYPE_VOID);

        if return_type != method.return_type {
            return false;
        }

        if func.params.len() != method.params.len() {
            return false;
        }

        for (fp, mp) in func.params.iter().zip(&method.params) {
            let func_param_type = self.resolve_type_from_ast(&fp.param_type);
            if func_param_type != mp.type_id {
                return false;
            }
        }

        if func.is_const != method.is_const {
            return false;
        }

        true
    }

    fn format_method_signature(&self, method: &MethodInfo) -> String {
        let params: Vec<String> = method
            .params
            .iter()
            .map(|p| format!("{}{}", if p.is_ref { "&" } else { "" }, p.type_id))
            .collect();

        format!(
            "{}({}){}",
            method.return_type,
            params.join(", "),
            if method.is_const { " const" } else { "" }
        )
    }

    // ==================== PASS 7: VALIDATE CONSTRUCTOR INITIALIZATION ====================

    fn validate_constructor_initialization(
        &self,
        script: &Script,
    ) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                for member in &class.members {
                    if let ClassMember::Func(func) = member {
                        if func.name == class.name && func.return_type.is_none() {
                            if let Err(e) = self.validate_constructor_requirements(class, func) {
                                errors.push(e);
                            }
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_constructor_requirements(
        &self,
        class: &Class,
        constructor: &Func,
    ) -> Result<(), SemanticError> {
        if !class.extends.is_empty() {
            let base_class_name = &class.extends[0];
            if !self.base_has_default_constructor(base_class_name) {
                if !self.constructor_calls_super(constructor) {
                    return Err(SemanticError::new(format!(
                        "Constructor '{}::{}' must call base constructor using super() because base class '{}' has no default constructor",
                        class.name, constructor.name, base_class_name
                    )));
                }
            }
        }

        for member in &class.members {
            if let ClassMember::Var(var) = member {
                let member_type = self.resolve_type_from_ast(&var.var_type);

                for decl in &var.declarations {
                    if decl.initializer.is_some() {
                        continue;
                    }

                    if !self.type_has_default_constructor(member_type) {
                        return Err(SemanticError::new(format!(
                            "Member '{}::{}' of type {} has no default constructor and must be initialized inline",
                            class.name, decl.name, member_type
                        )));
                    }
                }
            }
        }

        if let Some(body) = &constructor.body {
            if let Some(first_stmt) = body.statements.first() {
                if let Statement::Expr(Some(expr)) = first_stmt {
                    if let Expr::FuncCall(call) = expr {
                        if call.name == "super" {
                            // Good
                        } else if self.body_contains_super_call(body) {
                            return Err(SemanticError::new(format!(
                                "super() must be the first statement in constructor '{}'",
                                constructor.name
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn base_has_default_constructor(&self, base_class_name: &str) -> bool {
        if let Some(base_type) = self.script_types.get(base_class_name) {
            if let Some(constructors) = base_type.methods.get(base_class_name) {
                return constructors.iter().any(|c| c.params.is_empty());
            }
        }
        true
    }

    fn type_has_default_constructor(&self, type_id: TypeId) -> bool {
        if self.is_primitive_type(type_id) {
            return true;
        }

        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                if let Some(constructors) = type_info.methods.get(&type_info.name) {
                    return constructors.iter().any(|c| c.params.is_empty());
                }
                return true;
            }
        }

        true
    }

    fn constructor_calls_super(&self, constructor: &Func) -> bool {
        if let Some(body) = &constructor.body {
            return self.body_contains_super_call(body);
        }
        false
    }

    fn body_contains_super_call(&self, body: &StatBlock) -> bool {
        for stmt in &body.statements {
            if let Statement::Expr(Some(expr)) = stmt {
                if let Expr::FuncCall(call) = expr {
                    if call.name == "super" {
                        return true;
                    }
                }
            }
        }
        false
    }

    // ==================== PASS 8: SEMANTIC ANALYSIS (TYPE CHECKING) ====================

    fn analyze_script_node(&mut self, node: &ScriptNode) {
        match node {
            ScriptNode::Func(func) => self.analyze_function(func),
            ScriptNode::Class(class) => self.analyze_class(class),
            ScriptNode::Var(var) => self.analyze_global_var(var),
            ScriptNode::Namespace(ns) => self.analyze_namespace(ns),
            ScriptNode::Enum(enum_def) => self.analyze_enum(enum_def),
            ScriptNode::Interface(interface) => self.analyze_interface(interface),
            _ => {}
        }
    }

    fn analyze_function(&mut self, func: &Func) {
        self.current_function = Some(func.name.clone());
        self.symbol_table.push_scope();

        for param in &func.params {
            if let Some(name) = &param.name {
                let type_id = self.resolve_type_from_ast(&param.param_type);
                let is_ref = matches!(param.type_mod, Some(TypeMod::InOut) | Some(TypeMod::Out));

                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    type_id,
                    is_const: param.param_type.is_const,
                    is_handle: false,
                    is_reference: is_ref,
                    namespace: self.current_namespace.clone(),
                };
                self.symbol_table.insert(name.clone(), symbol);
            }
        }

        if let Some(body) = &func.body {
            self.analyze_statement_block(body);
        }

        self.symbol_table.pop_scope();
        self.current_function = None;
    }

    fn analyze_class(&mut self, class: &Class) {
        self.current_class = Some(class.name.clone());

        for member in &class.members {
            match member {
                ClassMember::Var(_var) => {
                    // Already analyzed in resolve_class_members
                }
                ClassMember::Func(func) => {
                    self.analyze_function(func);
                }
                ClassMember::VirtProp(_prop) => {
                    // Already analyzed in resolve_class_members
                }
                ClassMember::FuncDef(_) => {}
            }
        }

        self.current_class = None;
    }

    fn analyze_statement_block(&mut self, block: &StatBlock) {
        self.symbol_table.push_scope();

        for stmt in &block.statements {
            self.analyze_statement(stmt);
        }

        self.symbol_table.pop_scope();
    }

    fn analyze_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Var(var) => self.analyze_var_decl(var),
            Statement::Expr(Some(expr)) => {
                let _ = self.analyze_expr_context(expr);
            }
            Statement::If(if_stmt) => self.analyze_if(if_stmt),
            Statement::While(while_stmt) => self.analyze_while(while_stmt),
            Statement::DoWhile(do_while) => self.analyze_do_while(do_while),
            Statement::For(for_stmt) => self.analyze_for(for_stmt),
            Statement::ForEach(foreach) => self.analyze_foreach(foreach),
            Statement::Return(ret) => self.analyze_return(ret),
            Statement::Block(block) => self.analyze_statement_block(block),
            Statement::Switch(switch) => self.analyze_switch(switch),
            Statement::Break => {
                if let Err(e) = self.validate_break_in_loop() {
                    self.errors.push(e);
                }
            }
            Statement::Continue => {
                if let Err(e) = self.validate_continue_in_loop() {
                    self.errors.push(e);
                }
            }
            _ => {}
        }
    }

    fn analyze_var_decl(&mut self, var: &Var) {
        let type_id = self.resolve_type_from_ast(&var.var_type);

        for decl in &var.declarations {
            if let Some(init) = &decl.initializer {
                match init {
                    VarInit::Expr(expr) => {
                        if let Ok(ctx) = self.analyze_expr_context(expr) {
                            if let Err(e) = self.validate_type_match(
                                type_id,
                                ctx.result_type,
                                "variable initialization",
                            ) {
                                self.errors.push(e);
                            }
                        }
                    }
                    VarInit::InitList(init_list) => {
                        self.analyze_init_list(init_list, type_id);
                    }
                    VarInit::ArgList(args) => {
                        for arg in args {
                            let _ = self.analyze_expr_context(&arg.value);
                        }
                    }
                }
            }

            let symbol = Symbol {
                name: decl.name.clone(),
                kind: SymbolKind::Variable,
                type_id,
                is_const: var.var_type.is_const,
                is_handle: false,
                is_reference: false,
                namespace: self.current_namespace.clone(),
            };
            self.symbol_table.insert(decl.name.clone(), symbol);
        }
    }

    fn analyze_init_list(&mut self, init_list: &InitList, expected_type: TypeId) {
        for item in &init_list.items {
            match item {
                InitListItem::Expr(expr) => {
                    if let Ok(ctx) = self.analyze_expr_context(expr) {
                        if !self.types_compatible(expected_type, ctx.result_type) {
                            self.errors.push(SemanticError::new(format!(
                                "Init list item type {} incompatible with expected type {}",
                                ctx.result_type, expected_type
                            )));
                        }
                    }
                }
                InitListItem::InitList(nested) => {
                    self.analyze_init_list(nested, expected_type);
                }
            }
        }
    }

    fn analyze_if(&mut self, if_stmt: &IfStmt) {
        if let Ok(ctx) = self.analyze_expr_context(&if_stmt.condition) {
            if !self.is_convertible_to_bool(ctx.result_type) {
                self.errors.push(SemanticError::condition_must_be_bool());
            }
        }

        self.analyze_statement(&if_stmt.then_branch);

        if let Some(else_branch) = &if_stmt.else_branch {
            self.analyze_statement(else_branch);
        }
    }

    fn analyze_while(&mut self, while_stmt: &WhileStmt) {
        self.loop_depth += 1;

        if let Ok(ctx) = self.analyze_expr_context(&while_stmt.condition) {
            if !self.is_convertible_to_bool(ctx.result_type) {
                self.errors.push(SemanticError::condition_must_be_bool());
            }
        }

        self.analyze_statement(&while_stmt.body);

        self.loop_depth -= 1;
    }

    fn analyze_do_while(&mut self, do_while: &DoWhileStmt) {
        self.loop_depth += 1;

        self.analyze_statement(&do_while.body);

        if let Ok(ctx) = self.analyze_expr_context(&do_while.condition) {
            if !self.is_convertible_to_bool(ctx.result_type) {
                self.errors.push(SemanticError::condition_must_be_bool());
            }
        }

        self.loop_depth -= 1;
    }

    fn analyze_for(&mut self, for_stmt: &ForStmt) {
        self.symbol_table.push_scope();
        self.loop_depth += 1;

        match &for_stmt.init {
            ForInit::Var(var) => self.analyze_var_decl(var),
            ForInit::Expr(Some(expr)) => {
                let _ = self.analyze_expr_context(expr);
            }
            ForInit::Expr(None) => {}
        }

        if let Some(condition) = &for_stmt.condition {
            if let Ok(ctx) = self.analyze_expr_context(condition) {
                if !self.is_convertible_to_bool(ctx.result_type) {
                    self.errors.push(SemanticError::condition_must_be_bool());
                }
            }
        }

        for increment in &for_stmt.increment {
            let _ = self.analyze_expr_context(increment);
        }

        self.analyze_statement(&for_stmt.body);

        self.loop_depth -= 1;
        self.symbol_table.pop_scope();
    }

    // src/compiler/semantic.rs - Fixed analyze_foreach

    fn analyze_foreach(&mut self, foreach: &ForEachStmt) {
        self.symbol_table.push_scope();
        self.loop_depth += 1;

        // Analyze the iterable expression
        let collection_ctx = self.analyze_expr_context(&foreach.iterable);

        if let Ok(ctx) = collection_ctx {
            let element_type = self.get_iterable_element_type(ctx.result_type);

            // foreach.variables is Vec<(Type, String)>
            // Register each variable in the symbol table
            for (var_type, var_name) in &foreach.variables {
                let type_id = self.resolve_type_from_ast(var_type);

                // Validate type compatibility with collection element type
                if element_type != TYPE_AUTO && !self.types_compatible(type_id, element_type) {
                    self.errors.push(SemanticError::new(format!(
                        "ForEach variable '{}' type {} incompatible with collection element type {}",
                        var_name, type_id, element_type
                    )));
                }

                let symbol = Symbol {
                    name: var_name.clone(),
                    kind: SymbolKind::Variable,
                    type_id,
                    is_const: var_type.is_const,
                    is_handle: var_type
                        .modifiers
                        .iter()
                        .any(|m| matches!(m, TypeModifier::Handle)),
                    is_reference: false,
                    namespace: self.current_namespace.clone(),
                };
                self.symbol_table.insert(var_name.clone(), symbol);
            }
        }

        // Analyze the loop body
        self.analyze_statement(&foreach.body);

        self.loop_depth -= 1;
        self.symbol_table.pop_scope();
    }

    fn get_iterable_element_type(&self, collection_type: TypeId) -> TypeId {
        // AngelScript foreach protocol requires these methods:
        // - opForBegin() - returns iterator
        // - opForEnd(iterator) - returns bool
        // - opForValue(iterator) - returns element (this is what we need)
        // - opForNext(iterator) - returns next iterator

        // Check for opForValue method
        if let Some(method) = self.find_method(collection_type, "opForValue") {
            return method.return_type;
        }

        // Check if it's a built-in array type
        // Arrays have special handling in AngelScript
        if let Some(type_info) = self.find_type_by_id(collection_type) {
            // Check if type name contains "array" or ends with "[]"
            if type_info.name.contains("array") || type_info.name.ends_with("[]") {
                // For array<T>, we'd need to extract T from template parameters
                // This requires template type tracking which we haven't implemented yet
                // For now, return TYPE_AUTO to indicate type inference needed
                return TYPE_AUTO;
            }
        }

        // No foreach protocol found - return AUTO for type inference
        TYPE_AUTO
    }

    fn analyze_return(&mut self, ret: &ReturnStmt) {
        if let Some(value) = &ret.value {
            let _ = self.analyze_expr_context(value);
        }
    }

    fn analyze_switch(&mut self, switch: &SwitchStmt) {
        self.switch_depth += 1;

        let _ = self.analyze_expr_context(&switch.value);

        for case in &switch.cases {
            if let CasePattern::Value(expr) = &case.pattern {
                let _ = self.analyze_expr_context(expr);
            }

            for stmt in &case.statements {
                self.analyze_statement(stmt);
            }
        }

        self.switch_depth -= 1;
    }

    fn analyze_global_var(&mut self, var: &Var) {
        self.analyze_var_decl(var);
    }

    fn analyze_namespace(&mut self, ns: &Namespace) {
        let saved_namespace = self.current_namespace.clone();
        self.current_namespace.extend(ns.name.clone());

        for item in &ns.items {
            self.analyze_script_node(item);
        }

        self.current_namespace = saved_namespace;
    }

    fn analyze_enum(&mut self, enum_def: &Enum) {
        // Validate enum values
        for variant in &enum_def.variants {
            if let Some(value_expr) = &variant.value {
                if let Ok(ctx) = self.analyze_expr_context(value_expr) {
                    if !self.is_numeric_type(ctx.result_type) {
                        self.errors.push(SemanticError::new(format!(
                            "Enum variant '{}' value must be numeric, got type {}",
                            variant.name, ctx.result_type
                        )));
                    }
                }
            }
        }
    }

    fn analyze_interface(&mut self, _interface: &Interface) {
        // Interface members already validated in resolve_interface_member_details
    }

    // ==================== EXPRESSION CONTEXT ANALYSIS ====================

    pub fn analyze_expr_context(&mut self, expr: &Expr) -> Result<ExprContext, SemanticError> {
        match expr {
            Expr::Literal(lit) => self.analyze_literal_context(lit),
            Expr::VarAccess(scope, name) => self.analyze_var_access_context(scope, name),
            Expr::Binary(left, op, right) => self.analyze_binary_context(left, op, right),
            Expr::Unary(op, operand) => self.analyze_unary_context(op, operand),
            Expr::Postfix(expr, op) => self.analyze_postfix_context(expr, op),
            Expr::Ternary(cond, then_expr, else_expr) => {
                self.analyze_ternary_context(cond, then_expr, else_expr)
            }
            Expr::FuncCall(call) => self.analyze_func_call_context(call),
            Expr::ConstructCall(type_def, args) => {
                self.analyze_construct_call_context(type_def, args)
            }
            Expr::Cast(target_type, expr) => self.analyze_cast_context(target_type, expr),
            Expr::Lambda(lambda) => self.analyze_lambda_context(lambda),
            Expr::InitList(init_list) => self.analyze_init_list_context(init_list),
            Expr::Void => Ok(ExprContext::value(TYPE_VOID)),
        }
    }

    fn analyze_literal_context(&self, lit: &Literal) -> Result<ExprContext, SemanticError> {
        let type_id = match lit {
            Literal::Bool(_) => TYPE_BOOL,
            Literal::Number(n) => {
                // Check suffixes first
                if n.ends_with("ll") || n.ends_with("LL") {
                    TYPE_INT64
                } else if n.ends_with('f') || n.ends_with('F') {
                    // Float suffix - always float regardless of decimal point
                    TYPE_FLOAT
                } else if n.ends_with('d') || n.ends_with('D') {
                    // Double suffix
                    TYPE_DOUBLE
                } else if n.contains('.') || n.contains('e') || n.contains('E') {
                    // Has decimal point or exponent - default to double
                    TYPE_DOUBLE
                } else if n.ends_with('u') || n.ends_with('U') {
                    // Unsigned suffix
                    TYPE_UINT32
                } else {
                    // Plain integer
                    TYPE_INT32
                }
            }
            Literal::String(_) => TYPE_STRING,
            Literal::Null => {
                return Ok(ExprContext {
                    result_type: 0,
                    is_lvalue: false,
                    is_temporary: true,
                    is_handle: true,
                    is_const: true,
                    requires_cleanup: false,
                    is_reference: false,
                    is_nullable: true,
                    location: None,
                });
            }
            Literal::Bits(_) => TYPE_UINT32,
        };

        Ok(ExprContext::value(type_id).with_const())
    }

    fn analyze_var_access_context(
        &self,
        scope: &Scope,
        name: &str,
    ) -> Result<ExprContext, SemanticError> {
        let symbol = self
            .symbol_table
            .lookup_with_scope(name, scope)
            .ok_or_else(|| SemanticError::undefined_symbol(name))?;

        Ok(ExprContext {
            result_type: symbol.type_id,
            is_lvalue: true,
            is_temporary: false,
            is_handle: symbol.is_handle,
            is_const: symbol.is_const,
            requires_cleanup: false,
            is_reference: symbol.is_reference,
            is_nullable: symbol.is_handle,
            location: None,
        })
    }

    fn analyze_binary_context(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
    ) -> Result<ExprContext, SemanticError> {
        let left_ctx = self.analyze_expr_context(left)?;
        let right_ctx = self.analyze_expr_context(right)?;

        match op {
            BinaryOp::Assign => self.analyze_assignment_context(&left_ctx, &right_ctx, false),

            BinaryOp::AddAssign
            | BinaryOp::SubAssign
            | BinaryOp::MulAssign
            | BinaryOp::DivAssign
            | BinaryOp::ModAssign
            | BinaryOp::PowAssign
            | BinaryOp::BitAndAssign
            | BinaryOp::BitOrAssign
            | BinaryOp::BitXorAssign
            | BinaryOp::ShlAssign
            | BinaryOp::ShrAssign
            | BinaryOp::UShrAssign => {
                self.analyze_compound_assignment_context(&left_ctx, &right_ctx, op)
            }

            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => {
                self.validate_comparison(&left_ctx, &right_ctx, op)?;
                Ok(ExprContext::value(TYPE_BOOL))
            }

            BinaryOp::Is | BinaryOp::IsNot => {
                self.validate_identity_comparison(&left_ctx, &right_ctx)?;
                Ok(ExprContext::value(TYPE_BOOL))
            }

            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => Ok(ExprContext::value(TYPE_BOOL)),

            _ => self.analyze_arithmetic_binary_context(&left_ctx, &right_ctx, op),
        }
    }

    fn analyze_assignment_context(
        &self,
        left_ctx: &ExprContext,
        right_ctx: &ExprContext,
        is_handle_assign: bool,
    ) -> Result<ExprContext, SemanticError> {
        if !left_ctx.is_lvalue {
            return Err(SemanticError::not_an_lvalue());
        }

        if left_ctx.is_const {
            return Err(SemanticError::cannot_modify_const());
        }

        if is_handle_assign {
            if !left_ctx.is_handle || !right_ctx.is_handle {
                return Err(SemanticError::handle_assignment_requires_handles());
            }
        }

        self.validate_type_match(left_ctx.result_type, right_ctx.result_type, "assignment")?;

        Ok(ExprContext {
            result_type: left_ctx.result_type,
            is_lvalue: true,
            is_temporary: false,
            is_handle: left_ctx.is_handle,
            is_const: false,
            requires_cleanup: false,
            is_reference: left_ctx.is_reference,
            is_nullable: left_ctx.is_nullable,
            location: None,
        })
    }

    fn analyze_compound_assignment_context(
        &self,
        left_ctx: &ExprContext,
        right_ctx: &ExprContext,
        op: &BinaryOp,
    ) -> Result<ExprContext, SemanticError> {
        if !left_ctx.is_lvalue {
            return Err(SemanticError::not_an_lvalue());
        }

        if left_ctx.is_const {
            return Err(SemanticError::cannot_modify_const());
        }

        let base_op = self.get_base_operation(op);
        self.validate_binary_operation(left_ctx.result_type, right_ctx.result_type, &base_op)?;

        Ok(ExprContext {
            result_type: left_ctx.result_type,
            is_lvalue: true,
            is_temporary: false,
            is_handle: left_ctx.is_handle,
            is_const: false,
            requires_cleanup: false,
            is_reference: left_ctx.is_reference,
            is_nullable: left_ctx.is_nullable,
            location: None,
        })
    }

    fn analyze_arithmetic_binary_context(
        &self,
        left_ctx: &ExprContext,
        right_ctx: &ExprContext,
        op: &BinaryOp,
    ) -> Result<ExprContext, SemanticError> {
        self.validate_binary_operation(left_ctx.result_type, right_ctx.result_type, op)?;

        let result_type =
            self.resolve_binary_result_type(left_ctx.result_type, right_ctx.result_type, op);

        Ok(ExprContext::value(result_type))
    }

    fn analyze_unary_context(
        &mut self,
        op: &UnaryOp,
        operand: &Expr,
    ) -> Result<ExprContext, SemanticError> {
        let operand_ctx = self.analyze_expr_context(operand)?;

        match op {
            UnaryOp::PreInc | UnaryOp::PreDec => {
                if !operand_ctx.is_lvalue {
                    return Err(SemanticError::not_an_lvalue());
                }
                if operand_ctx.is_const {
                    return Err(SemanticError::cannot_modify_const());
                }
                Ok(operand_ctx)
            }

            UnaryOp::Plus => Ok(ExprContext::value(operand_ctx.result_type)),

            UnaryOp::Neg | UnaryOp::Not | UnaryOp::BitNot => {
                Ok(ExprContext::value(operand_ctx.result_type))
            }

            UnaryOp::Handle => {
                if operand_ctx.is_handle {
                    return Err(SemanticError::already_a_handle());
                }

                if !self.can_be_handle(operand_ctx.result_type) {
                    return Err(SemanticError::new(format!(
                        "Type {} cannot be used as a handle",
                        operand_ctx.result_type
                    )));
                }

                Ok(ExprContext::handle(operand_ctx.result_type, false))
            }
        }
    }

    fn analyze_postfix_context(
        &mut self,
        expr: &Expr,
        op: &PostfixOp,
    ) -> Result<ExprContext, SemanticError> {
        let expr_ctx = self.analyze_expr_context(expr)?;

        match op {
            PostfixOp::PostInc | PostfixOp::PostDec => {
                if !expr_ctx.is_lvalue {
                    return Err(SemanticError::not_an_lvalue());
                }
                if expr_ctx.is_const {
                    return Err(SemanticError::cannot_modify_const());
                }
                Ok(ExprContext::value(expr_ctx.result_type))
            }

            PostfixOp::MemberAccess(member) => {
                self.analyze_member_access_context(&expr_ctx, member)
            }

            PostfixOp::MemberCall(call) => self.analyze_method_call_context(&expr_ctx, call),

            PostfixOp::Index(indices) => self.analyze_index_access_context(&expr_ctx, indices),

            PostfixOp::Call(args) => self.analyze_functor_call_context(&expr_ctx, args),
        }
    }

    fn analyze_member_access_context(
        &self,
        object_ctx: &ExprContext,
        member: &str,
    ) -> Result<ExprContext, SemanticError> {
        // Check for property first
        if let Some(prop_info) = self.find_property(object_ctx.result_type, member) {
            return Ok(ExprContext {
                result_type: prop_info.type_id,
                is_lvalue: !object_ctx.is_const && !prop_info.is_readonly,
                is_temporary: false,
                is_handle: prop_info.is_handle,
                is_const: object_ctx.is_const || prop_info.is_readonly,
                requires_cleanup: false,
                is_reference: false,
                is_nullable: prop_info.is_handle,
                location: None,
            });
        }

        // Check for member field
        let member_info = self
            .find_member(object_ctx.result_type, member)
            .ok_or_else(|| SemanticError::unknown_member(member))?;

        Ok(ExprContext {
            result_type: member_info.type_id,
            is_lvalue: !object_ctx.is_const,
            is_temporary: false,
            is_handle: member_info.is_handle,
            is_const: object_ctx.is_const || member_info.is_const,
            requires_cleanup: false,
            is_reference: false,
            is_nullable: member_info.is_handle,
            location: None,
        })
    }

    fn analyze_method_call_context(
        &mut self,
        object_ctx: &ExprContext,
        call: &FuncCall,
    ) -> Result<ExprContext, SemanticError> {
        let method = self
            .find_method(object_ctx.result_type, &call.name)
            .ok_or_else(|| {
                SemanticError::new(format!(
                    "Method '{}' not found on type {}",
                    call.name, object_ctx.result_type
                ))
            })?;

        // Validate const correctness
        if object_ctx.is_const && !method.is_const {
            return Err(SemanticError::new(format!(
                "Cannot call non-const method '{}' on const object",
                call.name
            )));
        }

        // Validate arguments
        if call.args.len() != method.params.len() {
            return Err(SemanticError::new(format!(
                "Method '{}' expects {} arguments, got {}",
                call.name,
                method.params.len(),
                call.args.len()
            )));
        }

        for (arg, param) in call.args.iter().zip(&method.params) {
            let arg_ctx = self.analyze_expr_context(&arg.value)?;
            if !self.types_compatible(param.type_id, arg_ctx.result_type) {
                return Err(SemanticError::new(format!(
                    "Argument type {} incompatible with parameter type {}",
                    arg_ctx.result_type, param.type_id
                )));
            }
        }

        let mut ctx = ExprContext::value(method.return_type);
        if self.is_object_type(method.return_type) {
            ctx.requires_cleanup = true;
        }

        Ok(ctx)
    }

    fn analyze_index_access_context(
        &mut self,
        array_ctx: &ExprContext,
        indices: &[IndexArg],
    ) -> Result<ExprContext, SemanticError> {
        // Validate index expressions
        for index in indices {
            let index_ctx = self.analyze_expr_context(&index.value)?;
            if !self.is_numeric_type(index_ctx.result_type) {
                return Err(SemanticError::new(format!(
                    "Array index must be numeric, got type {}",
                    index_ctx.result_type
                )));
            }
        }

        // Get element type from array/indexer
        let element_type = self.get_indexer_return_type(array_ctx.result_type);

        Ok(ExprContext {
            result_type: element_type,
            is_lvalue: !array_ctx.is_const,
            is_temporary: false,
            is_handle: false,
            is_const: array_ctx.is_const,
            requires_cleanup: false,
            is_reference: false,
            is_nullable: false,
            location: None,
        })
    }

    fn get_indexer_return_type(&self, type_id: TypeId) -> TypeId {
        // Check if type has opIndex method
        if let Some(method) = self.find_method(type_id, "opIndex") {
            return method.return_type;
        }

        // Default to int for unknown types
        TYPE_INT32
    }

    fn analyze_functor_call_context(
        &mut self,
        functor_ctx: &ExprContext,
        args: &[Arg],
    ) -> Result<ExprContext, SemanticError> {
        // Check if type has opCall method
        let method = self
            .find_method(functor_ctx.result_type, "opCall")
            .ok_or_else(|| {
                SemanticError::new(format!(
                    "Type {} is not callable (no opCall method)",
                    functor_ctx.result_type
                ))
            })?;

        // Validate arguments
        if args.len() != method.params.len() {
            return Err(SemanticError::new(format!(
                "Functor expects {} arguments, got {}",
                method.params.len(),
                args.len()
            )));
        }

        for (arg, param) in args.iter().zip(&method.params) {
            let arg_ctx = self.analyze_expr_context(&arg.value)?;
            if !self.types_compatible(param.type_id, arg_ctx.result_type) {
                return Err(SemanticError::new(format!(
                    "Argument type {} incompatible with parameter type {}",
                    arg_ctx.result_type, param.type_id
                )));
            }
        }

        Ok(ExprContext::value(method.return_type))
    }

    fn analyze_ternary_context(
        &mut self,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<ExprContext, SemanticError> {
        let cond_ctx = self.analyze_expr_context(cond)?;
        let then_ctx = self.analyze_expr_context(then_expr)?;
        let else_ctx = self.analyze_expr_context(else_expr)?;

        if !self.is_convertible_to_bool(cond_ctx.result_type) {
            return Err(SemanticError::condition_must_be_bool());
        }

        let result_type = if then_ctx.result_type == else_ctx.result_type {
            then_ctx.result_type
        } else if self.types_compatible(then_ctx.result_type, else_ctx.result_type) {
            self.get_common_type(then_ctx.result_type, else_ctx.result_type)
        } else {
            return Err(SemanticError::incompatible_ternary_branches(
                then_ctx.result_type,
                else_ctx.result_type,
            ));
        };

        Ok(ExprContext::value(result_type))
    }

    fn analyze_func_call_context(&mut self, call: &FuncCall) -> Result<ExprContext, SemanticError> {
        if let Some(func_info) = self.symbol_table.lookup_with_scope(&call.name, &call.scope) {
            let return_type = func_info.type_id;
            let mut ctx = ExprContext::value(return_type);

            if self.is_object_type(return_type) {
                ctx.requires_cleanup = true;
            }

            return Ok(ctx);
        }

        let engine = self.engine.read().unwrap();
        let func = engine.global_functions.iter().find(|f| f.name == call.name);

        if let Some(engine_func) = func {
            let return_type = engine_func.return_type_id;

            let mut ctx = ExprContext::value(return_type);

            if self.is_object_type(return_type) {
                ctx.requires_cleanup = true;
            }

            return Ok(ctx);
        }

        Err(SemanticError::undefined_function(&call.name))
    }

    fn analyze_construct_call_context(
        &mut self,
        type_def: &Type,
        args: &[Arg],
    ) -> Result<ExprContext, SemanticError> {
        let type_id = self.resolve_type_from_ast(type_def);

        // Validate constructor exists with matching signature
        if let Some(type_info) = self.find_type_by_id(type_id) {
            if let Some(constructors) = type_info.methods.get(&type_info.name) {
                let matching_ctor = constructors
                    .iter()
                    .find(|ctor| ctor.params.len() == args.len());

                if matching_ctor.is_none() {
                    return Err(SemanticError::new(format!(
                        "No constructor found for type {} with {} arguments",
                        type_id,
                        args.len()
                    )));
                }
            }
        }

        Ok(ExprContext::value(type_id).with_cleanup())
    }

    fn analyze_cast_context(
        &mut self,
        target_type: &Type,
        expr: &Expr,
    ) -> Result<ExprContext, SemanticError> {
        let expr_ctx = self.analyze_expr_context(expr)?;
        let target_type_id = self.resolve_type_from_ast(target_type);

        if !self.is_cast_allowed(expr_ctx.result_type, target_type_id) {
            return Err(SemanticError::invalid_cast(
                expr_ctx.result_type,
                target_type_id,
            ));
        }

        Ok(ExprContext::value(target_type_id))
    }

    fn analyze_lambda_context(&mut self, lambda: &Lambda) -> Result<ExprContext, SemanticError> {
        // Create a funcdef type for this lambda
        let lambda_type_id = self.allocate_type_id();

        // Analyze lambda body
        self.symbol_table.push_scope();

        for param in &lambda.params {
            if let Some(name) = &param.name {
                let type_id = &param
                    .param_type
                    .as_ref()
                    .map(|pt| self.resolve_type_from_ast(&pt))
                    .unwrap_or(TYPE_VOID);
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    type_id: type_id.clone(),
                    is_const: param
                        .param_type
                        .as_ref()
                        .map_or_else(|| false, |pt| pt.is_const),
                    is_handle: false,
                    is_reference: false,
                    namespace: self.current_namespace.clone(),
                };
                self.symbol_table.insert(name.clone(), symbol);
            }
        }

        self.analyze_statement_block(&lambda.body);

        self.symbol_table.pop_scope();

        Ok(ExprContext::value(lambda_type_id))
    }

    fn analyze_init_list_context(
        &mut self,
        init_list: &InitList,
    ) -> Result<ExprContext, SemanticError> {
        // Analyze all items
        for item in &init_list.items {
            match item {
                InitListItem::Expr(expr) => {
                    let _ = self.analyze_expr_context(expr)?;
                }
                InitListItem::InitList(nested) => {
                    let _ = self.analyze_init_list_context(nested)?;
                }
            }
        }

        // Init list type depends on context - return auto for now
        Ok(ExprContext::value(TYPE_AUTO))
    }

    // ==================== VALIDATION HELPERS ====================

    fn validate_comparison(
        &self,
        left_ctx: &ExprContext,
        right_ctx: &ExprContext,
        _op: &BinaryOp,
    ) -> Result<(), SemanticError> {
        if !self.types_compatible(left_ctx.result_type, right_ctx.result_type) {
            return Err(SemanticError::incompatible_types(
                left_ctx.result_type,
                right_ctx.result_type,
            ));
        }
        Ok(())
    }

    fn validate_identity_comparison(
        &self,
        left_ctx: &ExprContext,
        right_ctx: &ExprContext,
    ) -> Result<(), SemanticError> {
        if !left_ctx.is_handle && !left_ctx.is_nullable {
            return Err(SemanticError::identity_requires_handles());
        }
        if !right_ctx.is_handle && !right_ctx.is_nullable {
            return Err(SemanticError::identity_requires_handles());
        }
        Ok(())
    }

    fn validate_binary_operation(
        &self,
        left_type: TypeId,
        right_type: TypeId,
        op: &BinaryOp,
    ) -> Result<(), SemanticError> {
        // Check primitives
        if self.is_primitive_type(left_type) && self.is_primitive_type(right_type) {
            return Ok(());
        }

        // Check operator overloads
        let operator_name = self.get_operator_method_name(op);
        if let Some(op_name) = operator_name {
            if self.has_operator_overload(left_type, &op_name) {
                return Ok(());
            }
        }

        Err(SemanticError::invalid_operation())
    }

    fn get_operator_method_name(&self, op: &BinaryOp) -> Option<String> {
        let name = match op {
            BinaryOp::Add => "opAdd",
            BinaryOp::Sub => "opSub",
            BinaryOp::Mul => "opMul",
            BinaryOp::Div => "opDiv",
            BinaryOp::Mod => "opMod",
            BinaryOp::Pow => "opPow",
            BinaryOp::Eq => "opEquals",
            BinaryOp::Ne => "opEquals",
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => "opCmp",
            BinaryOp::BitAnd => "opAnd",
            BinaryOp::BitOr => "opOr",
            BinaryOp::BitXor => "opXor",
            BinaryOp::Shl => "opShl",
            BinaryOp::Shr => "opShr",
            BinaryOp::UShr => "opUShr",
            _ => return None,
        };
        Some(name.to_string())
    }

    fn has_operator_overload(&self, type_id: TypeId, operator: &str) -> bool {
        // Check script-defined methods
        if self.find_method(type_id, operator).is_some() {
            return true;
        }

        // Check engine-registered methods
        let engine = self.engine.read().unwrap();
        for obj_type in engine.object_types.values() {
            if obj_type.type_id == type_id {
                for method in &obj_type.methods {
                    if method.name == operator {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn validate_type_match(
        &self,
        expected_type: TypeId,
        actual_type: TypeId,
        context: &str,
    ) -> Result<(), SemanticError> {
        if expected_type == actual_type {
            return Ok(());
        }

        if self.types_compatible(expected_type, actual_type) {
            return Ok(());
        }

        if self.is_derived_from(actual_type, expected_type) {
            return Ok(());
        }

        Err(SemanticError::new(format!(
            "{}: type mismatch - expected {}, got {}",
            context, expected_type, actual_type
        )))
    }

    fn validate_break_in_loop(&self) -> Result<(), SemanticError> {
        if self.loop_depth == 0 && self.switch_depth == 0 {
            return Err(SemanticError::new(
                "Break statement must be inside a loop or switch".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_continue_in_loop(&self) -> Result<(), SemanticError> {
        if self.loop_depth == 0 {
            return Err(SemanticError::new(
                "Continue statement must be inside a loop".to_string(),
            ));
        }
        Ok(())
    }

    fn get_base_operation(&self, op: &BinaryOp) -> BinaryOp {
        match op {
            BinaryOp::AddAssign => BinaryOp::Add,
            BinaryOp::SubAssign => BinaryOp::Sub,
            BinaryOp::MulAssign => BinaryOp::Mul,
            BinaryOp::DivAssign => BinaryOp::Div,
            BinaryOp::ModAssign => BinaryOp::Mod,
            BinaryOp::PowAssign => BinaryOp::Pow,
            BinaryOp::BitAndAssign => BinaryOp::BitAnd,
            BinaryOp::BitOrAssign => BinaryOp::BitOr,
            BinaryOp::BitXorAssign => BinaryOp::BitXor,
            BinaryOp::ShlAssign => BinaryOp::Shl,
            BinaryOp::ShrAssign => BinaryOp::Shr,
            BinaryOp::UShrAssign => BinaryOp::UShr,
            _ => op.clone(),
        }
    }

    // ==================== TYPE HELPERS ====================

    pub fn types_compatible(&self, type1: TypeId, type2: TypeId) -> bool {
        if type1 == type2 {
            return true;
        }

        if self.is_numeric_type(type1) && self.is_numeric_type(type2) {
            return true;
        }

        false
    }

    fn is_convertible_to_bool(&self, type_id: TypeId) -> bool {
        type_id == TYPE_BOOL || self.is_numeric_type(type_id) || self.is_handle_type(type_id)
    }

    fn is_numeric_type(&self, type_id: TypeId) -> bool {
        matches!(
            type_id,
            TYPE_INT8
                | TYPE_INT16
                | TYPE_INT32
                | TYPE_INT64
                | TYPE_UINT8
                | TYPE_UINT16
                | TYPE_UINT32
                | TYPE_UINT64
                | TYPE_FLOAT
                | TYPE_DOUBLE
        )
    }

    fn is_primitive_type(&self, type_id: TypeId) -> bool {
        type_id <= TYPE_AUTO
    }

    fn is_handle_type(&self, type_id: TypeId) -> bool {
        // Check if type can be a handle
        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                return type_info.can_be_handle;
            }
        }
        false
    }

    fn can_be_handle(&self, type_id: TypeId) -> bool {
        self.is_handle_type(type_id)
    }

    fn is_object_type(&self, type_id: TypeId) -> bool {
        !self.is_primitive_type(type_id)
    }

    fn get_common_type(&self, type1: TypeId, type2: TypeId) -> TypeId {
        if type1 == TYPE_DOUBLE || type2 == TYPE_DOUBLE {
            return TYPE_DOUBLE;
        }
        if type1 == TYPE_FLOAT || type2 == TYPE_FLOAT {
            return TYPE_FLOAT;
        }
        if type1 == TYPE_INT64 || type2 == TYPE_INT64 {
            return TYPE_INT64;
        }
        if type1 == TYPE_UINT64 || type2 == TYPE_UINT64 {
            return TYPE_UINT64;
        }
        TYPE_INT32
    }

    fn resolve_binary_result_type(&self, left: TypeId, right: TypeId, op: &BinaryOp) -> TypeId {
        if matches!(
            op,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            return TYPE_BOOL;
        }

        if matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Xor) {
            return TYPE_BOOL;
        }

        self.get_common_type(left, right)
    }

    fn is_cast_allowed(&self, from: TypeId, to: TypeId) -> bool {
        // Same type
        if from == to {
            return true;
        }

        // Numeric conversions
        if self.is_numeric_type(from) && self.is_numeric_type(to) {
            return true;
        }

        // Handle conversions
        if self.is_handle_type(from) && self.is_handle_type(to) {
            // Check inheritance
            if self.is_derived_from(from, to) || self.is_derived_from(to, from) {
                return true;
            }
        }

        // Check for opCast or opImplCast
        if self.has_operator_overload(from, "opCast")
            || self.has_operator_overload(from, "opImplCast")
        {
            return true;
        }

        false
    }

    fn find_member(&self, type_id: TypeId, member: &str) -> Option<MemberInfo> {
        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                return type_info.members.get(member).cloned();
            }
        }
        None
    }

    fn find_property(&self, type_id: TypeId, property: &str) -> Option<PropertyInfo> {
        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                return type_info.properties.get(property).cloned();
            }
        }
        None
    }

    fn find_method(&self, type_id: TypeId, method_name: &str) -> Option<MethodInfo> {
        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                if let Some(methods) = type_info.methods.get(method_name) {
                    return methods.first().cloned();
                }
            }
        }
        None
    }

    fn find_method_in_hierarchy(&self, type_id: TypeId, method_name: &str) -> Option<MethodInfo> {
        if let Some(method) = self.find_method(type_id, method_name) {
            return Some(method);
        }

        for type_info in self.script_types.values() {
            if type_info.type_id == type_id {
                if let Some(base_type_id) = type_info.base_class {
                    return self.find_method_in_hierarchy(base_type_id, method_name);
                }
            }
        }

        None
    }

    fn is_derived_from(&self, derived_type_id: TypeId, base_type_id: TypeId) -> bool {
        if derived_type_id == base_type_id {
            return true;
        }

        let mut current_type_id = derived_type_id;

        loop {
            let base_class = self
                .script_types
                .values()
                .find(|t| t.type_id == current_type_id)
                .and_then(|t| t.base_class);

            match base_class {
                Some(base) => {
                    if base == base_type_id {
                        return true;
                    }
                    current_type_id = base;
                }
                None => return false,
            }
        }
    }

    fn find_type_by_id(&self, type_id: TypeId) -> Option<&TypeInfo> {
        self.script_types.values().find(|t| t.type_id == type_id)
    }

    fn get_type_id_by_name(&self, name: &str) -> Option<TypeId> {
        self.script_types.get(name).map(|t| t.type_id)
    }

    pub fn resolve_type_from_ast(&self, type_def: &Type) -> TypeId {
        match &type_def.datatype {
            DataType::PrimType(name) => {
                if let Some(type_info) = self.script_types.get(name) {
                    return type_info.type_id;
                }

                if let Some(type_id) = self.lookup_type_id(name) {
                    return type_id;
                }

                TYPE_INT32
            }
            DataType::Identifier(name) => {
                if let Some(type_info) = self.script_types.get(name) {
                    return type_info.type_id;
                }

                if let Some(type_id) = self.lookup_type_id(name) {
                    return type_id;
                }

                0
            }
            DataType::Auto => TYPE_AUTO,
            DataType::Question => TYPE_AUTO,
        }
    }

    pub fn lookup_type_id(&self, name: &str) -> Option<TypeId> {
        self.script_types.get(name).map(|t| t.type_id)
    }

    pub fn get_expr_type(&self, expr: &Expr) -> TypeId {
        match expr {
            Expr::Literal(Literal::Bool(_)) => TYPE_BOOL,
            Expr::Literal(Literal::Number(n)) => {
                if n.contains('.') {
                    if n.ends_with('f') || n.ends_with('F') {
                        TYPE_FLOAT
                    } else {
                        TYPE_DOUBLE
                    }
                } else if n.ends_with("ll") || n.ends_with("LL") {
                    TYPE_INT64
                } else {
                    TYPE_INT32
                }
            }
            Expr::Literal(Literal::String(_)) => TYPE_STRING,
            Expr::VarAccess(_, name) => self
                .symbol_table
                .lookup(name)
                .map(|s| s.type_id)
                .unwrap_or(TYPE_INT32),
            _ => TYPE_INT32,
        }
    }

    // ==================== PASS 9: FINAL VALIDATION ====================

    fn final_validation(&self, script: &Script) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Validate no circular inheritance
        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                if let Err(e) = self.validate_no_circular_inheritance(&class.name) {
                    errors.push(e);
                }
            }
        }

        // Validate all abstract methods are implemented
        for item in &script.items {
            if let ScriptNode::Class(class) = item {
                if !class.modifiers.contains(&"abstract".to_string()) {
                    if let Err(e) = self.validate_no_unimplemented_abstract_methods(&class.name) {
                        errors.push(e);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_no_circular_inheritance(&self, class_name: &str) -> Result<(), SemanticError> {
        let mut visited = std::collections::HashSet::new();
        let mut current = class_name;

        loop {
            if !visited.insert(current) {
                return Err(SemanticError::new(format!(
                    "Circular inheritance detected involving class '{}'",
                    class_name
                )));
            }

            if let Some(type_info) = self.script_types.get(current) {
                if let Some(base_type_id) = type_info.base_class {
                    if let Some(base_type) = self.find_type_by_id(base_type_id) {
                        current = &base_type.name;
                        continue;
                    }
                }
            }

            break;
        }

        Ok(())
    }

    fn validate_no_unimplemented_abstract_methods(
        &self,
        class_name: &str,
    ) -> Result<(), SemanticError> {
        let class_type = self
            .script_types
            .get(class_name)
            .ok_or_else(|| SemanticError::new(format!("Class '{}' not found", class_name)))?;

        // If class itself is abstract, it doesn't need to implement abstract methods
        // (We'd need to track this in TypeInfo - for now assume non-abstract)

        // Walk up the inheritance chain and collect all abstract methods
        let mut current_type_id = class_type.base_class;
        let mut abstract_methods: Vec<(String, MethodInfo)> = Vec::new();

        while let Some(base_id) = current_type_id {
            if let Some(base_type) = self.find_type_by_id(base_id) {
                // Collect methods that are virtual but not implemented
                // (In a full implementation, we'd have an is_abstract flag on MethodInfo)
                // For now, we check if the method is virtual and has no body
                for (method_name, methods) in &base_type.methods {
                    for method in methods {
                        if method.is_virtual && !method.is_final {
                            // Check if this method is already in our list
                            let already_collected =
                                abstract_methods.iter().any(|(name, _)| name == method_name);

                            if !already_collected {
                                abstract_methods.push((method_name.clone(), method.clone()));
                            }
                        }
                    }
                }

                current_type_id = base_type.base_class;
            } else {
                break;
            }
        }

        // Check each abstract method is implemented in the class
        for (method_name, abstract_method) in abstract_methods {
            let is_implemented =
                class_type
                    .methods
                    .get(&method_name)
                    .map_or(false, |class_methods| {
                        class_methods.iter().any(|m| {
                            // Method must match signature and be marked as override
                            self.method_signatures_match(m, &abstract_method)
                                && (m.is_override || !m.is_virtual)
                        })
                    });

            if !is_implemented {
                return Err(SemanticError::new(format!(
                    "Class '{}' must implement abstract method '{}' with signature {}",
                    class_name,
                    method_name,
                    self.format_method_signature(&abstract_method)
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // tests/semantic_test.rs

    use crate::compiler::semantic::{TYPE_BOOL, TYPE_DOUBLE, TYPE_FLOAT, TYPE_INT32, TYPE_STRING};
    use crate::compiler::symbol::{Symbol, SymbolKind};
    use crate::core::engine::EngineInner;
    use crate::parser::ast::{
        BinaryOp, Class, ClassMember, DataType, Enum, EnumVariant, Expr, Func, IfStmt, Interface,
        Literal, Namespace, Param, ReturnStmt, Scope, Script, ScriptNode, StatBlock, Statement,
        Type, UnaryOp, Var, VarDecl, VarInit, WhileStmt,
    };
    use crate::SemanticAnalyzer;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicU32;
    use std::sync::{Arc, RwLock};
    // ==================== HELPER FUNCTIONS ====================

    fn create_analyzer() -> SemanticAnalyzer {
        SemanticAnalyzer::new(Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            next_type_id: AtomicU32::new(100),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        })))
    }

    fn int_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("int".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn const_int_type() -> Type {
        Type {
            is_const: true,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("int".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn void_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("void".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn bool_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("bool".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn float_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("float".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn string_type() -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::PrimType("string".to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn int_literal(value: i32) -> Expr {
        Expr::Literal(Literal::Number(value.to_string()))
    }

    fn bool_literal(value: bool) -> Expr {
        Expr::Literal(Literal::Bool(value))
    }

    fn float_literal(value: f32) -> Expr {
        Expr::Literal(Literal::Number(format!("{}f", value)))
    }

    fn string_literal(value: &str) -> Expr {
        Expr::Literal(Literal::String(value.to_string()))
    }

    fn null_literal() -> Expr {
        Expr::Literal(Literal::Null)
    }

    fn var_expr(name: &str) -> Expr {
        Expr::VarAccess(
            Scope {
                is_global: false,
                path: vec![],
            },
            name.to_string(),
        )
    }

    fn binary_expr(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::Binary(Box::new(left), op, Box::new(right))
    }

    fn unary_expr(op: UnaryOp, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand))
    }

    fn ternary_expr(cond: Expr, then_expr: Expr, else_expr: Expr) -> Expr {
        Expr::Ternary(Box::new(cond), Box::new(then_expr), Box::new(else_expr))
    }

    fn simple_func(name: &str, return_type: Option<Type>, body: StatBlock) -> Func {
        Func {
            modifiers: vec![],
            visibility: None,
            return_type,
            is_ref: false,
            name: name.to_string(),
            params: vec![],
            is_const: false,
            attributes: vec![],
            body: Some(body),
        }
    }

    fn func_with_params(
        name: &str,
        return_type: Option<Type>,
        params: Vec<Param>,
        body: StatBlock,
    ) -> Func {
        Func {
            modifiers: vec![],
            visibility: None,
            return_type,
            is_ref: false,
            name: name.to_string(),
            params,
            is_const: false,
            attributes: vec![],
            body: Some(body),
        }
    }

    fn param(name: &str, param_type: Type) -> Param {
        Param {
            param_type,
            type_mod: None,
            name: Some(name.to_string()),
            default_value: None,
            is_variadic: false,
        }
    }

    fn var_decl(var_type: Type, name: &str, init: Option<Expr>) -> Var {
        Var {
            visibility: None,
            var_type,
            declarations: vec![VarDecl {
                name: name.to_string(),
                initializer: init.map(VarInit::Expr),
            }],
        }
    }

    fn return_stmt(value: Option<Expr>) -> Statement {
        Statement::Return(ReturnStmt { value })
    }

    fn expr_stmt(expr: Expr) -> Statement {
        Statement::Expr(Some(expr))
    }

    fn var_stmt(var: Var) -> Statement {
        Statement::Var(var)
    }

    fn if_stmt(
        condition: Expr,
        then_branch: Statement,
        else_branch: Option<Statement>,
    ) -> Statement {
        Statement::If(IfStmt {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        })
    }

    fn while_stmt(condition: Expr, body: Statement) -> Statement {
        Statement::While(WhileStmt {
            condition,
            body: Box::new(body),
        })
    }

    fn block(statements: Vec<Statement>) -> StatBlock {
        StatBlock { statements }
    }

    // ==================== LITERAL TESTS ====================

    #[test]
    fn test_literal_int() {
        let mut analyzer = create_analyzer();

        let expr = int_literal(42);
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
        assert!(!ctx.is_lvalue);
        assert!(ctx.is_temporary);
        assert!(ctx.is_const);
    }

    #[test]
    fn test_literal_bool() {
        let mut analyzer = create_analyzer();

        let expr = bool_literal(true);
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
        assert!(ctx.is_const);
    }

    #[test]
    fn test_literal_float() {
        let mut analyzer = create_analyzer();

        let expr = float_literal(3.14);
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_FLOAT);
    }

    #[test]
    fn test_literal_string() {
        let mut analyzer = create_analyzer();

        let expr = string_literal("hello");
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_STRING);
    }

    #[test]
    fn test_literal_null() {
        let mut analyzer = create_analyzer();

        let expr = null_literal();
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(ctx.is_handle);
        assert!(ctx.is_nullable);
    }

    // ==================== VARIABLE TESTS ====================

    #[test]
    fn test_variable_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(
                    int_type(),
                    "x",
                    Some(int_literal(42)),
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_variable_access() {
        let mut analyzer = create_analyzer();

        // Manually add variable to symbol table
        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = var_expr("x");
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
        assert!(ctx.is_lvalue);
        assert!(!ctx.is_temporary);
    }

    #[test]
    fn test_undefined_variable() {
        let mut analyzer = create_analyzer();

        let expr = var_expr("undefined");
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_const_variable() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: true,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = var_expr("x");
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(ctx.is_const);
    }

    // ==================== ARITHMETIC OPERATIONS ====================

    #[test]
    fn test_addition() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Add, int_literal(2));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
        assert!(!ctx.is_lvalue);
        assert!(ctx.is_temporary);
    }

    #[test]
    fn test_subtraction() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(5), BinaryOp::Sub, int_literal(3));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_multiplication() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(3), BinaryOp::Mul, int_literal(4));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_division() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(10), BinaryOp::Div, int_literal(2));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_type_promotion_int_to_float() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Add, float_literal(2.0));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_FLOAT);
    }

    // ==================== COMPARISON OPERATIONS ====================

    #[test]
    fn test_equality() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Eq, int_literal(1));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    #[test]
    fn test_inequality() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Ne, int_literal(2));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    #[test]
    fn test_less_than() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Lt, int_literal(2));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    #[test]
    fn test_greater_than() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(2), BinaryOp::Gt, int_literal(1));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    // ==================== LOGICAL OPERATIONS ====================

    #[test]
    fn test_logical_and() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(bool_literal(true), BinaryOp::And, bool_literal(false));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    #[test]
    fn test_logical_or() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(bool_literal(true), BinaryOp::Or, bool_literal(false));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    // ==================== BITWISE OPERATIONS ====================

    #[test]
    fn test_bitwise_and() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(0xFF), BinaryOp::BitAnd, int_literal(0x0F));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_bitwise_or() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(0xF0), BinaryOp::BitOr, int_literal(0x0F));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_bitwise_xor() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(0xFF), BinaryOp::BitXor, int_literal(0x0F));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    // ==================== UNARY OPERATIONS ====================

    #[test]
    fn test_unary_negation() {
        let mut analyzer = create_analyzer();

        let expr = unary_expr(UnaryOp::Neg, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_unary_not() {
        let mut analyzer = create_analyzer();

        let expr = unary_expr(UnaryOp::Not, bool_literal(true));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_BOOL);
    }

    #[test]
    fn test_unary_plus() {
        let mut analyzer = create_analyzer();

        let expr = unary_expr(UnaryOp::Plus, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_pre_increment() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = unary_expr(UnaryOp::PreInc, var_expr("x"));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
        assert!(ctx.is_lvalue);
    }

    #[test]
    fn test_pre_increment_on_const() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: true,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = unary_expr(UnaryOp::PreInc, var_expr("x"));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_pre_increment_on_literal() {
        let mut analyzer = create_analyzer();

        let expr = unary_expr(UnaryOp::PreInc, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    // ==================== ASSIGNMENT OPERATIONS ====================

    #[test]
    fn test_simple_assignment() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = binary_expr(var_expr("x"), BinaryOp::Assign, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
        assert!(ctx.is_lvalue);
    }

    #[test]
    fn test_assignment_to_const() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: true,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = binary_expr(var_expr("x"), BinaryOp::Assign, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_assignment_to_literal() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Assign, int_literal(42));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_compound_assignment_add() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        let expr = binary_expr(var_expr("x"), BinaryOp::AddAssign, int_literal(5));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    // ==================== TERNARY OPERATOR ====================

    #[test]
    fn test_ternary_operator() {
        let mut analyzer = create_analyzer();

        let expr = ternary_expr(bool_literal(true), int_literal(1), int_literal(2));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert_eq!(ctx.result_type, TYPE_INT32);
    }

    #[test]
    fn test_ternary_incompatible_branches() {
        let mut analyzer = create_analyzer();

        let expr = ternary_expr(bool_literal(true), int_literal(1), string_literal("hello"));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_ternary_non_bool_condition() {
        let mut analyzer = create_analyzer();

        // Int can be converted to bool, so this should work
        let expr = ternary_expr(int_literal(1), int_literal(2), int_literal(3));
        let result = analyzer.analyze_expr_context(&expr);

        assert!(result.is_ok());
    }

    // ==================== FUNCTION TESTS ====================

    #[test]
    fn test_function_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check function was registered
        let symbol = analyzer.symbol_table.lookup("test");
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().kind, SymbolKind::Function);
    }

    #[test]
    fn test_function_with_parameters() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "add",
                Some(int_type()),
                vec![param("a", int_type()), param("b", int_type())],
                block(vec![return_stmt(Some(binary_expr(
                    var_expr("a"),
                    BinaryOp::Add,
                    var_expr("b"),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_function_return_type_check() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(int_literal(42)))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== CLASS TESTS ====================

    #[test]
    fn test_class_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check class was registered
        let symbol = analyzer.symbol_table.lookup("MyClass");
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().kind, SymbolKind::Type);
    }

    #[test]
    fn test_class_with_members() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![ClassMember::Var(var_decl(int_type(), "value", None))],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== NAMESPACE TESTS ====================

    #[test]
    fn test_namespace_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Namespace(Namespace {
                name: vec!["MyNamespace".to_string()],
                items: vec![ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![]),
                ))],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_namespace() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Namespace(Namespace {
                name: vec!["Outer".to_string(), "Inner".to_string()],
                items: vec![ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![]),
                ))],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== CONTROL FLOW TESTS ====================

    #[test]
    fn test_if_statement() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![if_stmt(
                    bool_literal(true),
                    expr_stmt(int_literal(1)),
                    None,
                )]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_if_else_statement() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![if_stmt(
                    bool_literal(true),
                    expr_stmt(int_literal(1)),
                    Some(expr_stmt(int_literal(2))),
                )]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_while_loop() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![while_stmt(
                    bool_literal(true),
                    expr_stmt(int_literal(1)),
                )]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== ENUM TESTS ====================

    #[test]
    fn test_enum_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Enum(Enum {
                modifiers: vec![],
                name: "Color".to_string(),
                variants: vec![
                    EnumVariant {
                        name: "RED".to_string(),
                        value: Some(int_literal(0)),
                    },
                    EnumVariant {
                        name: "GREEN".to_string(),
                        value: Some(int_literal(1)),
                    },
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check enum was registered
        let symbol = analyzer.symbol_table.lookup("Color");
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().kind, SymbolKind::Type);
    }

    // ==================== INTERFACE TESTS ====================

    #[test]
    fn test_interface_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Interface(Interface {
                modifiers: vec![],
                name: "ITest".to_string(),
                extends: vec![],
                members: vec![],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check interface was registered
        let symbol = analyzer.symbol_table.lookup("ITest");
        assert!(symbol.is_some());
        assert_eq!(symbol.unwrap().kind, SymbolKind::Type);
    }

    // ==================== TYPE COMPATIBILITY TESTS ====================

    #[test]
    fn test_type_compatibility_same_type() {
        let analyzer = create_analyzer();

        assert!(analyzer.types_compatible(TYPE_INT32, TYPE_INT32));
    }

    #[test]
    fn test_type_compatibility_numeric_promotion() {
        let analyzer = create_analyzer();

        assert!(analyzer.types_compatible(TYPE_INT32, TYPE_FLOAT));
        assert!(analyzer.types_compatible(TYPE_FLOAT, TYPE_DOUBLE));
    }

    #[test]
    fn test_type_incompatibility() {
        let analyzer = create_analyzer();

        assert!(!analyzer.types_compatible(TYPE_INT32, TYPE_STRING));
        assert!(!analyzer.types_compatible(TYPE_BOOL, TYPE_STRING));
    }

    // ==================== SCOPE TESTS ====================

    #[test]
    fn test_local_scope() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        assert!(analyzer.symbol_table.lookup("x").is_some());

        analyzer.symbol_table.pop_scope();

        assert!(analyzer.symbol_table.lookup("x").is_none());
    }

    #[test]
    fn test_nested_scopes() {
        let mut analyzer = create_analyzer();

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "outer".to_string(),
            Symbol {
                name: "outer".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        analyzer.symbol_table.push_scope();
        analyzer.symbol_table.insert(
            "inner".to_string(),
            Symbol {
                name: "inner".to_string(),
                kind: SymbolKind::Variable,
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
                is_reference: false,
                namespace: vec![],
            },
        );

        // Both should be visible in inner scope
        assert!(analyzer.symbol_table.lookup("outer").is_some());
        assert!(analyzer.symbol_table.lookup("inner").is_some());

        analyzer.symbol_table.pop_scope();

        // Only outer should be visible
        assert!(analyzer.symbol_table.lookup("outer").is_some());
        assert!(analyzer.symbol_table.lookup("inner").is_none());
    }
}
