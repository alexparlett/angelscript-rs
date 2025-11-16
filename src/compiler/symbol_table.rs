use crate::core::types::{
    TypeFlags, TypeId, TypeKind, TypeRegistration, TYPE_BOOL, TYPE_DOUBLE, TYPE_FLOAT, TYPE_INT16,
    TYPE_INT32, TYPE_INT64, TYPE_INT8, TYPE_STRING, TYPE_UINT16, TYPE_UINT32, TYPE_UINT64, TYPE_UINT8,
    TYPE_VOID,
};
use crate::parser::ast::*;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ExprContext {
    pub result_type: TypeId,
    pub is_handle: bool,
    pub is_const: bool,
    pub is_ref: bool,
    pub is_lvalue: bool,
    pub is_temporary: bool,
    pub resolved_var_index: Option<usize>,
    pub is_global: bool,
    pub global_address: Option<u32>,
}

impl ExprContext {
    pub fn new(result_type: TypeId) -> Self {
        Self {
            result_type,
            is_handle: false,
            is_const: false,
            is_ref: false,
            is_lvalue: false,
            is_temporary: true,
            resolved_var_index: None,
            is_global: false,
            global_address: None,
        }
    }

    pub fn lvalue(result_type: TypeId, is_const: bool) -> Self {
        Self {
            result_type,
            is_handle: false,
            is_const,
            is_ref: false,
            is_lvalue: true,
            is_temporary: false,
            resolved_var_index: None,
            is_global: false,
            global_address: None,
        }
    }

    pub fn local_var(result_type: TypeId, is_const: bool, var_index: usize) -> Self {
        Self {
            result_type,
            is_handle: false,
            is_const,
            is_ref: false,
            is_lvalue: true,
            is_temporary: false,
            resolved_var_index: Some(var_index),
            is_global: false,
            global_address: None,
        }
    }

    pub fn global_var(result_type: TypeId, is_const: bool, global_addr: u32) -> Self {
        Self {
            result_type,
            is_handle: false,
            is_const,
            is_ref: false,
            is_lvalue: true,
            is_temporary: false,
            resolved_var_index: None,
            is_global: true,
            global_address: Some(global_addr),
        }
    }

    pub fn handle(result_type: TypeId) -> Self {
        Self {
            result_type,
            is_handle: true,
            is_const: false,
            is_ref: false,
            is_lvalue: false,
            is_temporary: false,
            resolved_var_index: None,
            is_global: false,
            global_address: None,
        }
    }

    pub fn reference(result_type: TypeId, is_const: bool) -> Self {
        Self {
            result_type,
            is_handle: false,
            is_const,
            is_ref: true,
            is_lvalue: true,
            is_temporary: false,
            resolved_var_index: None,
            is_global: false,
            global_address: None,
        }
    }
}

