use crate::core::span::Span;
use crate::core::type_registry::{LocalVarInfo, ReturnFlags, TypeRegistry};
use crate::core::types::{FunctionId, TypeId};
use crate::parser::ast::{DataType, Expr, Type};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, RwLock};

/// Context information for an analyzed expression
#[derive(Debug, Clone)]
pub enum ExprContext {
    /// Literal value
    Literal { type_id: TypeId },

    /// Local variable
    LocalVar {
        type_id: TypeId,
        var_index: usize,
        is_const: bool,
    },

    /// Global variable
    GlobalVar {
        type_id: TypeId,
        global_address: u32,
        is_const: bool,
    },

    /// Function call (resolved to specific function)
    FunctionCall {
        return_type: TypeId,
        function_id: FunctionId,
        return_flags: ReturnFlags,
    },

    /// Method call (resolved to specific method)
    MethodCall {
        return_type: TypeId,
        function_id: FunctionId,
        return_flags: ReturnFlags,
    },

    /// Regular property access (direct HashMap access)
    PropertyAccess {
        property_type: TypeId,
        property_name: String,
        is_const: bool,
    },

    /// Virtual property access (uses getter/setter functions)
    VirtualProperty {
        property_type: TypeId,
        getter_id: Option<FunctionId>,
        setter_id: Option<FunctionId>,
        is_const: bool,
    },

    /// Temporary value (result of operation)
    Temporary { type_id: TypeId },

    /// Object handle
    Handle { type_id: TypeId },

    /// Reference to a value
    Reference { type_id: TypeId, is_const: bool },
}

impl ExprContext {
    /// Get the type of this expression
    pub fn get_type(&self) -> TypeId {
        match self {
            ExprContext::Literal { type_id } => *type_id,
            ExprContext::LocalVar { type_id, .. } => *type_id,
            ExprContext::GlobalVar { type_id, .. } => *type_id,
            ExprContext::FunctionCall { return_type, .. } => *return_type,
            ExprContext::MethodCall { return_type, .. } => *return_type,
            ExprContext::PropertyAccess { property_type, .. } => *property_type,
            ExprContext::VirtualProperty { property_type, .. } => *property_type,
            ExprContext::Temporary { type_id } => *type_id,
            ExprContext::Handle { type_id } => *type_id,
            ExprContext::Reference { type_id, .. } => *type_id,
        }
    }

    /// Check if this expression can be assigned to (is an lvalue)
    pub fn is_lvalue(&self) -> bool {
        match self {
            // ✅ All variables are lvalues (const is checked separately)
            ExprContext::LocalVar { .. } => true,
            ExprContext::GlobalVar { .. } => true,
            ExprContext::PropertyAccess { .. } => true,
            ExprContext::VirtualProperty { setter_id, .. } => setter_id.is_some(),
            ExprContext::Reference { .. } => true,
            // Functions that return references are lvalues
            ExprContext::FunctionCall { return_flags, .. } => {
                return_flags.contains(ReturnFlags::REF)
            }
            ExprContext::MethodCall { return_flags, .. } => return_flags.contains(ReturnFlags::REF),
            ExprContext::Literal { .. } => false,
            ExprContext::Temporary { .. } => false,
            ExprContext::Handle { .. } => false,
        }
    }

    /// Check if this expression is const
    pub fn is_const(&self) -> bool {
        match self {
            ExprContext::LocalVar { is_const, .. } => *is_const,
            ExprContext::GlobalVar { is_const, .. } => *is_const,
            ExprContext::PropertyAccess { is_const, .. } => *is_const,
            ExprContext::VirtualProperty { is_const, .. } => *is_const,
            ExprContext::Reference { is_const, .. } => *is_const,
            // Functions/methods that return const references are const
            ExprContext::FunctionCall { return_flags, .. } => {
                return_flags.contains(ReturnFlags::CONST)
            }
            ExprContext::MethodCall { return_flags, .. } => {
                return_flags.contains(ReturnFlags::CONST)
            }
            _ => false,
        }
    }

    /// Check if this is a temporary value
    pub fn is_temporary(&self) -> bool {
        matches!(self, ExprContext::Temporary { .. })
    }

    /// Get resolved function ID (for function/method calls)
    pub fn get_function_id(&self) -> Option<FunctionId> {
        match self {
            ExprContext::FunctionCall { function_id, .. } => Some(*function_id),
            ExprContext::MethodCall { function_id, .. } => Some(*function_id),
            _ => None,
        }
    }

    /// Get resolved variable index (for local variables)
    pub fn get_var_index(&self) -> Option<usize> {
        match self {
            ExprContext::LocalVar { var_index, .. } => Some(*var_index),
            _ => None,
        }
    }

    /// Get global address
    pub fn get_global_address(&self) -> Option<u32> {
        match self {
            ExprContext::GlobalVar { global_address, .. } => Some(*global_address),
            _ => None,
        }
    }

    /// Get property name (for regular properties)
    pub fn get_property_name(&self) -> Option<&str> {
        match self {
            ExprContext::PropertyAccess { property_name, .. } => Some(property_name),
            _ => None,
        }
    }

