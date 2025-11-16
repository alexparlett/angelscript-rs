use crate::compiler::symbol_table::{
    ExprContext, FunctionInfo, GlobalVarInfo, MemberInfo, ParamInfo, ScopeType, SymbolTable,
    TypeInfo, TypeUsage, VariableLocation,
};
use crate::core::engine::EngineInner;
use crate::core::error::{SemanticError, SemanticResult};
use crate::core::types::{
    BehaviourType, ObjectType, TypeFlags, TypeId, TypeKind, TypeRegistration, TYPE_AUTO,
    TYPE_BOOL, TYPE_DOUBLE, TYPE_FLOAT, TYPE_INT16, TYPE_INT32, TYPE_INT64, TYPE_INT8,
    TYPE_STRING, TYPE_UINT16, TYPE_UINT32, TYPE_UINT64, TYPE_UINT8, TYPE_VOID,
};
use crate::parser::ast::{
    AccessorKind, BinaryOp, CasePattern, Class, ClassMember, Expr, ForInit, Func, FuncDef,
    InitListItem, Literal, Param, PostfixOp, Script, ScriptNode, StatBlock, Statement, Type,
    TypeMod, TypeModifier, UnaryOp, Var, VarInit,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SemanticAnalyzer {
    pub engine: Arc<RwLock<EngineInner>>,
    pub symbol_table: SymbolTable,
    current_namespace: Vec<String>,
    current_class: Option<String>,
    errors: Vec<SemanticError>,
    loop_depth: u32,
    switch_depth: u32,
}

impl SemanticAnalyzer {
    pub fn new(engine: Arc<RwLock<EngineInner>>) -> Self {
        let mut analyzer = Self {
            engine: engine.clone(),
            symbol_table: SymbolTable::new(),
            current_namespace: Vec::new(),
            current_class: None,
            errors: Vec::new(),
            loop_depth: 0,
            switch_depth: 0,
        };

        analyzer.import_engine_registrations();

        analyzer
    }

    fn import_engine_registrations(&mut self) {
        let engine = self.engine.read().unwrap();

        for (type_name, obj_type) in &engine.object_types {
            let flags = self.convert_object_type_flags(obj_type);

            let mut type_info = TypeInfo {
                type_id: obj_type.type_id,
                name: type_name.clone(),
                kind: TypeKind::Class,
                flags,
                members: HashMap::new(),
                methods: HashMap::new(),
                base_class: None,
                registration: TypeRegistration::Application,
            };

            for prop in &obj_type.properties {
                let member_info = MemberInfo {
                    name: prop.name.clone(),
                    type_id: prop.type_id,
                    is_const: prop.is_const,
                    visibility: None,
                };
                type_info.members.insert(prop.name.clone(), member_info);
            }

            for method in &obj_type.methods {
                let full_name = format!("{}::{}", type_name, method.name);

                type_info
                    .methods
                    .entry(method.name.clone())
                    .or_insert_with(Vec::new)
                    .push(full_name.clone());

                let func_info = FunctionInfo {
                    name: method.name.clone(),
                    full_name: full_name.clone(),
                    return_type: method.return_type_id,
                    params: method
                        .params
                        .iter()
                        .map(|p| ParamInfo {
                            name: Some(p.name.clone()),
                            type_id: p.type_id,
                            is_ref: p.is_ref,
                            is_out: p.is_out,
                            default_value: None,
                        })
                        .collect(),
                    is_method: true,
                    class_type: Some(obj_type.type_id),
                    is_const: method.is_const,
                    is_virtual: method.is_virtual,
                    is_override: false,
                    address: 0,
                    is_system_func: true,
                    system_func_id: Some(method.function_id),
                };

                self.symbol_table.register_function(func_info);
            }

            for behaviour in &obj_type.behaviours {
                let behaviour_name = self.get_behaviour_name(behaviour.behaviour_type);
                let full_name = format!("{}::{}", type_name, behaviour_name);

                let func_info = FunctionInfo {
                    name: behaviour_name.clone(),
                    full_name: full_name.clone(),
                    return_type: behaviour.return_type_id,
                    params: behaviour
                        .params
                        .iter()
                        .map(|p| ParamInfo {
                            name: Some(p.name.clone()),
                            type_id: p.type_id,
                            is_ref: p.is_ref,
                            is_out: p.is_out,
                            default_value: None,
                        })
                        .collect(),
                    is_method: true,
                    class_type: Some(obj_type.type_id),
                    is_const: false,
                    is_virtual: false,
                    is_override: false,
                    address: 0,
                    is_system_func: true,
                    system_func_id: Some(behaviour.function_id),
                };

                self.symbol_table.register_function(func_info);

                type_info
                    .methods
                    .entry(behaviour_name)
                    .or_insert_with(Vec::new)
                    .push(full_name);
            }

            self.symbol_table.register_type(type_info);
        }

        for (idx, global_func) in engine.global_functions.iter().enumerate() {
            let func_info = FunctionInfo {
                name: global_func.name.clone(),
                full_name: global_func.name.clone(),
                return_type: global_func.return_type_id,
                params: global_func
                    .params
                    .iter()
                    .map(|p| ParamInfo {
                        name: Some(p.name.clone()),
                        type_id: p.type_id,
                        is_ref: p.is_ref,
                        is_out: p.is_out,
                        default_value: None,
                    })
                    .collect(),
                is_method: false,
                class_type: None,
                is_const: false,
                is_virtual: false,
                is_override: false,
                address: 0,
                is_system_func: true,
                system_func_id: Some(idx as u32),
            };

            self.symbol_table.register_function(func_info);
        }

        for (idx, global_prop) in engine.global_properties.iter().enumerate() {
            let global_info = GlobalVarInfo {
                name: global_prop.name.clone(),
                type_id: global_prop.type_id,
                is_const: global_prop.is_const,
                address: idx as u32,
            };

            self.symbol_table.register_global(global_info);
        }

        for (enum_name, enum_type) in &engine.enum_types {
            let type_info = TypeInfo {
                type_id: enum_type.type_id,
                name: enum_name.clone(),
                kind: TypeKind::Enum,
                flags: TypeFlags::ENUM,
                members: HashMap::new(),
                methods: HashMap::new(),
                base_class: None,
                registration: TypeRegistration::Application,
            };

            self.symbol_table.register_type(type_info);
        }

        for (interface_name, interface_type) in &engine.interface_types {
            let type_info = TypeInfo {
                type_id: interface_type.type_id,
                name: interface_name.clone(),
                kind: TypeKind::Interface,
                flags: TypeFlags::ABSTRACT,
                members: HashMap::new(),
                methods: HashMap::new(),
                base_class: None,
                registration: TypeRegistration::Application,
            };

            self.symbol_table.register_type(type_info);
        }

        for (funcdef_name, funcdef) in &engine.funcdefs {
            let type_info = TypeInfo {
                type_id: funcdef.type_id,
                name: funcdef_name.clone(),
                kind: TypeKind::Funcdef,
                flags: TypeFlags::FUNCDEF,
                members: HashMap::new(),
                methods: HashMap::new(),
                base_class: None,
                registration: TypeRegistration::Application,
            };

            self.symbol_table.register_type(type_info);
        }
    }

    fn convert_object_type_flags(&self, obj_type: &ObjectType) -> TypeFlags {
        let mut flags = TypeFlags::empty();

        if obj_type.flags.contains(TypeFlags::REF_TYPE) {
            flags |= TypeFlags::REF_TYPE;
        }
        if obj_type.flags.contains(TypeFlags::VALUE_TYPE) {
            flags |= TypeFlags::VALUE_TYPE;
        }
        if obj_type.flags.contains(TypeFlags::POD_TYPE) {
            flags |= TypeFlags::POD_TYPE;
        }

        for behaviour in &obj_type.behaviours {
            match behaviour.behaviour_type {
                BehaviourType::Construct => flags |= TypeFlags::APP_CLASS_CONSTRUCTOR,
                BehaviourType::Destruct => flags |= TypeFlags::APP_CLASS_DESTRUCTOR,
                _ => {}
            }
        }

        flags
    }

    fn get_behaviour_name(&self, behaviour_type: BehaviourType) -> String {
        match behaviour_type {
            BehaviourType::Construct => "constructor".to_string(),
            BehaviourType::Destruct => "destructor".to_string(),
            BehaviourType::AddRef => "AddRef".to_string(),
            BehaviourType::Release => "Release".to_string(),
            _ => format!("{:?}", behaviour_type),
        }
    }

    pub fn analyze(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        self.errors.clear();

        for item in &script.items {
            match item {
                ScriptNode::Class(class) => {
                    self.register_class_type(class);
                }
                ScriptNode::FuncDef(funcdef) => {
                    self.register_funcdef_type(funcdef);
                }
                ScriptNode::Namespace(ns) => {
                    self.current_namespace.extend(ns.name.clone());
                    for nested_item in &ns.items {
                        match nested_item {
                            ScriptNode::Class(class) => self.register_class_type(class),
                            ScriptNode::FuncDef(funcdef) => self.register_funcdef_type(funcdef),
                            _ => {}
                        }
                    }
                    self.current_namespace
                        .truncate(self.current_namespace.len() - ns.name.len());
                }
                _ => {}
            }
        }

        for item in &script.items {
            match item {
                ScriptNode::Func(func) => {
                    self.register_function_signature(func);
                }
                ScriptNode::Var(var) => {
                    self.register_global_var(var);
                }
                ScriptNode::Class(class) => {
                    self.register_class_members(class);
                }
                ScriptNode::Namespace(ns) => {
                    self.current_namespace.extend(ns.name.clone());
                    for nested_item in &ns.items {
                        match nested_item {
                            ScriptNode::Func(func) => self.register_function_signature(func),
                            ScriptNode::Var(var) => self.register_global_var(var),
                            ScriptNode::Class(class) => self.register_class_members(class),
                            _ => {}
                        }
                    }
                    self.current_namespace
                        .truncate(self.current_namespace.len() - ns.name.len());
                }
                _ => {}
            }
        }

        for item in &script.items {
            match item {
                ScriptNode::Func(func) => {
                    if let Err(e) = self.analyze_function(func) {
                        self.errors.push(e);
                    }
                }
                ScriptNode::Class(class) => {
                    if let Err(e) = self.analyze_class(class) {
                        self.errors.push(e);
                    }
                }
                ScriptNode::Namespace(ns) => {
                    self.current_namespace.extend(ns.name.clone());
                    for nested_item in &ns.items {
                        match nested_item {
                            ScriptNode::Func(func) => {
                                if let Err(e) = self.analyze_function(func) {
                                    self.errors.push(e);
                                }
                            }
                            ScriptNode::Class(class) => {
                                if let Err(e) = self.analyze_class(class) {
                                    self.errors.push(e);
                                }
                            }
                            _ => {}
                        }
                    }
                    self.current_namespace
                        .truncate(self.current_namespace.len() - ns.name.len());
                }
                _ => {}
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn register_class_type(&mut self, class: &Class) {
        let base_class = if !class.extends.is_empty() {
            let base_type_id = self.symbol_table.lookup_type(&class.extends[0]);

            if let Some(base_id) = base_type_id {
                if let Err(e) = self.validate_type_usage(base_id, TypeUsage::AsBaseClass) {
                    self.errors.push(e);
                }
            }

            base_type_id
        } else {
            None
        };

        let type_id = crate::core::types::allocate_type_id();

        let type_info = TypeInfo {
            type_id,
            name: class.name.clone(),
            kind: TypeKind::Class,
            flags: TypeFlags::REF_TYPE,
            members: HashMap::new(),
            methods: HashMap::new(),
            base_class,
            registration: TypeRegistration::Script,
        };

        self.symbol_table.register_type(type_info);
    }

    fn register_funcdef_type(&mut self, funcdef: &FuncDef) {
        let return_type = self
            .symbol_table
            .resolve_type_from_ast(&funcdef.return_type);

        let param_types: Vec<TypeId> = funcdef
            .params
            .iter()
            .map(|p| self.symbol_table.resolve_type_from_ast(&p.param_type))
            .collect();

        let funcdef_type_id = self
            .symbol_table
            .create_funcdef_type(return_type, param_types);

        let type_info = TypeInfo {
            type_id: funcdef_type_id,
            name: funcdef.name.clone(),
            kind: TypeKind::Funcdef,
            flags: TypeFlags::FUNCDEF,
            members: HashMap::new(),
            methods: HashMap::new(),
            base_class: None,
            registration: TypeRegistration::Script,
        };

        self.symbol_table.register_type(type_info);
    }

    fn register_class_members(&mut self, class: &Class) {
        let type_id = match self.symbol_table.lookup_type(&class.name) {
            Some(id) => id,
            None => return,
        };

        let mut type_info = match self.symbol_table.get_type(type_id) {
            Some(info) => info,
            None => return,
        };

        let type_info_mut = Arc::make_mut(&mut type_info);

        let saved_class = self.current_class.clone();
        self.current_class = Some(class.name.clone());

        for member in &class.members {
            match member {
                ClassMember::Var(var) => {
                    let member_type = self.symbol_table.resolve_type_from_ast(&var.var_type);

                    for decl in &var.declarations {
                        let member_info = MemberInfo {
                            name: decl.name.clone(),
                            type_id: member_type,
                            is_const: var.var_type.is_const,
                            visibility: var.visibility.clone(),
                        };

                        type_info_mut.members.insert(decl.name.clone(), member_info);

                        if let Some(VarInit::Expr(expr)) = &decl.initializer {
                            self.symbol_table.set_member_initializer(
                                class.name.clone(),
                                decl.name.clone(),
                                expr.clone(),
                            );
                        }
                    }
                }
                ClassMember::Func(func) => {
                    self.register_function_signature(func);

                    let method_name = func.name.clone();
                    let full_name = format!("{}::{}", class.name, method_name);

                    type_info_mut
                        .methods
                        .entry(method_name)
                        .or_insert_with(Vec::new)
                        .push(full_name);
                }
                ClassMember::VirtProp(prop) => {
                    let prop_type = self.symbol_table.resolve_type_from_ast(&prop.prop_type);

                    let member_info = MemberInfo {
                        name: prop.name.clone(),
                        type_id: prop_type,
                        is_const: false,
                        visibility: prop.visibility.clone(),
                    };

                    type_info_mut.members.insert(prop.name.clone(), member_info);

                    for accessor in &prop.accessors {
                        let method_name = match accessor.kind {
                            AccessorKind::Get => format!("get_{}", prop.name),
                            AccessorKind::Set => format!("set_{}", prop.name),
                        };

                        let full_name = format!("{}::{}", class.name, method_name);

                        let return_type = match accessor.kind {
                            AccessorKind::Get => prop_type,
                            AccessorKind::Set => TYPE_VOID,
                        };

                        let params = match accessor.kind {
                            AccessorKind::Get => vec![],
                            AccessorKind::Set => vec![ParamInfo {
                                name: Some("value".to_string()),
                                type_id: prop_type,
                                is_ref: false,
                                is_out: false,
                                default_value: None,
                            }],
                        };

                        let func_info = FunctionInfo {
                            name: method_name.clone(),
                            full_name: full_name.clone(),
                            return_type,
                            params,
                            is_method: true,
                            class_type: Some(type_id),
                            is_const: accessor.is_const,
                            is_virtual: false,
                            is_override: false,
                            address: 0,
                            is_system_func: false,
                            system_func_id: None,
                        };

                        self.symbol_table.register_function(func_info);

                        type_info_mut
                            .methods
                            .entry(method_name)
                            .or_insert_with(Vec::new)
                            .push(full_name);
                    }
                }
                _ => {}
            }
        }

        self.current_class = saved_class;

        self.symbol_table
            .register_type(Arc::try_unwrap(type_info).unwrap_or_else(|arc| (*arc).clone()));
    }

    fn register_function_signature(&mut self, func: &Func) {
        if let Err(e) = self.validate_function_params(func) {
            self.errors.push(e);
        }

        let return_type = func
            .return_type
            .as_ref()
            .map(|t| self.symbol_table.resolve_type_from_ast(t))
            .unwrap_or(TYPE_VOID);

        let params = func
            .params
            .iter()
            .map(|p| ParamInfo {
                name: p.name.clone(),
                type_id: self.symbol_table.resolve_type_from_ast(&p.param_type),
                is_ref: matches!(p.type_mod, Some(TypeMod::InOut) | Some(TypeMod::Out)),
                is_out: matches!(p.type_mod, Some(TypeMod::Out)),
                default_value: None,
            })
            .collect();

        let full_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func.name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func.name)
        } else {
            func.name.clone()
        };

        let func_info = FunctionInfo {
            name: func.name.clone(),
            full_name: full_name.clone(),
            return_type,
            params,
            is_method: self.current_class.is_some(),
            class_type: self
                .current_class
                .as_ref()
                .and_then(|name| self.symbol_table.lookup_type(name)),
            is_const: func.is_const,
            is_virtual: func.modifiers.contains(&"virtual".to_string()),
            is_override: func.modifiers.contains(&"override".to_string()),
            address: 0,
            is_system_func: false,
            system_func_id: None,
        };

        self.symbol_table.register_function(func_info);
    }

    fn validate_function_params(&self, func: &Func) -> SemanticResult<()> {
        for param in &func.params {
            let type_id = self.symbol_table.resolve_type_from_ast(&param.param_type);

            if let Some(type_mod) = &param.type_mod {
                self.validate_reference_param(type_id, type_mod, &param.param_type)?;
            }
        }

        Ok(())
    }

    fn validate_reference_param(
        &self,
        type_id: TypeId,
        type_mod: &TypeMod,
        param_type: &Type,
    ) -> SemanticResult<()> {
        if type_id <= TYPE_STRING {
            if matches!(type_mod, TypeMod::InOut) {
                return Err(SemanticError::ReferenceMismatch {
                    message:
                        "Primitive types cannot use 'inout' references. Use 'in' or 'out' instead."
                            .to_string(),
                    location: None,
                });
            }
            return Ok(());
        }

        let type_info = self.symbol_table.get_type(type_id);

        if let Some(type_info) = type_info {
            let is_value_type = type_info.flags.contains(TypeFlags::VALUE_TYPE);
            let is_ref_type = type_info.flags.contains(TypeFlags::REF_TYPE);

            match type_mod {
                TypeMod::InOut => {
                    if is_value_type {
                        return Err(SemanticError::ReferenceMismatch {
                            message: format!(
                                "Value type '{}' cannot use 'inout' references. AngelScript cannot guarantee the reference will remain valid during function execution. Use 'in' or 'out' instead.",
                                type_info.name
                            ),
                            location: None,
                        });
                    }

                    if !is_ref_type {
                        return Err(SemanticError::ReferenceMismatch {
                            message: format!(
                                "Type '{}' must be a reference type to use 'inout' references",
                                type_info.name
                            ),
                            location: None,
                        });
                    }
                }

                TypeMod::In => {
                    if !param_type.is_const && is_ref_type {
                        return Err(SemanticError::ReferenceMismatch {
                            message: format!(
                                "Reference type '{}' should use 'const &in' for input-only parameters to avoid unnecessary copies",
                                type_info.name
                            ),
                            location: None,
                        });
                    }
                }

                TypeMod::Out => {
                    if param_type.is_const {
                        return Err(SemanticError::ReferenceMismatch {
                            message: "Output parameters cannot be const".to_string(),
                            location: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn register_global_var(&mut self, var: &Var) {
        let type_id = self.symbol_table.resolve_type_from_ast(&var.var_type);

        for (idx, decl) in var.declarations.iter().enumerate() {
            let full_name = if !self.current_namespace.is_empty() {
                format!("{}::{}", self.current_namespace.join("::"), decl.name)
            } else {
                decl.name.clone()
            };

            let global_info = GlobalVarInfo {
                name: full_name,
                type_id,
                is_const: var.var_type.is_const,
                address: idx as u32,
            };

            self.symbol_table.register_global(global_info);
        }
    }

    fn analyze_function(&mut self, func: &Func) -> SemanticResult<()> {
        let func_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func.name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func.name)
        } else {
            func.name.clone()
        };

        self.symbol_table
            .push_scope(ScopeType::Function(func_name.clone()));

        if let Some(class_name) = &self.current_class {
            if let Some(class_type_id) = self.symbol_table.lookup_type(class_name) {
                self.symbol_table.register_local(
                    "this".to_string(),
                    class_type_id,
                    func.is_const,
                    true,
                );
            }
        }

        for param in &func.params {
            if let Some(name) = &param.name {
                let type_id = self.symbol_table.resolve_type_from_ast(&param.param_type);
                self.symbol_table
                    .register_local(name.clone(), type_id, false, true);
            }
        }

        if let Some(body) = &func.body {
            self.analyze_statement_block(body)?;
        }

        self.symbol_table.save_function_locals(func_name);
        self.symbol_table.pop_scope();

        Ok(())
    }

    fn analyze_class(&mut self, class: &Class) -> SemanticResult<()> {
        self.current_class = Some(class.name.clone());

        for member in &class.members {
            match member {
                ClassMember::Func(func) => {
                    self.analyze_function(func)?;
                }
                ClassMember::VirtProp(prop) => {
                    for accessor in &prop.accessors {
                        if let Some(body) = &accessor.body {
                            let func_name = match accessor.kind {
                                AccessorKind::Get => format!("get_{}", prop.name),
                                AccessorKind::Set => format!("set_{}", prop.name),
                            };

                            let func = Func {
                                modifiers: vec![],
                                visibility: prop.visibility.clone(),
                                return_type: match accessor.kind {
                                    AccessorKind::Get => Some(prop.prop_type.clone()),
                                    AccessorKind::Set => None,
                                },
                                is_ref: prop.is_ref,
                                name: func_name,
                                params: match accessor.kind {
                                    AccessorKind::Get => vec![],
                                    AccessorKind::Set => vec![Param {
                                        param_type: prop.prop_type.clone(),
                                        type_mod: None,
                                        name: Some("value".to_string()),
                                        default_value: None,
                                        is_variadic: false,
                                    }],
                                },
                                is_const: accessor.is_const,
                                attributes: accessor.attributes.clone(),
                                body: Some(body.clone()),
                            };

                            self.analyze_function(&func)?;
                        }
                    }
                }
                _ => {}
            }
        }

        let mut vtable = Vec::new();
        for member in &class.members {
            if let ClassMember::Func(func) = member {
                if func.modifiers.contains(&"virtual".to_string())
                    || func.modifiers.contains(&"override".to_string())
                {
                    let full_name = format!("{}::{}", class.name, func.name);
                    vtable.push(full_name);
                }
            }
        }

        if let Some(type_id) = self.symbol_table.lookup_type(&class.name) {
            self.symbol_table.set_vtable(type_id, vtable);
        }

        self.current_class = None;

        Ok(())
    }

    fn analyze_statement_block(&mut self, block: &StatBlock) -> SemanticResult<()> {
        for stmt in &block.statements {
            self.analyze_statement(stmt)?;
        }
        Ok(())
    }

    fn in_function(&self) -> bool {
        self.symbol_table.current_function_name().is_some()
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> SemanticResult<()> {
        match stmt {
            Statement::Var(var) => {
                let type_id = self.symbol_table.resolve_type_from_ast(&var.var_type);

                if type_id > TYPE_STRING {
                    self.validate_type_usage(type_id, TypeUsage::AsVariable)?;
                }

                let is_handle = var.var_type.modifiers.contains(&TypeModifier::Handle);

                if is_handle {
                    self.validate_type_usage(type_id, TypeUsage::AsHandle)?;
                }

                for decl in &var.declarations {
                    let has_duplicate_in_current_scope = self
                        .symbol_table
                        .scopes
                        .last()
                        .map(|scope| scope.has_variable(&decl.name))
                        .unwrap_or(false);

                    if has_duplicate_in_current_scope {
                        return Err(SemanticError::DuplicateDefinition {
                            name: decl.name.clone(),
                            location: None,
                            previous_location: None,
                        });
                    }

                    self.symbol_table.register_local(
                        decl.name.clone(),
                        type_id,
                        var.var_type.is_const,
                        false,
                    );

                    if let Some(VarInit::Expr(expr)) = &decl.initializer {
                        self.analyze_expr(expr)?;
                    }
                }
                Ok(())
            }

            Statement::Break => {
                if self.loop_depth == 0 && self.switch_depth == 0 {
                    Err(SemanticError::InvalidBreak { location: None })
                } else {
                    Ok(())
                }
            }

            Statement::Continue => {
                if self.loop_depth == 0 {
                    Err(SemanticError::InvalidContinue { location: None })
                } else {
                    Ok(())
                }
            }

            Statement::Expr(Some(expr)) => {
                self.analyze_expr(expr)?;
                Ok(())
            }

            Statement::Expr(None) => Ok(()),

            Statement::If(if_stmt) => {
                self.analyze_expr(&if_stmt.condition)?;
                self.analyze_statement(&if_stmt.then_branch)?;
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.analyze_statement(else_branch)?;
                }
                Ok(())
            }

            Statement::While(while_stmt) => {
                self.loop_depth += 1;
                self.analyze_expr(&while_stmt.condition)?;
                self.analyze_statement(&while_stmt.body)?;
                self.loop_depth -= 1;
                Ok(())
            }

            Statement::DoWhile(do_while) => {
                self.loop_depth += 1;
                self.analyze_statement(&do_while.body)?;
                self.analyze_expr(&do_while.condition)?;
                self.loop_depth -= 1;
                Ok(())
            }

            Statement::For(for_stmt) => {
                self.symbol_table.push_scope(ScopeType::Block);
                self.loop_depth += 1;

                match &for_stmt.init {
                    ForInit::Var(var) => {
                        let type_id = self.symbol_table.resolve_type_from_ast(&var.var_type);
                        for decl in &var.declarations {
                            self.symbol_table.register_local(
                                decl.name.clone(),
                                type_id,
                                false,
                                false,
                            );
                            if let Some(VarInit::Expr(expr)) = &decl.initializer {
                                self.analyze_expr(expr)?;
                            }
                        }
                    }
                    ForInit::Expr(Some(expr)) => {
                        self.analyze_expr(expr)?;
                    }
                    _ => {}
                }

                if let Some(cond) = &for_stmt.condition {
                    self.analyze_expr(cond)?;
                }

                for expr in &for_stmt.increment {
                    self.analyze_expr(expr)?;
                }

                self.analyze_statement(&for_stmt.body)?;

                self.loop_depth -= 1;

                if !self.in_function() {
                    self.symbol_table.pop_scope();
                }

                Ok(())
            }

            Statement::ForEach(foreach_stmt) => {
                self.symbol_table.push_scope(ScopeType::Block);
                self.loop_depth += 1;

                self.analyze_expr(&foreach_stmt.iterable)?;

                for (var_type, var_name) in &foreach_stmt.variables {
                    let type_id = self.symbol_table.resolve_type_from_ast(var_type);
                    self.symbol_table
                        .register_local(var_name.clone(), type_id, false, false);
                }

                self.analyze_statement(&foreach_stmt.body)?;

                self.loop_depth -= 1;

                if !self.in_function() {
                    self.symbol_table.pop_scope();
                }

                Ok(())
            }

            Statement::Switch(switch_stmt) => {
                self.switch_depth += 1;

                self.analyze_expr(&switch_stmt.value)?;

                for case in &switch_stmt.cases {
                    match &case.pattern {
                        CasePattern::Value(expr) => {
                            self.analyze_expr(expr)?;
                        }
                        CasePattern::Default => {}
                    }

                    for stmt in &case.statements {
                        self.analyze_statement(stmt)?;
                    }
                }

                self.switch_depth -= 1;
                Ok(())
            }

            Statement::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.analyze_expr(value)?;
                }
                Ok(())
            }

            Statement::Block(block) => {
                self.symbol_table.push_scope(ScopeType::Block);
                self.analyze_statement_block(block)?;

                if !self.in_function() {
                    self.symbol_table.pop_scope();
                }

                Ok(())
            }

            Statement::Try(try_stmt) => {
                self.analyze_statement_block(&try_stmt.try_block)?;
                self.analyze_statement_block(&try_stmt.catch_block)?;
                Ok(())
            }

            Statement::Using(_) => Ok(()),
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> SemanticResult<ExprContext> {
        let context = match expr {
            Expr::Literal(lit) => {
                let type_id = self.analyze_literal(lit);
                ExprContext::new(type_id)
            }

            Expr::VarAccess(_, name) => match self.symbol_table.lookup_variable(name) {
                Some(VariableLocation::Local(var)) => {
                    ExprContext::local_var(var.type_id, var.is_const, var.index)
                }
                Some(VariableLocation::Global(var)) => {
                    ExprContext::global_var(var.type_id, var.is_const, var.address)
                }
                None => {
                    return Err(SemanticError::undefined_symbol(name.clone()));
                }
            },

            Expr::Binary(left, op, right) => {
                let left_ctx = self.analyze_expr(left)?;
                let right_ctx = self.analyze_expr(right)?;
                self.analyze_binary_op(left_ctx, op, right_ctx)?
            }

            Expr::Unary(op, operand) => {
                let operand_ctx = self.analyze_expr(operand)?;

                if matches!(op, UnaryOp::Handle) {
                    self.validate_type_usage(operand_ctx.result_type, TypeUsage::AsHandle)?;
                }

                self.analyze_unary_op(op, operand_ctx)?
            }

            Expr::FuncCall(call) => {
                for arg in &call.args {
                    self.analyze_expr(&arg.value)?;
                }

                if let Some(func) = self.symbol_table.get_function(&call.name) {
                    ExprContext::new(func.return_type)
                } else {
                    return Err(SemanticError::undefined_function(call.name.clone()));
                }
            }

            Expr::Postfix(obj, op) => {
                let obj_ctx = self.analyze_expr(obj)?;
                self.analyze_postfix(obj_ctx, op)?
            }

            Expr::Ternary(cond, then_expr, else_expr) => {
                self.analyze_expr(cond)?;
                let then_ctx = self.analyze_expr(then_expr)?;
                let else_ctx = self.analyze_expr(else_expr)?;

                let result_type = if then_ctx.result_type == else_ctx.result_type {
                    then_ctx.result_type
                } else {
                    self.get_common_type(then_ctx.result_type, else_ctx.result_type)
                };

                ExprContext::new(result_type)
            }

            Expr::ConstructCall(type_def, args) => {
                for arg in args {
                    self.analyze_expr(&arg.value)?;
                }
                let type_id = self.symbol_table.resolve_type_from_ast(type_def);

                self.validate_type_usage(type_id, TypeUsage::AsVariable)?;
                self.validate_required_behaviours(type_id)?;

                ExprContext::handle(type_id)
            }

            Expr::Cast(target_type, expr) => {
                self.analyze_expr(expr)?;
                let type_id = self.symbol_table.resolve_type_from_ast(target_type);
                ExprContext::new(type_id)
            }

            Expr::Lambda(lambda) => {
                let lambda_name = format!("$lambda_{}", self.errors.len());

                self.symbol_table
                    .push_scope(ScopeType::Function(lambda_name.clone()));

                let mut param_types = Vec::new();
                for param in &lambda.params {
                    if let Some(name) = &param.name {
                        let type_id = if let Some(param_type) = &param.param_type {
                            self.symbol_table.resolve_type_from_ast(param_type)
                        } else {
                            TYPE_AUTO
                        };

                        param_types.push(type_id);

                        self.symbol_table
                            .register_local(name.clone(), type_id, false, true);
                    }
                }

                let mut return_type = TYPE_VOID;
                for stmt in &lambda.body.statements {
                    self.analyze_statement(stmt)?;

                    if let Statement::Return(ret) = stmt {
                        if let Some(value) = &ret.value {
                            if let Some(ret_type) = self.symbol_table.get_expr_type(value) {
                                return_type = ret_type;
                                break;
                            }
                        }
                    }
                }

                self.symbol_table.save_function_locals(lambda_name.clone());
                self.symbol_table.pop_scope();

                let funcdef_type = self
                    .symbol_table
                    .get_or_create_funcdef(return_type, &param_types);

                let func_info = FunctionInfo {
                    name: lambda_name.clone(),
                    full_name: lambda_name,
                    return_type,
                    params: lambda
                        .params
                        .iter()
                        .enumerate()
                        .map(|(i, p)| ParamInfo {
                            name: p.name.clone(),
                            type_id: param_types.get(i).copied().unwrap_or(TYPE_AUTO),
                            is_ref: matches!(p.type_mod, Some(TypeMod::InOut) | Some(TypeMod::Out)),
                            is_out: matches!(p.type_mod, Some(TypeMod::Out)),
                            default_value: None,
                        })
                        .collect(),
                    is_method: false,
                    class_type: None,
                    is_const: false,
                    is_virtual: false,
                    is_override: false,
                    address: 0,
                    is_system_func: false,
                    system_func_id: None,
                };

                self.symbol_table.register_function(func_info);

                ExprContext::new(funcdef_type)
            }

            Expr::InitList(init_list) => {
                let mut element_types = Vec::new();

                for item in &init_list.items {
                    match item {
                        InitListItem::Expr(expr) => {
                            let ctx = self.analyze_expr(expr)?;
                            element_types.push(ctx.result_type);
                        }
                        InitListItem::InitList(nested) => {
                            let ctx = self.analyze_expr(&Expr::InitList(nested.clone()))?;
                            element_types.push(ctx.result_type);
                        }
                    }
                }

                ExprContext::new(TYPE_VOID)
            }

            Expr::Void => ExprContext::new(TYPE_VOID),
        };

        self.symbol_table.set_expr_context(expr, context.clone());

        Ok(context)
    }

    fn analyze_literal(&self, lit: &Literal) -> TypeId {
        match lit {
            Literal::Bool(_) => TYPE_BOOL,
            Literal::Number(n) => {
                if n.ends_with("ull") || n.ends_with("ULL") {
                    return TYPE_UINT64;
                }
                if n.ends_with("ll") || n.ends_with("LL") {
                    return TYPE_INT64;
                }
                if n.ends_with("ul") || n.ends_with("UL") || n.ends_with("lu") || n.ends_with("LU")
                {
                    return TYPE_UINT32;
                }
                if n.ends_with("u") || n.ends_with("U") {
                    return TYPE_UINT32;
                }
                if n.ends_with("l") || n.ends_with("L") {
                    return TYPE_INT64;
                }
                if n.ends_with("f") || n.ends_with("F") {
                    return TYPE_FLOAT;
                }

                if n.contains('.') || n.contains('e') || n.contains('E') {
                    TYPE_DOUBLE
                } else {
                    TYPE_INT32
                }
            }
            Literal::String(_) => TYPE_STRING,
            Literal::Null => TYPE_VOID,
            Literal::Bits(_) => TYPE_UINT32,
        }
    }

    fn analyze_binary_op(
        &self,
        left_ctx: ExprContext,
        op: &BinaryOp,
        right_ctx: ExprContext,
    ) -> SemanticResult<ExprContext> {
        match op {
            BinaryOp::Assign
            | BinaryOp::AddAssign
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
                if !left_ctx.is_lvalue {
                    return Err(SemanticError::InvalidAssignment {
                        target: "expression".to_string(),
                        reason: "not an lvalue".to_string(),
                        location: None,
                    });
                }

                if left_ctx.is_const {
                    return Err(SemanticError::ConstViolation {
                        message: "cannot assign to const variable".to_string(),
                        location: None,
                    });
                }

                self.validate_type_usage(left_ctx.result_type, TypeUsage::InAssignment)?;

                Ok(left_ctx)
            }

            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::And
            | BinaryOp::Or => Ok(ExprContext::new(TYPE_BOOL)),

            _ => {
                let result_type = self.get_common_type(left_ctx.result_type, right_ctx.result_type);
                Ok(ExprContext::new(result_type))
            }
        }
    }

    fn analyze_unary_op(
        &self,
        op: &UnaryOp,
        operand_ctx: ExprContext,
    ) -> SemanticResult<ExprContext> {
        match op {
            UnaryOp::PreInc | UnaryOp::PreDec => {
                if !operand_ctx.is_lvalue {
                    return Err(SemanticError::InvalidOperation {
                        operation: format!("{:?}", op),
                        type_name: "non-lvalue".to_string(),
                        location: None,
                    });
                }

                if operand_ctx.is_const {
                    return Err(SemanticError::ConstViolation {
                        message: format!("cannot apply {:?} to const variable", op),
                        location: None,
                    });
                }

                Ok(operand_ctx)
            }

            UnaryOp::Handle => Ok(ExprContext::handle(operand_ctx.result_type)),

            UnaryOp::Not => Ok(ExprContext::new(TYPE_BOOL)),

            UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot => {
                Ok(ExprContext::new(operand_ctx.result_type))
            }
        }
    }

    fn analyze_postfix(
        &mut self,
        obj_ctx: ExprContext,
        op: &PostfixOp,
    ) -> SemanticResult<ExprContext> {
        match op {
            PostfixOp::PostInc | PostfixOp::PostDec => {
                if !obj_ctx.is_lvalue {
                    return Err(SemanticError::InvalidOperation {
                        operation: format!("{:?}", op),
                        type_name: "non-lvalue".to_string(),
                        location: None,
                    });
                }

                if obj_ctx.is_const {
                    return Err(SemanticError::ConstViolation {
                        message: format!("cannot apply {:?} to const variable", op),
                        location: None,
                    });
                }

                Ok(ExprContext::new(obj_ctx.result_type))
            }

            PostfixOp::MemberAccess(member) => {
                if let Some(type_info) = self.symbol_table.get_type(obj_ctx.result_type) {
                    if let Some(member_info) = type_info.members.get(member) {
                        let is_lvalue = obj_ctx.is_lvalue && !member_info.is_const;
                        let is_const = obj_ctx.is_const || member_info.is_const;

                        if is_lvalue {
                            Ok(ExprContext::lvalue(member_info.type_id, is_const))
                        } else {
                            Ok(ExprContext::new(member_info.type_id))
                        }
                    } else {
                        Err(SemanticError::undefined_member(
                            type_info.name.clone(),
                            member.clone(),
                        ))
                    }
                } else {
                    Err(SemanticError::InvalidOperation {
                        operation: "member access".to_string(),
                        type_name: format!("type {}", obj_ctx.result_type),
                        location: None,
                    })
                }
            }

            PostfixOp::MemberCall(call) => {
                for arg in &call.args {
                    self.analyze_expr(&arg.value)?;
                }

                if let Some(type_info) = self.symbol_table.get_type(obj_ctx.result_type) {
                    if let Some(method_overloads) = type_info.methods.get(&call.name) {
                        if let Some(full_name) = method_overloads.first() {
                            if let Some(func_info) = self.symbol_table.get_function(full_name) {
                                return Ok(ExprContext::new(func_info.return_type));
                            }
                        }
                    }

                    Err(SemanticError::undefined_member(
                        type_info.name.clone(),
                        call.name.clone(),
                    ))
                } else {
                    Err(SemanticError::InvalidOperation {
                        operation: "method call".to_string(),
                        type_name: format!("type {}", obj_ctx.result_type),
                        location: None,
                    })
                }
            }

            PostfixOp::Index(_indices) => {
                if obj_ctx.is_lvalue {
                    Ok(ExprContext::lvalue(TYPE_INT32, obj_ctx.is_const))
                } else {
                    Ok(ExprContext::new(TYPE_INT32))
                }
            }

            PostfixOp::Call(_args) => Ok(ExprContext::new(TYPE_VOID)),
        }
    }

    fn get_common_type(&self, type1: TypeId, type2: TypeId) -> TypeId {
        if type1 == type2 {
            return type1;
        }

        let rank = |t: TypeId| -> u32 {
            match t {
                TYPE_DOUBLE => 6,
                TYPE_FLOAT => 5,
                TYPE_INT64 => 4,
                TYPE_UINT64 => 4,
                TYPE_UINT32 => 3,
                TYPE_INT32 => 2,
                TYPE_INT16 => 1,
                TYPE_UINT16 => 1,
                TYPE_INT8 => 0,
                TYPE_UINT8 => 0,
                TYPE_BOOL => 0,
                _ => 2,
            }
        };

        if rank(type1) > rank(type2) {
            type1
        } else {
            type2
        }
    }

    pub fn get_expr_context(&self, expr: &Expr) -> Option<&ExprContext> {
        self.symbol_table.get_expr_context(expr)
    }

    pub fn analyze_expr_context(&mut self, expr: &Expr) -> SemanticResult<ExprContext> {
        self.analyze_expr(expr)
    }

    fn validate_type_usage(&self, type_id: TypeId, usage: TypeUsage) -> SemanticResult<()> {
        let type_info = self
            .symbol_table
            .get_type(type_id)
            .ok_or_else(|| SemanticError::undefined_type(format!("type {}", type_id)))?;

        let engine = self.engine.read().unwrap();
        let obj_type = engine.object_types.values().find(|t| t.type_id == type_id);

        if let Some(obj_type) = obj_type {
            match usage {
                TypeUsage::AsHandle => {
                    if obj_type.flags.contains(TypeFlags::NOHANDLE) {
                        return Err(SemanticError::InvalidHandle {
                            message: format!("Type '{}' cannot be used as handle", type_info.name),
                            location: None,
                        });
                    }
                }

                TypeUsage::AsBaseClass => {
                    if obj_type.flags.contains(TypeFlags::NOINHERIT) {
                        return Err(SemanticError::Internal {
                            message: format!(
                                "Type '{}' is final and cannot be inherited",
                                type_info.name
                            ),
                            location: None,
                        });
                    }
                }

                TypeUsage::AsVariable => {
                    if obj_type.flags.contains(TypeFlags::ABSTRACT) {
                        return Err(SemanticError::InstantiateAbstract {
                            class: type_info.name.clone(),
                            location: None,
                        });
                    }
                }

                TypeUsage::InAssignment => {
                    if obj_type.flags.contains(TypeFlags::SCOPED) {
                        return Err(SemanticError::InvalidAssignment {
                            target: type_info.name.clone(),
                            reason: "scoped type cannot be assigned".to_string(),
                            location: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn validate_required_behaviours(&self, type_id: TypeId) -> SemanticResult<()> {
        let type_info = self
            .symbol_table
            .get_type(type_id)
            .ok_or_else(|| SemanticError::undefined_type(format!("type {}", type_id)))?;

        // ✅ Only validate behaviours for application-registered types
        if type_info.registration != TypeRegistration::Application {
            return Ok(()); // Script types don't need behaviours
        }

        // ✅ Only validate if it's a class (not enum, funcdef, etc.)
        if type_info.kind != TypeKind::Class {
            return Ok(());
        }

        let engine = self.engine.read().unwrap();
        let obj_type = engine
            .object_types
            .values()
            .find(|t| t.type_id == type_id)
            .ok_or_else(|| SemanticError::undefined_type(type_info.name.clone()))?;

        // ✅ Reference types need AddRef/Release (unless NOCOUNT)
        if obj_type.flags.contains(TypeFlags::REF_TYPE) {
            if !obj_type.flags.contains(TypeFlags::NOCOUNT) {
                let has_addref = obj_type
                    .behaviours
                    .iter()
                    .any(|b| b.behaviour_type == BehaviourType::AddRef);
                let has_release = obj_type
                    .behaviours
                    .iter()
                    .any(|b| b.behaviour_type == BehaviourType::Release);

                if !has_addref || !has_release {
                    return Err(SemanticError::Internal {
                        message: format!(
                            "Reference type '{}' must have AddRef and Release behaviours",
                            type_info.name
                        ),
                        location: None,
                    });
                }
            }

            // ✅ Need factory to instantiate (unless NOHANDLE)
            let has_factory = obj_type
                .behaviours
                .iter()
                .any(|b| b.behaviour_type == BehaviourType::Construct);

            if !has_factory && !obj_type.flags.contains(TypeFlags::NOHANDLE) {
                return Err(SemanticError::Internal {
                    message: format!(
                        "Reference type '{}' must have a factory behaviour",
                        type_info.name
                    ),
                    location: None,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::core::types::{
        AccessSpecifier, BehaviourInfo, EnumType, FuncdefInfo, GlobalFunction, GlobalProperty,
        MethodParam, ObjectMethod, ObjectProperty, TypeKind, TYPE_DOUBLE, TYPE_INT64,
        TYPE_UINT32, TYPE_UINT64, TYPE_VOID,
    };
    use crate::parser::ast::{
        AccessorKind, Arg, BinaryOp, Case, CasePattern, Class, ClassMember, DataType, Expr,
        ForEachStmt, Func, FuncCall, FuncDef, IfStmt, IndexArg, InitList, InitListItem, Lambda,
        LambdaParam, Namespace, Param, PostfixOp, PropertyAccessor, ReturnStmt, Scope, Script,
        ScriptNode, StatBlock, Statement, SwitchStmt, TryStmt, Type, TypeMod, TypeModifier,
        UnaryOp, Var, VarDecl, VarInit, VirtProp, Visibility, WhileStmt,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};
    // ==================== HELPER FUNCTIONS ====================

    fn create_analyzer() -> SemanticAnalyzer {
        SemanticAnalyzer::new(Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
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

    fn class_type(name: &str) -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::Identifier(name.to_string()),
            template_types: vec![],
            modifiers: vec![],
        }
    }

    fn handle_type(name: &str) -> Type {
        Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            datatype: DataType::Identifier(name.to_string()),
            template_types: vec![],
            modifiers: vec![TypeModifier::Handle],
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

    fn func_call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::FuncCall(FuncCall {
            scope: Scope {
                is_global: false,
                path: vec![],
            },
            name: name.to_string(),
            template_types: vec![],
            args: args
                .into_iter()
                .map(|e| Arg {
                    name: None,
                    value: e,
                })
                .collect(),
        })
    }

    fn method_call(obj: Expr, method: &str, args: Vec<Expr>) -> Expr {
        Expr::Postfix(
            Box::new(obj),
            PostfixOp::MemberCall(FuncCall {
                scope: Scope {
                    is_global: false,
                    path: vec![],
                },
                name: method.to_string(),
                template_types: vec![],
                args: args
                    .into_iter()
                    .map(|e| Arg {
                        name: None,
                        value: e,
                    })
                    .collect(),
            }),
        )
    }

    fn member_access(obj: Expr, member: &str) -> Expr {
        Expr::Postfix(Box::new(obj), PostfixOp::MemberAccess(member.to_string()))
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

    fn param_with_mod(name: &str, param_type: Type, type_mod: TypeMod) -> Param {
        Param {
            param_type,
            type_mod: Some(type_mod),
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

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(int_literal(42)))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Check expression was analyzed
        let expr = int_literal(42);
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert!(ctx.is_some());
        assert_eq!(ctx.unwrap().result_type, TYPE_INT32);
        assert!(ctx.unwrap().is_temporary);
    }

    #[test]
    fn test_literal_bool() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(bool_literal(true)))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = bool_literal(true);
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_BOOL);
    }

    #[test]
    fn test_literal_float() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(float_type()),
                block(vec![return_stmt(Some(float_literal(3.14)))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = float_literal(3.14);
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_FLOAT);
    }

    #[test]
    fn test_literal_string() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(string_type()),
                block(vec![return_stmt(Some(string_literal("hello")))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = string_literal("hello");
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_STRING);
    }

    #[test]
    fn test_literal_null() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![return_stmt(Some(null_literal()))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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

        // Check variable was registered in function locals
        let func_locals = analyzer.symbol_table.get_function_locals("test");
        assert!(func_locals.is_some());
        assert_eq!(func_locals.unwrap().total_count, 1);
    }

    #[test]
    fn test_variable_access() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(42)))),
                    return_stmt(Some(var_expr("x"))),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Check variable access was analyzed
        let expr = var_expr("x");
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        if let Some(ctx) = ctx {
            assert_eq!(ctx.result_type, TYPE_INT32);
            assert!(ctx.is_lvalue);
            assert!(!ctx.is_temporary);
        }
    }

    #[test]
    fn test_undefined_variable() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(var_expr("undefined")))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::UndefinedSymbol { .. }))
        );
    }

    #[test]
    fn test_const_variable() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![
                    var_stmt(var_decl(const_int_type(), "x", Some(int_literal(42)))),
                    return_stmt(Some(var_expr("x"))),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = var_expr("x");
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert!(ctx.unwrap().is_const);
    }

    #[test]
    fn test_duplicate_variable() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(1)))),
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(2)))),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::DuplicateDefinition { .. }))
        );
    }

    // ==================== ARITHMETIC OPERATIONS ====================

    #[test]
    fn test_addition() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(1),
                    BinaryOp::Add,
                    int_literal(2),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Add, int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_INT32);
        assert!(!ctx.unwrap().is_lvalue);
        assert!(ctx.unwrap().is_temporary);
    }

    #[test]
    fn test_subtraction() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(5),
                    BinaryOp::Sub,
                    int_literal(3),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiplication() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(3),
                    BinaryOp::Mul,
                    int_literal(4),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_division() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(10),
                    BinaryOp::Div,
                    int_literal(2),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_promotion_int_to_float() {
        let mut analyzer = create_analyzer();

        let expr = binary_expr(int_literal(1), BinaryOp::Add, float_literal(2.0));

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(float_type()),
                block(vec![return_stmt(Some(expr.clone()))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Add, float_literal(2.0));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_FLOAT);
    }

    // ==================== COMPARISON OPERATIONS ====================

    #[test]
    fn test_equality() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(1),
                    BinaryOp::Eq,
                    int_literal(1),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Eq, int_literal(1));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_BOOL);
    }

    #[test]
    fn test_less_than() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(1),
                    BinaryOp::Lt,
                    int_literal(2),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Lt, int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_BOOL);
    }

    #[test]
    fn test_greater_than() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(2),
                    BinaryOp::Gt,
                    int_literal(1),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== LOGICAL OPERATIONS ====================

    #[test]
    fn test_logical_and() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(binary_expr(
                    bool_literal(true),
                    BinaryOp::And,
                    bool_literal(false),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(bool_literal(true), BinaryOp::And, bool_literal(false));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_BOOL);
    }

    #[test]
    fn test_logical_or() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(binary_expr(
                    bool_literal(true),
                    BinaryOp::Or,
                    bool_literal(false),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== BITWISE OPERATIONS ====================

    #[test]
    fn test_bitwise_and() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(0xFF),
                    BinaryOp::BitAnd,
                    int_literal(0x0F),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(0xFF), BinaryOp::BitAnd, int_literal(0x0F));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_INT32);
    }

    #[test]
    fn test_bitwise_or() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(0xF0),
                    BinaryOp::BitOr,
                    int_literal(0x0F),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bitwise_xor() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(binary_expr(
                    int_literal(0xFF),
                    BinaryOp::BitXor,
                    int_literal(0x0F),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== UNARY OPERATIONS ====================

    #[test]
    fn test_unary_negation() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(unary_expr(
                    UnaryOp::Neg,
                    int_literal(42),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = unary_expr(UnaryOp::Neg, int_literal(42));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_INT32);
    }

    #[test]
    fn test_unary_not() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(bool_type()),
                block(vec![return_stmt(Some(unary_expr(
                    UnaryOp::Not,
                    bool_literal(true),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = unary_expr(UnaryOp::Not, bool_literal(true));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_BOOL);
    }

    #[test]
    fn test_pre_increment_on_variable() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(unary_expr(UnaryOp::PreInc, var_expr("x"))),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_pre_increment_on_const() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(const_int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(unary_expr(UnaryOp::PreInc, var_expr("x"))),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. }))
        );
    }

    #[test]
    fn test_pre_increment_on_literal() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(unary_expr(
                    UnaryOp::PreInc,
                    int_literal(42),
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidOperation { .. }))
        );
    }

    // ==================== ASSIGNMENT OPERATIONS ====================

    #[test]
    fn test_simple_assignment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", None)),
                    expr_stmt(binary_expr(
                        var_expr("x"),
                        BinaryOp::Assign,
                        int_literal(42),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_assignment_to_const() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(const_int_type(), "x", Some(int_literal(10)))),
                    expr_stmt(binary_expr(
                        var_expr("x"),
                        BinaryOp::Assign,
                        int_literal(42),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. }))
        );
    }

    #[test]
    fn test_assignment_to_literal() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(binary_expr(
                    int_literal(1),
                    BinaryOp::Assign,
                    int_literal(42),
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidAssignment { .. }))
        );
    }

    #[test]
    fn test_compound_assignment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(10)))),
                    expr_stmt(binary_expr(
                        var_expr("x"),
                        BinaryOp::AddAssign,
                        int_literal(5),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    // ==================== TERNARY OPERATOR ====================

    #[test]
    fn test_ternary_operator() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(ternary_expr(
                    bool_literal(true),
                    int_literal(1),
                    int_literal(2),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = ternary_expr(bool_literal(true), int_literal(1), int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_INT32);
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
        let func = analyzer.symbol_table.get_function("test");
        assert!(func.is_some());
        assert_eq!(func.unwrap().name, "test");
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

        // Check parameters were registered
        let func_locals = analyzer.symbol_table.get_function_locals("add");
        assert!(func_locals.is_some());
        assert_eq!(func_locals.unwrap().param_count, 2);
    }

    #[test]
    fn test_function_call() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(simple_func(
                    "helper",
                    Some(int_type()),
                    block(vec![return_stmt(Some(int_literal(42)))]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![return_stmt(Some(func_call("helper", vec![])))]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_undefined_function_call() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(func_call("undefined", vec![]))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::UndefinedFunction { .. }))
        );
    }

    #[test]
    fn test_function_with_ref_parameters() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "modify",
                Some(void_type()),
                vec![param_with_mod("x", int_type(), TypeMod::Out)],
                block(vec![expr_stmt(binary_expr(
                    var_expr("x"),
                    BinaryOp::Assign,
                    int_literal(42),
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("modify");
        assert!(func.is_some());
        assert!(func.unwrap().params[0].is_out);
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
        let type_id = analyzer.symbol_table.lookup_type("MyClass");
        assert!(type_id.is_some());
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

        // Check member was registered
        let type_id = analyzer.symbol_table.lookup_type("MyClass").unwrap();
        let type_info = analyzer.symbol_table.get_type(type_id).unwrap();
        assert!(type_info.members.contains_key("value"));
    }

    #[test]
    fn test_class_with_methods() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![ClassMember::Func(simple_func(
                    "method",
                    Some(void_type()),
                    block(vec![]),
                ))],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check method was registered
        let func = analyzer.symbol_table.get_function("MyClass::method");
        assert!(func.is_some());
        assert!(func.unwrap().is_method);
    }

    #[test]
    fn test_class_constructor() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(var_decl(int_type(), "value", None)),
                    ClassMember::Func(func_with_params(
                        "MyClass",
                        None, // Constructor has no return type
                        vec![param("v", int_type())],
                        block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "value"),
                            BinaryOp::Assign,
                            var_expr("v"),
                        ))]),
                    )),
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_class_inheritance() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Base".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(var_decl(int_type(), "baseValue", None))],
                }),
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Derived".to_string(),
                    extends: vec!["Base".to_string()],
                    members: vec![ClassMember::Var(var_decl(int_type(), "derivedValue", None))],
                }),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check inheritance was registered
        let derived_id = analyzer.symbol_table.lookup_type("Derived").unwrap();
        let derived_info = analyzer.symbol_table.get_type(derived_id).unwrap();
        assert!(derived_info.base_class.is_some());

        let base_id = analyzer.symbol_table.lookup_type("Base").unwrap();
        assert_eq!(derived_info.base_class.unwrap(), base_id);
    }

    #[test]
    fn test_member_access() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(var_decl(int_type(), "value", None))],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        return_stmt(Some(member_access(var_expr("obj"), "value"))),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_undefined_member_access() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        expr_stmt(member_access(var_expr("obj"), "undefined")),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::UndefinedMember { .. }))
        );
    }

    #[test]
    fn test_method_call() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Func(simple_func(
                        "method",
                        Some(int_type()),
                        block(vec![return_stmt(Some(int_literal(42)))]),
                    ))],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        return_stmt(Some(method_call(var_expr("obj"), "method", vec![]))),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    // ==================== NAMESPACE TESTS ====================

    #[test]
    fn test_namespace() {
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

        // Check function was registered with namespace
        let func = analyzer.symbol_table.get_function("MyNamespace::test");
        assert!(func.is_some());
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

        // Check function was registered with full namespace path
        let func = analyzer.symbol_table.get_function("Outer::Inner::test");
        assert!(func.is_some());
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

    #[test]
    fn test_break_outside_loop() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Break]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidBreak { .. }))
        );
    }

    #[test]
    fn test_continue_outside_loop() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Continue]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidContinue { .. }))
        );
    }

    #[test]
    fn test_break_inside_loop() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![while_stmt(bool_literal(true), Statement::Break)]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== SCOPE TESTS ====================

    #[test]
    fn test_variable_scope() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "outer", Some(int_literal(1)))),
                    Statement::Block(block(vec![
                        var_stmt(var_decl(int_type(), "inner", Some(int_literal(2)))),
                        expr_stmt(var_expr("outer")), // Should be accessible
                        expr_stmt(var_expr("inner")), // Should be accessible
                    ])),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_variable_shadowing() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(1)))),
                    Statement::Block(block(vec![
                        var_stmt(var_decl(int_type(), "x", Some(int_literal(2)))), // Shadow outer x
                    ])),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        // Shadowing is allowed
        assert!(result.is_ok());
    }

    // ==================== HANDLE TESTS ====================

    #[test]
    fn test_handle_type() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![var_stmt(var_decl(
                        handle_type("MyClass"),
                        "obj",
                        None,
                    ))]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_assignment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(handle_type("MyClass"), "obj1", None)),
                        var_stmt(var_decl(handle_type("MyClass"), "obj2", None)),
                        expr_stmt(binary_expr(
                            var_expr("obj1"),
                            BinaryOp::Assign,
                            var_expr("obj2"),
                        )),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== VIRTUAL METHODS ====================

    #[test]
    fn test_virtual_method() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Base".to_string(),
                extends: vec![],
                members: vec![ClassMember::Func(Func {
                    modifiers: vec!["virtual".to_string()],
                    visibility: None,
                    return_type: Some(void_type()),
                    is_ref: false,
                    name: "method".to_string(),
                    params: vec![],
                    is_const: false,
                    attributes: vec![],
                    body: Some(block(vec![])),
                })],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check vtable was built
        let type_id = analyzer.symbol_table.lookup_type("Base").unwrap();
        let vtable = analyzer.symbol_table.get_vtable(type_id);
        assert!(vtable.is_some());
        assert_eq!(vtable.unwrap().len(), 1);
    }

    #[test]
    fn test_override_method() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Base".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Func(Func {
                        modifiers: vec!["virtual".to_string()],
                        visibility: None,
                        return_type: Some(void_type()),
                        is_ref: false,
                        name: "method".to_string(),
                        params: vec![],
                        is_const: false,
                        attributes: vec![],
                        body: Some(block(vec![])),
                    })],
                }),
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Derived".to_string(),
                    extends: vec!["Base".to_string()],
                    members: vec![ClassMember::Func(Func {
                        modifiers: vec!["override".to_string()],
                        visibility: None,
                        return_type: Some(void_type()),
                        is_ref: false,
                        name: "method".to_string(),
                        params: vec![],
                        is_const: false,
                        attributes: vec![],
                        body: Some(block(vec![])),
                    })],
                }),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check override was registered in vtable
        let derived_id = analyzer.symbol_table.lookup_type("Derived").unwrap();
        let vtable = analyzer.symbol_table.get_vtable(derived_id);
        assert!(vtable.is_some());
    }

    // ==================== MEMBER INITIALIZERS ====================

    #[test]
    fn test_member_initializer() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![ClassMember::Var(var_decl(
                    int_type(),
                    "value",
                    Some(int_literal(100)),
                ))],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check initializer was saved
        let initializers = analyzer.symbol_table.get_member_initializers("MyClass");
        assert!(initializers.is_some());
        assert!(initializers.unwrap().contains_key("value"));
    }

    // ==================== COMPLEX INTEGRATION TESTS ====================

    #[test]
    fn test_complete_class_with_all_features() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Player".to_string(),
                extends: vec![],
                members: vec![
                    // Member variables with initializers
                    ClassMember::Var(var_decl(int_type(), "health", Some(int_literal(100)))),
                    ClassMember::Var(var_decl(float_type(), "speed", Some(float_literal(5.0)))),
                    ClassMember::Var(var_decl(string_type(), "name", None)),
                    // Constructor
                    ClassMember::Func(func_with_params(
                        "Player",
                        None,
                        vec![param("playerName", string_type())],
                        block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "name"),
                            BinaryOp::Assign,
                            var_expr("playerName"),
                        ))]),
                    )),
                    // Methods
                    ClassMember::Func(func_with_params(
                        "takeDamage",
                        Some(void_type()),
                        vec![param("amount", int_type())],
                        block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "health"),
                            BinaryOp::SubAssign,
                            var_expr("amount"),
                        ))]),
                    )),
                    ClassMember::Func(simple_func(
                        "isAlive",
                        Some(bool_type()),
                        block(vec![return_stmt(Some(binary_expr(
                            member_access(var_expr("this"), "health"),
                            BinaryOp::Gt,
                            int_literal(0),
                        )))]),
                    )),
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Verify everything was registered correctly
        let type_id = analyzer.symbol_table.lookup_type("Player").unwrap();
        let type_info = analyzer.symbol_table.get_type(type_id).unwrap();

        assert_eq!(type_info.members.len(), 3);
        assert!(type_info.members.contains_key("health"));
        assert!(type_info.members.contains_key("speed"));
        assert!(type_info.members.contains_key("name"));

        assert!(type_info.methods.contains_key("takeDamage"));
        assert!(type_info.methods.contains_key("isAlive"));

        let initializers = analyzer.symbol_table.get_member_initializers("Player");
        assert!(initializers.is_some());
        assert_eq!(initializers.unwrap().len(), 2); // health and speed
    }

    #[test]
    fn test_global_and_local_variables() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Var(var_decl(int_type(), "globalVar", Some(int_literal(10)))),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![
                        var_stmt(var_decl(int_type(), "localVar", Some(int_literal(20)))),
                        return_stmt(Some(binary_expr(
                            var_expr("globalVar"),
                            BinaryOp::Add,
                            var_expr("localVar"),
                        ))),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Check global was registered
        let global = analyzer.symbol_table.get_global("globalVar");
        assert!(global.is_some());

        // Check local was registered in function
        let func_locals = analyzer.symbol_table.get_function_locals("test");
        assert!(func_locals.is_some());
        assert!(func_locals.unwrap().variable_map.contains_key("localVar"));
    }

    #[test]
    fn test_nested_member_access() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Position".to_string(),
                    extends: vec![],
                    members: vec![
                        ClassMember::Var(var_decl(float_type(), "x", None)),
                        ClassMember::Var(var_decl(float_type(), "y", None)),
                    ],
                }),
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Player".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(var_decl(
                        class_type("Position"),
                        "pos",
                        None,
                    ))],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(float_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("Player"), "player", None)),
                        return_stmt(Some(member_access(
                            member_access(var_expr("player"), "pos"),
                            "x",
                        ))),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_method_with_const_this() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(var_decl(int_type(), "value", None)),
                    ClassMember::Func(Func {
                        modifiers: vec![],
                        visibility: None,
                        return_type: Some(int_type()),
                        is_ref: false,
                        name: "getValue".to_string(),
                        params: vec![],
                        is_const: true, // Const method
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(member_access(
                            var_expr("this"),
                            "value",
                        )))])),
                    }),
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("MyClass::getValue");
        assert!(func.is_some());
        assert!(func.unwrap().is_const);
    }

    // ==================== EXPRESSION CONTEXT TESTS ====================

    #[test]
    fn test_expression_context_caching() {
        let mut analyzer = create_analyzer();

        let expr1 = int_literal(42);
        let expr2 = int_literal(42); // Same value, different instance

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(expr1.clone()))]),
            ))],
        };

        analyzer.analyze(&script).unwrap();

        // Both expressions should have contexts
        let ctx1 = analyzer.symbol_table.get_expr_context(&expr1);
        let ctx2 = analyzer.symbol_table.get_expr_context(&expr2);

        assert!(ctx1.is_some());
        assert!(ctx2.is_some());
        assert_eq!(ctx1.unwrap().result_type, ctx2.unwrap().result_type);
    }

    #[test]
    fn test_lvalue_vs_rvalue() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(binary_expr(
                        var_expr("x"),
                        BinaryOp::Assign,
                        int_literal(42),
                    )),
                ]),
            ))],
        };

        analyzer.analyze(&script).unwrap();

        // Variable access is lvalue
        let var_ctx = analyzer.symbol_table.get_expr_context(&var_expr("x"));
        assert!(var_ctx.unwrap().is_lvalue);

        // Literal is not lvalue
        let lit_ctx = analyzer.symbol_table.get_expr_context(&int_literal(42));
        assert!(!lit_ctx.unwrap().is_lvalue);
    }

    // ==================== POSTFIX OPERATIONS ====================

    #[test]
    fn test_post_increment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(Expr::Postfix(Box::new(var_expr("x")), PostfixOp::PostInc)),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_post_increment_on_const() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(const_int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(Expr::Postfix(Box::new(var_expr("x")), PostfixOp::PostInc)),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. }))
        );
    }

    // ==================== LAMBDA TESTS ====================

    #[test]
    fn test_lambda_simple() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: Some(int_type()),
                type_mod: None,
                name: Some("x".to_string()),
            }],
            body: block(vec![return_stmt(Some(binary_expr(
                var_expr("x"),
                BinaryOp::Mul,
                int_literal(2),
            )))]),
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_lambda_with_auto_params() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: None, // Auto type
                type_mod: None,
                name: Some("x".to_string()),
            }],
            body: block(vec![return_stmt(Some(var_expr("x")))]),
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_lambda_no_params() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![],
            body: block(vec![return_stmt(Some(int_literal(42)))]),
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lambda_void_return() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![],
            body: block(vec![expr_stmt(int_literal(42))]), // No return
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== FUNCDEF TESTS ====================

    #[test]
    fn test_funcdef_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::FuncDef(FuncDef {
                modifiers: vec![],
                return_type: int_type(),
                is_ref: false,
                name: "Callback".to_string(),
                params: vec![param("value", int_type())],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Check funcdef was registered as a type
        let type_id = analyzer.symbol_table.lookup_type("Callback");
        assert!(type_id.is_some());

        let type_info = analyzer.symbol_table.get_type(type_id.unwrap()).unwrap();
        assert_eq!(type_info.kind, TypeKind::Funcdef);
    }

    #[test]
    fn test_funcdef_with_multiple_params() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::FuncDef(FuncDef {
                modifiers: vec![],
                return_type: void_type(),
                is_ref: false,
                name: "EventHandler".to_string(),
                params: vec![param("code", int_type()), param("message", string_type())],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_funcdef_no_params() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::FuncDef(FuncDef {
                modifiers: vec![],
                return_type: int_type(),
                is_ref: false,
                name: "Getter".to_string(),
                params: vec![],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== VIRTUAL PROPERTY TESTS ====================

    #[test]
    fn test_virtual_property_get_set() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(var_decl(int_type(), "_value", None)),
                    ClassMember::VirtProp(VirtProp {
                        visibility: None,
                        prop_type: int_type(),
                        is_ref: false,
                        name: "value".to_string(),
                        accessors: vec![
                            PropertyAccessor {
                                kind: AccessorKind::Get,
                                is_const: true,
                                attributes: vec![],
                                body: Some(block(vec![return_stmt(Some(member_access(
                                    var_expr("this"),
                                    "_value",
                                )))])),
                            },
                            PropertyAccessor {
                                kind: AccessorKind::Set,
                                is_const: false,
                                attributes: vec![],
                                body: Some(block(vec![expr_stmt(binary_expr(
                                    member_access(var_expr("this"), "_value"),
                                    BinaryOp::Assign,
                                    var_expr("value"),
                                ))])),
                            },
                        ],
                    }),
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Check getter and setter were registered as methods
        let get_func = analyzer.symbol_table.get_function("MyClass::get_value");
        assert!(get_func.is_some());
        assert!(get_func.unwrap().is_const);

        let set_func = analyzer.symbol_table.get_function("MyClass::set_value");
        assert!(set_func.is_some());
        assert!(!set_func.unwrap().is_const);
    }

    #[test]
    fn test_virtual_property_readonly() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![ClassMember::VirtProp(VirtProp {
                    visibility: None,
                    prop_type: int_type(),
                    is_ref: false,
                    name: "readonly".to_string(),
                    accessors: vec![PropertyAccessor {
                        kind: AccessorKind::Get,
                        is_const: true,
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(int_literal(42)))])),
                    }],
                })],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // Only getter should exist
        let get_func = analyzer.symbol_table.get_function("MyClass::get_readonly");
        assert!(get_func.is_some());

        let set_func = analyzer.symbol_table.get_function("MyClass::set_readonly");
        assert!(set_func.is_none());
    }

    // ==================== CAST TESTS ====================
    #[test]
    fn test_cast_int_to_float() {
        let mut analyzer = create_analyzer();

        // ✅ Create the expression ONCE
        let expr = Expr::Cast(float_type(), Box::new(int_literal(42)));

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(float_type()),
                block(vec![return_stmt(Some(expr.clone()))]), // Clone it
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        // ✅ Check using the ORIGINAL expr (not a new one)
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert!(ctx.is_some(), "Cast expression not analyzed");
        assert_eq!(ctx.unwrap().result_type, TYPE_FLOAT);
    }

    // ==================== CONSTRUCT CALL TESTS ====================

    #[test]
    fn test_construct_call_no_args() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Func(simple_func(
                        "MyClass",
                        None,
                        block(vec![]),
                    ))],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![expr_stmt(Expr::ConstructCall(
                        class_type("MyClass"),
                        vec![],
                    ))]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);

        assert!(
            result.is_ok(),
            "Should allow construction of defined class: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_construct_call_with_args() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![
                        ClassMember::Var(var_decl(int_type(), "value", None)),
                        ClassMember::Func(func_with_params(
                            "MyClass",
                            None,
                            vec![param("v", int_type())],
                            block(vec![expr_stmt(binary_expr(
                                member_access(var_expr("this"), "value"),
                                BinaryOp::Assign,
                                var_expr("v"),
                            ))]),
                        )),
                    ],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![expr_stmt(Expr::ConstructCall(
                        class_type("MyClass"),
                        vec![Arg {
                            name: None,
                            value: int_literal(42),
                        }],
                    ))]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    // ==================== INIT LIST TESTS ====================

    #[test]
    fn test_init_list_simple() {
        let mut analyzer = create_analyzer();

        let init_list = InitList {
            items: vec![
                InitListItem::Expr(int_literal(1)),
                InitListItem::Expr(int_literal(2)),
                InitListItem::Expr(int_literal(3)),
            ],
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::InitList(init_list))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_list_nested() {
        let mut analyzer = create_analyzer();

        let init_list = InitList {
            items: vec![
                InitListItem::InitList(InitList {
                    items: vec![
                        InitListItem::Expr(int_literal(1)),
                        InitListItem::Expr(int_literal(2)),
                    ],
                }),
                InitListItem::InitList(InitList {
                    items: vec![
                        InitListItem::Expr(int_literal(3)),
                        InitListItem::Expr(int_literal(4)),
                    ],
                }),
            ],
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::InitList(init_list))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_list_mixed_types() {
        let mut analyzer = create_analyzer();

        let init_list = InitList {
            items: vec![
                InitListItem::Expr(int_literal(1)),
                InitListItem::Expr(float_literal(2.0)),
                InitListItem::Expr(int_literal(3)),
            ],
        };

        let expr = Expr::InitList(init_list.clone());

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(expr.clone())]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert!(ctx.is_some());
    }

    // ==================== CONSTRUCT CALL TESTS ====================

    #[test]
    fn test_var_init_with_arglist() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![
                        ClassMember::Var(var_decl(int_type(), "value", None)),
                        ClassMember::Func(func_with_params(
                            "MyClass",
                            None,
                            vec![param("v", int_type())],
                            block(vec![expr_stmt(binary_expr(
                                member_access(var_expr("this"), "value"),
                                BinaryOp::Assign,
                                var_expr("v"),
                            ))]),
                        )),
                    ],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![Statement::Var(Var {
                        visibility: None,
                        var_type: class_type("MyClass"),
                        declarations: vec![VarDecl {
                            name: "obj".to_string(),
                            initializer: Some(VarInit::ArgList(vec![Arg {
                                name: None,
                                value: int_literal(42),
                            }])),
                        }],
                    })]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    // ==================== FOREACH TESTS ====================

    #[test]
    fn test_foreach_simple() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("array"), "arr", None)),
                    Statement::ForEach(ForEachStmt {
                        variables: vec![(int_type(), "val".to_string())],
                        iterable: var_expr("arr"),
                        body: Box::new(expr_stmt(var_expr("val"))),
                    }),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Check loop variable was registered
        // Note: It's in a popped scope, so we can't check it directly
        // But the analysis should have succeeded
    }

    #[test]
    fn test_foreach_with_index() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("array"), "arr", None)),
                    Statement::ForEach(ForEachStmt {
                        variables: vec![
                            (int_type(), "val".to_string()),
                            (int_type(), "idx".to_string()),
                        ],
                        iterable: var_expr("arr"),
                        body: Box::new(expr_stmt(var_expr("val"))),
                    }),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_foreach_break() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("array"), "arr", None)),
                    Statement::ForEach(ForEachStmt {
                        variables: vec![(int_type(), "val".to_string())],
                        iterable: var_expr("arr"),
                        body: Box::new(Statement::Break),
                    }),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok()); // Break inside foreach is valid
    }

    // ==================== SWITCH TESTS ====================

    #[test]
    fn test_switch_statement() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(1)))),
                    Statement::Switch(SwitchStmt {
                        value: var_expr("x"),
                        cases: vec![
                            Case {
                                pattern: CasePattern::Value(int_literal(1)),
                                statements: vec![expr_stmt(int_literal(10))],
                            },
                            Case {
                                pattern: CasePattern::Value(int_literal(2)),
                                statements: vec![expr_stmt(int_literal(20))],
                            },
                            Case {
                                pattern: CasePattern::Default,
                                statements: vec![expr_stmt(int_literal(30))],
                            },
                        ],
                    }),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_switch_with_break() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Switch(SwitchStmt {
                    value: int_literal(1),
                    cases: vec![Case {
                        pattern: CasePattern::Value(int_literal(1)),
                        statements: vec![Statement::Break],
                    }],
                })]),
            ))],
        };

        let result = analyzer.analyze(&script);
        // Break outside loop but inside switch - should this be valid?
        // In AngelScript, break in switch is valid
        assert!(result.is_ok());
    }

    // ==================== TRY/CATCH TESTS ====================

    #[test]
    fn test_try_catch() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Try(TryStmt {
                    try_block: block(vec![expr_stmt(int_literal(1))]),
                    catch_block: block(vec![expr_stmt(int_literal(2))]),
                })]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== LITERAL SUFFIX TESTS ====================

    #[test]
    fn test_literal_uint() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(Literal::Number(
                    "42u".to_string(),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42u".to_string()));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_UINT32);
    }

    #[test]
    fn test_literal_int64() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(Literal::Number(
                    "42ll".to_string(),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42ll".to_string()));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_INT64);
    }

    #[test]
    fn test_literal_uint64() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(Literal::Number(
                    "42ull".to_string(),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42ull".to_string()));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_UINT64);
    }

    #[test]
    fn test_literal_float_no_decimal() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(Literal::Number(
                    "2f".to_string(),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("2f".to_string()));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_FLOAT);
    }

    #[test]
    fn test_literal_double_default() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(Literal::Number(
                    "3.14".to_string(),
                )))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("3.14".to_string()));
        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert_eq!(ctx.unwrap().result_type, TYPE_DOUBLE);
    }

    // ==================== POSTFIX INDEX TESTS ====================

    #[test]
    fn test_array_index() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("array"), "arr", None)),
                    expr_stmt(Expr::Postfix(
                        Box::new(var_expr("arr")),
                        PostfixOp::Index(vec![IndexArg {
                            name: None,
                            value: int_literal(0),
                        }]),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_index_assignment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("array"), "arr", None)),
                    expr_stmt(binary_expr(
                        Expr::Postfix(
                            Box::new(var_expr("arr")),
                            PostfixOp::Index(vec![IndexArg {
                                name: None,
                                value: int_literal(0),
                            }]),
                        ),
                        BinaryOp::Assign,
                        int_literal(42),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== FUNCTOR CALL TESTS ====================

    #[test]
    fn test_functor_call() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("Functor"), "f", None)),
                    expr_stmt(Expr::Postfix(
                        Box::new(var_expr("f")),
                        PostfixOp::Call(vec![Arg {
                            name: None,
                            value: int_literal(42),
                        }]),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    // ==================== COMPLEX INTEGRATION TESTS ====================

    #[test]
    fn test_lambda_in_variable() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: Some(int_type()),
                type_mod: None,
                name: Some("x".to_string()),
            }],
            body: block(vec![return_stmt(Some(binary_expr(
                var_expr("x"),
                BinaryOp::Add,
                int_literal(1),
            )))]),
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Var(Var {
                    visibility: None,
                    var_type: Type {
                        is_const: false,
                        scope: Scope {
                            is_global: false,
                            path: vec![],
                        },
                        datatype: DataType::Auto,
                        template_types: vec![],
                        modifiers: vec![],
                    },
                    declarations: vec![VarDecl {
                        name: "callback".to_string(),
                        initializer: Some(VarInit::Expr(Expr::Lambda(lambda))),
                    }],
                })]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_class_with_virtual_property_and_methods() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Counter".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(var_decl(int_type(), "_count", Some(int_literal(0)))),
                    ClassMember::VirtProp(VirtProp {
                        visibility: Some(Visibility::Public),
                        prop_type: int_type(),
                        is_ref: false,
                        name: "count".to_string(),
                        accessors: vec![
                            PropertyAccessor {
                                kind: AccessorKind::Get,
                                is_const: true,
                                attributes: vec![],
                                body: Some(block(vec![return_stmt(Some(member_access(
                                    var_expr("this"),
                                    "_count",
                                )))])),
                            },
                            PropertyAccessor {
                                kind: AccessorKind::Set,
                                is_const: false,
                                attributes: vec![],
                                body: Some(block(vec![expr_stmt(binary_expr(
                                    member_access(var_expr("this"), "_count"),
                                    BinaryOp::Assign,
                                    var_expr("value"),
                                ))])),
                            },
                        ],
                    }),
                    ClassMember::Func(simple_func(
                        "increment",
                        Some(void_type()),
                        block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "_count"),
                            BinaryOp::AddAssign,
                            int_literal(1),
                        ))]),
                    )),
                ],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Verify virtual property accessors were registered
        let get_func = analyzer.symbol_table.get_function("Counter::get_count");
        assert!(get_func.is_some());

        let set_func = analyzer.symbol_table.get_function("Counter::set_count");
        assert!(set_func.is_some());

        // Verify regular method was registered
        let inc_func = analyzer.symbol_table.get_function("Counter::increment");
        assert!(inc_func.is_some());
    }

    #[test]
    fn test_all_literal_suffixes() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    expr_stmt(Expr::Literal(Literal::Number("42".to_string()))), // int
                    expr_stmt(Expr::Literal(Literal::Number("42u".to_string()))), // uint
                    expr_stmt(Expr::Literal(Literal::Number("42l".to_string()))), // int64
                    expr_stmt(Expr::Literal(Literal::Number("42ll".to_string()))), // int64
                    expr_stmt(Expr::Literal(Literal::Number("42ul".to_string()))), // uint32
                    expr_stmt(Expr::Literal(Literal::Number("42ull".to_string()))), // uint64
                    expr_stmt(Expr::Literal(Literal::Number("3.14".to_string()))), // double
                    expr_stmt(Expr::Literal(Literal::Number("3.14f".to_string()))), // float
                    expr_stmt(Expr::Literal(Literal::Number("2f".to_string()))), // float (no decimal)
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        // Verify all types are correct
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42".to_string())))
                .unwrap()
                .result_type,
            TYPE_INT32
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42u".to_string())))
                .unwrap()
                .result_type,
            TYPE_UINT32
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42ll".to_string())))
                .unwrap()
                .result_type,
            TYPE_INT64
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42ull".to_string())))
                .unwrap()
                .result_type,
            TYPE_UINT64
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("3.14".to_string())))
                .unwrap()
                .result_type,
            TYPE_DOUBLE
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("3.14f".to_string())))
                .unwrap()
                .result_type,
            TYPE_FLOAT
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("2f".to_string())))
                .unwrap()
                .result_type,
            TYPE_FLOAT
        );
    }

    // ==================== APPLICATION REGISTRATION TESTS ====================

    #[test]
    fn test_registered_global_function() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: vec![GlobalFunction {
                name: "print".to_string(),
                return_type_id: TYPE_VOID,
                params: vec![MethodParam {
                    name: "msg".to_string(),
                    type_id: TYPE_STRING,
                    is_ref: true,
                    is_out: false,
                    is_const: true,
                }],
                function_id: 0,
            }],
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(func_call(
                    "print",
                    vec![string_literal("Hello")],
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered function");

        let func = analyzer.symbol_table.get_function("print");
        assert!(func.is_some());
        assert!(func.unwrap().is_system_func);
    }

    #[test]
    fn test_registered_object_type() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![ObjectProperty {
                            name: "health".to_string(),
                            type_id: TYPE_INT32,
                            is_handle: false,
                            is_const: false,
                            access: AccessSpecifier::Public,
                        }],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("Enemy"), "enemy", None)),
                    expr_stmt(binary_expr(
                        member_access(var_expr("enemy"), "health"),
                        BinaryOp::Assign,
                        int_literal(100),
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should recognize registered type and property"
        );

        let type_id = analyzer.symbol_table.lookup_type("Enemy");
        assert_eq!(type_id, Some(100));

        let type_info = analyzer.symbol_table.get_type(100).unwrap();
        assert_eq!(type_info.kind, TypeKind::Class);
        assert!(type_info.members.contains_key("health"));
    }

    #[test]
    fn test_registered_object_method() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![ObjectMethod {
                            name: "takeDamage".to_string(),
                            return_type_id: TYPE_VOID,
                            params: vec![MethodParam {
                                name: "amount".to_string(),
                                type_id: TYPE_INT32,
                                is_ref: false,
                                is_out: false,
                                is_const: false,
                            }],
                            is_const: false,
                            is_virtual: false,
                            is_final: false,
                            access: AccessSpecifier::Public,
                            function_id: 0,
                        }],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(class_type("Enemy"), "enemy", None)),
                    expr_stmt(method_call(
                        var_expr("enemy"),
                        "takeDamage",
                        vec![int_literal(50)],
                    )),
                ]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered method");

        let method_func = analyzer.symbol_table.get_function("Enemy::takeDamage");
        assert!(method_func.is_some());
        assert!(method_func.unwrap().is_system_func);
    }

    #[test]
    fn test_registered_enum() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Color".to_string(),
                    EnumType {
                        type_id: 100,
                        name: "Color".to_string(),
                        values: {
                            let mut values = HashMap::new();
                            values.insert("Red".to_string(), 0);
                            values.insert("Green".to_string(), 1);
                            values.insert("Blue".to_string(), 2);
                            values
                        },
                    },
                );
                map
            },
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(class_type("Color"), "c", None))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered enum");

        let type_id = analyzer.symbol_table.lookup_type("Color");
        assert_eq!(type_id, Some(100));

        let type_info = analyzer.symbol_table.get_type(100).unwrap();
        assert_eq!(type_info.kind, TypeKind::Enum);
    }

    #[test]
    fn test_registered_funcdef() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: {
                let mut map = HashMap::new();
                map.insert(
                    "Callback".to_string(),
                    FuncdefInfo {
                        type_id: 100,
                        name: "Callback".to_string(),
                        return_type_id: TYPE_VOID,
                        params: vec![MethodParam {
                            name: "code".to_string(),
                            type_id: TYPE_INT32,
                            is_ref: false,
                            is_out: false,
                            is_const: false,
                        }],
                    },
                );
                map
            },
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(Var {
                    visibility: None,
                    var_type: Type {
                        is_const: false,
                        scope: Scope {
                            is_global: false,
                            path: vec![],
                        },
                        datatype: DataType::Identifier("Callback".to_string()),
                        template_types: vec![],
                        modifiers: vec![TypeModifier::Handle],
                    },
                    declarations: vec![VarDecl {
                        name: "cb".to_string(),
                        initializer: None,
                    }],
                })]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered funcdef");

        let type_id = analyzer.symbol_table.lookup_type("Callback");
        assert_eq!(type_id, Some(100));

        let type_info = analyzer.symbol_table.get_type(100).unwrap();
        assert_eq!(type_info.kind, TypeKind::Funcdef);
    }

    #[test]
    fn test_registered_global_property() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: HashMap::new(),
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: vec![GlobalProperty {
                name: "g_score".to_string(),
                type_id: TYPE_INT32,
                is_const: false,
                is_handle: false,
            }],
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(binary_expr(
                    var_expr("g_score"),
                    BinaryOp::Assign,
                    int_literal(100),
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should recognize registered global property"
        );

        let global = analyzer.symbol_table.get_global("g_score");
        assert!(global.is_some());
    }

    #[test]
    fn test_mixed_script_and_registered_types() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Enemy".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Enemy".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![ObjectProperty {
                            name: "health".to_string(),
                            type_id: TYPE_INT32,
                            is_handle: false,
                            is_const: false,
                            access: AccessSpecifier::Public,
                        }],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Player".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(var_decl(
                        class_type("Enemy"),
                        "target",
                        None,
                    ))],
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("Player"), "player", None)),
                        expr_stmt(binary_expr(
                            member_access(member_access(var_expr("player"), "target"), "health"),
                            BinaryOp::Assign,
                            int_literal(50),
                        )),
                    ]),
                )),
            ],
        };

        let result = analyzer.analyze(&script);

        assert!(
            result.is_ok(),
            "Should handle mix of script and registered types: {:?}",
            result.err()
        );

        let player_type = analyzer.symbol_table.lookup_type("Player");
        assert!(player_type.is_some());

        let enemy_type = analyzer.symbol_table.lookup_type("Enemy");
        assert_eq!(enemy_type, Some(100));
    }

    #[test]
    fn test_undefined_registered_function() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(func_call("undefinedSystemFunc", vec![]))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::UndefinedFunction { .. }))
        );
    }

    #[test]
    fn test_registered_type_with_behaviours() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "RefCounted".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "RefCounted".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![
                            BehaviourInfo {
                                behaviour_type: BehaviourType::Construct,
                                function_id: 0,
                                return_type_id: 100,
                                params: vec![],
                            },
                            BehaviourInfo {
                                behaviour_type: BehaviourType::AddRef,
                                function_id: 1,
                                return_type_id: TYPE_VOID,
                                params: vec![],
                            },
                            BehaviourInfo {
                                behaviour_type: BehaviourType::Release,
                                function_id: 2,
                                return_type_id: TYPE_VOID,
                                params: vec![],
                            },
                        ],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(
                    class_type("RefCounted"),
                    "obj",
                    None,
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize type with behaviours");

        let factory_func = analyzer.symbol_table.get_function("RefCounted::factory");
        assert!(factory_func.is_some());
        assert!(factory_func.unwrap().is_system_func);

        let addref_func = analyzer.symbol_table.get_function("RefCounted::AddRef");
        assert!(addref_func.is_some());
    }

    #[test]
    fn test_value_type_cannot_use_inout() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: int_type(),
                    type_mod: Some(TypeMod::InOut),
                    name: Some("value".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Value types cannot use 'inout' references");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ReferenceMismatch { .. }))
        );
    }

    #[test]
    fn test_value_type_can_use_in() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: int_type(),
                    type_mod: Some(TypeMod::In),
                    name: Some("value".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Value types can use 'in' references");
    }

    #[test]
    fn test_value_type_can_use_out() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: int_type(),
                    type_mod: Some(TypeMod::Out),
                    name: Some("value".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Value types can use 'out' references");
    }

    #[test]
    fn test_ref_type_can_use_inout() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "RefType".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "RefType".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: class_type("RefType"),
                    type_mod: Some(TypeMod::InOut),
                    name: Some("obj".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Reference types can use 'inout' references");
    }

    #[test]
    fn test_nohandle_type_cannot_be_handle() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "NoHandleType".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "NoHandleType".to_string(),
                        flags: TypeFlags::REF_TYPE | TypeFlags::NOHANDLE,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(
                    handle_type("NoHandleType"),
                    "obj",
                    None,
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "NOHANDLE types cannot be used as handles");
    }

    #[test]
    fn test_noinherit_type_cannot_be_inherited() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "FinalType".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "FinalType".to_string(),
                        flags: TypeFlags::REF_TYPE | TypeFlags::NOINHERIT,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Derived".to_string(),
                extends: vec!["FinalType".to_string()],
                members: vec![],
            })],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "NOINHERIT types cannot be inherited");
    }

    #[test]
    fn test_abstract_type_cannot_be_instantiated() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "AbstractType".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "AbstractType".to_string(),
                        flags: TypeFlags::REF_TYPE | TypeFlags::ABSTRACT,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(
                    class_type("AbstractType"),
                    "obj",
                    None,
                ))]),
            ))],
        };

        let result = analyzer.analyze(&script);

        assert!(result.is_err(), "Abstract types cannot be instantiated");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InstantiateAbstract { .. }))
        );
    }

    #[test]
    fn test_primitive_cannot_use_inout() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: int_type(),
                    type_mod: Some(TypeMod::InOut),
                    name: Some("value".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Primitives cannot use 'inout'");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ReferenceMismatch { .. }))
        );
    }

    #[test]
    fn test_value_type_with_inout_fails() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "Vector3".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "Vector3".to_string(),
                        flags: TypeFlags::VALUE_TYPE, // ✅ Application value type
                        properties: vec![ObjectProperty {
                            name: "x".to_string(),
                            type_id: TYPE_FLOAT,
                            is_handle: false,
                            is_const: false,
                            access: AccessSpecifier::Public,
                        }],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: class_type("Vector3"),
                    type_mod: Some(TypeMod::InOut),
                    name: Some("vec".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Application value types cannot use 'inout'"
        );
    }

    #[test]
    fn test_ref_type_with_inout_succeeds() {
        let engine = Arc::new(RwLock::new(EngineInner {
            object_types: {
                let mut map = HashMap::new();
                map.insert(
                    "RefType".to_string(),
                    ObjectType {
                        type_id: 100,
                        name: "RefType".to_string(),
                        flags: TypeFlags::REF_TYPE,
                        properties: vec![],
                        methods: vec![],
                        behaviours: vec![],
                        rust_type_id: None,
                    },
                );
                map
            },
            interface_types: HashMap::new(),
            enum_types: HashMap::new(),
            global_functions: Vec::new(),
            global_properties: Vec::new(),
            modules: HashMap::new(),
            funcdefs: HashMap::new(),
        }));

        let mut analyzer = SemanticAnalyzer::new(engine);

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "test",
                Some(void_type()),
                vec![Param {
                    param_type: class_type("RefType"),
                    type_mod: Some(TypeMod::InOut),
                    name: Some("obj".to_string()),
                    default_value: None,
                    is_variadic: false,
                }],
                block(vec![]),
            ))],
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Reference types can use 'inout'");
    }
}