pub struct SymbolTable {
    types: HashMap<TypeId, Arc<TypeInfo>>,
    types_by_name: HashMap<String, TypeId>,
    functions: HashMap<String, Arc<FunctionInfo>>,
    globals: HashMap<String, Arc<GlobalVarInfo>>,
    pub scopes: Vec<Scope>,
    function_locals: HashMap<String, Arc<FunctionLocals>>,
    expr_types: HashMap<ExprId, TypeId>,
    expr_contexts: HashMap<ExprId, ExprContext>,
    member_initializers: HashMap<String, HashMap<String, Expr>>,
    vtables: HashMap<TypeId, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct FunctionLocals {
    pub locals: Vec<LocalVarInfo>,
    pub variable_map: HashMap<String, usize>,
    pub param_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
pub struct LocalVarInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_const: bool,
    pub is_param: bool,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, LocalVarInfo>,
    scope_type: ScopeType,
    next_index: usize,
}

impl Scope {
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeType {
    Global,
    Function(String),
    Block,
    Loop,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub name: String,
    pub kind: TypeKind,
    pub flags: TypeFlags,
    pub members: HashMap<String, MemberInfo>,
    pub methods: HashMap<String, Vec<String>>,
    pub base_class: Option<TypeId>,
    pub registration: TypeRegistration,
}

#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_const: bool,
    pub visibility: Option<Visibility>,
}

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub full_name: String,
    pub return_type: TypeId,
    pub params: Vec<ParamInfo>,
    pub is_method: bool,
    pub class_type: Option<TypeId>,
    pub is_const: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub address: u32,
    pub is_system_func: bool,
    pub system_func_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub name: Option<String>,
    pub type_id: TypeId,
    pub is_ref: bool,
    pub is_out: bool,
    pub default_value: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct GlobalVarInfo {
    pub name: String,
    pub type_id: TypeId,
    pub is_const: bool,
    pub address: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExprId(u64);

impl ExprId {
    pub fn from_expr(expr: &Expr) -> Self {
        let mut hasher = DefaultHasher::new();
        expr.hash(&mut hasher);
        ExprId(hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub enum VariableLocation {
    Local(LocalVarInfo),
    Global(Arc<GlobalVarInfo>),
}

#[derive(Debug, Clone, Copy)]
pub enum TypeUsage {
    AsHandle,
    AsBaseClass,
    AsVariable,
    InAssignment,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            types: HashMap::new(),
            types_by_name: HashMap::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            scopes: vec![Scope {
                variables: HashMap::new(),
                scope_type: ScopeType::Global,
                next_index: 0,
            }],
            function_locals: HashMap::new(),
            expr_types: HashMap::new(),
            expr_contexts: HashMap::new(),
            member_initializers: HashMap::new(),
            vtables: HashMap::new(),
        };

        table.register_primitives();
        table
    }

    fn register_primitives(&mut self) {
        let primitives = vec![
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
                kind: TypeKind::Primitive,
                flags: TypeFlags::POD_TYPE | TypeFlags::VALUE_TYPE,
                members: HashMap::new(),
                methods: HashMap::new(),
                base_class: None,
                registration: TypeRegistration::Script,
            };

            self.types.insert(type_id, Arc::new(type_info));
            self.types_by_name.insert(name.to_string(), type_id);
        }
    }

    pub fn register_type(&mut self, type_info: TypeInfo) {
        let name = type_info.name.clone();
        self.types_by_name.insert(name, type_info.type_id.clone());
        self.types.insert(type_info.type_id, Arc::new(type_info));
    }

    pub fn get_type(&self, type_id: TypeId) -> Option<Arc<TypeInfo>> {
        self.types.get(&type_id).cloned()
    }

    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.types_by_name.get(name).copied()
    }

    pub fn resolve_type_from_ast(&self, type_def: &Type) -> TypeId {
        let result = match &type_def.datatype {
            DataType::PrimType(name) => self.lookup_type(name).unwrap_or(TYPE_VOID),
            DataType::Identifier(name) => self.lookup_type(name).unwrap_or(TYPE_VOID),
            _ => TYPE_VOID,
        };
        result
    }

    pub fn register_function(&mut self, func_info: FunctionInfo) {
        let name = func_info.full_name.clone();
        self.functions.insert(name, Arc::new(func_info));
    }

    pub fn get_function(&self, name: &str) -> Option<Arc<FunctionInfo>> {
        self.functions.get(name).cloned()
    }

    pub fn update_function_address(&mut self, name: &str, address: u32) {
        if let Some(func) = self.functions.get_mut(name) {
            Arc::make_mut(func).address = address;
        }
    }

    pub fn register_global(&mut self, global_info: GlobalVarInfo) {
        let name = global_info.name.clone();
        self.globals.insert(name, Arc::new(global_info));
    }

    pub fn get_global(&self, name: &str) -> Option<Arc<GlobalVarInfo>> {
        self.globals.get(name).cloned()
    }

    pub fn push_scope(&mut self, scope_type: ScopeType) {
        let next_index = if let Some(last_scope) = self.scopes.last() {
            last_scope.next_index
        } else {
            0
        };

        self.scopes.push(Scope {
            variables: HashMap::new(),
            scope_type,
            next_index,
        });
    }

    pub fn pop_scope(&mut self) {
        if let Some(scope) = self.scopes.last() {
            if matches!(scope.scope_type, ScopeType::Function(_)) {
                let mut count = 0;
                for s in self.scopes.iter().rev() {
                    count += 1;
                    if matches!(s.scope_type, ScopeType::Global) {
                        count -= 1;
                        break;
                    }
                }

                for _ in 0..count {
                    self.scopes.pop();
                }
                return;
            }
        }

        self.scopes.pop();
    }

    pub fn register_local(
        &mut self,
        name: String,
        type_id: TypeId,
        is_const: bool,
        is_param: bool,
    ) -> usize {
        if let Some(scope) = self.scopes.last_mut() {
            let index = scope.next_index;
            scope.next_index += 1;

            let var_info = LocalVarInfo {
                name: name.clone(),
                type_id,
                is_const,
                is_param,
                index,
            };

            scope.variables.insert(name, var_info);
            index
        } else {
            0
        }
    }

    pub fn lookup_local(&self, name: &str) -> Option<&LocalVarInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.variables.get(name) {
                return Some(var);
            }
        }
        None
    }

    pub fn lookup_variable(&self, name: &str) -> Option<VariableLocation> {
        if let Some(local) = self.lookup_local(name) {
            return Some(VariableLocation::Local(local.clone()));
        }

        if let Some(global) = self.get_global(name) {
            return Some(VariableLocation::Global(global));
        }

        None
    }

    pub fn save_function_locals(&mut self, func_name: String) {
        let func_scope_idx = self
            .scopes
            .iter()
            .position(|s| matches!(&s.scope_type, ScopeType::Function(name) if name == &func_name));

        if let Some(func_idx) = func_scope_idx {
            let mut all_locals = Vec::new();

            for (idx, scope) in self.scopes.iter().enumerate() {
                if idx >= func_idx {
                    all_locals.extend(scope.variables.values().cloned());
                }
            }

            all_locals.sort_by_key(|v| v.index);

            let param_count = all_locals.iter().filter(|v| v.is_param).count();
            let total_count = all_locals.len();

            let variable_map: HashMap<_, _> = all_locals
                .iter()
                .map(|v| (v.name.clone(), v.index))
                .collect();

            let func_locals = FunctionLocals {
                locals: all_locals,
                variable_map,
                param_count,
                total_count,
            };

            self.function_locals
                .insert(func_name, Arc::new(func_locals));
        }
    }

    pub fn get_function_locals(&self, func_name: &str) -> Option<Arc<FunctionLocals>> {
        self.function_locals.get(func_name).cloned()
    }

    pub fn set_expr_type(&mut self, expr: &Expr, type_id: TypeId) {
        let expr_id = ExprId::from_expr(expr);
        self.expr_types.insert(expr_id, type_id);
    }

    pub fn get_expr_type(&self, expr: &Expr) -> Option<TypeId> {
        let expr_id = ExprId::from_expr(expr);
        self.expr_types.get(&expr_id).copied()
    }

    pub fn set_expr_context(&mut self, expr: &Expr, context: ExprContext) {
        let expr_id = ExprId::from_expr(expr);
        self.expr_types.insert(expr_id.clone(), context.result_type);
        self.expr_contexts.insert(expr_id, context);
    }

    pub fn get_expr_context(&self, expr: &Expr) -> Option<&ExprContext> {
        let expr_id = ExprId::from_expr(expr);
        self.expr_contexts.get(&expr_id)
    }

    pub fn set_member_initializer(&mut self, class_name: String, member_name: String, expr: Expr) {
        self.member_initializers
            .entry(class_name)
            .or_insert_with(HashMap::new)
            .insert(member_name, expr);
    }

    pub fn get_member_initializers(&self, class_name: &str) -> Option<&HashMap<String, Expr>> {
        self.member_initializers.get(class_name)
    }

    pub fn get_member_initializers_cloned(
        &self,
        class_name: &str,
    ) -> Option<HashMap<String, Expr>> {
        self.member_initializers.get(class_name).cloned()
    }

    pub fn set_vtable(&mut self, type_id: TypeId, methods: Vec<String>) {
        self.vtables.insert(type_id, methods);
    }

    pub fn get_vtable(&self, type_id: TypeId) -> Option<&Vec<String>> {
        self.vtables.get(&type_id)
    }

    pub fn current_function_name(&self) -> Option<&str> {
        for scope in self.scopes.iter().rev() {
            if let ScopeType::Function(name) = &scope.scope_type {
                return Some(name);
            }
        }
        None
    }

    pub fn in_loop(&self) -> bool {
        self.scopes.iter().any(|s| s.scope_type == ScopeType::Loop)
    }

    pub fn create_funcdef_type(&mut self, return_type: TypeId, param_types: Vec<TypeId>) -> TypeId {
        let signature = format!(
            "funcdef_{}_({})",
            return_type,
            param_types
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("_")
        );

        if let Some(existing_id) = self.lookup_type(&signature) {
            return existing_id;
        }

        let type_id = crate::core::types::allocate_type_id();

        let type_info = TypeInfo {
            type_id,
            name: signature,
            kind: TypeKind::Funcdef,
            flags: TypeFlags::FUNCDEF,
            members: HashMap::new(),
            methods: HashMap::new(),
            base_class: None,
            registration: TypeRegistration::Script,
        };

        self.register_type(type_info);

        type_id
    }

    pub fn get_or_create_funcdef(&mut self, return_type: TypeId, param_types: &[TypeId]) -> TypeId {
        self.create_funcdef_type(return_type, param_types.to_vec())
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