    /// Get virtual property accessors (getter/setter IDs)
    pub fn get_property_accessors(&self) -> Option<(Option<FunctionId>, Option<FunctionId>)> {
        match self {
            ExprContext::VirtualProperty {
                getter_id,
                setter_id,
                ..
            } => Some((*getter_id, *setter_id)),
            _ => None,
        }
    }

    /// Get return flags (for function/method calls)
    pub fn get_return_flags(&self) -> Option<ReturnFlags> {
        match self {
            ExprContext::FunctionCall { return_flags, .. } => Some(*return_flags),
            ExprContext::MethodCall { return_flags, .. } => Some(*return_flags),
            _ => None,
        }
    }

    /// Check if this expression returns a reference (for function/method calls)
    pub fn returns_ref(&self) -> bool {
        match self {
            ExprContext::FunctionCall { return_flags, .. } => {
                return_flags.contains(ReturnFlags::REF)
            }
            ExprContext::MethodCall { return_flags, .. } => return_flags.contains(ReturnFlags::REF),
            _ => false,
        }
    }

    /// Check if this is a virtual property
    pub fn is_virtual_property(&self) -> bool {
        matches!(self, ExprContext::VirtualProperty { .. })
    }

    /// Check if this is a handle
    pub fn is_handle(&self) -> bool {
        matches!(self, ExprContext::Handle { .. })
    }

    /// Check if this is a reference
    pub fn is_ref(&self) -> bool {
        matches!(self, ExprContext::Reference { .. })
    }
}

pub struct SymbolTable {
    registry: Arc<RwLock<TypeRegistry>>,

    pub scopes: Vec<Scope>,

    expr_contexts: HashMap<ExprId, ExprContext>,

    current_namespace: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashMap<String, LocalVarInfo>,
    pub scope_type: ScopeType,
    pub next_index: usize,
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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ExprId(u64);

impl ExprId {
    pub fn from_expr(expr: &Expr) -> Self {
        let mut hasher = DefaultHasher::new();
        expr.hash(&mut hasher);
        ExprId(hasher.finish())
    }
}

impl SymbolTable {
    pub fn new(registry: Arc<RwLock<TypeRegistry>>) -> Self {
        Self {
            registry,
            scopes: vec![Scope {
                variables: HashMap::new(),
                scope_type: ScopeType::Global,
                next_index: 0,
            }],
            expr_contexts: HashMap::new(),
            current_namespace: Vec::new(),
        }
    }

    pub fn get_type(&self, type_id: TypeId) -> Option<Arc<crate::core::type_registry::TypeInfo>> {
        self.registry.read().unwrap().get_type(type_id)
    }

    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.registry
            .read()
            .unwrap()
            .lookup_type(name, &self.current_namespace)
    }

    pub fn resolve_type_from_ast(&self, type_def: &Type) -> TypeId {
        let name = match &type_def.datatype {
            DataType::PrimType(n) => n,
            DataType::Identifier(n) => n,
            DataType::Auto => return crate::core::types::TYPE_AUTO,
            DataType::Question => return crate::core::types::TYPE_VOID,
        };

        let type_id = self
            .lookup_type(name)
            .unwrap_or(crate::core::types::TYPE_VOID);

        // Resolve typedefs
        let registry = self.registry.read().unwrap();
        registry.resolve_typedef(type_id)
    }

    pub fn get_function(
        &self,
        name: &str,
    ) -> Option<Arc<crate::core::type_registry::FunctionInfo>> {
        self.registry
            .read()
            .unwrap()
            .find_function(name, &self.current_namespace)
    }

    pub fn get_global(&self, name: &str) -> Option<Arc<crate::core::type_registry::GlobalInfo>> {
        self.registry.read().unwrap().get_global(name)
    }

    pub fn push_scope(&mut self, scope_type: ScopeType) {
        let next_index = self.scopes.last().map(|s| s.next_index).unwrap_or(0);

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
        span: Option<Span>,
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
                definition_span: span,
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

    pub fn set_expr_context(&mut self, expr: &Expr, context: ExprContext) {
        let expr_id = ExprId::from_expr(expr);
        self.expr_contexts.insert(expr_id, context);
    }

    pub fn get_expr_context(&self, expr: &Expr) -> Option<&ExprContext> {
        let expr_id = ExprId::from_expr(expr);
        self.expr_contexts.get(&expr_id)
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

    pub fn set_namespace(&mut self, namespace: Vec<String>) {
        self.current_namespace = namespace;
    }

    pub fn get_namespace(&self) -> &[String] {
        &self.current_namespace
    }

    pub fn collect_function_locals(&self, func_name: &str) -> Vec<LocalVarInfo> {
        let func_scope_idx = self
            .scopes
            .iter()
            .position(|s| matches!(&s.scope_type, ScopeType::Function(name) if name == func_name));

        if let Some(func_idx) = func_scope_idx {
            let mut all_locals = Vec::new();

            for (idx, scope) in self.scopes.iter().enumerate() {
                if idx >= func_idx {
                    all_locals.extend(scope.variables.values().cloned());
                }
            }

            all_locals.sort_by_key(|v| v.index);
            all_locals
        } else {
            Vec::new()
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(TypeRegistry::new())))
    }
}
