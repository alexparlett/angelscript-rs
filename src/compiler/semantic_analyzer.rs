use crate::compiler::symbol_table::{ExprContext, ScopeType, SymbolTable};
use crate::core::engine_properties::EngineProperty;
use crate::core::error::{SemanticError, SemanticResult};
use crate::core::span::Span;
use crate::core::type_registry::{
    FunctionFlags, FunctionImpl, FunctionInfo, FunctionKind, GlobalInfo, ParameterFlags,
    ParameterInfo, PropertyFlags, PropertyInfo, ReturnFlags, TypeInfo, TypeRegistry, VTableEntry,
};
use crate::core::types::{
    AccessSpecifier, BehaviourType, FunctionId, TYPE_AUTO, TYPE_BOOL, TYPE_DOUBLE, TYPE_FLOAT,
    TYPE_INT8, TYPE_INT16, TYPE_INT32, TYPE_INT64, TYPE_STRING, TYPE_UINT8, TYPE_UINT16,
    TYPE_UINT32, TYPE_UINT64, TYPE_VOID, TypeFlags, TypeId, TypeKind, TypeRegistration,
    allocate_function_id, allocate_type_id,
};
use crate::parser::ast::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SemanticAnalyzer {
    registry: Arc<RwLock<TypeRegistry>>,
    pub symbol_table: SymbolTable,
    current_namespace: Vec<String>,
    current_class: Option<String>,
    current_function: Option<FunctionId>,
    errors: Vec<SemanticError>,
    loop_depth: u32,
    switch_depth: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum TypeUsage {
    AsHandle,
    AsBaseClass,
    AsVariable,
    InAssignment,
}

impl SemanticAnalyzer {
    pub fn new(registry: Arc<RwLock<TypeRegistry>>) -> Self {
        let symbol_table = SymbolTable::new(Arc::clone(&registry));

        Self {
            registry,
            symbol_table,
            current_namespace: Vec::new(),
            current_class: None,
            current_function: None,
            errors: Vec::new(),
            loop_depth: 0,
            switch_depth: 0,
        }
    }

    pub fn analyze(&mut self, script: &Script) -> Result<(), Vec<SemanticError>> {
        self.errors.clear();

        // Phase 0: Register typedefs
        for item in &script.items {
            match item {
                ScriptNode::Typedef(typedef) => {
                    self.register_typedef(typedef);
                }
                ScriptNode::Namespace(ns) => {
                    self.current_namespace.extend(ns.name.clone());
                    for nested_item in &ns.items {
                        if let ScriptNode::Typedef(typedef) = nested_item {
                            self.register_typedef(typedef);
                        }
                    }
                    self.current_namespace
                        .truncate(self.current_namespace.len() - ns.name.len());
                }
                _ => {}
            }
        }

        // Phase 1: Register type declarations
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

        // Phase 2: Register function signatures and class members
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

        // Phase 3: Analyze function bodies
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

    fn analyze_expr(&mut self, expr: &Expr) -> SemanticResult<ExprContext> {
        let context = match expr {
            Expr::Literal(lit, _span) => {
                let type_id = self.analyze_literal(lit);
                ExprContext::Temporary { type_id }
            }

            Expr::VarAccess(_, name, span) => {
                if let Some(local) = self.symbol_table.lookup_local(name) {
                    ExprContext::LocalVar {
                        type_id: local.type_id,
                        var_index: local.index,
                        is_const: local.is_const,
                    }
                } else if let Some(global) = self.symbol_table.get_global(name) {
                    ExprContext::GlobalVar {
                        type_id: global.type_id,
                        global_address: global.address,
                        is_const: global.is_const,
                    }
                } else {
                    return Err(SemanticError::UndefinedSymbol {
                        name: name.clone(),
                        span: span.clone(),
                    });
                }
            }

            Expr::Binary(left, op, right, span) => {
                let left_ctx = self.analyze_expr(left)?;
                let right_ctx = self.analyze_expr(right)?;
                self.analyze_binary_op(left_ctx, op, right_ctx, span.as_ref())?
            }

            Expr::Unary(op, operand, span) => {
                let operand_ctx = self.analyze_expr(operand)?;

                if matches!(op, UnaryOp::Handle) {
                    self.validate_type_usage(
                        operand_ctx.get_type(),
                        TypeUsage::AsHandle,
                        span.as_ref(),
                    )?;
                }

                self.analyze_unary_op(op, operand_ctx, span.as_ref())?
            }

            Expr::FuncCall(call, span) => {
                let mut arg_types = Vec::new();
                for arg in &call.args {
                    let arg_ctx = self.analyze_expr(&arg.value)?;
                    arg_types.push(arg_ctx.get_type());
                }

                match self.resolve_function_overload(&call.name, &arg_types) {
                    Ok(function_id) => {
                        let registry = self.registry.read().unwrap();
                        let func_info = registry.get_function(function_id).unwrap();

                        let min_args = func_info
                            .parameters
                            .iter()
                            .filter(|p| p.default_expr.is_none())
                            .count();
                        let max_args = func_info.parameters.len();

                        if call.args.len() < min_args || call.args.len() > max_args {
                            return Err(SemanticError::ArgumentCountMismatch {
                                expected: min_args,
                                found: call.args.len(),
                                span: span.clone(),
                            });
                        }

                        ExprContext::FunctionCall {
                            return_type: func_info.return_type,
                            function_id,
                            return_flags: func_info.return_flags,
                        }
                    }
                    Err(candidates) if candidates.is_empty() => {
                        return Err(SemanticError::UndefinedFunction {
                            name: call.name.clone(),
                            span: span.clone(),
                        });
                    }
                    Err(candidates) => {
                        let registry = self.registry.read().unwrap();
                        let candidate_names: Vec<String> = candidates
                            .iter()
                            .filter_map(|&id| {
                                registry.get_function(id).map(|f| f.full_name.clone())
                            })
                            .collect();

                        return Err(SemanticError::AmbiguousCall {
                            name: call.name.clone(),
                            candidates: candidate_names,
                            span: span.clone(),
                        });
                    }
                }
            }

            Expr::Postfix(obj, op, span) => {
                let obj_ctx = self.analyze_expr(obj)?;
                self.analyze_postfix(obj_ctx, op, span.as_ref())?
            }

            Expr::Ternary(cond, then_expr, else_expr, _span) => {
                self.analyze_expr(cond)?;
                let then_ctx = self.analyze_expr(then_expr)?;
                let else_ctx = self.analyze_expr(else_expr)?;

                let result_type = if then_ctx.get_type() == else_ctx.get_type() {
                    then_ctx.get_type()
                } else {
                    self.get_common_type(then_ctx.get_type(), else_ctx.get_type())
                };

                ExprContext::Temporary {
                    type_id: result_type,
                }
            }

            Expr::ConstructCall(type_def, args, span) => {
                for arg in args {
                    self.analyze_expr(&arg.value)?;
                }
                let type_id = self.symbol_table.resolve_type_from_ast(type_def);

                self.validate_type_usage(type_id, TypeUsage::AsVariable, span.as_ref())?;
                self.validate_required_behaviours(type_id, span.as_ref())?;

                ExprContext::Handle { type_id }
            }

            Expr::Cast(target_type, expr, _span) => {
                self.analyze_expr(expr)?;
                let type_id = self.symbol_table.resolve_type_from_ast(target_type);
                ExprContext::Temporary { type_id }
            }

            Expr::Lambda(lambda, span) => self.analyze_lambda(lambda, span.as_ref())?,

            Expr::InitList(init_list) => {
                for item in &init_list.items {
                    match item {
                        InitListItem::Expr(expr) => {
                            self.analyze_expr(expr)?;
                        }
                        InitListItem::InitList(nested) => {
                            self.analyze_expr(&Expr::InitList(nested.clone()))?;
                        }
                    }
                }

                ExprContext::Temporary { type_id: TYPE_VOID }
            }

            Expr::Void(_) => ExprContext::Temporary { type_id: TYPE_VOID },
        };

        self.symbol_table.set_expr_context(expr, context.clone());

        Ok(context)
    }

    fn analyze_binary_op(
        &mut self,
        left_ctx: ExprContext,
        op: &BinaryOp,
        right_ctx: ExprContext,
        span: Option<&Span>,
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
                if let ExprContext::VirtualProperty {
                    setter_id,
                    is_const,
                    ..
                } = &left_ctx
                {
                    if setter_id.is_none() {
                        return Err(SemanticError::ConstViolation {
                            message: "Cannot assign to read-only property".to_string(),
                            span: span.cloned(),
                        });
                    }

                    if *is_const {
                        return Err(SemanticError::ConstViolation {
                            message: "Cannot assign to property on const object".to_string(),
                            span: span.cloned(),
                        });
                    }

                    return Ok(ExprContext::Temporary {
                        type_id: left_ctx.get_type(),
                    });
                }

                if !left_ctx.is_lvalue() {
                    return Err(SemanticError::InvalidAssignment {
                        target: "expression".to_string(),
                        reason: "not an lvalue".to_string(),
                        span: span.cloned(),
                    });
                }

                if left_ctx.is_const() {
                    return Err(SemanticError::ConstViolation {
                        message: "cannot assign to const variable".to_string(),
                        span: span.cloned(),
                    });
                }

                self.validate_type_usage(left_ctx.get_type(), TypeUsage::InAssignment, span)?;

                Ok(ExprContext::Temporary {
                    type_id: left_ctx.get_type(),
                })
            }

            BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::And
            | BinaryOp::Or => Ok(ExprContext::Temporary { type_id: TYPE_BOOL }),

            _ => {
                let result_type = self.get_common_type(left_ctx.get_type(), right_ctx.get_type());
                Ok(ExprContext::Temporary {
                    type_id: result_type,
                })
            }
        }
    }

    fn analyze_unary_op(
        &self,
        op: &UnaryOp,
        operand_ctx: ExprContext,
        span: Option<&Span>,
    ) -> SemanticResult<ExprContext> {
        match op {
            UnaryOp::PreInc | UnaryOp::PreDec => {
                if !operand_ctx.is_lvalue() {
                    return Err(SemanticError::InvalidOperation {
                        operation: format!("{:?}", op),
                        type_name: "non-lvalue".to_string(),
                        span: span.cloned(),
                    });
                }

                if operand_ctx.is_const() {
                    return Err(SemanticError::ConstViolation {
                        message: format!("cannot apply {:?} to const variable", op),
                        span: span.cloned(),
                    });
                }

                Ok(ExprContext::Temporary {
                    type_id: operand_ctx.get_type(),
                })
            }

            UnaryOp::Handle => Ok(ExprContext::Handle {
                type_id: operand_ctx.get_type(),
            }),

            UnaryOp::Not => Ok(ExprContext::Temporary { type_id: TYPE_BOOL }),

            UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot => Ok(ExprContext::Temporary {
                type_id: operand_ctx.get_type(),
            }),
        }
    }

    fn analyze_postfix(
        &mut self,
        obj_ctx: ExprContext,
        op: &PostfixOp,
        span: Option<&Span>,
    ) -> SemanticResult<ExprContext> {
        match op {
            PostfixOp::PostInc | PostfixOp::PostDec => {
                if !obj_ctx.is_lvalue() {
                    return Err(SemanticError::InvalidOperation {
                        operation: format!("{:?}", op),
                        type_name: "non-lvalue".to_string(),
                        span: span.cloned(),
                    });
                }

                if obj_ctx.is_const() {
                    return Err(SemanticError::ConstViolation {
                        message: format!("cannot apply {:?} to const variable", op),
                        span: span.cloned(),
                    });
                }

                Ok(ExprContext::Temporary {
                    type_id: obj_ctx.get_type(),
                })
            }

            PostfixOp::MemberAccess(member) => {
                let obj_type = obj_ctx.get_type();

                if let Some(type_info) = self.symbol_table.get_type(obj_type) {
                    if let Some(member_info) = type_info.get_property(member) {
                        if member_info.getter.is_some() || member_info.setter.is_some() {
                            if member_info.getter.is_none() {
                                return Err(SemanticError::ConstViolation {
                                    message: format!(
                                        "Cannot read write-only property '{}'",
                                        member
                                    ),
                                    span: span.cloned(),
                                });
                            }

                            if obj_ctx.is_const() {
                                if let Some(getter_id) = member_info.getter {
                                    let registry = self.registry.read().unwrap();
                                    if let Some(getter_func) = registry.get_function(getter_id) {
                                        if !getter_func.flags.contains(FunctionFlags::CONST) {
                                            return Err(SemanticError::ConstViolation {
                                                message: format!(
                                                    "Cannot access property '{}' on const object - getter is not const",
                                                    member
                                                ),
                                                span: span.cloned(),
                                            });
                                        }
                                    }
                                }
                            }

                            return Ok(ExprContext::VirtualProperty {
                                property_type: member_info.type_id,
                                getter_id: member_info.getter,
                                setter_id: member_info.setter,
                                is_const: obj_ctx.is_const() || member_info.setter.is_none(),
                            });
                        }

                        let is_const =
                            obj_ctx.is_const() || member_info.flags.contains(PropertyFlags::CONST);

                        Ok(ExprContext::PropertyAccess {
                            property_type: member_info.type_id,
                            property_name: member.clone(),
                            is_const,
                        })
                    } else {
                        Err(SemanticError::UndefinedMember {
                            type_name: type_info.name.clone(),
                            member: member.clone(),
                            span: span.cloned(),
                        })
                    }
                } else {
                    Err(SemanticError::InvalidOperation {
                        operation: "member access".to_string(),
                        type_name: format!("type {}", obj_type),
                        span: span.cloned(),
                    })
                }
            }

            PostfixOp::MemberCall(call) => {
                let obj_type = obj_ctx.get_type();

                let mut arg_types = Vec::new();
                for arg in &call.args {
                    let arg_ctx = self.analyze_expr(&arg.value)?;
                    arg_types.push(arg_ctx.get_type());
                }

                match self.resolve_method_overload(
                    obj_type,
                    &call.name,
                    &arg_types,
                    obj_ctx.is_const(),
                ) {
                    Ok(function_id) => {
                        let registry = self.registry.read().unwrap();
                        let func_info = registry.get_function(function_id).unwrap();

                        Ok(ExprContext::MethodCall {
                            return_type: func_info.return_type,
                            function_id,
                            return_flags: func_info.return_flags,
                        })
                    }
                    Err(candidates) if candidates.is_empty() => {
                        let registry = self.registry.read().unwrap();
                        let type_info = registry.get_type(obj_type);

                        if let Some(type_info) = type_info {
                            Err(SemanticError::UndefinedMember {
                                type_name: type_info.name.clone(),
                                member: call.name.clone(),
                                span: span.cloned(),
                            })
                        } else {
                            Err(SemanticError::InvalidOperation {
                                operation: "method call".to_string(),
                                type_name: format!("type {}", obj_type),
                                span: span.cloned(),
                            })
                        }
                    }
                    Err(candidates) => {
                        let registry = self.registry.read().unwrap();
                        let candidate_names: Vec<String> = candidates
                            .iter()
                            .filter_map(|&id| {
                                registry.get_function(id).map(|f| f.full_name.clone())
                            })
                            .collect();

                        Err(SemanticError::AmbiguousCall {
                            name: call.name.clone(),
                            candidates: candidate_names,
                            span: span.cloned(),
                        })
                    }
                }
            }

            PostfixOp::Index(_indices) => Ok(ExprContext::Temporary {
                type_id: TYPE_INT32,
            }),

            PostfixOp::Call(_args) => Ok(ExprContext::Temporary { type_id: TYPE_VOID }),
        }
    }

    // ✅ Function overload resolution (returns Result with candidate list)
    fn resolve_function_overload(
        &self,
        name: &str,
        arg_types: &[TypeId],
    ) -> Result<FunctionId, Vec<FunctionId>> {
        let registry = self.registry.read().unwrap();

        let mut candidates = registry.get_functions_by_name(name);

        candidates.retain(|f| f.namespace == self.current_namespace || f.namespace.is_empty());

        if candidates.is_empty() {
            return Err(Vec::new());
        }

        candidates.retain(|f| {
            let min_args = f
                .parameters
                .iter()
                .filter(|p| p.default_expr.is_none())
                .count();
            let max_args = f.parameters.len();
            arg_types.len() >= min_args && arg_types.len() <= max_args
        });

        if candidates.is_empty() {
            return Err(Vec::new());
        }

        // Phase 1: Exact matches
        let exact_matches: Vec<FunctionId> = candidates
            .iter()
            .filter(|f| {
                f.parameters
                    .iter()
                    .take(arg_types.len())
                    .zip(arg_types)
                    .all(|(param, &arg_type)| {
                        let param_type = registry.resolve_typedef(param.type_id);
                        let arg_type = registry.resolve_typedef(arg_type);
                        param_type == arg_type
                    })
            })
            .map(|f| f.function_id)
            .collect();

        if exact_matches.len() > 1 {
            return Err(exact_matches);
        }

        if exact_matches.len() == 1 {
            return Ok(exact_matches[0]);
        }

        // Phase 2: Conversion matches
        let conversion_matches: Vec<FunctionId> = candidates
            .iter()
            .filter(|f| {
                f.parameters
                    .iter()
                    .take(arg_types.len())
                    .zip(arg_types)
                    .all(|(param, &arg_type)| {
                        let param_type = registry.resolve_typedef(param.type_id);
                        let arg_type = registry.resolve_typedef(arg_type);

                        param_type == arg_type
                            || registry.can_implicitly_convert(arg_type, param_type)
                    })
            })
            .map(|f| f.function_id)
            .collect();

        if conversion_matches.len() > 1 {
            return Err(conversion_matches);
        }

        if conversion_matches.len() == 1 {
            return Ok(conversion_matches[0]);
        }

        Err(Vec::new())
    }

    // ✅ Method overload resolution
    fn resolve_method_overload(
        &self,
        type_id: TypeId,
        method_name: &str,
        arg_types: &[TypeId],
        is_const_context: bool,
    ) -> Result<FunctionId, Vec<FunctionId>> {
        let registry = self.registry.read().unwrap();

        let candidates = registry.get_methods_by_name(type_id, method_name);

        if candidates.is_empty() {
            return Err(Vec::new());
        }

        // Filter by argument count
        let valid_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|f| {
                let min_args = f
                    .parameters
                    .iter()
                    .filter(|p| p.default_expr.is_none())
                    .count();
                let max_args = f.parameters.len();
                arg_types.len() >= min_args && arg_types.len() <= max_args
            })
            .collect();

        if valid_candidates.is_empty() {
            return Err(Vec::new());
        }

        // Phase 1: Exact matches (with const checking)
        let exact_matches: Vec<FunctionId> = valid_candidates
            .iter()
            .filter(|f| {
                let type_match = f
                    .parameters
                    .iter()
                    .take(arg_types.len())
                    .zip(arg_types)
                    .all(|(param, &arg_type)| {
                        let param_type = registry.resolve_typedef(param.type_id);
                        let arg_type = registry.resolve_typedef(arg_type);
                        param_type == arg_type
                    });

                if !type_match {
                    return false;
                }

                // Check const qualifier
                if is_const_context {
                    matches!(f.kind, FunctionKind::Method { is_const: true })
                } else {
                    true
                }
            })
            .map(|f| f.function_id)
            .collect();

        if exact_matches.len() > 1 {
            return Err(exact_matches);
        }

        if exact_matches.len() == 1 {
            return Ok(exact_matches[0]);
        }

        // Phase 2: Conversion matches
        let conversion_matches: Vec<FunctionId> = valid_candidates
            .iter()
            .filter(|f| {
                let type_match = f
                    .parameters
                    .iter()
                    .take(arg_types.len())
                    .zip(arg_types)
                    .all(|(param, &arg_type)| {
                        let param_type = registry.resolve_typedef(param.type_id);
                        let arg_type = registry.resolve_typedef(arg_type);

                        param_type == arg_type
                            || registry.can_implicitly_convert(arg_type, param_type)
                    });

                if !type_match {
                    return false;
                }

                if is_const_context {
                    matches!(f.kind, FunctionKind::Method { is_const: true })
                } else {
                    true
                }
            })
            .map(|f| f.function_id)
            .collect();

        if conversion_matches.len() > 1 {
            return Err(conversion_matches);
        }

        if conversion_matches.len() == 1 {
            return Ok(conversion_matches[0]);
        }

        Err(Vec::new())
    }

    fn analyze_lambda(
        &mut self,
        lambda: &Lambda,
        span: Option<&Span>,
    ) -> SemanticResult<ExprContext> {
        let lambda_name = format!("$lambda_{}", self.errors.len());
        let function_id = allocate_function_id();

        self.current_function = Some(function_id);
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

                self.symbol_table.register_local(
                    name.clone(),
                    type_id,
                    false,
                    true,
                    param.span.clone(),
                );
            }
        }

        let mut return_type = TYPE_VOID;
        for stmt in &lambda.body.statements {
            self.analyze_statement(stmt)?;

            if let Statement::Return(ret) = stmt {
                if let Some(value) = &ret.value {
                    if let Some(ctx) = self.symbol_table.get_expr_context(value) {
                        return_type = ctx.get_type();
                        break;
                    }
                }
            }
        }

        let locals = self.symbol_table.collect_function_locals(&lambda_name);

        let parameters = lambda
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| ParameterInfo {
                name: p.name.clone(),
                type_id: param_types.get(i).copied().unwrap_or(TYPE_AUTO),
                flags: match p.type_mod {
                    Some(TypeMod::In) => ParameterFlags::IN,
                    Some(TypeMod::Out) => ParameterFlags::OUT,
                    Some(TypeMod::InOut) => ParameterFlags::INOUT,
                    None => ParameterFlags::IN,
                },
                default_expr: None,
                definition_span: p.span.clone(),
            })
            .collect();

        let func_info = FunctionInfo {
            function_id,
            name: lambda_name.clone(),
            full_name: lambda_name,
            namespace: self.current_namespace.clone(),

            return_type,
            return_flags: ReturnFlags::empty(),
            parameters,

            kind: FunctionKind::Lambda,
            flags: FunctionFlags::PUBLIC,

            owner_type: None,
            vtable_index: None,

            implementation: FunctionImpl::Script {
                bytecode_offset: 0,
                module_id: 0,
            },

            definition_span: span.cloned(),

            locals,

            bytecode_address: None,
            local_count: 0,
        };

        if let Err(e) = self.registry.write().unwrap().register_function(func_info) {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: None,
            });
        }

        self.symbol_table.pop_scope();
        self.current_function = None;

        let funcdef_type = self.create_funcdef_type(return_type, &param_types);

        Ok(ExprContext::Handle {
            type_id: funcdef_type,
        })
    }

    fn create_funcdef_type(&mut self, return_type: TypeId, param_types: &[TypeId]) -> TypeId {
        let signature = format!(
            "funcdef_{}_({})",
            return_type,
            param_types
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("_")
        );

        if let Some(existing_id) = self.symbol_table.lookup_type(&signature) {
            return existing_id;
        }

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: signature,
            namespace: Vec::new(),
            kind: TypeKind::Funcdef,
            flags: TypeFlags::FUNCDEF,
            registration: TypeRegistration::Script,

            properties: Vec::new(),
            methods: HashMap::new(),

            base_type: None,
            interfaces: Vec::new(),

            behaviours: HashMap::new(),

            rust_type_id: None,

            vtable: Vec::new(),

            definition_span: None,
        };

        let type_id = type_info.type_id;

        if self
            .registry
            .write()
            .unwrap()
            .register_type(type_info)
            .is_err()
        {
            return TYPE_VOID;
        }

        type_id
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

    fn register_typedef(&mut self, typedef: &Typedef) {
        let aliased_type_id = self.symbol_table.lookup_type(&typedef.prim_type);

        if aliased_type_id.is_none() {
            self.errors.push(SemanticError::UndefinedType {
                name: typedef.prim_type.clone(),
                span: typedef.span.clone(),
            });
            return;
        }

        let aliased_type_id = aliased_type_id.unwrap();

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: typedef.name.clone(),
            namespace: self.current_namespace.clone(),
            kind: TypeKind::Typedef,
            flags: TypeFlags::TYPEDEF,
            registration: TypeRegistration::Script,

            properties: Vec::new(),
            methods: HashMap::new(),

            base_type: Some(aliased_type_id),
            interfaces: Vec::new(),

            behaviours: HashMap::new(),

            rust_type_id: None,

            vtable: Vec::new(),

            definition_span: typedef.span.clone(),
        };

        if let Err(e) = self.registry.write().unwrap().register_type(type_info) {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: typedef.span.clone(),
            });
        }
    }

    fn register_class_type(&mut self, class: &Class) {
        let base_class = if !class.extends.is_empty() {
            let base_type_id = self.symbol_table.lookup_type(&class.extends[0]);

            if let Some(base_id) = base_type_id {
                if let Err(e) =
                    self.validate_type_usage(base_id, TypeUsage::AsBaseClass, class.span.as_ref())
                {
                    self.errors.push(e);
                }
            }

            base_type_id
        } else {
            None
        };

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: class.name.clone(),
            namespace: self.current_namespace.clone(),
            kind: TypeKind::Class,
            flags: TypeFlags::REF_TYPE,
            registration: TypeRegistration::Script,

            properties: Vec::new(),
            methods: HashMap::new(),

            base_type: base_class,
            interfaces: Vec::new(),

            behaviours: HashMap::new(),

            rust_type_id: None,

            vtable: Vec::new(),

            definition_span: class.span.clone(),
        };

        if let Err(e) = self.registry.write().unwrap().register_type(type_info) {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: class.span.clone(),
            });
        }
    }

    fn register_funcdef_type(&mut self, funcdef: &FuncDef) {
        let _return_type = self
            .symbol_table
            .resolve_type_from_ast(&funcdef.return_type);

        let type_info = TypeInfo {
            type_id: allocate_type_id(),
            name: funcdef.name.clone(),
            namespace: self.current_namespace.clone(),
            kind: TypeKind::Funcdef,
            flags: TypeFlags::FUNCDEF,
            registration: TypeRegistration::Script,

            properties: Vec::new(),
            methods: HashMap::new(),

            base_type: None,
            interfaces: Vec::new(),

            behaviours: HashMap::new(),

            rust_type_id: None,

            vtable: Vec::new(),

            definition_span: funcdef.span.clone(),
        };

        if let Err(e) = self.registry.write().unwrap().register_type(type_info) {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: funcdef.span.clone(),
            });
        }
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

        let parameters = func
            .params
            .iter()
            .map(|p| ParameterInfo {
                name: p.name.clone(),
                type_id: self.symbol_table.resolve_type_from_ast(&p.param_type),
                flags: match p.type_mod {
                    Some(TypeMod::In) => ParameterFlags::IN,
                    Some(TypeMod::Out) => ParameterFlags::OUT,
                    Some(TypeMod::InOut) => ParameterFlags::INOUT,
                    None => ParameterFlags::IN,
                } | if p.param_type.is_const {
                    ParameterFlags::CONST
                } else {
                    ParameterFlags::empty()
                },
                default_expr: p.default_value.as_ref().map(|expr| Arc::new(expr.clone())),
                definition_span: p.span.clone(),
            })
            .collect();

        let full_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func.name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func.name)
        } else {
            func.name.clone()
        };

        let function_id = allocate_function_id();

        let mut flags = FunctionFlags::PUBLIC;
        if func.is_const {
            flags |= FunctionFlags::CONST;
        }
        if func.modifiers.contains(&"virtual".to_string()) {
            flags |= FunctionFlags::VIRTUAL;
        }
        if func.modifiers.contains(&"override".to_string()) {
            flags |= FunctionFlags::OVERRIDE;
        }
        if func.modifiers.contains(&"final".to_string()) {
            flags |= FunctionFlags::FINAL;
        }
        if func.modifiers.contains(&"abstract".to_string()) {
            flags |= FunctionFlags::ABSTRACT;
        }
        if let Some(Visibility::Private) = &func.visibility {
            flags |= FunctionFlags::PRIVATE;
        }
        if let Some(Visibility::Protected) = &func.visibility {
            flags |= FunctionFlags::PROTECTED;
        }

        let mut return_flags = ReturnFlags::empty();
        if func.is_ref {
            return_flags |= ReturnFlags::REF;
        }
        // Check if the return type itself is const (for const references)
        if let Some(ret_type) = &func.return_type {
            if ret_type.is_const {
                return_flags |= ReturnFlags::CONST;
            }
        }

        let func_info = FunctionInfo {
            function_id,
            name: func.name.clone(),
            full_name: full_name.clone(),
            namespace: self.current_namespace.clone(),

            return_type,
            return_flags,

            parameters,

            kind: if self.current_class.is_some() {
                FunctionKind::Method {
                    is_const: func.is_const,
                }
            } else {
                FunctionKind::Global
            },
            flags,

            owner_type: self
                .current_class
                .as_ref()
                .and_then(|name| self.symbol_table.lookup_type(name)),
            vtable_index: None,

            implementation: FunctionImpl::Script {
                bytecode_offset: 0,
                module_id: 0,
            },

            definition_span: func.span.clone(),
            locals: Vec::new(),

            bytecode_address: None,
            local_count: 0,
        };

        let registry = self.registry.read().unwrap();
        let existing_funcs = registry.get_functions_by_name(&func.name);

        for existing in &existing_funcs {
            // Check if this is a duplicate signature
            if self.is_duplicate_signature(&func_info, existing) {
                self.errors.push(SemanticError::DuplicateFunction {
                    name: func.name.clone(),
                    span: func.span.clone(),
                });
                return;
            }
        }
        drop(registry);

        // Now register
        if let Err(e) = self.registry.write().unwrap().register_function(func_info) {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: func.span.clone(),
            });
        }

        if let Some(class_name) = &self.current_class {
            if let Some(type_id) = self.symbol_table.lookup_type(class_name) {
                if let Err(e) = self.registry.write().unwrap().add_method(
                    type_id,
                    func.name.clone(),
                    function_id,
                ) {
                    self.errors.push(SemanticError::Internal {
                        message: e,
                        span: func.span.clone(),
                    });
                }
            }
        }
    }

    fn is_duplicate_signature(
        &self,
        new_func: &FunctionInfo,
        existing: &Arc<FunctionInfo>,
    ) -> bool {
        // Same namespace
        if new_func.namespace != existing.namespace {
            return false;
        }

        // Same owner (class)
        if new_func.owner_type != existing.owner_type {
            return false;
        }

        // Same parameter count
        if new_func.parameters.len() != existing.parameters.len() {
            return false;
        }

        // Same parameter types
        for (new_param, existing_param) in new_func.parameters.iter().zip(&existing.parameters) {
            if new_param.type_id != existing_param.type_id {
                return false;
            }
            // Parameter const-ness matters for overload resolution
            if new_param.flags.contains(ParameterFlags::CONST)
                != existing_param.flags.contains(ParameterFlags::CONST)
            {
                return false;
            }
        }

        // For methods, const-ness matters
        match (&new_func.kind, &existing.kind) {
            (FunctionKind::Method { is_const: nc }, FunctionKind::Method { is_const: ec }) => {
                if nc != ec {
                    return false; // Different const-ness = different overload
                }
            }
            _ => {}
        }

        // All checks passed - this is a duplicate
        true
    }

    fn validate_function_params(&mut self, func: &Func) -> SemanticResult<()> {
        let mut has_default = false;

        for param in &func.params {
            let type_id = self.symbol_table.resolve_type_from_ast(&param.param_type);

            if let Some(default_expr) = &param.default_value {
                has_default = true;

                let default_ctx = self.analyze_expr(default_expr)?;

                let param_type = self.registry.read().unwrap().resolve_typedef(type_id);
                let default_type = self
                    .registry
                    .read()
                    .unwrap()
                    .resolve_typedef(default_ctx.get_type());

                if param_type != default_type {
                    let registry = self.registry.read().unwrap();
                    if !registry.can_implicitly_convert(default_type, param_type) {
                        return Err(SemanticError::TypeMismatch {
                            expected: format!("type {}", param_type),
                            found: format!("type {}", default_type),
                            span: param.span.clone(),
                        });
                    }
                }

                if !self.is_constant_expression(default_expr) {
                    return Err(SemanticError::Internal {
                        message: "Default argument must be a constant expression".to_string(),
                        span: param.span.clone(),
                    });
                }
            } else if has_default {
                return Err(SemanticError::Internal {
                    message: "Non-default parameter after default parameter".to_string(),
                    span: param.span.clone(),
                });
            }

            if let Some(type_mod) = &param.type_mod {
                self.validate_reference_param(
                    type_id,
                    type_mod,
                    &param.param_type,
                    param.span.as_ref(),
                )?;
            }
        }

        Ok(())
    }

    fn is_constant_expression(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Literal(_, _) => true,
            Expr::Unary(UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot, operand, _) => {
                self.is_constant_expression(operand)
            }
            Expr::Binary(left, op, right, _) => {
                let allowed_op = matches!(
                    op,
                    BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                );

                allowed_op
                    && self.is_constant_expression(left)
                    && self.is_constant_expression(right)
            }
            Expr::Ternary(cond, then_expr, else_expr, _) => {
                self.is_constant_expression(cond)
                    && self.is_constant_expression(then_expr)
                    && self.is_constant_expression(else_expr)
            }
            _ => false,
        }
    }

    fn validate_reference_param(
        &self,
        type_id: TypeId,
        type_mod: &TypeMod,
        param_type: &Type,
        span: Option<&Span>,
    ) -> SemanticResult<()> {
        if type_id <= TYPE_STRING {
            if matches!(type_mod, TypeMod::InOut) {
                return Err(SemanticError::ReferenceMismatch {
                    message:
                        "Primitive types cannot use 'inout' references. Use 'in' or 'out' instead."
                            .to_string(),
                    span: span.cloned(),
                });
            }
            return Ok(());
        }

        let type_info = self.symbol_table.get_type(type_id);

        if let Some(type_info) = type_info {
            let is_value_type = type_info.is_value_type();
            let is_ref_type = type_info.is_ref_type();

            match type_mod {
                TypeMod::InOut => {
                    if is_value_type {
                        return Err(SemanticError::ReferenceMismatch {
                            message: format!(
                                "Value type '{}' cannot use 'inout' references. AngelScript cannot guarantee the reference will remain valid during function execution. Use 'in' or 'out' instead.",
                                type_info.name
                            ),
                            span: span.cloned(),
                        });
                    }

                    if !is_ref_type {
                        return Err(SemanticError::ReferenceMismatch {
                            message: format!(
                                "Type '{}' must be a reference type to use 'inout' references",
                                type_info.name
                            ),
                            span: span.cloned(),
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
                            span: span.cloned(),
                        });
                    }
                }

                TypeMod::Out => {
                    if param_type.is_const {
                        return Err(SemanticError::ReferenceMismatch {
                            message: "Output parameters cannot be const".to_string(),
                            span: span.cloned(),
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

            let global_info = GlobalInfo {
                name: full_name,
                type_id,
                is_const: var.var_type.is_const,
                address: idx as u32,
                definition_span: decl.span.clone(),
            };

            if let Err(e) = self.registry.write().unwrap().register_global(global_info) {
                self.errors.push(SemanticError::Internal {
                    message: e,
                    span: decl.span.clone(),
                });
            }
        }
    }

    fn register_class_members(&mut self, class: &Class) {
        let type_id = match self.symbol_table.lookup_type(&class.name) {
            Some(id) => id,
            None => return,
        };

        let saved_class = self.current_class.clone();
        self.current_class = Some(class.name.clone());

        for member in &class.members {
            match member {
                ClassMember::Var(var) => {
                    let member_type = self.symbol_table.resolve_type_from_ast(&var.var_type);
                    for decl in &var.declarations {
                        let property_info = PropertyInfo {
                            name: decl.name.clone(),
                            type_id: member_type,
                            offset: None,
                            access: match &var.visibility {
                                Some(Visibility::Private) => AccessSpecifier::Private,
                                Some(Visibility::Protected) => AccessSpecifier::Protected,
                                Some(Visibility::Public) | None => AccessSpecifier::Public,
                            },
                            flags: if var.var_type.is_const {
                                PropertyFlags::CONST | PropertyFlags::PUBLIC
                            } else {
                                PropertyFlags::PUBLIC
                            },
                            getter: None,
                            setter: None,
                            definition_span: decl.span.clone(),
                        };

                        if let Err(e) = self
                            .registry
                            .write()
                            .unwrap()
                            .add_property(type_id, property_info)
                        {
                            self.errors.push(SemanticError::Internal {
                                message: e,
                                span: decl.span.clone(),
                            });
                        }
                    }
                }
                ClassMember::Func(func) => {
                    self.register_function_signature(func);
                }
                ClassMember::VirtProp(prop) => {
                    self.register_virtual_property(type_id, prop);
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
                    if let Some(func_info) = self
                        .symbol_table
                        .get_function(&format!("{}::{}", class.name, func.name))
                    {
                        vtable.push(VTableEntry {
                            method_name: func.name.clone(),
                            function_id: func_info.function_id,
                            override_of: None,
                        });
                    }
                }
            }
        }

        if !vtable.is_empty() {
            if let Err(e) = self
                .registry
                .write()
                .unwrap()
                .update_vtable(type_id, vtable)
            {
                self.errors.push(SemanticError::Internal {
                    message: e,
                    span: class.span.clone(),
                });
            }
        }

        self.current_class = saved_class;
    }

    fn register_virtual_property(&mut self, type_id: TypeId, prop: &VirtProp) {
        let prop_type = self.symbol_table.resolve_type_from_ast(&prop.prop_type);

        let mut getter_id = None;
        let mut setter_id = None;

        for accessor in &prop.accessors {
            let func_name = match accessor.kind {
                AccessorKind::Get => format!("get_{}", prop.name),
                AccessorKind::Set => format!("set_{}", prop.name),
            };

            let full_name = if let Some(class_name) = &self.current_class {
                format!("{}::{}", class_name, func_name)
            } else {
                func_name.clone()
            };

            let function_id = allocate_function_id();

            let return_type = match accessor.kind {
                AccessorKind::Get => prop_type,
                AccessorKind::Set => TYPE_VOID,
            };

            let parameters = match accessor.kind {
                AccessorKind::Get => Vec::new(),
                AccessorKind::Set => vec![ParameterInfo {
                    name: Some("value".to_string()),
                    type_id: prop_type,
                    flags: ParameterFlags::IN,
                    default_expr: None,
                    definition_span: None,
                }],
            };

            let func_info = FunctionInfo {
                function_id,
                name: func_name.clone(),
                full_name: full_name.clone(),
                namespace: self.current_namespace.clone(),

                return_type,
                return_is_ref: false,
                return_is_auto_handle: false,
                parameters,

                kind: FunctionKind::Method {
                    is_const: accessor.is_const,
                },
                flags: if accessor.is_const {
                    FunctionFlags::PUBLIC | FunctionFlags::CONST
                } else {
                    FunctionFlags::PUBLIC
                },

                owner_type: Some(type_id),
                vtable_index: None,

                implementation: FunctionImpl::Script {
                    bytecode_offset: 0,
                    module_id: 0,
                },

                definition_span: accessor.span.clone(),
                locals: Vec::new(),

                bytecode_address: None,
                local_count: 0,
            };

            if let Err(e) = self.registry.write().unwrap().register_function(func_info) {
                self.errors.push(SemanticError::Internal {
                    message: e,
                    span: accessor.span.clone(),
                });
            }

            if let Err(e) =
                self.registry
                    .write()
                    .unwrap()
                    .add_method(type_id, func_name, function_id)
            {
                self.errors.push(SemanticError::Internal {
                    message: e,
                    span: accessor.span.clone(),
                });
            }

            match accessor.kind {
                AccessorKind::Get => getter_id = Some(function_id),
                AccessorKind::Set => setter_id = Some(function_id),
            }
        }

        let property_info = PropertyInfo {
            name: prop.name.clone(),
            type_id: prop_type,
            offset: None,
            access: match &prop.visibility {
                Some(Visibility::Private) => AccessSpecifier::Private,
                Some(Visibility::Protected) => AccessSpecifier::Protected,
                Some(Visibility::Public) | None => AccessSpecifier::Public,
            },
            flags: PropertyFlags::VIRTUAL | PropertyFlags::PUBLIC,
            getter: getter_id,
            setter: setter_id,
            definition_span: prop.span.clone(),
        };

        if let Err(e) = self
            .registry
            .write()
            .unwrap()
            .add_property(type_id, property_info)
        {
            self.errors.push(SemanticError::Internal {
                message: e,
                span: prop.span.clone(),
            });
        }
    }

    fn analyze_function(&mut self, func: &Func) -> SemanticResult<()> {
        // Phase 6: Reject @+ (AutoHandle) in script code - it's FFI-only
        if let Some(return_type) = &func.return_type {
            if return_type
                .modifiers
                .iter()
                .any(|m| matches!(m, TypeModifier::AutoHandle))
            {
                return Err(SemanticError::new(
                    format!(
                        "Auto handles (@+) are only allowed in FFI function registration, not in script code"
                    ),
                    func.span.clone(),
                ));
            }
        }

        let func_name = if let Some(class_name) = &self.current_class {
            format!("{}::{}", class_name, func.name)
        } else if !self.current_namespace.is_empty() {
            format!("{}::{}", self.current_namespace.join("::"), func.name)
        } else {
            func.name.clone()
        };

        let func_info = self
            .symbol_table
            .get_function(&func_name)
            .ok_or_else(|| SemanticError::undefined_function(func_name.clone()))?;

        self.current_function = Some(func_info.function_id);

        self.symbol_table
            .push_scope(ScopeType::Function(func_name.clone()));

        if let Some(class_name) = &self.current_class {
            if let Some(class_type_id) = self.symbol_table.lookup_type(class_name) {
                self.symbol_table.register_local(
                    "this".to_string(),
                    class_type_id,
                    func.is_const,
                    true,
                    None,
                );
            }
        }

        for param in &func.params {
            // Phase 6: Reject @+ (AutoHandle) in script code - it's FFI-only
            if param
                .param_type
                .modifiers
                .iter()
                .any(|m| matches!(m, TypeModifier::AutoHandle))
            {
                return Err(SemanticError::new(
                    format!(
                        "Auto handles (@+) are only allowed in FFI function registration, not in script code"
                    ),
                    param.span.clone(),
                ));
            }

            if let Some(name) = &param.name {
                let type_id = self.symbol_table.resolve_type_from_ast(&param.param_type);
                self.symbol_table.register_local(
                    name.clone(),
                    type_id,
                    false,
                    true,
                    param.span.clone(),
                );
            }
        }

        if let Some(body) = &func.body {
            self.analyze_statement_block(body)?;

            if func.is_ref {
                self.validate_function_return_references(func, body)?;
            }
        }

        let locals = self.symbol_table.collect_function_locals(&func_name);

        if let Some(function_id) = self.current_function {
            if let Err(e) = self
                .registry
                .write()
                .unwrap()
                .update_function_locals(function_id, locals)
            {
                self.errors.push(SemanticError::Internal {
                    message: e,
                    span: func.span.clone(),
                });
            }
        }

        self.symbol_table.pop_scope();
        self.current_function = None;

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

                            let full_name = format!("{}::{}", class.name, func_name);

                            if let Some(func_info) = self.symbol_table.get_function(&full_name) {
                                self.current_function = Some(func_info.function_id);
                            }

                            self.symbol_table
                                .push_scope(ScopeType::Function(full_name.clone()));

                            if let Some(class_type_id) = self.symbol_table.lookup_type(&class.name)
                            {
                                self.symbol_table.register_local(
                                    "this".to_string(),
                                    class_type_id,
                                    accessor.is_const,
                                    true,
                                    None,
                                );
                            }

                            if matches!(accessor.kind, AccessorKind::Set) {
                                let prop_type =
                                    self.symbol_table.resolve_type_from_ast(&prop.prop_type);
                                self.symbol_table.register_local(
                                    "value".to_string(),
                                    prop_type,
                                    false,
                                    true,
                                    None,
                                );
                            }

                            self.analyze_statement_block(body)?;

                            let locals = self.symbol_table.collect_function_locals(&full_name);

                            if let Some(function_id) = self.current_function {
                                if let Err(e) = self
                                    .registry
                                    .write()
                                    .unwrap()
                                    .update_function_locals(function_id, locals)
                                {
                                    self.errors.push(SemanticError::Internal {
                                        message: e,
                                        span: accessor.span.clone(),
                                    });
                                }
                            }

                            self.symbol_table.pop_scope();
                            self.current_function = None;
                        }
                    }
                }
                _ => {}
            }
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
        self.current_function.is_some()
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> SemanticResult<()> {
        match stmt {
            Statement::Var(var) => {
                let type_id = self.symbol_table.resolve_type_from_ast(&var.var_type);

                if type_id > TYPE_STRING && type_id != TYPE_AUTO {
                    self.validate_type_usage(type_id, TypeUsage::AsVariable, var.span.as_ref())?;
                }

                let is_handle = var.var_type.modifiers.contains(&TypeModifier::Handle);

                if is_handle {
                    self.validate_type_usage(type_id, TypeUsage::AsHandle, var.span.as_ref())?;
                }

                for decl in &var.declarations {
                    let has_duplicate_in_current_scope = self
                        .symbol_table
                        .scopes
                        .last()
                        .map(|scope| scope.variables.contains_key(&decl.name))
                        .unwrap_or(false);

                    if has_duplicate_in_current_scope {
                        return Err(SemanticError::DuplicateDefinition {
                            name: decl.name.clone(),
                            span: decl.span.clone(),
                            previous_span: None,
                        });
                    }

                    self.symbol_table.register_local(
                        decl.name.clone(),
                        type_id,
                        var.var_type.is_const,
                        false,
                        decl.span.clone(),
                    );

                    if let Some(VarInit::Expr(expr)) = &decl.initializer {
                        self.analyze_expr(expr)?;
                    }
                }
                Ok(())
            }

            Statement::Break(span) => {
                if self.loop_depth == 0 && self.switch_depth == 0 {
                    Err(SemanticError::InvalidBreak { span: span.clone() })
                } else {
                    Ok(())
                }
            }

            Statement::Continue(span) => {
                if self.loop_depth == 0 {
                    Err(SemanticError::InvalidContinue { span: span.clone() })
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
                                decl.span.clone(),
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
                        .register_local(var_name.clone(), type_id, false, false, None);
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

    fn validate_type_usage(
        &self,
        type_id: TypeId,
        usage: TypeUsage,
        span: Option<&Span>,
    ) -> SemanticResult<()> {
        if type_id == TYPE_AUTO || type_id == TYPE_VOID {
            return Ok(());
        }

        let type_info = self.symbol_table.get_type(type_id);

        if let Some(type_info) = type_info {
            match usage {
                TypeUsage::AsHandle => {
                    if !type_info.can_be_handle() {
                        return Err(SemanticError::InvalidHandle {
                            message: format!("Type '{}' cannot be used as handle", type_info.name),
                            span: span.cloned(),
                        });
                    }
                }

                TypeUsage::AsBaseClass => {
                    if !type_info.can_be_inherited() {
                        return Err(SemanticError::Internal {
                            message: format!(
                                "Type '{}' is final and cannot be inherited",
                                type_info.name
                            ),
                            span: span.cloned(),
                        });
                    }
                }

                TypeUsage::AsVariable => {
                    if type_info.is_abstract() {
                        return Err(SemanticError::InstantiateAbstract {
                            class: type_info.name.clone(),
                            span: span.cloned(),
                        });
                    }
                }

                TypeUsage::InAssignment => {
                    if type_info.flags.contains(TypeFlags::SCOPED) {
                        return Err(SemanticError::InvalidAssignment {
                            target: type_info.name.clone(),
                            reason: "scoped type cannot be assigned".to_string(),
                            span: span.cloned(),
                        });
                    }
                }
            }

            Ok(())
        } else {
            Err(SemanticError::UndefinedType {
                name: format!("type {}", type_id),
                span: span.cloned(),
            })
        }
    }

    fn validate_required_behaviours(
        &self,
        type_id: TypeId,
        span: Option<&Span>,
    ) -> SemanticResult<()> {
        if let Some(type_info) = self.symbol_table.get_type(type_id) {
            if type_info.registration != TypeRegistration::Application {
                return Ok(());
            }

            if type_info.kind != TypeKind::Class {
                return Ok(());
            }

            if type_info.is_ref_type() {
                if !type_info.flags.contains(TypeFlags::NOCOUNT) {
                    let has_addref = type_info.behaviours.contains_key(&BehaviourType::AddRef);
                    let has_release = type_info.behaviours.contains_key(&BehaviourType::Release);

                    if !has_addref || !has_release {
                        return Err(SemanticError::Internal {
                            message: format!(
                                "Reference type '{}' must have AddRef and Release behaviours",
                                type_info.name
                            ),
                            span: span.cloned(),
                        });
                    }
                }

                let has_factory = type_info.behaviours.contains_key(&BehaviourType::Construct);

                if !has_factory && !type_info.flags.contains(TypeFlags::NOHANDLE) {
                    return Err(SemanticError::Internal {
                        message: format!(
                            "Reference type '{}' must have a factory behaviour",
                            type_info.name
                        ),
                        span: span.cloned(),
                    });
                }
            }

            Ok(())
        } else {
            Err(SemanticError::UndefinedType {
                name: format!("type {}", type_id),
                span: span.cloned(),
            })
        }
    }

    fn validate_function_return_references(
        &self,
        func: &Func,
        body: &StatBlock,
    ) -> SemanticResult<()> {
        let allow_unsafe = self
            .registry
            .read()
            .unwrap()
            .get_property(EngineProperty::AllowUnsafeReferences)
            != 0;

        if allow_unsafe {
            return Ok(());
        }

        self.validate_return_statements_in_block(body, func)?;

        Ok(())
    }

    fn validate_return_statements_in_block(
        &self,
        block: &StatBlock,
        func: &Func,
    ) -> SemanticResult<()> {
        for stmt in &block.statements {
            match stmt {
                Statement::Return(ret) => {
                    self.validate_return_reference(ret, func)?;
                }
                Statement::If(if_stmt) => {
                    self.validate_return_statements_in_statement(&if_stmt.then_branch, func)?;
                    if let Some(else_branch) = &if_stmt.else_branch {
                        self.validate_return_statements_in_statement(else_branch, func)?;
                    }
                }
                Statement::Block(inner_block) => {
                    self.validate_return_statements_in_block(inner_block, func)?;
                }
                Statement::While(while_stmt) => {
                    self.validate_return_statements_in_statement(&while_stmt.body, func)?;
                }
                Statement::For(for_stmt) => {
                    self.validate_return_statements_in_statement(&for_stmt.body, func)?;
                }
                Statement::Switch(switch_stmt) => {
                    for case in &switch_stmt.cases {
                        for case_stmt in &case.statements {
                            self.validate_return_statements_in_statement(case_stmt, func)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_return_statements_in_statement(
        &self,
        stmt: &Statement,
        func: &Func,
    ) -> SemanticResult<()> {
        match stmt {
            Statement::Return(ret) => self.validate_return_reference(ret, func),
            Statement::Block(block) => self.validate_return_statements_in_block(block, func),
            Statement::If(if_stmt) => {
                self.validate_return_statements_in_statement(&if_stmt.then_branch, func)?;
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.validate_return_statements_in_statement(else_branch, func)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn validate_return_reference(&self, ret_stmt: &ReturnStmt, func: &Func) -> SemanticResult<()> {
        if !func.is_ref {
            return Ok(());
        }

        let value = match &ret_stmt.value {
            Some(v) => v,
            None => return Ok(()),
        };

        // Check if the return type is const
        let return_is_const = func
            .return_type
            .as_ref()
            .map(|t| t.is_const)
            .unwrap_or(false);

        match value {
            Expr::Postfix(obj, PostfixOp::MemberAccess(member_name), _) => {
                if let Expr::VarAccess(_, name, _) = obj.as_ref() {
                    if name == "this" {
                        // Check const-correctness: if returning non-const reference,
                        // must be from non-const method
                        if !return_is_const && func.is_const {
                            return Err(SemanticError::InvalidReturn {
                                span: ret_stmt.span.clone(),
                            });
                        }
                        return Ok(());
                    }
                }

                if let Some(ctx) = self.symbol_table.get_expr_context(obj) {
                    if ctx.is_lvalue() && !ctx.is_temporary() {
                        // Validate const-correctness
                        if !return_is_const && ctx.is_const() {
                            return Err(SemanticError::InvalidReturn {
                                span: ret_stmt.span.clone(),
                            });
                        }
                        return Ok(());
                    }
                }

                Err(SemanticError::InvalidReturn {
                    span: ret_stmt.span.clone(),
                })
            }

            Expr::VarAccess(_, name, _) => {
                if let Some(local) = self.symbol_table.lookup_local(name) {
                    if local.is_param {
                        // Check const-correctness for parameters
                        if !return_is_const && local.is_const {
                            return Err(SemanticError::InvalidReturn {
                                span: ret_stmt.span.clone(),
                            });
                        }
                        return Ok(());
                    } else {
                        // Cannot return reference to local variable
                        return Err(SemanticError::InvalidReturn {
                            span: ret_stmt.span.clone(),
                        });
                    }
                } else if let Some(global) = self.symbol_table.get_global(name) {
                    // Check const-correctness for globals
                    if !return_is_const && global.is_const {
                        return Err(SemanticError::InvalidReturn {
                            span: ret_stmt.span.clone(),
                        });
                    }
                    return Ok(());
                }

                Err(SemanticError::InvalidReturn {
                    span: ret_stmt.span.clone(),
                })
            }

            Expr::Postfix(_, PostfixOp::Index(_), _) => {
                // For indexed access, check the base expression
                if let Expr::Postfix(base, _, _) = value {
                    if let Some(ctx) = self.symbol_table.get_expr_context(base) {
                        // Validate const-correctness
                        if !return_is_const && ctx.is_const() {
                            return Err(SemanticError::InvalidReturn {
                                span: ret_stmt.span.clone(),
                            });
                        }
                    }
                }
                Ok(())
            }

            _ => Err(SemanticError::InvalidReturn {
                span: ret_stmt.span.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::type_registry::TypeRegistry;

    use std::sync::{Arc, RwLock};

    fn create_analyzer() -> SemanticAnalyzer {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));
        SemanticAnalyzer::new(registry)
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
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
        }
    }

    fn int_literal(value: i32) -> Expr {
        Expr::Literal(Literal::Number(value.to_string()), None)
    }

    fn bool_literal(value: bool) -> Expr {
        Expr::Literal(Literal::Bool(value), None)
    }

    fn float_literal(value: f32) -> Expr {
        Expr::Literal(Literal::Number(format!("{}f", value)), None)
    }

    fn string_literal(value: &str) -> Expr {
        Expr::Literal(Literal::String(value.to_string()), None)
    }

    fn null_literal() -> Expr {
        Expr::Literal(Literal::Null, None)
    }

    fn var_expr(name: &str) -> Expr {
        Expr::VarAccess(
            Scope {
                is_global: false,
                path: vec![],
            },
            name.to_string(),
            None,
        )
    }

    fn binary_expr(left: Expr, op: BinaryOp, right: Expr) -> Expr {
        Expr::Binary(Box::new(left), op, Box::new(right), None)
    }

    fn unary_expr(op: UnaryOp, operand: Expr) -> Expr {
        Expr::Unary(op, Box::new(operand), None)
    }

    fn ternary_expr(cond: Expr, then_expr: Expr, else_expr: Expr) -> Expr {
        Expr::Ternary(
            Box::new(cond),
            Box::new(then_expr),
            Box::new(else_expr),
            None,
        )
    }

    fn func_call(name: &str, args: Vec<Expr>) -> Expr {
        Expr::FuncCall(
            FuncCall {
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
                        span: None,
                    })
                    .collect(),
                span: None,
            },
            None,
        )
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
                        span: None,
                    })
                    .collect(),
                span: None,
            }),
            None,
        )
    }

    fn member_access(obj: Expr, member: &str) -> Expr {
        Expr::Postfix(
            Box::new(obj),
            PostfixOp::MemberAccess(member.to_string()),
            None,
        )
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
            span: None,
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
            span: None,
        }
    }

    fn param(name: &str, param_type: Type) -> Param {
        Param {
            param_type,
            type_mod: None,
            name: Some(name.to_string()),
            default_value: None,
            is_variadic: false,
            span: None,
        }
    }

    fn param_with_mod(name: &str, param_type: Type, type_mod: TypeMod) -> Param {
        Param {
            param_type,
            type_mod: Some(type_mod),
            name: Some(name.to_string()),
            default_value: None,
            is_variadic: false,
            span: None,
        }
    }

    fn var_decl(var_type: Type, name: &str, init: Option<Expr>) -> Var {
        Var {
            visibility: None,
            var_type,
            declarations: vec![VarDecl {
                name: name.to_string(),
                initializer: init.map(VarInit::Expr),
                span: None,
            }],
            span: None,
        }
    }

    fn return_stmt(value: Option<Expr>) -> Statement {
        Statement::Return(ReturnStmt { value, span: None })
    }

    fn expr_stmt(expr: Expr) -> Statement {
        Statement::Expr(Some(expr))
    }

    fn var_stmt(var: Var) -> Statement {
        Statement::Var(var)
    }

    fn while_stmt(condition: Expr, body: Statement) -> Statement {
        Statement::While(WhileStmt {
            condition,
            body: Box::new(body),
            span: None,
        })
    }

    fn block(statements: Vec<Statement>) -> StatBlock {
        StatBlock {
            statements,
            span: None,
        }
    }

    #[test]
    fn test_literal_int() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(int_literal(42)))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let expr = int_literal(42);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = bool_literal(true);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_BOOL);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = float_literal(3.14);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_FLOAT);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = string_literal("hello");
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_STRING);
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let expr = var_expr("x");
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
        assert!(ctx.is_lvalue());
        assert!(!ctx.is_temporary());
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = var_expr("x");
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert!(ctx.is_const());
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Add, int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
        assert!(!ctx.is_lvalue());
        assert!(ctx.is_temporary());
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Add, float_literal(2.0));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_FLOAT);
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Eq, int_literal(1));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_BOOL);
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = unary_expr(UnaryOp::Neg, int_literal(42));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - incrementing const");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. })),
            "Expected ConstViolation, got: {:?}",
            errors
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
            span: None,
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. })),
            "Expected ConstViolation, got: {:?}",
            errors
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
            span: None,
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
    fn test_function_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_class_declaration() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "MyClass".to_string(),
                extends: vec![],
                members: vec![],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let type_id = analyzer.symbol_table.lookup_type("MyClass").unwrap();
        let type_info = analyzer.symbol_table.get_type(type_id).unwrap();
        assert!(type_info.get_property("value").is_some());
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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("MyClass::method");
        assert!(func.is_some());
        assert_eq!(func.unwrap().kind, FunctionKind::Method { is_const: false });
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
                    span: None,
                }),
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "Derived".to_string(),
                    extends: vec!["Base".to_string()],
                    members: vec![ClassMember::Var(var_decl(int_type(), "derivedValue", None))],
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let derived_id = analyzer.symbol_table.lookup_type("Derived").unwrap();
        let derived_info = analyzer.symbol_table.get_type(derived_id).unwrap();
        assert!(derived_info.base_type.is_some());

        let base_id = analyzer.symbol_table.lookup_type("Base").unwrap();
        assert_eq!(derived_info.base_type.unwrap(), base_id);
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
                    span: None,
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
            span: None,
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
                    span: None,
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
            span: None,
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
                    span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_break_outside_loop() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![Statement::Break(None)]),
            ))],
            span: None,
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
                block(vec![Statement::Continue(None)]),
            ))],
            span: None,
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
                block(vec![while_stmt(bool_literal(true), Statement::Break(None))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Value types can use 'out' references");
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
            span: None,
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(1), BinaryOp::Lt, int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_BOOL);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(bool_literal(true), BinaryOp::And, bool_literal(false));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_BOOL);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = binary_expr(int_literal(0xFF), BinaryOp::BitAnd, int_literal(0x0F));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = unary_expr(UnaryOp::Not, bool_literal(true));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_BOOL);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = ternary_expr(bool_literal(true), int_literal(1), int_literal(2));
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT32);
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
            span: None,
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
            span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("modify");
        assert!(func.is_some());
        assert!(
            func.unwrap().parameters[0]
                .flags
                .contains(ParameterFlags::OUT)
        );
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
                        None,
                        vec![param("v", int_type())],
                        block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "value"),
                            BinaryOp::Assign,
                            var_expr("v"),
                        ))]),
                    )),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("Outer::Inner::test");
        assert!(func.is_some());
    }

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
                        expr_stmt(var_expr("outer")),
                        expr_stmt(var_expr("inner")),
                    ])),
                ]),
            ))],
            span: None,
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
                    Statement::Block(block(vec![var_stmt(var_decl(
                        int_type(),
                        "x",
                        Some(int_literal(2)),
                    ))])),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                    span: None,
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
            span: None,
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
                    span: None,
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                    span: None,
                })],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let type_id = analyzer.symbol_table.lookup_type("Base").unwrap();
        let type_info = analyzer.symbol_table.get_type(type_id).unwrap();
        assert_eq!(type_info.vtable.len(), 1);
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
                        span: None,
                    })],
                    span: None,
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
                        span: None,
                    })],
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let derived_id = analyzer.symbol_table.lookup_type("Derived").unwrap();
        let derived_info = analyzer.symbol_table.get_type(derived_id).unwrap();
        assert!(!derived_info.vtable.is_empty());
    }

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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complete_class_with_all_features() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Player".to_string(),
                extends: vec![],
                members: vec![
                    ClassMember::Var(var_decl(int_type(), "health", Some(int_literal(100)))),
                    ClassMember::Var(var_decl(float_type(), "speed", Some(float_literal(5.0)))),
                    ClassMember::Var(var_decl(string_type(), "name", None)),
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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let type_id = analyzer.symbol_table.lookup_type("Player").unwrap();
        let type_info = analyzer.symbol_table.get_type(type_id).unwrap();

        assert_eq!(type_info.properties.len(), 3);
        assert!(type_info.get_property("health").is_some());
        assert!(type_info.get_property("speed").is_some());
        assert!(type_info.get_property("name").is_some());

        assert!(type_info.get_method("takeDamage").is_some());
        assert!(type_info.get_method("isAlive").is_some());
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let global = analyzer.symbol_table.get_global("globalVar");
        assert!(global.is_some());
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
                    span: None,
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
                    span: None,
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
            span: None,
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
                        is_const: true,
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(member_access(
                            var_expr("this"),
                            "value",
                        )))])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let func = analyzer.symbol_table.get_function("MyClass::getValue");
        assert!(func.is_some());
        assert_eq!(func.unwrap().kind, FunctionKind::Method { is_const: true });
    }

    #[test]
    fn test_expression_context_caching() {
        let mut analyzer = create_analyzer();

        let expr1 = int_literal(42);
        let expr2 = int_literal(42);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(int_type()),
                block(vec![return_stmt(Some(expr1.clone()))]),
            ))],
            span: None,
        };

        analyzer.analyze(&script).unwrap();

        let ctx1 = analyzer.symbol_table.get_expr_context(&expr1);
        let ctx2 = analyzer.symbol_table.get_expr_context(&expr2);

        assert!(ctx1.is_some());
        assert!(ctx2.is_some());
        assert_eq!(ctx1.unwrap().get_type(), ctx2.unwrap().get_type());
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
            span: None,
        };

        analyzer.analyze(&script).unwrap();

        let var_ctx = analyzer
            .symbol_table
            .get_expr_context(&var_expr("x"))
            .unwrap();
        assert!(var_ctx.is_lvalue());

        let lit_ctx = analyzer
            .symbol_table
            .get_expr_context(&int_literal(42))
            .unwrap();
        assert!(!lit_ctx.is_lvalue());
    }

    #[test]
    fn test_post_increment() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    var_stmt(var_decl(int_type(), "x", Some(int_literal(0)))),
                    expr_stmt(Expr::Postfix(
                        Box::new(var_expr("x")),
                        PostfixOp::PostInc,
                        None,
                    )),
                ]),
            ))],
            span: None,
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
                    expr_stmt(Expr::Postfix(
                        Box::new(var_expr("x")),
                        PostfixOp::PostInc,
                        None,
                    )),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - incrementing const");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. })),
            "Expected ConstViolation, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_lambda_simple() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: Some(int_type()),
                type_mod: None,
                name: Some("x".to_string()),
                span: None,
            }],
            body: block(vec![return_stmt(Some(binary_expr(
                var_expr("x"),
                BinaryOp::Mul,
                int_literal(2),
            )))]),
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda, None))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_lambda_with_auto_params() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: None,
                type_mod: None,
                name: Some("x".to_string()),
                span: None,
            }],
            body: block(vec![return_stmt(Some(var_expr("x")))]),
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda, None))]),
            ))],
            span: None,
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
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda, None))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let type_id = analyzer.symbol_table.lookup_type("Callback");
        assert!(type_id.is_some());

        let type_info = analyzer.symbol_table.get_type(type_id.unwrap()).unwrap();
        assert_eq!(type_info.kind, TypeKind::Funcdef);
    }

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
                                span: None,
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
                                span: None,
                            },
                        ],
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let get_func = analyzer.symbol_table.get_function("MyClass::get_value");
        assert!(get_func.is_some());
        assert_eq!(
            get_func.unwrap().kind,
            FunctionKind::Method { is_const: true }
        );

        let set_func = analyzer.symbol_table.get_function("MyClass::set_value");
        assert!(set_func.is_some());
        assert_eq!(
            set_func.unwrap().kind,
            FunctionKind::Method { is_const: false }
        );
    }

    #[test]
    fn test_cast_int_to_float() {
        let mut analyzer = create_analyzer();

        let expr = Expr::Cast(float_type(), Box::new(int_literal(42)), None);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(float_type()),
                block(vec![return_stmt(Some(expr.clone()))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_FLOAT);
    }

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
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![expr_stmt(Expr::ConstructCall(
                        class_type("MyClass"),
                        vec![],
                        None,
                    ))]),
                )),
            ],
            span: None,
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
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![expr_stmt(Expr::ConstructCall(
                        class_type("MyClass"),
                        vec![Arg {
                            name: None,
                            value: int_literal(42),
                            span: None,
                        }],
                        None,
                    ))]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

    #[test]
    fn test_init_list_simple() {
        let mut analyzer = create_analyzer();

        let init_list = InitList {
            items: vec![
                InitListItem::Expr(int_literal(1)),
                InitListItem::Expr(int_literal(2)),
                InitListItem::Expr(int_literal(3)),
            ],
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::InitList(init_list))]),
            ))],
            span: None,
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
                    span: None,
                }),
                InitListItem::InitList(InitList {
                    items: vec![
                        InitListItem::Expr(int_literal(3)),
                        InitListItem::Expr(int_literal(4)),
                    ],
                    span: None,
                }),
            ],
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::InitList(init_list))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                            span: None,
                        }]),
                        None,
                    )),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_literal_uint() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(
                    Literal::Number("42u".to_string()),
                    None,
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42u".to_string()), None);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_UINT32);
    }

    #[test]
    fn test_literal_int64() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(
                    Literal::Number("42ll".to_string()),
                    None,
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42ll".to_string()), None);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_INT64);
    }

    #[test]
    fn test_literal_uint64() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(
                    Literal::Number("42ull".to_string()),
                    None,
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("42ull".to_string()), None);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_UINT64);
    }

    #[test]
    fn test_literal_float_no_decimal() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(
                    Literal::Number("2f".to_string()),
                    None,
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("2f".to_string()), None);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_FLOAT);
    }

    #[test]
    fn test_literal_double_default() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Literal(
                    Literal::Number("3.14".to_string()),
                    None,
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let expr = Expr::Literal(Literal::Number("3.14".to_string()), None);
        let ctx = analyzer.symbol_table.get_expr_context(&expr).unwrap();
        assert_eq!(ctx.get_type(), TYPE_DOUBLE);
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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lambda_void_return() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![],
            body: block(vec![expr_stmt(int_literal(42))]),
            span: None,
        };

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(Expr::Lambda(lambda, None))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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
                span: None,
            })],
            span: None,
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
                span: None,
            })],
            span: None,
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
            span: None,
        };

        let expr = Expr::InitList(init_list.clone());

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(expr.clone())]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());

        let ctx = analyzer.symbol_table.get_expr_context(&expr);
        assert!(ctx.is_some());
    }

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
                    span: None,
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
                                span: None,
                            }])),
                            span: None,
                        }],
                        span: None,
                    })]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
    }

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
                        span: None,
                    }),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());
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
                        span: None,
                    }),
                ]),
            ))],
            span: None,
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
                        body: Box::new(Statement::Break(None)),
                        span: None,
                    }),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                                span: None,
                            },
                            Case {
                                pattern: CasePattern::Value(int_literal(2)),
                                statements: vec![expr_stmt(int_literal(20))],
                                span: None,
                            },
                            Case {
                                pattern: CasePattern::Default,
                                statements: vec![expr_stmt(int_literal(30))],
                                span: None,
                            },
                        ],
                        span: None,
                    }),
                ]),
            ))],
            span: None,
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
                        statements: vec![Statement::Break(None)],
                        span: None,
                    }],
                    span: None,
                })]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

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
                    span: None,
                })]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_all_literal_suffixes() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![
                    expr_stmt(Expr::Literal(Literal::Number("42".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("42u".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("42l".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("42ll".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("42ul".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("42ull".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("3.14".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("3.14f".to_string()), None)),
                    expr_stmt(Expr::Literal(Literal::Number("2f".to_string()), None)),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_INT32
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42u".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_UINT32
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42ll".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_INT64
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("42ull".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_UINT64
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("3.14".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_DOUBLE
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("3.14f".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_FLOAT
        );
        assert_eq!(
            analyzer
                .symbol_table
                .get_expr_context(&Expr::Literal(Literal::Number("2f".to_string()), None))
                .unwrap()
                .get_type(),
            TYPE_FLOAT
        );
    }

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
                            span: None,
                        }]),
                        None,
                    )),
                ]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lambda_in_variable() {
        let mut analyzer = create_analyzer();

        let lambda = Lambda {
            params: vec![LambdaParam {
                param_type: Some(int_type()),
                type_mod: None,
                name: Some("x".to_string()),
                span: None,
            }],
            body: block(vec![return_stmt(Some(binary_expr(
                var_expr("x"),
                BinaryOp::Add,
                int_literal(1),
            )))]),
            span: None,
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
                        span: None,
                    },
                    declarations: vec![VarDecl {
                        name: "callback".to_string(),
                        initializer: Some(VarInit::Expr(Expr::Lambda(lambda, None))),
                        span: None,
                    }],
                    span: None,
                })]),
            ))],
            span: None,
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
                                span: None,
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
                                span: None,
                            },
                        ],
                        span: None,
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
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Analysis failed: {:?}", result.err());

        let get_func = analyzer.symbol_table.get_function("Counter::get_count");
        assert!(get_func.is_some());

        let set_func = analyzer.symbol_table.get_function("Counter::set_count");
        assert!(set_func.is_some());

        let inc_func = analyzer.symbol_table.get_function("Counter::increment");
        assert!(inc_func.is_some());
    }

    #[test]
    fn test_registered_global_function() {
        use crate::core::type_registry::{
            FunctionFlags, FunctionImpl, FunctionInfo, FunctionKind, ParameterFlags, ParameterInfo,
        };
        use crate::core::types::allocate_function_id;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let func_info = FunctionInfo {
                function_id: allocate_function_id(),
                name: "print".to_string(),
                full_name: "print".to_string(),
                namespace: vec![],
                return_type: TYPE_VOID,
                return_is_ref: false,
                return_is_auto_handle: false,
                parameters: vec![ParameterInfo {
                    name: Some("msg".to_string()),
                    type_id: TYPE_STRING,
                    flags: ParameterFlags::IN | ParameterFlags::CONST,
                    is_auto_handle: false,
                    default_expr: None,
                    definition_span: None,
                }],
                kind: FunctionKind::Global,
                flags: FunctionFlags::PUBLIC,
                owner_type: None,
                vtable_index: None,
                implementation: FunctionImpl::Native { system_id: 0 },
                definition_span: None,
                locals: vec![],
                bytecode_address: None,
                local_count: 0,
            };
            reg.register_function(func_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![expr_stmt(func_call(
                    "print",
                    vec![string_literal("Hello")],
                ))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered function");

        let func = analyzer.symbol_table.get_function("print");
        assert!(func.is_some());
    }

    #[test]
    fn test_registered_object_type() {
        use crate::core::type_registry::{PropertyFlags, PropertyInfo};
        use crate::core::types::allocate_type_id;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Enemy".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            let type_id = type_info.type_id;
            reg.register_type(type_info).unwrap();

            reg.add_property(
                type_id,
                PropertyInfo {
                    name: "health".to_string(),
                    type_id: TYPE_INT32,
                    offset: None,
                    access: AccessSpecifier::Public,
                    flags: PropertyFlags::PUBLIC,
                    getter: None,
                    setter: None,
                    definition_span: None,
                },
            )
            .unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should recognize registered type and property"
        );

        let type_id = analyzer.symbol_table.lookup_type("Enemy");
        assert!(type_id.is_some());

        let type_info = analyzer.symbol_table.get_type(type_id.unwrap()).unwrap();
        assert_eq!(type_info.kind, TypeKind::Class);
        assert!(type_info.get_property("health").is_some());
    }

    #[test]
    fn test_registered_object_method() {
        use crate::core::type_registry::{
            FunctionFlags, FunctionImpl, FunctionInfo, FunctionKind, ParameterFlags, ParameterInfo,
        };
        use crate::core::types::allocate_function_id;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Enemy".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            let type_id = type_info.type_id;
            reg.register_type(type_info).unwrap();

            let func_id = allocate_function_id();
            let func_info = FunctionInfo {
                function_id: func_id,
                name: "takeDamage".to_string(),
                full_name: "Enemy::takeDamage".to_string(),
                namespace: vec![],
                return_type: TYPE_VOID,
                return_is_ref: false,
                return_is_auto_handle: false,
                parameters: vec![ParameterInfo {
                    name: Some("amount".to_string()),
                    type_id: TYPE_INT32,
                    flags: ParameterFlags::IN,
                    is_auto_handle: false,
                    default_expr: None,
                    definition_span: None,
                }],
                kind: FunctionKind::Method { is_const: false },
                flags: FunctionFlags::PUBLIC,
                owner_type: Some(type_id),
                vtable_index: None,
                implementation: FunctionImpl::Native { system_id: 0 },
                definition_span: None,
                locals: vec![],
                bytecode_address: None,
                local_count: 0,
            };
            reg.register_function(func_info).unwrap();
            reg.add_method(type_id, "takeDamage".to_string(), func_id)
                .unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered method");

        let method_func = analyzer.symbol_table.get_function("Enemy::takeDamage");
        assert!(method_func.is_some());
    }

    #[test]
    fn test_registered_enum() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Color".to_string(),
                namespace: vec![],
                kind: TypeKind::Enum,
                flags: TypeFlags::ENUM,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

        let script = Script {
            items: vec![ScriptNode::Func(simple_func(
                "test",
                Some(void_type()),
                block(vec![var_stmt(var_decl(class_type("Color"), "c", None))]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered enum");

        let type_id = analyzer.symbol_table.lookup_type("Color");
        assert!(type_id.is_some());

        let type_info = analyzer.symbol_table.get_type(type_id.unwrap()).unwrap();
        assert_eq!(type_info.kind, TypeKind::Enum);
    }

    #[test]
    fn test_registered_funcdef() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Callback".to_string(),
                namespace: vec![],
                kind: TypeKind::Funcdef,
                flags: TypeFlags::FUNCDEF,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
                        span: None,
                    },
                    declarations: vec![VarDecl {
                        name: "cb".to_string(),
                        initializer: None,
                        span: None,
                    }],
                    span: None,
                })]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize registered funcdef");

        let type_id = analyzer.symbol_table.lookup_type("Callback");
        assert!(type_id.is_some());

        let type_info = analyzer.symbol_table.get_type(type_id.unwrap()).unwrap();
        assert_eq!(type_info.kind, TypeKind::Funcdef);
    }

    #[test]
    fn test_registered_global_property() {
        use crate::core::type_registry::GlobalInfo;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let global_info = GlobalInfo {
                name: "g_score".to_string(),
                type_id: TYPE_INT32,
                is_const: false,
                address: 0,
                definition_span: None,
            };
            reg.register_global(global_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
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
        use crate::core::type_registry::PropertyInfo;
        use crate::core::types::allocate_type_id;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Enemy".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            let type_id = type_info.type_id;
            reg.register_type(type_info).unwrap();

            reg.add_property(
                type_id,
                PropertyInfo {
                    name: "health".to_string(),
                    type_id: TYPE_INT32,
                    offset: None,
                    access: AccessSpecifier::Public,
                    flags: PropertyFlags::PUBLIC,
                    getter: None,
                    setter: None,
                    definition_span: None,
                },
            )
            .unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
                    span: None,
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
            span: None,
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
        assert!(enemy_type.is_some());
    }

    #[test]
    fn test_function_overload_exact_match() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", float_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        expr_stmt(func_call("foo", vec![int_literal(42)])),
                        expr_stmt(func_call("foo", vec![float_literal(3.14)])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());
    }

    #[test]
    fn test_function_overload_implicit_conversion() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", float_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        // int can convert to float
                        expr_stmt(func_call("foo", vec![int_literal(42)])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should allow int->float conversion");
    }

    #[test]
    fn test_function_overload_ambiguous() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())], // ❌ Duplicate signature
                    block(vec![]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should not allow ambiguous overload");
    }

    #[test]
    fn test_method_overload_parameter_count() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![
                        ClassMember::Func(simple_func("method", Some(void_type()), block(vec![]))),
                        ClassMember::Func(func_with_params(
                            "method",
                            Some(void_type()),
                            vec![param("x", int_type())],
                            block(vec![]),
                        )),
                        ClassMember::Func(func_with_params(
                            "method",
                            Some(void_type()),
                            vec![param("x", int_type()), param("y", int_type())],
                            block(vec![]),
                        )),
                    ],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        expr_stmt(method_call(var_expr("obj"), "method", vec![])),
                        expr_stmt(method_call(var_expr("obj"), "method", vec![int_literal(1)])),
                        expr_stmt(method_call(
                            var_expr("obj"),
                            "method",
                            vec![int_literal(1), int_literal(2)],
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());
    }

    #[test]
    fn test_virtual_property_getter_usage() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
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
                            accessors: vec![PropertyAccessor {
                                kind: AccessorKind::Get,
                                is_const: true,
                                attributes: vec![],
                                body: Some(block(vec![return_stmt(Some(member_access(
                                    var_expr("this"),
                                    "_value",
                                )))])),
                                span: None,
                            }],
                            span: None,
                        }),
                    ],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        // Accessing obj.value should use getter
                        return_stmt(Some(member_access(var_expr("obj"), "value"))),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());

        // Verify the property access is marked as virtual
        let access_expr = member_access(var_expr("obj"), "value");
        let ctx = analyzer
            .symbol_table
            .get_expr_context(&access_expr)
            .unwrap();
        assert!(
            ctx.is_virtual_property(),
            "Should be marked as virtual property"
        );
        assert!(ctx.get_property_accessors().is_some(), "Should have getter");
    }

    #[test]
    fn test_virtual_property_setter_usage() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
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
                                    span: None,
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
                                    span: None,
                                },
                            ],
                            span: None,
                        }),
                    ],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        // Assigning to obj.value should use setter
                        expr_stmt(binary_expr(
                            member_access(var_expr("obj"), "value"),
                            BinaryOp::Assign,
                            int_literal(42),
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());
    }

    #[test]
    fn test_virtual_property_readonly() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
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
                            span: None,
                        }],
                        span: None,
                    })],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        // Should fail - no setter
                        expr_stmt(binary_expr(
                            member_access(var_expr("obj"), "readonly"),
                            BinaryOp::Assign,
                            int_literal(42),
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - property is read-only");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. }))
        );
    }

    #[test]
    fn test_virtual_property_const_context() {
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
                                span: None,
                            },
                            PropertyAccessor {
                                kind: AccessorKind::Set,
                                is_const: false, // ✅ Non-const setter
                                attributes: vec![],
                                body: Some(block(vec![expr_stmt(binary_expr(
                                    member_access(var_expr("this"), "_value"),
                                    BinaryOp::Assign,
                                    var_expr("value"),
                                ))])),
                                span: None,
                            },
                        ],
                        span: None,
                    }),
                    ClassMember::Func(Func {
                        modifiers: vec![],
                        visibility: None,
                        return_type: Some(int_type()),
                        is_ref: false,
                        name: "constMethod".to_string(),
                        params: vec![],
                        is_const: true,
                        attributes: vec![],
                        body: Some(block(vec![
                            // In const method, can read property
                            return_stmt(Some(member_access(var_expr("this"), "value"))),
                        ])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should allow reading in const method");
    }

    #[test]
    fn test_virtual_property_cannot_write_in_const_method() {
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
                                span: None,
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
                                span: None,
                            },
                        ],
                        span: None,
                    }),
                    ClassMember::Func(Func {
                        modifiers: vec![],
                        visibility: None,
                        return_type: Some(void_type()),
                        is_ref: false,
                        name: "constMethod".to_string(),
                        params: vec![],
                        is_const: true,
                        attributes: vec![],
                        body: Some(block(vec![expr_stmt(binary_expr(
                            member_access(var_expr("this"), "value"),
                            BinaryOp::Assign,
                            int_literal(42),
                        ))])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - can't write in const method");
    }

    #[test]
    fn test_overload_with_typedef() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Typedef(Typedef {
                    prim_type: "int".to_string(),
                    name: "MyInt".to_string(),
                    span: None,
                }),
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(
                            Type {
                                is_const: false,
                                scope: Scope {
                                    is_global: false,
                                    path: vec![],
                                },
                                datatype: DataType::Identifier("MyInt".to_string()),
                                template_types: vec![],
                                modifiers: vec![],
                                span: None,
                            },
                            "x",
                            Some(int_literal(42)),
                        )),
                        // Should match foo(int) even though x is MyInt (typedef of int)
                        expr_stmt(func_call("foo", vec![var_expr("x")])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should resolve through typedef");
    }

    #[test]
    fn test_no_matching_overload() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        // No overload for foo(string)
                        expr_stmt(func_call("foo", vec![string_literal("hello")])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - no matching overload");
    }

    #[test]
    fn test_overload_best_match() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", int_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![param("x", float_type())],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        // Should prefer exact match (int) over conversion (float)
                        expr_stmt(func_call("foo", vec![int_literal(42)])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok());
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
            span: None,
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
        use crate::core::types::allocate_function_id;

        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "RefCounted".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            let type_id = type_info.type_id;
            reg.register_type(type_info).unwrap();

            let construct_id = allocate_function_id();
            reg.add_behaviour(type_id, BehaviourType::Construct, construct_id)
                .unwrap();

            let addref_id = allocate_function_id();
            reg.add_behaviour(type_id, BehaviourType::AddRef, addref_id)
                .unwrap();

            let release_id = allocate_function_id();
            reg.add_behaviour(type_id, BehaviourType::Release, release_id)
                .unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should recognize type with behaviours");
    }

    #[test]
    fn test_ref_type_can_use_inout() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "RefType".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Reference types can use 'inout' references");
    }

    #[test]
    fn test_nohandle_type_cannot_be_handle() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "NoHandleType".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE | TypeFlags::NOHANDLE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "NOHANDLE types cannot be used as handles");
    }

    #[test]
    fn test_noinherit_type_cannot_be_inherited() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "FinalType".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE | TypeFlags::NOINHERIT,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

        let script = Script {
            items: vec![ScriptNode::Class(Class {
                modifiers: vec![],
                name: "Derived".to_string(),
                extends: vec!["FinalType".to_string()],
                members: vec![],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "NOINHERIT types cannot be inherited");
    }

    #[test]
    fn test_abstract_type_cannot_be_instantiated() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "AbstractType".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE | TypeFlags::ABSTRACT,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
            span: None,
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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
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
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "Vector3".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::VALUE_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Application value types cannot use 'inout'"
        );
    }

    #[test]
    fn test_ref_type_with_inout_succeeds() {
        let registry = Arc::new(RwLock::new(TypeRegistry::new()));

        {
            let mut reg = registry.write().unwrap();
            let type_info = TypeInfo {
                type_id: allocate_type_id(),
                name: "RefType".to_string(),
                namespace: vec![],
                kind: TypeKind::Class,
                flags: TypeFlags::REF_TYPE,
                registration: TypeRegistration::Application,
                properties: vec![],
                methods: HashMap::new(),
                base_type: None,
                interfaces: vec![],
                behaviours: HashMap::new(),
                rust_type_id: None,
                vtable: vec![],
                definition_span: None,
            };
            reg.register_type(type_info).unwrap();
        }

        let mut analyzer = SemanticAnalyzer::new(registry);

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
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Reference types can use 'inout' references");
    }

    #[test]
    fn test_virtual_property_mixed_with_regular() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![
                        // Regular property
                        ClassMember::Var(var_decl(int_type(), "regularProp", None)),
                        // Virtual property
                        ClassMember::VirtProp(VirtProp {
                            visibility: None,
                            prop_type: int_type(),
                            is_ref: false,
                            name: "virtualProp".to_string(),
                            accessors: vec![
                                PropertyAccessor {
                                    kind: AccessorKind::Get,
                                    is_const: true,
                                    attributes: vec![],
                                    body: Some(block(vec![return_stmt(Some(int_literal(42)))])),
                                    span: None,
                                },
                                PropertyAccessor {
                                    kind: AccessorKind::Set,
                                    is_const: false,
                                    attributes: vec![],
                                    body: Some(block(vec![])),
                                    span: None,
                                },
                            ],
                            span: None,
                        }),
                    ],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        // Both should work
                        expr_stmt(binary_expr(
                            member_access(var_expr("obj"), "regularProp"),
                            BinaryOp::Assign,
                            int_literal(1),
                        )),
                        expr_stmt(binary_expr(
                            member_access(var_expr("obj"), "virtualProp"),
                            BinaryOp::Assign,
                            int_literal(2),
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());

        // Verify contexts are different
        let regular = member_access(var_expr("obj"), "regularProp");
        let virtual_prop = member_access(var_expr("obj"), "virtualProp");

        let regular_ctx = analyzer.symbol_table.get_expr_context(&regular).unwrap();
        assert!(
            !regular_ctx.is_virtual_property(),
            "Regular property should not be virtual"
        );

        let virtual_ctx = analyzer
            .symbol_table
            .get_expr_context(&virtual_prop)
            .unwrap();
        assert!(
            virtual_ctx.is_virtual_property(),
            "Virtual property should be marked"
        );
        assert!(virtual_ctx.get_property_accessors().is_some());
        assert!(virtual_ctx.get_property_accessors().is_some());
    }

    #[test]
    fn test_virtual_property_writeonly() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::VirtProp(VirtProp {
                        visibility: None,
                        prop_type: int_type(),
                        is_ref: false,
                        name: "writeonly".to_string(),
                        accessors: vec![PropertyAccessor {
                            kind: AccessorKind::Set,
                            is_const: false,
                            attributes: vec![],
                            body: Some(block(vec![])),
                            span: None,
                        }],
                        span: None,
                    })],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(int_type()),
                    block(vec![
                        var_stmt(var_decl(class_type("MyClass"), "obj", None)),
                        return_stmt(Some(member_access(var_expr("obj"), "writeonly"))),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - write-only property");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::ConstViolation { .. }))
        );
    }

    #[test]
    fn test_virtual_property_in_const_object() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::VirtProp(VirtProp {
                        visibility: None,
                        prop_type: int_type(),
                        is_ref: false,
                        name: "value".to_string(),
                        accessors: vec![
                            PropertyAccessor {
                                kind: AccessorKind::Get,
                                is_const: true,
                                attributes: vec![],
                                body: Some(block(vec![return_stmt(Some(int_literal(42)))])),
                                span: None,
                            },
                            PropertyAccessor {
                                kind: AccessorKind::Set,
                                is_const: false,
                                attributes: vec![],
                                body: Some(block(vec![])),
                                span: None,
                            },
                        ],
                        span: None,
                    })],
                    span: None,
                }),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        var_stmt(var_decl(
                            Type {
                                is_const: true,
                                scope: Scope {
                                    is_global: false,
                                    path: vec![],
                                },
                                datatype: DataType::Identifier("MyClass".to_string()),
                                template_types: vec![],
                                modifiers: vec![],
                                span: None,
                            },
                            "obj",
                            None,
                        )),
                        // ✅ Can read (const getter)
                        expr_stmt(member_access(var_expr("obj"), "value")),
                        // ❌ Can't write (non-const setter)
                        expr_stmt(binary_expr(
                            member_access(var_expr("obj"), "value"),
                            BinaryOp::Assign,
                            int_literal(42),
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - can't write to const object");
    }

    #[test]
    fn test_default_argument_basic() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![Param {
                        param_type: int_type(),
                        type_mod: None,
                        name: Some("x".to_string()),
                        default_value: Some(int_literal(42)),
                        is_variadic: false,
                        span: None,
                    }],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        expr_stmt(func_call("foo", vec![int_literal(10)])),
                        expr_stmt(func_call("foo", vec![])),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());
    }

    #[test]
    fn test_default_argument_type_mismatch() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "foo",
                Some(void_type()),
                vec![Param {
                    param_type: int_type(),
                    type_mod: None,
                    name: Some("x".to_string()),
                    default_value: Some(string_literal("wrong")), // ❌ String for int param
                    is_variadic: false,
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - type mismatch in default");

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::TypeMismatch { .. }))
        );
    }

    #[test]
    fn test_default_argument_non_constant() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Var(var_decl(int_type(), "globalVar", Some(int_literal(42)))),
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![Param {
                        param_type: int_type(),
                        type_mod: None,
                        name: Some("x".to_string()),
                        default_value: Some(var_expr("globalVar")), // ❌ Not a constant
                        is_variadic: false,
                        span: None,
                    }],
                    block(vec![]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_err(), "Should fail - default must be constant");
    }

    #[test]
    fn test_default_argument_ordering() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "foo",
                Some(void_type()),
                vec![
                    Param {
                        param_type: int_type(),
                        type_mod: None,
                        name: Some("a".to_string()),
                        default_value: Some(int_literal(1)),
                        is_variadic: false,
                        span: None,
                    },
                    Param {
                        param_type: int_type(),
                        type_mod: None,
                        name: Some("b".to_string()),
                        default_value: None, // ❌ Non-default after default
                        is_variadic: false,
                        span: None,
                    },
                ],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Should fail - non-default param after default"
        );
    }

    #[test]
    fn test_default_argument_multiple() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Func(func_with_params(
                    "foo",
                    Some(void_type()),
                    vec![
                        param("a", int_type()),
                        Param {
                            param_type: int_type(),
                            type_mod: None,
                            name: Some("b".to_string()),
                            default_value: Some(int_literal(10)),
                            is_variadic: false,
                            span: None,
                        },
                        Param {
                            param_type: int_type(),
                            type_mod: None,
                            name: Some("c".to_string()),
                            default_value: Some(int_literal(20)),
                            is_variadic: false,
                            span: None,
                        },
                    ],
                    block(vec![]),
                )),
                ScriptNode::Func(simple_func(
                    "test",
                    Some(void_type()),
                    block(vec![
                        expr_stmt(func_call("foo", vec![int_literal(1)])),
                        expr_stmt(func_call("foo", vec![int_literal(1), int_literal(2)])),
                        expr_stmt(func_call(
                            "foo",
                            vec![int_literal(1), int_literal(2), int_literal(3)],
                        )),
                    ]),
                )),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Failed to analyze: {:?}", result.err());
    }

    #[test]
    fn test_default_argument_with_conversion() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(func_with_params(
                "foo",
                Some(void_type()),
                vec![Param {
                    param_type: float_type(),
                    type_mod: None,
                    name: Some("x".to_string()),
                    default_value: Some(int_literal(42)), // ✅ int->float conversion OK
                    is_variadic: false,
                    span: None,
                }],
                block(vec![]),
            ))],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow implicit conversion in default"
        );
    }

    #[test]
    fn test_return_reference_to_member() {
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
                        is_ref: true, // ✅ Returns reference
                        name: "getValue".to_string(),
                        params: vec![],
                        is_const: false,
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(member_access(
                            var_expr("this"),
                            "value",
                        )))])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should allow returning reference to member");
    }

    #[test]
    fn test_return_reference_to_local() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(Func {
                modifiers: vec![],
                visibility: None,
                return_type: Some(int_type()),
                is_ref: true,
                name: "getRef".to_string(),
                params: vec![],
                is_const: false,
                attributes: vec![],
                body: Some(block(vec![
                    var_stmt(var_decl(int_type(), "local", Some(int_literal(42)))),
                    // ❌ Returning reference to local
                    return_stmt(Some(var_expr("local"))),
                ])),
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Should fail - returning reference to local"
        );

        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SemanticError::InvalidReturn { .. }))
        );
    }

    #[test]
    fn test_return_reference_to_parameter() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                // ✅ Create a class to use as parameter type
                ScriptNode::Class(Class {
                    modifiers: vec![],
                    name: "MyClass".to_string(),
                    extends: vec![],
                    members: vec![ClassMember::Var(var_decl(int_type(), "value", None))],
                    span: None,
                }),
                // ✅ Function that returns reference to parameter
                ScriptNode::Func(Func {
                    modifiers: vec![],
                    visibility: None,
                    return_type: Some(class_type("MyClass")),
                    is_ref: true, // Returns reference
                    name: "identity".to_string(),
                    params: vec![Param {
                        param_type: class_type("MyClass"),
                        type_mod: Some(TypeMod::InOut), // ✅ OK for reference types
                        name: Some("obj".to_string()),
                        default_value: None,
                        is_variadic: false,
                        span: None,
                    }],
                    is_const: false,
                    attributes: vec![],
                    body: Some(block(vec![return_stmt(Some(var_expr("obj")))])),
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow returning reference to parameter: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_return_reference_to_global() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Var(var_decl(int_type(), "globalVar", Some(int_literal(42)))),
                ScriptNode::Func(Func {
                    modifiers: vec![],
                    visibility: None,
                    return_type: Some(int_type()),
                    is_ref: true,
                    name: "getGlobal".to_string(),
                    params: vec![],
                    is_const: false,
                    attributes: vec![],
                    body: Some(block(vec![return_stmt(Some(var_expr("globalVar")))])),
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should allow returning reference to global");
    }

    #[test]
    fn test_return_reference_unsafe_mode() {
        let mut analyzer = create_analyzer();

        // ✅ Enable unsafe references
        analyzer
            .registry
            .write()
            .unwrap()
            .set_property(EngineProperty::AllowUnsafeReferences, 1);

        let script = Script {
            items: vec![ScriptNode::Func(Func {
                modifiers: vec![],
                visibility: None,
                return_type: Some(int_type()),
                is_ref: true,
                name: "getRef".to_string(),
                params: vec![],
                is_const: false,
                attributes: vec![],
                body: Some(block(vec![
                    var_stmt(var_decl(int_type(), "local", Some(int_literal(42)))),
                    // ✅ Allowed in unsafe mode
                    return_stmt(Some(var_expr("local"))),
                ])),
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(result.is_ok(), "Should allow in unsafe mode");
    }

    #[test]
    fn test_const_return_reference_to_member() {
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
                        return_type: Some(const_int_type()),
                        is_ref: true, // const int& getValue()
                        name: "getValue".to_string(),
                        params: vec![],
                        is_const: true, // const method
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(member_access(
                            var_expr("this"),
                            "value",
                        )))])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow const method to return const reference to member: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_const_return_reference_prevents_mutable_from_const_method() {
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
                        return_type: Some(int_type()), // Non-const reference
                        is_ref: true,
                        name: "getValue".to_string(),
                        params: vec![],
                        is_const: true, // const method trying to return non-const ref - ERROR
                        attributes: vec![],
                        body: Some(block(vec![return_stmt(Some(member_access(
                            var_expr("this"),
                            "value",
                        )))])),
                        span: None,
                    }),
                ],
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Should NOT allow const method to return non-const reference"
        );
    }

    #[test]
    fn test_const_return_reference_to_const_global() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Var(var_decl(
                    const_int_type(),
                    "CONSTANT",
                    Some(int_literal(42)),
                )),
                ScriptNode::Func(Func {
                    modifiers: vec![],
                    visibility: None,
                    return_type: Some(const_int_type()),
                    is_ref: true, // const int&
                    name: "getConstant".to_string(),
                    params: vec![],
                    is_const: false,
                    attributes: vec![],
                    body: Some(block(vec![return_stmt(Some(var_expr("CONSTANT")))])),
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow returning const ref to const global: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_const_return_reference_prevents_mutable_from_const_global() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![
                ScriptNode::Var(var_decl(
                    const_int_type(),
                    "CONSTANT",
                    Some(int_literal(42)),
                )),
                ScriptNode::Func(Func {
                    modifiers: vec![],
                    visibility: None,
                    return_type: Some(int_type()), // Non-const reference
                    is_ref: true,
                    name: "getConstant".to_string(),
                    params: vec![],
                    is_const: false,
                    attributes: vec![],
                    body: Some(block(vec![return_stmt(Some(var_expr("CONSTANT")))])),
                    span: None,
                }),
            ],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Should NOT allow returning non-const ref to const global"
        );
    }

    #[test]
    fn test_const_return_reference_to_const_param() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(Func {
                modifiers: vec![],
                visibility: None,
                return_type: Some(const_int_type()),
                is_ref: true, // const int&
                name: "echo".to_string(),
                params: vec![param_with_mod("value", const_int_type(), TypeMod::In)],
                is_const: false,
                attributes: vec![],
                body: Some(block(vec![return_stmt(Some(var_expr("value")))])),
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow returning const ref to const param: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_const_return_reference_prevents_mutable_from_const_param() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(Func {
                modifiers: vec![],
                visibility: None,
                return_type: Some(int_type()), // Non-const reference
                is_ref: true,
                name: "echo".to_string(),
                params: vec![param_with_mod("value", const_int_type(), TypeMod::In)],
                is_const: false,
                attributes: vec![],
                body: Some(block(vec![return_stmt(Some(var_expr("value")))])),
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_err(),
            "Should NOT allow returning non-const ref to const param"
        );
    }

    #[test]
    fn test_mutable_return_reference_allows_mutable_param() {
        let mut analyzer = create_analyzer();

        let script = Script {
            items: vec![ScriptNode::Func(Func {
                modifiers: vec![],
                visibility: None,
                return_type: Some(int_type()), // Mutable reference
                is_ref: true,
                name: "echo".to_string(),
                params: vec![param_with_mod("value", int_type(), TypeMod::InOut)], // Mutable param
                is_const: false,
                attributes: vec![],
                body: Some(block(vec![return_stmt(Some(var_expr("value")))])),
                span: None,
            })],
            span: None,
        };

        let result = analyzer.analyze(&script);
        assert!(
            result.is_ok(),
            "Should allow returning mutable ref to mutable param: {:?}",
            result.err()
        );
    }
}