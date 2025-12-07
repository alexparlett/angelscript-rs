//! Expression type checking.
//!
//! This module contains all `check_*` methods for type-checking expressions
//! and emitting bytecode.

use crate::ast::{
    AssignOp, BinaryOp, PostfixOp, UnaryOp,
    expr::{
        AssignExpr, BinaryExpr, CallExpr, CastExpr, Expr, IdentExpr, IndexExpr, InitListExpr,
        LambdaExpr, LiteralExpr, LiteralKind, MemberAccess, MemberExpr, ParenExpr, PostfixExpr,
        TernaryExpr, UnaryExpr,
    },
};
use crate::codegen::Instruction;
use crate::lexer::Span;
use crate::semantic::{
    Conversion, DataType, MethodSignature, OperatorBehavior,
    SemanticErrorKind, TypeDef, TypeId, BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE,
    INT32_TYPE, NULL_TYPE, VOID_TYPE,
};
use crate::semantic::types::type_def::FunctionId;

use super::{ExprContext, FunctionCompiler};

impl<'ast> FunctionCompiler<'ast> {
    /// Type checks an expression and returns its type.
    ///
    /// Returns None if type checking failed (error already recorded).
    pub(super) fn check_expr(&mut self, expr: &'ast Expr<'ast>) -> Option<ExprContext> {
        match expr {
            Expr::Literal(lit) => self.check_literal(lit),
            Expr::Ident(ident) => self.check_ident(ident),
            Expr::Binary(binary) => self.check_binary(binary),
            Expr::Unary(unary) => self.check_unary(unary),
            Expr::Assign(assign) => self.check_assign(assign),
            Expr::Ternary(ternary) => self.check_ternary(ternary),
            Expr::Call(call) => self.check_call(call),
            Expr::Index(index) => self.check_index(index),
            Expr::Member(member) => self.check_member(member),
            Expr::Postfix(postfix) => self.check_postfix(postfix),
            Expr::Cast(cast) => self.check_cast(cast),
            Expr::Lambda(lambda) => self.check_lambda(lambda),
            Expr::InitList(init_list) => self.check_init_list(init_list),
            Expr::Paren(paren) => self.check_paren(paren),
        }
    }

    /// Type checks a literal expression.
    /// Literals are always rvalues (temporary values).
    pub(super) fn check_literal(&mut self, lit: &LiteralExpr) -> Option<ExprContext> {
        let type_id = match &lit.kind {
            LiteralKind::Int(_) => INT32_TYPE, // Default integer literals to int32 (matches 'int' type)
            LiteralKind::Float(_) => FLOAT_TYPE,
            LiteralKind::Double(_) => DOUBLE_TYPE,
            LiteralKind::Bool(_) => BOOL_TYPE,
            LiteralKind::String(s) => {
                let idx = self.bytecode.add_string_constant(s.clone());
                self.bytecode.emit(Instruction::PushString(idx));
                // String type is an FFI type, look up by name
                let string_type = self.context.lookup_type("string").unwrap_or(VOID_TYPE);
                return Some(ExprContext::rvalue(DataType::simple(string_type)));
            }
            LiteralKind::Null => {
                self.bytecode.emit(Instruction::PushNull);
                return Some(ExprContext::rvalue(DataType::simple(NULL_TYPE)));
            }
        };

        // Emit bytecode for literal
        match &lit.kind {
            LiteralKind::Int(i) => self.bytecode.emit(Instruction::PushInt(*i)),
            LiteralKind::Float(f) => self.bytecode.emit(Instruction::PushFloat(*f)),
            LiteralKind::Double(d) => self.bytecode.emit(Instruction::PushDouble(*d)),
            LiteralKind::Bool(b) => self.bytecode.emit(Instruction::PushBool(*b)),
            _ => unreachable!(),
        };

        Some(ExprContext::rvalue(DataType::simple(type_id)))
    }

    /// Type checks an identifier expression.
    /// Variables are lvalues (mutable unless marked const).
    /// Enum values (EnumName::VALUE) are rvalues (integer constants).
    /// The `this` keyword resolves to the current object in method bodies.
    /// Unqualified identifiers in methods resolve to class members (implicit `this`).
    pub(super) fn check_ident(&mut self, ident: &IdentExpr<'ast>) -> Option<ExprContext> {
        let name = ident.ident.name;

        // Check if this is a scoped identifier (e.g., EnumName::VALUE or Namespace::EnumName::VALUE)
        if let Some(scope) = ident.scope {
            // Build the qualified type name from scope segments (no intermediate Vec)
            let type_name = Self::build_scope_name(&scope);

            // Try to look up as an enum type - first with the given name, then with namespace prefix,
            // then in imported namespaces
            let type_id = self.context.lookup_type(&type_name).or_else(|| {
                // If not found and we're in a namespace, try with namespace prefix
                if !self.namespace_path.is_empty() {
                    let qualified_type_name = Self::build_qualified_name_from_path(&self.namespace_path, &type_name);
                    if let Some(id) = self.context.lookup_type(&qualified_type_name) {
                        return Some(id);
                    }
                }
                // Try imported namespaces
                for ns in &self.imported_namespaces {
                    let imported_qualified = format!("{}::{}", ns, type_name);
                    if let Some(id) = self.context.lookup_type(&imported_qualified) {
                        return Some(id);
                    }
                }
                None
            });

            if let Some(type_id) = type_id {
                let typedef = self.context.get_type(type_id);
                if typedef.is_enum() {
                    // Look up the enum value
                    if let Some(value) = self.context.lookup_enum_value(type_id, name) {
                        // Emit instruction to push the enum value as an integer constant
                        self.bytecode.emit(Instruction::PushInt(value));
                        // Enum values are rvalues of the enum type (implicitly convertible to int)
                        return Some(ExprContext::rvalue(DataType::simple(type_id)));
                    } else {
                        // Enum exists but value doesn't
                        self.error(
                            SemanticErrorKind::UndefinedVariable,
                            ident.span,
                            format!("enum '{}' has no value named '{}'", type_name, name),
                        );
                        return None;
                    }
                }
            }

            // Not an enum - try namespace-qualified global variable
            let qualified_name = format!("{}::{}", type_name, name);
            if let Some(global_var) = self.context.lookup_global_var(&qualified_name) {
                // Emit load global instruction (using string constant for qualified name)
                let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
                self.bytecode.emit(Instruction::LoadGlobal(name_idx));
                let is_mutable = !global_var.data_type.is_const;
                return Some(ExprContext::lvalue(global_var.data_type.clone(), is_mutable));
            }

            // Not found as enum or global variable
            self.error(
                SemanticErrorKind::UndefinedVariable,
                ident.span,
                format!("undefined identifier '{}::{}'", type_name, name),
            );
            return None;
        }

        // Check for explicit 'this' keyword
        if name == "this" {
            let class_id = match self.current_class {
                Some(id) => id,
                None => {
                    self.error(
                        SemanticErrorKind::UndefinedVariable,
                        ident.span,
                        "'this' can only be used in class methods",
                    );
                    return None;
                }
            };
            self.bytecode.emit(Instruction::LoadThis);
            // 'this' is an lvalue (you can access members on it, but can't reassign it)
            // The object itself is mutable (you can modify fields through it)
            return Some(ExprContext::lvalue(DataType::simple(class_id), true));
        }

        // Check local variables first (locals shadow class members)
        if let Some(local_var) = self.local_scope.lookup(name) {
            let offset = local_var.stack_offset;
            self.bytecode.emit(Instruction::LoadLocal(offset));
            let is_mutable = !local_var.data_type.is_const;
            return Some(ExprContext::lvalue(local_var.data_type.clone(), is_mutable));
        }

        // Check for implicit class member access (when inside a method)
        if let Some(class_id) = self.current_class
            && let Some(result) = self.try_implicit_member_access(class_id, name, ident.span) {
                return Some(result);
            }

        // Check global variables in registry
        // First try the unqualified name (for global scope variables)
        if let Some(global_var) = self.context.lookup_global_var(name) {
            // Emit load global instruction (using string constant for name)
            let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
            self.bytecode.emit(Instruction::LoadGlobal(name_idx));
            let is_mutable = !global_var.data_type.is_const;
            return Some(ExprContext::lvalue(global_var.data_type.clone(), is_mutable));
        }

        // If we're inside a namespace, try looking up with the namespace-qualified name
        // This allows code in `namespace Foo` to reference `Foo::PI` as just `PI`
        if !self.namespace_path.is_empty() {
            let qualified_name = Self::build_qualified_name_from_path(&self.namespace_path, name);
            if let Some(global_var) = self.context.lookup_global_var(&qualified_name) {
                let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
                self.bytecode.emit(Instruction::LoadGlobal(name_idx));
                let is_mutable = !global_var.data_type.is_const;
                return Some(ExprContext::lvalue(global_var.data_type.clone(), is_mutable));
            }
        }

        // Try to look up as an enum value in imported namespaces (e.g., `using namespace Color; Red;`)
        for ns in &self.imported_namespaces {
            // Look up the enum type in the imported namespace
            let qualified_enum = format!("{}::{}", ns, name);

            // First, check if this is a global variable in the imported namespace
            if let Some(global_var) = self.context.lookup_global_var(&qualified_enum) {
                let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
                self.bytecode.emit(Instruction::LoadGlobal(name_idx));
                let is_mutable = !global_var.data_type.is_const;
                return Some(ExprContext::lvalue(global_var.data_type.clone(), is_mutable));
            }

            // Check if this is an enum value by looking for it in all enum types in the namespace
            // We need to search all enum types since the name might be an unscoped enum value like `Red`
            // First, collect enum types from this namespace
            for (type_name, &type_id) in self.context.type_by_name() {
                if type_name.starts_with(ns) && type_name.starts_with(&format!("{}::", ns)) {
                    let typedef = self.context.get_type(type_id);
                    if typedef.is_enum()
                        && let Some(value) = self.context.lookup_enum_value(type_id, name) {
                            self.bytecode.emit(Instruction::PushInt(value));
                            return Some(ExprContext::rvalue(DataType::simple(type_id)));
                        }
                }
            }
        }

        // Not found in locals or globals
        self.error(
            SemanticErrorKind::UndefinedVariable,
            ident.span,
            format!("variable '{}' is not defined", name),
        );
        None
    }

    /// Try to resolve an identifier as an implicit class member access.
    /// This implements the implicit `this.member` semantics for unqualified identifiers
    /// inside method bodies.
    ///
    /// Returns Some(ExprContext) if the name matches a field or property,
    /// None otherwise (no error reported - caller should continue with other lookups).
    pub(super) fn try_implicit_member_access(
        &mut self,
        class_id: TypeId,
        name: &str,
        span: Span,
    ) -> Option<ExprContext> {
        let class_def = self.context.get_type(class_id);

        match class_def {
            TypeDef::Class { fields, properties, .. } => {
                // Check properties (getter access)
                if let Some(accessors) = properties.get(name)
                    && let Some(getter_id) = accessors.getter {
                        let getter = self.context.get_function(getter_id);
                        let return_type = getter.return_type().clone();

                        // Emit LoadThis followed by CallMethod for the getter
                        self.bytecode.emit(Instruction::LoadThis);
                        self.bytecode.emit(Instruction::CallMethod(getter_id.as_u32()));

                        // Properties accessed via getter are rvalues (unless there's also a setter)
                        // If there's a setter, we could make it an lvalue, but for simplicity
                        // we return rvalue here - assignment will use check_member for the setter
                        return Some(ExprContext::rvalue(return_type));
                    }

                // Check fields first
                for (field_idx, field) in fields.iter().enumerate() {
                    if field.name == name {
                        // Emit LoadThis followed by LoadField
                        self.bytecode.emit(Instruction::LoadThis);
                        self.bytecode.emit(Instruction::LoadField(field_idx as u32));
                        let is_mutable = !field.data_type.is_const;
                        return Some(ExprContext::lvalue(field.data_type.clone(), is_mutable));
                    }
                }

                // Also check base class for inherited members
                if let TypeDef::Class { base_class: Some(base_id), .. } = class_def {
                    // Recursively check base class
                    return self.try_implicit_member_access(*base_id, name, span);
                }

                None
            }
            _ => None,
        }
    }

    /// Type checks a binary expression.
    /// Binary expressions always produce rvalues (temporary results).
    pub(super) fn check_binary(&mut self, binary: &BinaryExpr<'ast>) -> Option<ExprContext> {
        let left_ctx = self.check_expr(binary.left)?;
        let right_ctx = self.check_expr(binary.right)?;

        // Try operator overloading first (for binary arithmetic/bitwise operators)
        let result_type = match binary.op {
            // Arithmetic operators with overloading support
            BinaryOp::Add => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpAdd,
                    OperatorBehavior::OpAddR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Sub => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpSub,
                    OperatorBehavior::OpSubR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Mul => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpMul,
                    OperatorBehavior::OpMulR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Div => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpDiv,
                    OperatorBehavior::OpDivR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Mod => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpMod,
                    OperatorBehavior::OpModR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Pow => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpPow,
                    OperatorBehavior::OpPowR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Bitwise operators with overloading support
            BinaryOp::BitwiseAnd => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpAnd,
                    OperatorBehavior::OpAndR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::BitwiseOr => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpOr,
                    OperatorBehavior::OpOrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::BitwiseXor => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpXor,
                    OperatorBehavior::OpXorR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftLeft => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpShl,
                    OperatorBehavior::OpShlR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftRight => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpShr,
                    OperatorBehavior::OpShrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftRightUnsigned => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpUShr,
                    OperatorBehavior::OpUShrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Comparison operators - try opEquals for ==, !=
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Try opEquals first (returns bool)
                if let Some(func_id) = self.context.find_operator_method(left_ctx.data_type.type_id, OperatorBehavior::OpEquals) {
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    // For !=, negate the result
                    if binary.op == BinaryOp::NotEqual {
                        self.bytecode.emit(Instruction::Not);
                    }
                    return Some(ExprContext::rvalue(DataType::simple(BOOL_TYPE)));
                }
                // Fall back to primitive comparison
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Relational operators - try opCmp for <, <=, >, >=
            BinaryOp::Less | BinaryOp::LessEqual
            | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                // Try opCmp first (returns int: negative/zero/positive)
                if let Some(func_id) = self.context.find_operator_method(left_ctx.data_type.type_id, OperatorBehavior::OpCmp) {
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    // Compare result with zero based on operator
                    self.bytecode.emit(Instruction::PushInt(0));
                    let cmp_instr = match binary.op {
                        BinaryOp::Less => Instruction::LessThan,          // opCmp() < 0
                        BinaryOp::LessEqual => Instruction::LessEqual,     // opCmp() <= 0
                        BinaryOp::Greater => Instruction::GreaterThan,     // opCmp() > 0
                        BinaryOp::GreaterEqual => Instruction::GreaterEqual, // opCmp() >= 0
                        _ => unreachable!(),
                    };
                    self.bytecode.emit(cmp_instr);
                    return Some(ExprContext::rvalue(DataType::simple(BOOL_TYPE)));
                }
                // Fall back to primitive comparison
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Logical operators (no overloading in AngelScript)
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => {
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Handle identity comparison operators
            BinaryOp::Is | BinaryOp::NotIs => {
                // Both operands must be handles or null
                let left_is_handle = left_ctx.data_type.is_handle || left_ctx.data_type.type_id == NULL_TYPE;
                let right_is_handle = right_ctx.data_type.is_handle || right_ctx.data_type.type_id == NULL_TYPE;

                if !left_is_handle {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        binary.span,
                        "left operand of 'is' must be a handle type",
                    );
                    return None;
                }
                if !right_is_handle {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        binary.span,
                        "right operand of 'is' must be a handle type",
                    );
                    return None;
                }

                // Emit pointer equality comparison
                let instr = if binary.op == BinaryOp::Is {
                    Instruction::Equal
                } else {
                    Instruction::NotEqual
                };
                self.bytecode.emit(instr);
                return Some(ExprContext::rvalue(DataType::simple(BOOL_TYPE)));
            }
        };

        // If operator overload was used, we already returned above
        // Otherwise, emit primitive bytecode instruction
        let instr = match binary.op {
            BinaryOp::Add => Instruction::Add,
            BinaryOp::Sub => Instruction::Sub,
            BinaryOp::Mul => Instruction::Mul,
            BinaryOp::Div => Instruction::Div,
            BinaryOp::Mod => Instruction::Mod,
            BinaryOp::Pow => Instruction::Pow,
            BinaryOp::BitwiseAnd => Instruction::BitAnd,
            BinaryOp::BitwiseOr => Instruction::BitOr,
            BinaryOp::BitwiseXor => Instruction::BitXor,
            BinaryOp::ShiftLeft => Instruction::ShiftLeft,
            BinaryOp::ShiftRight => Instruction::ShiftRight,
            BinaryOp::ShiftRightUnsigned => Instruction::ShiftRightUnsigned,
            BinaryOp::LogicalAnd => Instruction::LogicalAnd,
            BinaryOp::LogicalOr => Instruction::LogicalOr,
            BinaryOp::LogicalXor => Instruction::LogicalXor,
            BinaryOp::Equal => Instruction::Equal,
            BinaryOp::NotEqual => Instruction::NotEqual,
            BinaryOp::Less => Instruction::LessThan,
            BinaryOp::LessEqual => Instruction::LessEqual,
            BinaryOp::Greater => Instruction::GreaterThan,
            BinaryOp::GreaterEqual => Instruction::GreaterEqual,
            BinaryOp::Is | BinaryOp::NotIs => {
                // Already handled above with early return
                unreachable!("is/!is operators return early")
            }
        };

        self.bytecode.emit(instr);
        Some(ExprContext::rvalue(result_type))
    }

    /// Checks if a binary operation is valid and returns the result type.
    pub(super) fn check_binary_op(
        &mut self,
        op: BinaryOp,
        left: &DataType,
        right: &DataType,
        span: Span,
    ) -> Option<DataType> {
        // Void type cannot be used in binary operations
        if left.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                span,
                "cannot use void expression as left operand",
            );
            return None;
        }
        if right.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                span,
                "cannot use void expression as right operand",
            );
            return None;
        }

        // For simplicity, we'll use basic type rules
        // In a complete implementation, this would be more sophisticated

        match op {
            // Arithmetic operators: require numeric types
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => {
                if self.is_numeric(left) && self.is_numeric(right) {
                    // Result is the "larger" type
                    Some(self.promote_numeric(left, right))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires numeric operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Bitwise operators: require integer types (bool is implicitly converted to int)
            BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::ShiftRightUnsigned => {
                if self.is_bitwise_compatible(left) && self.is_bitwise_compatible(right) {
                    // If either operand is bool, result is int32; otherwise promote
                    if left.type_id == BOOL_TYPE || right.type_id == BOOL_TYPE {
                        Some(DataType::simple(INT32_TYPE))
                    } else {
                        Some(self.promote_numeric(left, right))
                    }
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires integer operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Logical operators: require bool types
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => {
                if left.type_id == BOOL_TYPE && right.type_id == BOOL_TYPE {
                    Some(DataType::simple(BOOL_TYPE))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires bool operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Comparison operators: result is bool
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                // Allow comparison of compatible types
                Some(DataType::simple(BOOL_TYPE))
            }

            // Type comparison
            BinaryOp::Is | BinaryOp::NotIs => Some(DataType::simple(BOOL_TYPE)),
        }
    }

    /// Type checks a unary expression.
    /// Most unary operations produce rvalues, but ++x/--x preserve lvalue-ness.
    pub(super) fn check_unary(&mut self, unary: &UnaryExpr<'ast>) -> Option<ExprContext> {
        // Special case: @ operator on function name to create function handle
        // This must be handled before check_expr because function names aren't variables
        if unary.op == UnaryOp::HandleOf
            && let Expr::Ident(ident) = unary.operand {
                // Check if this identifier is a function name (not a variable)
                let name = ident.ident.name;

                // Build qualified name if scoped (no intermediate Vec)
                let qualified_name = if let Some(scope) = ident.scope {
                    let scope_name = Self::build_scope_name(&scope);
                    let mut result = String::with_capacity(scope_name.len() + 2 + name.len());
                    result.push_str(&scope_name);
                    result.push_str("::");
                    result.push_str(name);
                    result
                } else if !self.namespace_path.is_empty() {
                    // Try with current namespace first
                    Self::build_qualified_name_from_path(&self.namespace_path, name)
                } else {
                    name.to_string()
                };

                // Check if there's an expected funcdef type for validation
                if let Some(funcdef_type_id) = self.expected_funcdef_type {
                    // Try to find a compatible function
                    if let Some(func_id) = self.context.find_compatible_function(&qualified_name, funcdef_type_id) {
                        // Emit FuncPtr instruction
                        self.bytecode.emit(Instruction::FuncPtr(func_id.as_u32()));
                        // Return funcdef handle type
                        return Some(ExprContext::rvalue(DataType::with_handle(funcdef_type_id, false)));
                    }

                    // Try without namespace if that failed
                    if !self.namespace_path.is_empty()
                        && let Some(func_id) = self.context.find_compatible_function(name, funcdef_type_id) {
                            self.bytecode.emit(Instruction::FuncPtr(func_id.as_u32()));
                            return Some(ExprContext::rvalue(DataType::with_handle(funcdef_type_id, false)));
                        }

                    // Function not found or not compatible
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        unary.span,
                        format!("no function '{}' compatible with funcdef type", name),
                    );
                    return None;
                }

                // No expected funcdef type - check if it's a function and error appropriately
                if !self.context.lookup_functions(&qualified_name).is_empty()
                    || !self.context.lookup_functions(name).is_empty()
                {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        unary.span,
                        "cannot infer function handle type - explicit funcdef context required",
                    );
                    return None;
                }

                // Not a function, fall through to normal handling (will try as variable)
            }

        let operand_ctx = self.check_expr(unary.operand)?;

        // Void type cannot be used in unary operations
        if operand_ctx.data_type.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                unary.span,
                "cannot use void expression as operand",
            );
            return None;
        }

        match unary.op {
            UnaryOp::Neg => {
                // Try opNeg operator overload first
                if let Some(result_type) = self.try_unary_operator_overload(
                    OperatorBehavior::OpNeg,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                // Fall back to primitive negation
                if self.is_numeric(&operand_ctx.data_type) {
                    self.bytecode.emit(Instruction::Negate);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '-' requires numeric operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::LogicalNot => {
                // No operator overloading for logical NOT in AngelScript
                if operand_ctx.data_type.type_id == BOOL_TYPE {
                    self.bytecode.emit(Instruction::Not);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '!' requires bool operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::BitwiseNot => {
                // Try opCom operator overload first
                if let Some(result_type) = self.try_unary_operator_overload(
                    OperatorBehavior::OpCom,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                // Fall back to primitive bitwise NOT
                if self.is_integer(&operand_ctx.data_type) {
                    self.bytecode.emit(Instruction::BitNot);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '~' requires integer operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::Plus => {
                // No operator overloading for unary + in AngelScript
                // Unary + is a no-op for numeric types, produces rvalue
                if self.is_numeric(&operand_ctx.data_type) {
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '+' requires numeric operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::PreInc | UnaryOp::PreDec => {
                // Try opPreInc/opPreDec operator overload first
                let operator = if unary.op == UnaryOp::PreInc {
                    OperatorBehavior::OpPreInc
                } else {
                    OperatorBehavior::OpPreDec
                };

                if let Some(result_type) = self.try_unary_operator_overload(
                    operator,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    // Operator overloads for ++/-- return new value, but still need lvalue check
                    if !operand_ctx.is_lvalue {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            unary.span,
                            format!("{} requires an lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                        );
                        return None;
                    }
                    if !operand_ctx.is_mutable {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            unary.span,
                            format!("{} requires a mutable lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                        );
                        return None;
                    }
                    // Overloaded operators return rvalue of their return type
                    return Some(ExprContext::rvalue(result_type));
                }

                // Fall back to primitive pre-increment/decrement
                // Pre-increment/decrement require mutable lvalue and return lvalue
                if !operand_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!("{} requires an lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                    );
                    return None;
                }
                if !operand_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!("{} requires a mutable lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                    );
                    return None;
                }

                let instr = if unary.op == UnaryOp::PreInc {
                    Instruction::PreIncrement
                } else {
                    Instruction::PreDecrement
                };
                self.bytecode.emit(instr);

                // Returns lvalue with same mutability
                Some(operand_ctx)
            }

            UnaryOp::HandleOf => {
                // @ operator - handle reference, produces rvalue
                // This converts a value to a handle type
                let mut handle_type = operand_ctx.data_type.clone();
                handle_type.is_handle = true;
                Some(ExprContext::rvalue(handle_type))
            }
        }
    }

    /// Type checks an assignment expression.
    /// Assignments require a mutable lvalue as target and produce an rvalue.
    pub(super) fn check_assign(&mut self, assign: &AssignExpr<'ast>) -> Option<ExprContext> {
        use AssignOp::*;

        match assign.op {
            Assign => {
                // Special handling for index expressions: obj[idx] = value
                // Try set_opIndex accessor if opIndex doesn't exist
                if let Expr::Index(index_expr) = assign.target
                    && let Some(result) = self.check_index_assignment(index_expr, assign.value, assign.span) {
                        return Some(result);
                    }
                    // If check_index_assignment returns None, fall through to regular assignment
                    // (this shouldn't happen as check_index_assignment handles all cases)

                // Special handling for member access: obj.prop = value
                // Check for property setter (set_X pattern)
                if let Expr::Member(member_expr) = assign.target
                    && let MemberAccess::Field(field_name) = &member_expr.member
                        && let Some(result) = self.check_member_property_assignment(member_expr, field_name.name, assign.value, assign.span) {
                            return Some(result);
                        }
                        // If returns None, property doesn't exist - fall through to regular assignment

                // Special handling for handle assignment: @handle_var = value
                // In AngelScript, @var on the LHS means "assign to the handle variable"
                if let Expr::Unary(unary) = assign.target
                    && unary.op == UnaryOp::HandleOf {
                        // This is a handle assignment - get the underlying lvalue
                        let operand_ctx = self.check_expr(unary.operand)?;

                        // The underlying operand must be an lvalue and a handle type
                        if !operand_ctx.is_lvalue {
                            self.error(
                                SemanticErrorKind::InvalidOperation,
                                unary.operand.span(),
                                "handle assignment target must be an lvalue",
                            );
                            return None;
                        }

                        if !operand_ctx.data_type.is_handle {
                            self.error(
                                SemanticErrorKind::InvalidOperation,
                                unary.operand.span(),
                                "handle assignment target must be a handle type",
                            );
                            return None;
                        }

                        if !operand_ctx.is_mutable {
                            self.error(
                                SemanticErrorKind::InvalidOperation,
                                unary.operand.span(),
                                "cannot assign to a const handle",
                            );
                            return None;
                        }

                        // Check if target is a funcdef handle (for function reference assignment)
                        let is_funcdef_target = matches!(
                            self.context.get_type(operand_ctx.data_type.type_id),
                            TypeDef::Funcdef { .. }
                        );

                        if is_funcdef_target {
                            self.expected_funcdef_type = Some(operand_ctx.data_type.type_id);
                        }

                        let value_ctx = self.check_expr(assign.value)?;

                        self.expected_funcdef_type = None;

                        // Check type compatibility
                        // For handle assignment, the value must be convertible to the handle type
                        if let Some(conversion) = value_ctx.data_type.can_convert_to(&operand_ctx.data_type, self.context) {
                            self.emit_conversion(&conversion);
                        } else if value_ctx.data_type.type_id != operand_ctx.data_type.type_id {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                assign.span,
                                format!(
                                    "cannot assign '{}' to handle of type '{}'",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(&operand_ctx.data_type)
                                ),
                            );
                            return None;
                        }

                        // Emit store instruction for the handle
                        // The bytecode emitter should have already emitted code to load the target address
                        // and the value - we just need to emit a store
                        self.bytecode.emit(Instruction::StoreHandle);

                        return Some(ExprContext::rvalue(operand_ctx.data_type.clone()));
                    }

                // Simple assignment: target = value
                let target_ctx = self.check_expr(assign.target)?;

                // Check if target is a funcdef handle (for function reference assignment)
                let is_funcdef_target = target_ctx.data_type.is_handle
                    && matches!(
                        self.context.get_type(target_ctx.data_type.type_id),
                        TypeDef::Funcdef { .. }
                    );

                // Set expected funcdef type for RHS evaluation
                if is_funcdef_target {
                    self.expected_funcdef_type = Some(target_ctx.data_type.type_id);
                }

                let value_ctx = self.check_expr(assign.value)?;

                // Clear expected funcdef type
                self.expected_funcdef_type = None;

                // Cannot assign a void expression
                if value_ctx.data_type.type_id == VOID_TYPE {
                    self.error(
                        SemanticErrorKind::VoidExpression,
                        assign.value.span(),
                        "cannot use void expression as assignment value",
                    );
                    return None;
                }

                // Check that target is a mutable lvalue
                if !target_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to an rvalue",
                    );
                    return None;
                }
                if !target_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to a const lvalue",
                    );
                    return None;
                }

                // Try opAssign operator overload first (for user-defined types)
                if let Some(func_id) = self.context.find_operator_method(target_ctx.data_type.type_id, OperatorBehavior::OpAssign) {
                    // Call opAssign(value) on target
                    // Stack: [target, value] → target.opAssign(value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let func = self.context.get_function(func_id);
                    return Some(ExprContext::rvalue(func.return_type().clone()));
                }

                // Fall back to primitive assignment with type conversion
                // Check if value is assignable to target and emit conversion if needed
                if let Some(conversion) = value_ctx.data_type.can_convert_to(&target_ctx.data_type, self.context) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            assign.span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' (explicit cast required)",
                                self.type_name(&value_ctx.data_type),
                                self.type_name(&target_ctx.data_type)
                            ),
                        );
                    } else {
                        // Emit conversion instruction if needed
                        self.emit_conversion(&conversion);
                    }
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "cannot assign value of type '{}' to variable of type '{}'",
                            self.type_name(&value_ctx.data_type),
                            self.type_name(&target_ctx.data_type)
                        ),
                    );
                }

                // Assignment produces rvalue of target type
                Some(ExprContext::rvalue(target_ctx.data_type))
            }

            // Compound assignment operators: try operator overload first, then desugar
            // e.g., x += 5  =>  x.opAddAssign(5) OR x = x + 5
            AddAssign | SubAssign | MulAssign | DivAssign | ModAssign | PowAssign |
            AndAssign | OrAssign | XorAssign | ShlAssign | ShrAssign |
            UshrAssign => {
                // Check target first (this is what we're assigning to)
                let target_ctx = self.check_expr(assign.target)?;

                // Check that target is a mutable lvalue
                if !target_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to an rvalue",
                    );
                    return None;
                }
                if !target_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to a const lvalue",
                    );
                    return None;
                }

                // Check value (RHS)
                let value_ctx = self.check_expr(assign.value)?;

                // Cannot use void expression in compound assignment
                if value_ctx.data_type.type_id == VOID_TYPE {
                    self.error(
                        SemanticErrorKind::VoidExpression,
                        assign.value.span(),
                        "cannot use void expression as assignment value",
                    );
                    return None;
                }

                // Try compound assignment operator overload first
                let compound_op = match assign.op {
                    AddAssign => OperatorBehavior::OpAddAssign,
                    SubAssign => OperatorBehavior::OpSubAssign,
                    MulAssign => OperatorBehavior::OpMulAssign,
                    DivAssign => OperatorBehavior::OpDivAssign,
                    ModAssign => OperatorBehavior::OpModAssign,
                    PowAssign => OperatorBehavior::OpPowAssign,
                    AndAssign => OperatorBehavior::OpAndAssign,
                    OrAssign => OperatorBehavior::OpOrAssign,
                    XorAssign => OperatorBehavior::OpXorAssign,
                    ShlAssign => OperatorBehavior::OpShlAssign,
                    ShrAssign => OperatorBehavior::OpShrAssign,
                    UshrAssign => OperatorBehavior::OpUShrAssign,
                    _ => unreachable!(),
                };

                if let Some(func_id) = self.context.find_operator_method(target_ctx.data_type.type_id, compound_op) {
                    // Call opXxxAssign(value) on target
                    // Stack: [target, value] → target.opAddAssign(value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let func = self.context.get_function(func_id);
                    return Some(ExprContext::rvalue(func.return_type().clone()));
                }

                // Fall back to desugaring: x += y  =>  x = x + y

                // Determine the binary operator equivalent
                let binary_op = match assign.op {
                    AddAssign => BinaryOp::Add,
                    SubAssign => BinaryOp::Sub,
                    MulAssign => BinaryOp::Mul,
                    DivAssign => BinaryOp::Div,
                    ModAssign => BinaryOp::Mod,
                    PowAssign => BinaryOp::Pow,
                    AndAssign => BinaryOp::BitwiseAnd,
                    OrAssign => BinaryOp::BitwiseOr,
                    XorAssign => BinaryOp::BitwiseXor,
                    ShlAssign => BinaryOp::ShiftLeft,
                    ShrAssign => BinaryOp::ShiftRight,
                    UshrAssign => BinaryOp::ShiftRightUnsigned,
                    _ => unreachable!(),
                };

                // Perform the binary operation type checking
                // This validates that the operation is valid for these types
                let result_type = self.check_binary_op(
                    binary_op,
                    &target_ctx.data_type,
                    &value_ctx.data_type,
                    assign.span,
                )?;

                // Result should be assignable back to target
                if !self.is_assignable(&result_type, &target_ctx.data_type) {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "compound assignment result type '{}' is not assignable to target type '{}'",
                            self.type_name(&result_type),
                            self.type_name(&target_ctx.data_type)
                        ),
                    );
                }

                // Emit the corresponding binary operation instruction
                let instr = match binary_op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Mod => Instruction::Mod,
                    BinaryOp::Pow => Instruction::Pow,
                    BinaryOp::BitwiseAnd => Instruction::BitAnd,
                    BinaryOp::BitwiseOr => Instruction::BitOr,
                    BinaryOp::BitwiseXor => Instruction::BitXor,
                    BinaryOp::ShiftLeft => Instruction::ShiftLeft,
                    BinaryOp::ShiftRight => Instruction::ShiftRight,
                    BinaryOp::ShiftRightUnsigned => Instruction::ShiftRightUnsigned,
                    _ => unreachable!(),
                };
                self.bytecode.emit(instr);

                // Assignment produces rvalue of target type
                Some(ExprContext::rvalue(target_ctx.data_type))
            }
        }
    }

    /// Type checks a ternary expression.
    /// Ternary expressions produce rvalues (temporary values).
    pub(super) fn check_ternary(&mut self, ternary: &TernaryExpr<'ast>) -> Option<ExprContext> {
        // Check condition
        let cond_ctx = self.check_expr(ternary.condition)?;
        if cond_ctx.data_type.type_id != BOOL_TYPE {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.condition.span(),
                format!(
                    "ternary condition must be bool, found '{}'",
                    self.type_name(&cond_ctx.data_type)
                ),
            );
        }

        // Check both branches
        let then_ctx = self.check_expr(ternary.then_expr)?;
        let else_ctx = self.check_expr(ternary.else_expr)?;

        // Void type cannot be used in ternary branches
        if then_ctx.data_type.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                ternary.then_expr.span(),
                "cannot use void expression in ternary branch",
            );
            return None;
        }
        if else_ctx.data_type.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                ternary.else_expr.span(),
                "cannot use void expression in ternary branch",
            );
            return None;
        }

        // Both branches should have compatible types
        // For simplicity, we'll require exact match
        if !self.is_assignable(&then_ctx.data_type, &else_ctx.data_type) {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.span,
                format!(
                    "ternary branches have incompatible types: '{}' and '{}'",
                    self.type_name(&then_ctx.data_type),
                    self.type_name(&else_ctx.data_type)
                ),
            );
        }

        Some(ExprContext::rvalue(then_ctx.data_type))
    }

    /// Type checks a function call.
    /// Function calls produce rvalues (unless they return a reference, which we don't handle yet).
    pub(super) fn check_call(&mut self, call: &CallExpr<'ast>) -> Option<ExprContext> {
        // Determine what we're calling FIRST (before type-checking arguments)
        // This allows us to provide expected funcdef context for lambda inference
        match call.callee {
            Expr::Ident(ident_expr) => {
                // Build qualified name (handling scope if present) - no intermediate Vec
                let (name, is_absolute_scope) = if let Some(scope) = ident_expr.scope {
                    let name = if scope.segments.is_empty() {
                        // Absolute scope with no prefix (e.g., ::globalFunction)
                        ident_expr.ident.name.to_string()
                    } else {
                        let scope_name = Self::build_scope_name(&scope);
                        let mut result = String::with_capacity(scope_name.len() + 2 + ident_expr.ident.name.len());
                        result.push_str(&scope_name);
                        result.push_str("::");
                        result.push_str(ident_expr.ident.name);
                        result
                    };
                    (name, scope.is_absolute)
                } else {
                    (ident_expr.ident.name.to_string(), false)
                };

                // Special handling for 'super' - resolve to base class constructor
                if name == "super" {
                    // Get current class context
                    let class_id = match self.current_class {
                        Some(id) => id,
                        None => {
                            self.error(
                                SemanticErrorKind::UndefinedVariable,
                                call.span,
                                "'super' can only be used in class methods/constructors",
                            );
                            return None;
                        }
                    };

                    // Get the class definition
                    let class_def = self.context.get_type(class_id);

                    // Check if class has a base class
                    let base_id = match class_def {
                        TypeDef::Class { base_class, .. } => match base_class {
                            Some(base) => *base,
                            None => {
                                self.error(
                                    SemanticErrorKind::UndefinedVariable,
                                    call.span,
                                    "'super' can only be used in classes with inheritance",
                                );
                                return None;
                            }
                        },
                        _ => {
                            self.error(
                                SemanticErrorKind::UndefinedVariable,
                                call.span,
                                "'super' can only be used in class methods",
                            );
                            return None;
                        }
                    };

                    // Type-check arguments WITHOUT funcdef inference for super calls
                    let mut arg_contexts = Vec::with_capacity(call.args.len());
                    for arg in call.args {
                        let arg_ctx = self.check_expr(arg.value)?;
                        arg_contexts.push(arg_ctx);
                    }
                    let mut arg_types = Vec::with_capacity(arg_contexts.len());
                    for ctx in &arg_contexts {
                        arg_types.push(ctx.data_type.clone());
                    }

                    // Find matching base constructor
                    let base_constructors = self.context.script().find_constructors(base_id);
                    let (matching_ctor, conversions) = self.find_best_function_overload(
                        &base_constructors,
                        &arg_types,
                        call.span,
                    )?;

                    let func_def = self.context.script().get_function(matching_ctor);

                    // Validate reference parameters
                    self.validate_reference_parameters(func_def, &arg_contexts, call.args, call.span)?;

                    // Emit conversion instructions for arguments
                    for conv in conversions.into_iter().flatten() {
                        self.emit_conversion(&conv);
                    }

                    // Emit regular Call instruction - base constructor executes with current 'this'
                    self.bytecode.emit(Instruction::Call(matching_ctor.as_u32()));

                    // Constructors return void
                    return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
                }

                // Check for base class method call pattern: BaseClass::method(args)
                // This is when inside a derived class and calling the parent's implementation directly
                if let Some(scope) = ident_expr.scope
                    && !scope.is_absolute && scope.segments.len() == 1 {
                        let scope_name = scope.segments[0].name;
                        let method_name = ident_expr.ident.name;

                        // Check if we're in a class method and the scope refers to a base class
                        if let Some(current_class_id) = self.current_class
                            && let Some(base_class_id) = self.get_base_class_by_name(current_class_id, scope_name) {
                                // This is a base class method call - load 'this' and call the base method
                                // Look up the method in the base class (pre-allocate)
                                // Note: base classes are always script classes (FFI class extension is forbidden)
                                let all_methods = self.context.get_methods(base_class_id);
                                let mut base_methods = Vec::with_capacity(all_methods.len().min(4));
                                for func_id in all_methods {
                                    let func = self.context.get_function(func_id);
                                    if func.name() == method_name {
                                        base_methods.push(func_id);
                                    }
                                }

                                if !base_methods.is_empty() {
                                    // Load 'this' for the method call
                                    self.bytecode.emit(Instruction::LoadLocal(0)); // 'this' is always local 0

                                    // Type-check arguments
                                    let mut arg_contexts = Vec::with_capacity(call.args.len());
                                    for arg in call.args {
                                        let arg_ctx = self.check_expr(arg.value)?;
                                        arg_contexts.push(arg_ctx);
                                    }
                                    let mut arg_types = Vec::with_capacity(arg_contexts.len());
                                    for ctx in &arg_contexts {
                                        arg_types.push(ctx.data_type.clone());
                                    }

                                    // Find best matching overload
                                    let (method_id, conversions) = self.find_best_function_overload(
                                        &base_methods,
                                        &arg_types,
                                        call.span,
                                    )?;

                                    let func_ref = self.context.get_function(method_id);

                                    // Validate reference parameters
                                    self.validate_reference_parameters_ref(&func_ref, &arg_contexts, call.args, call.span)?;

                                    // Emit any needed conversions
                                    for c in conversions.into_iter().flatten() {
                                        self.emit_conversion(&c);
                                    }

                                    // Emit call instruction
                                    self.bytecode.emit(Instruction::Call(method_id.as_u32()));

                                    return Some(ExprContext::rvalue(func_ref.return_type().clone()));
                                }
                            }
                    }

                // Check if this is a local variable (could be funcdef handle or class with opCall)
                if ident_expr.scope.is_none() {  // Only check locals for unqualified names
                    // Extract type info before mutable operations to avoid borrow conflicts
                    let var_info = self.local_scope.lookup(&name).map(|var| {
                        (var.data_type.type_id, var.data_type.is_handle)
                    });

                    if let Some((var_type_id, is_handle)) = var_info {
                        // Check for funcdef handle
                        if is_handle {
                            let type_def = self.context.get_type(var_type_id);
                            if let TypeDef::Funcdef { params, return_type, .. } = type_def {
                                // This is a funcdef variable
                                let _callee_ctx = self.check_expr(call.callee)?;

                                // Type-check arguments WITHOUT funcdef inference for now
                                let mut arg_contexts = Vec::with_capacity(call.args.len());
                                for arg in call.args {
                                    let arg_ctx = self.check_expr(arg.value)?;
                                    arg_contexts.push(arg_ctx);
                                }

                                // Clone params and return_type to avoid borrow issues
                                let params = params.clone();
                                let return_type = return_type.clone();

                                // Validate arguments
                                if arg_contexts.len() != params.len() {
                                    self.error(
                                        SemanticErrorKind::TypeMismatch,
                                        call.span,
                                        format!("funcdef call expects {} arguments but {} were provided",
                                            params.len(), arg_contexts.len()),
                                    );
                                    return None;
                                }

                                // Validate and emit conversions for each argument
                                for (i, (arg_ctx, param)) in arg_contexts.iter().zip(params.iter()).enumerate() {
                                    if let Some(conv) = arg_ctx.data_type.can_convert_to(param, self.context) {
                                        self.emit_conversion(&conv);
                                    } else {
                                        self.error(
                                            SemanticErrorKind::TypeMismatch,
                                            call.args[i].span,
                                            format!("argument {} type mismatch in funcdef call", i),
                                        );
                                        return None;
                                    }
                                }

                                // Emit CallPtr instruction
                                self.bytecode.emit(Instruction::CallPtr);

                                // Return the funcdef's return type
                                return Some(ExprContext::rvalue(return_type));
                            }
                        }

                        // Check for class with opCall operator (callable objects)
                        // This handles cases like: Functor f; f(5); where Functor has opCall(int)
                        if let Some(func_id) = self.context.find_operator_method(var_type_id, OperatorBehavior::OpCall) {
                            // Evaluate the callee (load the object)
                            let _callee_ctx = self.check_expr(call.callee)?;

                            // Type-check arguments
                            let mut arg_contexts = Vec::with_capacity(call.args.len());
                            for arg in call.args {
                                let arg_ctx = self.check_expr(arg.value)?;
                                arg_contexts.push(arg_ctx);
                            }

                            let func_ref = self.context.get_function(func_id);

                            // Validate argument count
                            if arg_contexts.len() != func_ref.param_count() {
                                self.error(
                                    SemanticErrorKind::WrongArgumentCount,
                                    call.span,
                                    format!("opCall expects {} arguments but {} were provided",
                                        func_ref.param_count(), arg_contexts.len()),
                                );
                                return None;
                            }

                            // Validate reference parameters
                            self.validate_reference_parameters_ref(&func_ref, &arg_contexts, call.args, call.span)?;

                            // Emit conversions for arguments that need conversion
                            for (i, arg_ctx) in arg_contexts.iter().enumerate() {
                                let param_type = func_ref.param_type(i);
                                if arg_ctx.data_type.type_id != param_type.type_id {
                                    if let Some(conv) = arg_ctx.data_type.can_convert_to(param_type, self.context) {
                                        if conv.is_implicit {
                                            self.emit_conversion(&conv);
                                        } else {
                                            self.error(
                                                SemanticErrorKind::TypeMismatch,
                                                call.args[i].span,
                                                format!("argument {} requires explicit conversion", i + 1),
                                            );
                                            return None;
                                        }
                                    } else {
                                        self.error(
                                            SemanticErrorKind::TypeMismatch,
                                            call.args[i].span,
                                            format!("cannot convert argument {} from '{}' to '{}'",
                                                i + 1,
                                                self.type_name(&arg_ctx.data_type),
                                                self.type_name(param_type)),
                                        );
                                        return None;
                                    }
                                }
                            }

                            self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                            return Some(ExprContext::rvalue(func_ref.return_type().clone()));
                        }
                    }
                }

                // Check if this is a type name (constructor call)
                // First, check if there are type arguments (e.g., array<int>())
                let type_id = if !ident_expr.type_args.is_empty() {
                    // Build the full type name (e.g., "array<int>")
                    // Template should already be instantiated during type compilation
                    let mut arg_names: Vec<&str> = Vec::with_capacity(ident_expr.type_args.len());
                    for arg in ident_expr.type_args {
                        if let Some(dt) = self.resolve_type_expr(arg) {
                            let typedef = self.context.get_type(dt.type_id);
                            arg_names.push(typedef.name());
                        } else {
                            return None; // Error already reported
                        }
                    }
                    // Build full type name without intermediate allocations
                    let capacity = name.len() + 2 + arg_names.iter().map(|n| n.len()).sum::<usize>()
                        + if arg_names.len() > 1 { (arg_names.len() - 1) * 2 } else { 0 };
                    let mut full_type_name = String::with_capacity(capacity);
                    full_type_name.push_str(&name);
                    full_type_name.push('<');
                    for (i, arg_name) in arg_names.iter().enumerate() {
                        if i > 0 {
                            full_type_name.push_str(", ");
                        }
                        full_type_name.push_str(arg_name);
                    }
                    full_type_name.push('>');
                    self.context.lookup_type(&full_type_name)
                } else {
                    // Simple type lookup - try raw name first, then namespace-qualified, then imports
                    // Try raw name first, then progressively qualified names
                    self.context.lookup_type(&name).or_else(|| {
                        // Try ancestor namespaces (current, then parent, then grandparent, etc.)
                        if !self.namespace_path.is_empty() {
                            // Try full namespace first
                            let qualified_name = self.build_qualified_name(&name);
                            if let Some(type_id) = self.context.lookup_type(&qualified_name) {
                                return Some(type_id);
                            }
                            // Try progressively shorter namespace prefixes
                            for prefix_len in (1..self.namespace_path.len()).rev() {
                                let ancestor_qualified = Self::build_qualified_name_from_path(
                                    &self.namespace_path[..prefix_len],
                                    &name,
                                );
                                if let Some(type_id) = self.context.lookup_type(&ancestor_qualified) {
                                    return Some(type_id);
                                }
                            }
                        }
                        // Try imported namespaces
                        for ns in &self.imported_namespaces {
                            let imported_qualified = format!("{}::{}", ns, name);
                            if let Some(type_id) = self.context.lookup_type(&imported_qualified) {
                                return Some(type_id);
                            }
                        }
                        None
                    })
                };

                if let Some(type_id) = type_id {
                    // Type-check arguments WITHOUT funcdef inference context for constructor calls
                    let mut arg_contexts = Vec::with_capacity(call.args.len());
                    for arg in call.args {
                        let arg_ctx = self.check_expr(arg.value)?;
                        arg_contexts.push(arg_ctx);
                    }
                    return self.check_constructor_call(type_id, &arg_contexts, call.span);
                }

                // Regular function call - look up candidates
                // For unqualified names (not absolute scope), try:
                // 1. Current namespace
                // 2. Global scope
                // 3. Imported namespaces
                let candidates: Vec<FunctionId> = if !is_absolute_scope && ident_expr.scope.is_none() {
                    // Try namespace-qualified name first
                    if !self.namespace_path.is_empty() {
                        let qualified_name = self.build_qualified_name(&name);
                        let ns_candidates = self.context.lookup_functions(&qualified_name);
                        if !ns_candidates.is_empty() {
                            ns_candidates.to_vec()
                        } else {
                            // Try global scope
                            let global = self.context.lookup_functions(&name);
                            if !global.is_empty() {
                                global.to_vec()
                            } else {
                                // Try imported namespaces
                                self.lookup_function_in_imports(&name)
                            }
                        }
                    } else {
                        // Not in a namespace, try global then imports
                        let global = self.context.lookup_functions(&name);
                        if !global.is_empty() {
                            global.to_vec()
                        } else {
                            self.lookup_function_in_imports(&name)
                        }
                    }
                } else {
                    self.context.lookup_functions(&name).to_vec()
                };

                if candidates.is_empty() {
                    self.error(
                        SemanticErrorKind::UndefinedVariable,
                        call.span,
                        format!("undefined function or type '{}'", name),
                    );
                    return None;
                }

                // Two-pass approach for lambda type inference with overloaded functions:
                // Pass 1: Identify which arguments are lambdas and type-check non-lambda args
                // Pass 2: Use narrowed candidates to infer funcdef types for lambda args

                // Identify lambda argument positions (pre-allocate with estimated capacity)
                let mut lambda_positions = Vec::with_capacity(call.args.len().min(4)); // Most calls have few lambdas
                for (i, arg) in call.args.iter().enumerate() {
                    if matches!(arg.value, Expr::Lambda(_)) {
                        lambda_positions.push(i);
                    }
                }

                // If there are lambdas and multiple candidates, use two-pass approach
                let mut arg_contexts = Vec::with_capacity(call.args.len());

                if !lambda_positions.is_empty() && candidates.len() > 1 {
                    // Pass 1: Type-check non-lambda arguments first
                    let mut non_lambda_types: Vec<Option<DataType>> = vec![None; call.args.len()];
                    for (i, arg) in call.args.iter().enumerate() {
                        if !lambda_positions.contains(&i) {
                            let arg_ctx = self.check_expr(arg.value)?;
                            non_lambda_types[i] = Some(arg_ctx.data_type.clone());
                            arg_contexts.push(arg_ctx);
                        }
                    }

                    // Narrow candidates based on non-lambda argument types (pre-allocate)
                    let mut narrowed_candidates = Vec::with_capacity(candidates.len());
                    for &func_id in &candidates {
                        let func_ref = self.context.get_function(func_id);
                        // Check argument count (considering defaults)
                        let min_params = func_ref.required_param_count();
                        if call.args.len() < min_params || call.args.len() > func_ref.param_count() {
                            continue;
                        }
                        // Check non-lambda argument types match
                        let mut matches = true;
                        for (i, opt_type) in non_lambda_types.iter().enumerate() {
                            if let Some(arg_type) = opt_type
                                && i < func_ref.param_count() {
                                    let param_type = func_ref.param_type(i);
                                    // Check if types are compatible (exact match or implicit conversion)
                                    if arg_type.type_id != param_type.type_id
                                        && arg_type.can_convert_to(param_type, self.context).is_none_or(|c| !c.is_implicit) {
                                            matches = false;
                                            break;
                                        }
                                }
                        }
                        if matches {
                            narrowed_candidates.push(func_id);
                        }
                    }

                    // Pass 2: Type-check lambda arguments with inferred funcdef types
                    let expected_param_types = if narrowed_candidates.len() == 1 {
                        let func_ref = self.context.get_function(narrowed_candidates[0]);
                        Some(func_ref.param_types())
                    } else {
                        None
                    };

                    // Now type-check lambda arguments with context
                    let mut full_arg_contexts = Vec::with_capacity(call.args.len());
                    let mut non_lambda_idx = 0;
                    for (i, arg) in call.args.iter().enumerate() {
                        if lambda_positions.contains(&i) {
                            // Set expected_funcdef_type for lambda inference
                            if let Some(ref params) = expected_param_types
                                && i < params.len() {
                                    let param_type = &params[i];
                                    if param_type.is_handle {
                                        let type_def = self.context.get_type(param_type.type_id);
                                        if matches!(type_def, TypeDef::Funcdef { .. }) {
                                            self.expected_funcdef_type = Some(param_type.type_id);
                                        }
                                    }
                                }
                            let arg_ctx = self.check_expr(arg.value)?;
                            full_arg_contexts.push(arg_ctx);
                            self.expected_funcdef_type = None;
                        } else {
                            // Use already computed non-lambda context
                            full_arg_contexts.push(arg_contexts[non_lambda_idx].clone());
                            non_lambda_idx += 1;
                        }
                    }
                    arg_contexts = full_arg_contexts;
                } else {
                    // Simple case: single candidate or no lambdas
                    let expected_param_types = if candidates.len() == 1 {
                        let func_ref = self.context.get_function(candidates[0]);
                        Some(func_ref.param_types())
                    } else {
                        None
                    };

                    for (i, arg) in call.args.iter().enumerate() {
                        // Set expected_funcdef_type if this parameter expects a funcdef
                        if let Some(ref params) = expected_param_types
                            && i < params.len() {
                                let param_type = &params[i];
                                if param_type.is_handle {
                                    let type_def = self.context.get_type(param_type.type_id);
                                    if matches!(type_def, TypeDef::Funcdef { .. }) {
                                        self.expected_funcdef_type = Some(param_type.type_id);
                                    }
                                }
                            }

                        let arg_ctx = self.check_expr(arg.value)?;
                        arg_contexts.push(arg_ctx);

                        self.expected_funcdef_type = None;
                    }
                }

                // Extract types for overload resolution
                let mut arg_types = Vec::with_capacity(arg_contexts.len());
                for ctx in &arg_contexts {
                    arg_types.push(ctx.data_type.clone());
                }

                // Find best matching overload
                let (matching_func, conversions) = self.find_best_function_overload(
                    &candidates,
                    &arg_types,
                    call.span,
                )?;

                let func_ref = self.context.get_function(matching_func);

                // Compile default arguments if fewer args provided than params
                if arg_contexts.len() < func_ref.param_count() {
                    // Default argument expressions only exist on script functions
                    if let Some(func_def) = func_ref.as_script() {
                        for i in arg_contexts.len()..func_def.params.len() {
                            if let Some(default_expr) = func_def.params[i].default {
                                // Compile the default argument expression inline
                                let default_ctx = self.check_expr(default_expr)?;

                                // Apply implicit conversion if needed
                                if let Some(conv) = default_ctx.data_type.can_convert_to(&func_def.params[i].data_type, self.context) {
                                    self.emit_conversion(&conv);
                                }
                            } else {
                                // No default arg for this parameter - error
                                self.error(
                                    SemanticErrorKind::TypeMismatch,
                                    call.span,
                                    format!("function '{}' expects {} arguments but {} were provided",
                                        func_def.name, func_def.params.len(), arg_contexts.len()),
                                );
                                return None;
                            }
                        }
                    } else {
                        // FFI function with defaults - the VM handles default value injection
                        // For now, we emit a placeholder or handle differently
                        // TODO: Implement FFI default argument handling if needed
                        self.error(
                            SemanticErrorKind::WrongArgumentCount,
                            call.span,
                            format!("function '{}' expects {} arguments but {} were provided (FFI default arguments not yet supported at compile time)",
                                func_ref.name(), func_ref.param_count(), arg_contexts.len()),
                        );
                        return None;
                    }
                }

                // Validate reference parameters BEFORE emitting conversions
                self.validate_reference_parameters_ref(&func_ref, &arg_contexts, call.args, call.span)?;

                // Emit conversion instructions for explicitly provided arguments
                for conv in conversions.into_iter().flatten() {
                    self.emit_conversion(&conv);
                }

                // Emit call instruction
                self.bytecode.emit(Instruction::Call(matching_func.as_u32()));

                // Function calls produce rvalues
                Some(ExprContext::rvalue(func_ref.return_type().clone()))
            }
            _ => {
                // Complex call expression (e.g., obj(args) with opCall, function pointer, lambda)
                let callee_ctx = self.check_expr(call.callee)?;

                // Type-check arguments WITHOUT funcdef inference for opCall
                let mut arg_contexts = Vec::with_capacity(call.args.len());
                for arg in call.args {
                    let arg_ctx = self.check_expr(arg.value)?;
                    arg_contexts.push(arg_ctx);
                }

                // Try opCall operator overload (allows objects to be called like functions)
                if let Some(func_id) = self.context.find_operator_method(callee_ctx.data_type.type_id, OperatorBehavior::OpCall) {
                    // Call opCall(args) on callee
                    // Stack: [callee, arg1, arg2, ...] → callee.opCall(arg1, arg2, ...)

                    let func_ref = self.context.get_function(func_id);

                    // Validate argument count
                    if arg_contexts.len() != func_ref.param_count() {
                        self.error(
                            SemanticErrorKind::WrongArgumentCount,
                            call.span,
                            format!("opCall expects {} arguments but {} were provided",
                                func_ref.param_count(), arg_contexts.len()),
                        );
                        return None;
                    }

                    // Validate reference parameters
                    self.validate_reference_parameters_ref(&func_ref, &arg_contexts, call.args, call.span)?;

                    // Emit conversions for arguments that need conversion
                    for (i, arg_ctx) in arg_contexts.iter().enumerate() {
                        let param_type = func_ref.param_type(i);
                        if arg_ctx.data_type.type_id != param_type.type_id {
                            if let Some(conv) = arg_ctx.data_type.can_convert_to(param_type, self.context) {
                                if conv.is_implicit {
                                    self.emit_conversion(&conv);
                                } else {
                                    self.error(
                                        SemanticErrorKind::TypeMismatch,
                                        call.args[i].span,
                                        format!("argument {} requires explicit conversion", i + 1),
                                    );
                                    return None;
                                }
                            } else {
                                self.error(
                                    SemanticErrorKind::TypeMismatch,
                                    call.args[i].span,
                                    format!("cannot convert argument {} from '{}' to '{}'",
                                        i + 1,
                                        self.type_name(&arg_ctx.data_type),
                                        self.type_name(param_type)),
                                );
                                return None;
                            }
                        }
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    return Some(ExprContext::rvalue(func_ref.return_type().clone()));
                }

                // No opCall found - check if it's a funcdef/function pointer
                if callee_ctx.data_type.is_handle {
                    let type_def = self.context.get_type(callee_ctx.data_type.type_id);

                    if let TypeDef::Funcdef { params, return_type, .. } = type_def {
                        // This is a funcdef handle - validate arguments
                        if arg_contexts.len() != params.len() {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                call.span,
                                format!("funcdef call expects {} arguments but {} were provided",
                                    params.len(), arg_contexts.len()),
                            );
                            return None;
                        }

                        // Validate and emit conversions for each argument
                        for (i, (arg_ctx, param)) in arg_contexts.iter().zip(params.iter()).enumerate() {
                            if let Some(conv) = arg_ctx.data_type.can_convert_to(param, self.context) {
                                self.emit_conversion(&conv);
                            } else {
                                self.error(
                                    SemanticErrorKind::TypeMismatch,
                                    call.args[i].span,
                                    format!("argument {} type mismatch in funcdef call", i),
                                );
                                return None;
                            }
                        }

                        // Emit CallPtr instruction to invoke through function pointer
                        // Stack: [funcdef_handle, arg1, arg2, ...] → result
                        self.bytecode.emit(Instruction::CallPtr);

                        // Return the funcdef's return type
                        return Some(ExprContext::rvalue(return_type.clone()));
                    }
                }

                // Not callable
                self.error(
                    SemanticErrorKind::NotCallable,
                    call.span,
                    format!("type '{}' is not callable (no opCall operator or funcdef)", self.type_name(&callee_ctx.data_type)),
                );
                None
            }
        }
    }

    /// Type checks a constructor call (e.g., `Player(100, "Bob")`).
    ///
    /// The instantiation method depends on TypeKind:
    /// - Reference (FFI types like array, dictionary): use factories
    /// - Value and ScriptObject: use constructors
    pub(super) fn check_constructor_call(
        &mut self,
        type_id: TypeId,
        arg_contexts: &[ExprContext],
        span: Span,
    ) -> Option<ExprContext> {
        // Extract types for overload resolution
        let mut arg_types = Vec::with_capacity(arg_contexts.len());
        for ctx in arg_contexts {
            arg_types.push(ctx.data_type.clone());
        }

        let typedef = self.context.get_type(type_id);
        let type_name = typedef.name().to_string();
        let use_factory = typedef.type_kind().uses_factories();

        // Get factories or constructors based on type kind
        let candidates = if use_factory {
            self.context.find_factories(type_id)
        } else {
            self.context.find_constructors(type_id)
        };

        if candidates.is_empty() {
            let kind_name = if use_factory { "factories" } else { "constructors" };
            self.error(
                SemanticErrorKind::UndefinedFunction,
                span,
                format!("type '{}' has no {}", type_name, kind_name),
            );
            return None;
        }

        // Find best matching constructor/factory using existing overload resolution
        let (matching_func, conversions) = self.find_best_function_overload(&candidates, &arg_types, span)?;

        // Emit conversion instructions for arguments
        for conv in conversions.into_iter().flatten() {
            self.emit_conversion(&conv);
        }

        // Emit constructor or factory call instruction
        if use_factory {
            self.bytecode.emit(Instruction::CallFactory {
                type_id: type_id.as_u32(),
                func_id: matching_func.as_u32(),
            });
        } else {
            self.bytecode.emit(Instruction::CallConstructor {
                type_id: type_id.as_u32(),
                func_id: matching_func.as_u32(),
            });
        }

        // Constructor/factory calls produce rvalues (newly constructed objects)
        Some(ExprContext::rvalue(DataType::simple(type_id)))
    }

    /// Type checks an index expression.
    /// Index expressions (arr[i]) are lvalues if the array is an lvalue.
    ///
    /// AngelScript supports two forms of indexing:
    /// - Single-arg: `arr[i]` calls `opIndex(i)` with 1 parameter
    /// - Multi-arg: `m[i, j]` calls `opIndex(i, j)` with multiple parameters
    ///
    /// Note: Multi-dimensional chaining (`arr[0][1]`) is handled by the parser
    /// creating nested IndexExpr nodes, so each call to check_index handles
    /// one bracket pair with potentially multiple arguments.
    pub(super) fn check_index(&mut self, index: &IndexExpr<'ast>) -> Option<ExprContext> {
        // Evaluate the base object
        let current_ctx = self.check_expr(index.object)?;

        // Empty index is invalid
        if index.indices.is_empty() {
            self.error(
                SemanticErrorKind::InvalidOperation,
                index.span,
                "index expression requires at least one index".to_string(),
            );
            return None;
        }

        // Evaluate all index arguments first
        let mut idx_contexts = Vec::new();
        for idx_item in index.indices {
            let idx_ctx = self.check_expr(idx_item.index)?;
            idx_contexts.push((idx_ctx, idx_item.span));
        }

        // Try to find opIndex for the object type (priority 1)
        if let Some(func_id) = self.context.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndex) {
            let func = self.context.get_function(func_id);

            // Check parameter count matches
            if func.param_count() != idx_contexts.len() {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    index.span,
                    format!(
                        "opIndex expects {} parameter(s), found {}",
                        func.param_count(),
                        idx_contexts.len()
                    ),
                );
                return None;
            }

            // Type check each index argument against corresponding opIndex parameter
            for (i, (idx_ctx, idx_span)) in idx_contexts.iter().enumerate() {
                let param_type = func.param_type(i);

                if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.context) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            *idx_span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' for opIndex parameter {} (explicit cast required)",
                                self.type_name(&idx_ctx.data_type),
                                self.type_name(param_type),
                                i + 1
                            ),
                        );
                        return None;
                    }
                    self.emit_conversion(&conversion);
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        *idx_span,
                        format!(
                            "opIndex parameter {} expects type '{}', found '{}'",
                            i + 1,
                            self.type_name(param_type),
                            self.type_name(&idx_ctx.data_type)
                        ),
                    );
                    return None;
                }
            }

            // Call opIndex on current object
            // Stack: [object, idx1, idx2, ...] → object.opIndex(idx1, idx2, ...)
            self.bytecode.emit(Instruction::Call(func_id.as_u32()));

            // opIndex returns a reference, so result is an lvalue
            let is_mutable = current_ctx.is_mutable && !func.return_type().is_const;
            return Some(ExprContext::lvalue(func.return_type().clone(), is_mutable));
        }

        // Try get_opIndex accessor (priority 2)
        if let Some(func_id) = self.context.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexGet) {
            let func = self.context.get_function(func_id);

            // Check parameter count matches
            if func.param_count() != idx_contexts.len() {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    index.span,
                    format!(
                        "get_opIndex expects {} parameter(s), found {}",
                        func.param_count(),
                        idx_contexts.len()
                    ),
                );
                return None;
            }

            // Type check each index argument
            for (i, (idx_ctx, idx_span)) in idx_contexts.iter().enumerate() {
                let param_type = func.param_type(i);

                if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.context) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            *idx_span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' for get_opIndex parameter {} (explicit cast required)",
                                self.type_name(&idx_ctx.data_type),
                                self.type_name(param_type),
                                i + 1
                            ),
                        );
                        return None;
                    }
                    self.emit_conversion(&conversion);
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        *idx_span,
                        format!(
                            "get_opIndex parameter {} expects type '{}', found '{}'",
                            i + 1,
                            self.type_name(param_type),
                            self.type_name(&idx_ctx.data_type)
                        ),
                    );
                    return None;
                }
            }

            // Call get_opIndex on current object
            self.bytecode.emit(Instruction::Call(func_id.as_u32()));

            // get_opIndex returns a value (read-only), so result is an rvalue
            return Some(ExprContext::rvalue(func.return_type().clone()));
        }

        // No opIndex or get_opIndex registered for this type
        self.error(
            SemanticErrorKind::InvalidOperation,
            index.span,
            format!("type '{}' does not support indexing", self.type_name(&current_ctx.data_type)),
        );
        None
    }

    /// Type checks an index assignment expression: obj[idx] = value
    /// This handles set_opIndex property accessor.
    /// Returns None if error occurred, Some(ExprContext) for the assignment result.
    pub(super) fn check_index_assignment(
        &mut self,
        index: &IndexExpr<'ast>,
        value: &'ast Expr<'ast>,
        span: Span,
    ) -> Option<ExprContext> {
        // For multi-dimensional indexing like arr[0][1] = value:
        // - Process all but the last index using regular opIndex/get_opIndex (read context)
        // - Use set_opIndex only for the final index with the assignment value

        // Start with the base object
        let mut current_ctx = self.check_expr(index.object)?;

        // Process all indices except the last one in read context
        let last_idx = index.indices.len() - 1;
        for (i, idx_item) in index.indices.iter().enumerate() {
            // Evaluate the index expression for this dimension
            let idx_ctx = self.check_expr(idx_item.index)?;

            if i < last_idx {
                // Not the final index - use regular opIndex/get_opIndex (read context)
                // This is the same logic as check_index
                if let Some(func_id) = self.context.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndex) {
                    let func = self.context.get_function(func_id);

                    if func.param_count() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("opIndex must have exactly 1 parameter, found {}", func.param_count()),
                        );
                        return None;
                    }

                    let param_type = func.param_type(0);
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let is_mutable = current_ctx.is_mutable && !func.return_type().is_const;
                    current_ctx = ExprContext::lvalue(func.return_type().clone(), is_mutable);
                } else if let Some(func_id) = self.context.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexGet) {
                    let func = self.context.get_function(func_id);

                    if func.param_count() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("get_opIndex must have exactly 1 parameter, found {}", func.param_count()),
                        );
                        return None;
                    }

                    let param_type = func.param_type(0);
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for get_opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "get_opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    current_ctx = ExprContext::rvalue(func.return_type().clone());
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!("type '{}' does not support indexing", self.type_name(&current_ctx.data_type)),
                    );
                    return None;
                }
            } else {
                // Final index - try opIndex first (returns reference), then set_opIndex
                // For assignment, we prefer the non-const opIndex if available
                if let Some(func_id) = self.context.find_operator_method_with_mutability(current_ctx.data_type.type_id, OperatorBehavior::OpIndex, true) {
                    // opIndex exists - use regular assignment through reference
                    let func = self.context.get_function(func_id);

                    if func.param_count() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("opIndex must have exactly 1 parameter, found {}", func.param_count()),
                        );
                        return None;
                    }

                    let param_type = func.param_type(0);
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let is_mutable = current_ctx.is_mutable && !func.return_type().is_const;
                    current_ctx = ExprContext::lvalue(func.return_type().clone(), is_mutable);

                    // Now handle assignment to the returned reference
                    // Check that it's a mutable lvalue
                    if !current_ctx.is_lvalue {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            span,
                            "opIndex did not return an lvalue reference",
                        );
                        return None;
                    }
                    if !current_ctx.is_mutable {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            span,
                            "cannot assign to const indexed element",
                        );
                        return None;
                    }

                    // Type check the value being assigned
                    let value_ctx = self.check_expr(value)?;
                    if let Some(conversion) = value_ctx.data_type.can_convert_to(&current_ctx.data_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}'",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(&current_ctx.data_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            span,
                            format!(
                                "cannot assign value of type '{}' to indexed element of type '{}'",
                                self.type_name(&value_ctx.data_type),
                                self.type_name(&current_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Emit store instruction (handled by VM based on lvalue on stack)
                    return Some(ExprContext::rvalue(current_ctx.data_type));

                } else if let Some(func_id) = self.context.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexSet) {
                    // No opIndex, but set_opIndex exists
                    let func = self.context.get_function(func_id);

                    // set_opIndex should have exactly 2 parameters: (index, value)
                    if func.param_count() != 2 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("set_opIndex must have exactly 2 parameters, found {}", func.param_count()),
                        );
                        return None;
                    }

                    let index_param_type = func.param_type(0);
                    let value_param_type = func.param_type(1);

                    // Type check the index argument
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(index_param_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for set_opIndex index parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(index_param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "set_opIndex index parameter expects type '{}', found '{}'",
                                self.type_name(index_param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Type check the value argument
                    let value_ctx = self.check_expr(value)?;
                    if let Some(conversion) = value_ctx.data_type.can_convert_to(value_param_type, self.context) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for set_opIndex value parameter",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(value_param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            span,
                            format!(
                                "set_opIndex value parameter expects type '{}', found '{}'",
                                self.type_name(value_param_type),
                                self.type_name(&value_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Call set_opIndex(index, value) on current object
                    // Stack: [object, index, value] → object.set_opIndex(index, value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));

                    // Assignment expression returns the assigned value as rvalue
                    return Some(ExprContext::rvalue(value_ctx.data_type));

                } else {
                    // No opIndex or set_opIndex found
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!("type '{}' does not support index assignment", self.type_name(&current_ctx.data_type)),
                    );
                    return None;
                }
            }
        }

        // Should never reach here (loop always returns in last iteration)
        None
    }

    /// Type checks a member property assignment: obj.prop = value
    /// This handles set_X property accessor.
    /// Returns Some(result) if a property setter was found and used,
    /// or None if no property exists (caller should fall back to regular field assignment).
    pub(super) fn check_member_property_assignment(
        &mut self,
        member: &MemberExpr<'ast>,
        property_name: &str,
        value: &'ast Expr<'ast>,
        span: Span,
    ) -> Option<ExprContext> {
        // First evaluate the object expression
        let object_ctx = self.check_expr(member.object)?;

        // Check if the object type has a property with this name
        let property = self.context.find_property(object_ctx.data_type.type_id, property_name)?;

        // Property exists - check for setter
        let setter_id = match property.setter {
            Some(id) => id,
            None => {
                // Property is read-only (no setter)
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    span,
                    format!(
                        "property '{}' on type '{}' is read-only",
                        property_name,
                        self.type_name(&object_ctx.data_type)
                    ),
                );
                return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE))); // Return Some to prevent fallback
            }
        };

        // Check visibility access for the property
        if !self.check_visibility_access(property.visibility, object_ctx.data_type.type_id) {
            self.report_access_violation(
                property.visibility,
                property_name,
                &self.type_name(&object_ctx.data_type),
                span,
            );
            return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE))); // Return Some to prevent fallback
        }

        // Get setter function to validate value type
        let setter_func = self.context.get_function(setter_id);

        // Setter should have exactly one parameter (the value)
        if setter_func.param_count() != 1 {
            self.error(
                SemanticErrorKind::InvalidOperation,
                span,
                format!(
                    "property setter 'set_{}' must have exactly 1 parameter, found {}",
                    property_name,
                    setter_func.param_count()
                ),
            );
            return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
        }

        let value_param_type = setter_func.param_type(0);

        // Type check the value expression
        let value_ctx = self.check_expr(value)?;

        // Cannot assign a void expression
        if value_ctx.data_type.type_id == VOID_TYPE {
            self.error(
                SemanticErrorKind::VoidExpression,
                value.span(),
                "cannot use void expression as property value",
            );
            return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
        }

        // Check type conversion for value
        if let Some(conversion) = value_ctx.data_type.can_convert_to(value_param_type, self.context) {
            if !conversion.is_implicit {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    span,
                    format!(
                        "cannot implicitly convert '{}' to '{}' for property '{}' setter",
                        self.type_name(&value_ctx.data_type),
                        self.type_name(value_param_type),
                        property_name
                    ),
                );
                return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
            }
            self.emit_conversion(&conversion);
        } else {
            self.error(
                SemanticErrorKind::TypeMismatch,
                span,
                format!(
                    "property '{}' setter expects type '{}', found '{}'",
                    property_name,
                    self.type_name(value_param_type),
                    self.type_name(&value_ctx.data_type)
                ),
            );
            return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
        }

        // Call setter method: object.set_prop(value)
        self.bytecode.emit(Instruction::CallMethod(setter_id.as_u32()));

        // Property assignment returns the assigned value as rvalue
        Some(ExprContext::rvalue(value_ctx.data_type))
    }

    /// Type checks a member access expression.
    /// Field access (obj.field) is an lvalue if obj is an lvalue.
    /// Method calls (obj.method()) always return rvalues.
    pub(super) fn check_member(&mut self, member: &MemberExpr<'ast>) -> Option<ExprContext> {
        let object_ctx = self.check_expr(member.object)?;

        // Check that the object is a class/interface type
        let typedef = self.context.get_type(object_ctx.data_type.type_id);

        match &member.member {
            MemberAccess::Field(field_name) => {
                // Look up the field in the class (including inherited fields)
                match typedef {
                    TypeDef::Class { .. } => {
                        // First check for property accessor (get_X pattern)
                        // Property accessors take precedence over direct field access
                        if let Some(property) = self.context.find_property(object_ctx.data_type.type_id, field_name.name) {
                            if let Some(getter_id) = property.getter {
                                // Check visibility access for the property
                                if !self.check_visibility_access(property.visibility, object_ctx.data_type.type_id) {
                                    self.report_access_violation(
                                        property.visibility,
                                        field_name.name,
                                        &self.type_name(&object_ctx.data_type),
                                        member.span,
                                    );
                                    return None;
                                }

                                // Check const-correctness: if object is const, getter must be const
                                let is_const_object = object_ctx.data_type.is_const || object_ctx.data_type.is_handle_to_const;
                                let getter_func = self.context.get_function(getter_id);
                                if is_const_object && !getter_func.traits().is_const {
                                    self.error(
                                        SemanticErrorKind::InvalidOperation,
                                        member.span,
                                        format!(
                                            "cannot call non-const property getter '{}' on const object of type '{}'",
                                            field_name.name,
                                            self.type_name(&object_ctx.data_type)
                                        ),
                                    );
                                    return None;
                                }

                                // Emit method call to getter
                                self.bytecode.emit(Instruction::CallMethod(getter_id.as_u32()));

                                // Property getter returns rvalue (can't assign to it directly)
                                // This is a property accessor, not a reference
                                return Some(ExprContext::rvalue(getter_func.return_type().clone()));
                            } else {
                                // Property exists but is write-only (no getter)
                                self.error(
                                    SemanticErrorKind::InvalidOperation,
                                    member.span,
                                    format!(
                                        "property '{}' on type '{}' is write-only",
                                        field_name.name,
                                        self.type_name(&object_ctx.data_type)
                                    ),
                                );
                                return None;
                            }
                        }

                        // No property accessor found, try field lookup
                        // Find the field by name, checking class hierarchy
                        if let Some((field_index, field_def, defining_class_id)) =
                            self.find_field_in_hierarchy(object_ctx.data_type.type_id, field_name.name)
                        {
                            // Check visibility access (use defining class for visibility check)
                            if !self.check_visibility_access(field_def.visibility, defining_class_id) {
                                self.report_access_violation(
                                    field_def.visibility,
                                    &field_def.name,
                                    &self.type_name(&object_ctx.data_type),
                                    member.span,
                                );
                                return None;
                            }

                            // Emit load field instruction (using field index)
                            self.bytecode.emit(Instruction::LoadField(field_index as u32));

                            // If the object is const, the field should also be const
                            let mut field_type = field_def.data_type.clone();
                            if object_ctx.data_type.is_const || object_ctx.data_type.is_handle_to_const {
                                field_type.is_const = true;
                            }

                            // Field access is lvalue if object is lvalue
                            // Mutability depends on both object and field
                            let is_mutable = object_ctx.is_mutable && !field_type.is_const;
                            Some(ExprContext::lvalue(field_type, is_mutable))
                        } else {
                            self.error(
                                SemanticErrorKind::UndefinedField,
                                member.span,
                                format!(
                                    "type '{}' has no field '{}'",
                                    self.type_name(&object_ctx.data_type),
                                    field_name.name
                                ),
                            );
                            None
                        }
                    }
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            member.span,
                            format!(
                                "type '{}' does not support field access",
                                self.type_name(&object_ctx.data_type)
                            ),
                        );
                        None
                    }
                }
            }
            MemberAccess::Method { name, args } => {
                // Verify the object is a class type (includes template instances like array<T>)
                match typedef {
                    TypeDef::Class { .. } => {
                        // Look up methods with this name on the type
                        let candidates = self.context.find_methods_by_name(object_ctx.data_type.type_id, name.name);

                        if candidates.is_empty() {
                            self.error(
                                SemanticErrorKind::UndefinedMethod,
                                member.span,
                                format!(
                                    "type '{}' has no method '{}'",
                                    self.type_name(&object_ctx.data_type),
                                    name.name
                                ),
                            );
                            return None;
                        }

                        // Filter by const-correctness first
                        let is_const_object = object_ctx.data_type.is_const || object_ctx.data_type.is_handle_to_const;

                        let const_filtered: Vec<_> = if is_const_object {
                            // Const objects can only call const methods (pre-allocate)
                            let mut filtered = Vec::with_capacity(candidates.len());
                            for func_id in candidates {
                                let func_ref = self.context.get_function(func_id);
                                if func_ref.traits().is_const {
                                    filtered.push(func_id);
                                }
                            }
                            filtered
                        } else {
                            // Non-const objects can call both const and non-const methods
                            candidates
                        };

                        if const_filtered.is_empty() {
                            self.error(
                                SemanticErrorKind::InvalidOperation,
                                member.span,
                                format!(
                                    "no const method '{}' found for const object of type '{}'",
                                    name.name,
                                    self.type_name(&object_ctx.data_type)
                                ),
                            );
                            return None;
                        }

                        // Type check arguments with lambda inference support
                        // When there's a single matching method, we can infer funcdef types for lambdas
                        let mut arg_contexts = Vec::with_capacity(args.len());
                        let expected_param_types = if const_filtered.len() == 1 {
                            let func_ref = self.context.get_function(const_filtered[0]);
                            Some(func_ref.param_types())
                        } else {
                            None
                        };

                        for (i, arg) in args.iter().enumerate() {
                            // Set expected_funcdef_type if this parameter expects a funcdef
                            if let Some(ref params) = expected_param_types
                                && i < params.len() {
                                    let param_type = &params[i];
                                    if param_type.is_handle {
                                        let type_def = self.context.get_type(param_type.type_id);
                                        if matches!(type_def, TypeDef::Funcdef { .. }) {
                                            self.expected_funcdef_type = Some(param_type.type_id);
                                        }
                                    }
                                }

                            let arg_ctx = self.check_expr(arg.value)?;
                            arg_contexts.push(arg_ctx);

                            self.expected_funcdef_type = None;
                        }

                        // Extract types for overload resolution
                        let mut arg_types = Vec::with_capacity(arg_contexts.len());
                        for ctx in &arg_contexts {
                            arg_types.push(ctx.data_type.clone());
                        }

                        // Find best matching overload from const-filtered candidates
                        let (matching_method, conversions) = self.find_best_function_overload(
                            &const_filtered,
                            &arg_types,
                            member.span,
                        )?;

                        let func_ref = self.context.get_function(matching_method);

                        // Check visibility access
                        if !self.check_visibility_access(func_ref.visibility(), object_ctx.data_type.type_id) {
                            self.report_access_violation(
                                func_ref.visibility(),
                                func_ref.name(),
                                &self.type_name(&object_ctx.data_type),
                                member.span,
                            );
                            return None;
                        }

                        // Validate reference parameters
                        self.validate_reference_parameters_ref(&func_ref, &arg_contexts, args, member.span)?;

                        // Emit conversion instructions for arguments
                        for conv in conversions.into_iter().flatten() {
                            self.emit_conversion(&conv);
                        }

                        // Emit method call instruction
                        self.bytecode.emit(Instruction::CallMethod(matching_method.as_u32()));

                        // Method calls return rvalues
                        Some(ExprContext::rvalue(func_ref.return_type().clone()))
                    }
                    TypeDef::Interface { methods, .. } => {
                        // Type check arguments first
                        let mut arg_contexts = Vec::with_capacity(args.len());
                        for arg in *args {
                            let arg_ctx = self.check_expr(arg.value)?;
                            arg_contexts.push(arg_ctx);
                        }

                        // Extract types for signature matching
                        let mut arg_types = Vec::with_capacity(arg_contexts.len());
                        for ctx in &arg_contexts {
                            arg_types.push(ctx.data_type.clone());
                        }

                        // Find the method signature on the interface
                        let matching_methods: Vec<(usize, &MethodSignature)> = methods.iter()
                            .enumerate()
                            .filter(|(_, sig)| sig.name == name.name)
                            .collect();

                        if matching_methods.is_empty() {
                            self.error(
                                SemanticErrorKind::UndefinedMethod,
                                member.span,
                                format!(
                                    "interface '{}' has no method '{}'",
                                    self.type_name(&object_ctx.data_type),
                                    name.name
                                ),
                            );
                            return None;
                        }

                        // Find best matching signature based on argument types
                        // For interfaces, we don't have FunctionIds, so we do simple signature matching
                        let mut best_match: Option<(usize, &MethodSignature, Vec<Option<Conversion>>)> = None;

                        for (method_index, sig) in &matching_methods {
                            if sig.params.len() != arg_types.len() {
                                continue;
                            }

                            // Check if all arguments are compatible
                            let mut conversions = Vec::with_capacity(arg_types.len());
                            let mut all_match = true;

                            for (arg_type, param_type) in arg_types.iter().zip(sig.params.iter()) {
                                if let Some(conv) = arg_type.can_convert_to(param_type, self.context) {
                                    conversions.push(Some(conv));
                                } else {
                                    all_match = false;
                                    break;
                                }
                            }

                            if all_match {
                                best_match = Some((*method_index, *sig, conversions));
                                break;
                            }
                        }

                        let (method_index, sig, conversions) = match best_match {
                            Some(m) => m,
                            None => {
                                self.error(
                                    SemanticErrorKind::WrongArgumentCount,
                                    member.span,
                                    format!(
                                        "no matching overload for method '{}' on interface '{}'",
                                        name.name,
                                        self.type_name(&object_ctx.data_type)
                                    ),
                                );
                                return None;
                            }
                        };

                        // Emit conversion instructions for arguments
                        for conv in conversions.into_iter().flatten() {
                            self.emit_conversion(&conv);
                        }

                        // Emit interface method call instruction
                        self.bytecode.emit(Instruction::CallInterfaceMethod(
                            object_ctx.data_type.type_id.as_u32(),
                            method_index as u32,
                        ));

                        // Interface method calls return rvalues
                        Some(ExprContext::rvalue(sig.return_type.clone()))
                    }
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            member.span,
                            format!(
                                "type '{}' does not support method calls",
                                self.type_name(&object_ctx.data_type)
                            ),
                        );
                        None
                    }
                }
            }
        }
    }

    /// Type checks a postfix expression.
    /// x++ and x-- require mutable lvalues and produce rvalues.
    pub(super) fn check_postfix(&mut self, postfix: &PostfixExpr<'ast>) -> Option<ExprContext> {
        let operand_ctx = self.check_expr(postfix.operand)?;

        // Try operator overload first
        let operator = match postfix.op {
            PostfixOp::PostInc => OperatorBehavior::OpPostInc,
            PostfixOp::PostDec => OperatorBehavior::OpPostDec,
        };

        if let Some(result_type) = self.try_unary_operator_overload(
            operator,
            &operand_ctx.data_type,
            postfix.span,
        ) {
            // Operator overloads still require lvalue check
            if !operand_ctx.is_lvalue {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    postfix.span,
                    format!("{} requires an lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
                );
                return None;
            }
            if !operand_ctx.is_mutable {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    postfix.span,
                    format!("{} requires a mutable lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
                );
                return None;
            }
            return Some(ExprContext::rvalue(result_type));
        }

        // Fall back to primitive postfix operators
        // Post-increment/decrement require mutable lvalue
        if !operand_ctx.is_lvalue {
            self.error(
                SemanticErrorKind::InvalidOperation,
                postfix.span,
                format!("{} requires an lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
            );
            return None;
        }
        if !operand_ctx.is_mutable {
            self.error(
                SemanticErrorKind::InvalidOperation,
                postfix.span,
                format!("{} requires a mutable lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
            );
            return None;
        }

        match postfix.op {
            PostfixOp::PostInc => {
                self.bytecode.emit(Instruction::PostIncrement);
            }
            PostfixOp::PostDec => {
                self.bytecode.emit(Instruction::PostDecrement);
            }
        }

        // Returns rvalue of the operand's type
        Some(ExprContext::rvalue(operand_ctx.data_type))
    }

    /// Type checks a cast expression.
    /// Casts produce rvalues.
    ///
    /// In AngelScript, `cast<Type>(expr)` is a handle cast operation that:
    /// - Always produces a handle to the target type (Type@)
    /// - Works for any object handle to any class/interface handle
    /// - Returns null at runtime if the object doesn't implement the target type
    pub(super) fn check_cast(&mut self, cast: &CastExpr<'ast>) -> Option<ExprContext> {
        let expr_ctx = self.check_expr(cast.expr)?;
        let mut target_type = self.resolve_type_expr(&cast.target_type)?;

        // The cast<> syntax in AngelScript is a handle cast operation.
        // If the target type is a class or interface, it's implicitly a handle.
        let target_typedef = self.context.get_type(target_type.type_id);
        if matches!(target_typedef, TypeDef::Class { .. } | TypeDef::Interface { .. }) {
            target_type.is_handle = true;
        }

        // Check if conversion is valid
        if let Some(conversion) = expr_ctx.data_type.can_convert_to(&target_type, self.context) {
            // Emit the appropriate conversion instruction
            self.emit_conversion(&conversion);
            Some(ExprContext::rvalue(target_type))
        } else if self.is_handle_to_handle_cast(&expr_ctx.data_type, &target_type) {
            // Handle-to-handle casts are always allowed at compile time.
            // At runtime, they return null if the object doesn't implement the target type.
            // This supports patterns like: cast<IDamageable>(entity)
            self.bytecode.emit(Instruction::Cast(target_type.type_id));
            Some(ExprContext::rvalue(target_type))
        } else {
            self.error(
                SemanticErrorKind::TypeMismatch,
                cast.span,
                format!(
                    "cannot convert from '{}' to '{}'",
                    self.type_name(&expr_ctx.data_type),
                    self.type_name(&target_type)
                ),
            );
            None
        }
    }

    /// Check if this is a valid handle-to-handle cast.
    /// In AngelScript, any object handle can be cast to any class/interface handle.
    /// The cast succeeds at runtime if the actual object implements the target type.
    pub(super) fn is_handle_to_handle_cast(&self, source: &DataType, target: &DataType) -> bool {
        // Both must be handles
        if !source.is_handle || !target.is_handle {
            return false;
        }

        // Source must be a class or interface
        let source_typedef = self.context.get_type(source.type_id);
        let source_is_object = matches!(
            source_typedef,
            TypeDef::Class { .. } | TypeDef::Interface { .. }
        );

        // Target must be a class or interface
        let target_typedef = self.context.get_type(target.type_id);
        let target_is_object = matches!(
            target_typedef,
            TypeDef::Class { .. } | TypeDef::Interface { .. }
        );

        source_is_object && target_is_object
    }

    /// Type checks a lambda expression.
    /// Lambdas produce rvalues (function references).
    pub(super) fn check_lambda(&mut self, lambda: &LambdaExpr<'ast>) -> Option<ExprContext> {
        // Get expected funcdef type from context (set by check_call or assignment)
        let funcdef_type_id = match self.expected_funcdef_type {
            Some(type_id) => type_id,
            None => {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    lambda.span,
                    "cannot infer lambda type - explicit funcdef context required",
                );
                return None;
            }
        };

        // Get funcdef signature
        let funcdef = self.context.get_type(funcdef_type_id);
        let (expected_params, expected_return) = match funcdef {
            TypeDef::Funcdef { params, return_type, .. } => (params, return_type),
            _ => {
                self.error(
                    SemanticErrorKind::InternalError,
                    lambda.span,
                    "expected funcdef type for lambda",
                );
                return None;
            }
        };

        // Validate parameter count
        if lambda.params.len() != expected_params.len() {
            self.error(
                SemanticErrorKind::TypeMismatch,
                lambda.span,
                format!(
                    "lambda parameter count mismatch: expected {}, got {}",
                    expected_params.len(),
                    lambda.params.len()
                ),
            );
            return None;
        }

        // Validate explicit parameter types if provided
        for (i, (lambda_param, expected_param)) in
            lambda.params.iter().zip(expected_params.iter()).enumerate()
        {
            if let Some(param_ty) = &lambda_param.ty {
                let mut explicit_type = self.resolve_type_expr(&param_ty.ty)?;

                // Apply ref modifier from parameter declaration
                explicit_type.ref_modifier = match param_ty.ref_kind {
                    crate::ast::RefKind::None => crate::semantic::RefModifier::None,
                    crate::ast::RefKind::Ref => crate::semantic::RefModifier::InOut, // Plain & defaults to inout
                    crate::ast::RefKind::RefIn => crate::semantic::RefModifier::In,
                    crate::ast::RefKind::RefOut => crate::semantic::RefModifier::Out,
                    crate::ast::RefKind::RefInOut => crate::semantic::RefModifier::InOut,
                };

                // Validate base type matches
                if explicit_type.type_id != expected_param.type_id {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        lambda_param.span,
                        format!("lambda parameter {} type mismatch: expected '{}', found '{}'",
                            i,
                            self.type_name(expected_param),
                            self.type_name(&explicit_type)),
                    );
                    return None;
                }

                // Validate reference modifier matches
                if explicit_type.ref_modifier != expected_param.ref_modifier {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        lambda_param.span,
                        format!("lambda parameter {} reference modifier mismatch", i),
                    );
                    return None;
                }

                // Validate handle modifier matches
                if explicit_type.is_handle != expected_param.is_handle {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        lambda_param.span,
                        format!("lambda parameter {} handle modifier mismatch", i),
                    );
                    return None;
                }
            }
        }

        // Validate return type if specified
        if let Some(ret_ty) = &lambda.return_type {
            let explicit_return = self.resolve_type_expr(&ret_ty.ty)?;
            if explicit_return.type_id != expected_return.type_id {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    lambda.span,
                    "lambda return type mismatch",
                );
                return None;
            }
        }

        // Allocate FunctionId for this lambda
        let lambda_id = self.next_lambda_id;
        self.next_lambda_id += 1;

        // Capture all variables in current scope
        let captured_vars = self.local_scope.capture_all_variables();

        // Build parameters for compile_block: funcdef params + captured vars
        let mut all_vars = Vec::new();
        for (i, param) in lambda.params.iter().enumerate() {
            let param_name = param.name
                .map(|id| id.name.to_string())
                .unwrap_or_else(|| format!("_param{}", i));
            all_vars.push((param_name, expected_params[i].clone()));
        }
        for cap in &captured_vars {
            all_vars.push((cap.name.clone(), cap.data_type.clone()));
        }

        // ✨ COMPILE LAMBDA IMMEDIATELY using compile_block
        let compiled = FunctionCompiler::compile_block(
            self.context,
            expected_return.clone(),
            &all_vars,
            lambda.body,
        );

        // Store compiled bytecode in compiled_functions map
        self.compiled_functions.insert(FunctionId(lambda_id), compiled.bytecode);

        // Merge errors from lambda compilation
        self.errors.extend(compiled.errors);

        // Emit FuncPtr instruction to push lambda handle onto stack
        self.bytecode.emit(Instruction::FuncPtr(lambda_id));

        // Return funcdef handle type (rvalue)
        Some(ExprContext::rvalue(DataType::with_handle(
            funcdef_type_id,
            false,
        )))
    }

    /// Type checks an initializer list.
    /// Initializer lists produce rvalues (newly constructed arrays/objects).
    ///
    /// Init lists require an explicit target type from context (e.g., variable declaration).
    /// The target type must have `list_factory` (for reference types) or `list_construct`
    /// (for value types) behavior registered.
    ///
    /// Examples:
    /// - `array<int> arr = {1, 2, 3}` - target is array<int>, uses list_factory
    /// - `StringBuffer sb = {"hello", "world"}` - target is StringBuffer, uses list behavior
    /// - `MyVec3 v = {1.0, 2.0, 3.0}` - target is MyVec3 (value type), uses list_construct
    pub(super) fn check_init_list(&mut self, init_list: &InitListExpr<'ast>) -> Option<ExprContext> {
        use crate::ast::InitElement;

        // Init lists require an explicit target type from context
        let target_type_id = match self.expected_init_list_target {
            Some(id) => id,
            None => {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    init_list.span,
                    "initializer list requires explicit target type (e.g., 'array<int> arr = {...}')".to_string(),
                );
                return None;
            }
        };

        // Look up behaviors for the target type
        let behaviors = match self.context.get_behaviors(target_type_id) {
            Some(b) => b.clone(),
            None => {
                let type_name = self.context.get_type(target_type_id).name();
                self.error(
                    SemanticErrorKind::MissingListBehavior,
                    init_list.span,
                    format!(
                        "Type '{}' does not support initialization list syntax (no behaviors registered)",
                        type_name
                    ),
                );
                return None;
            }
        };

        // Check for list_factory (reference types) or list_construct (value types)
        // The presence of a behavior determines which one to use
        let (list_func_id, is_reference_type) = if let Some(factory) = behaviors.list_factory {
            (factory, true)
        } else if let Some(construct) = behaviors.list_construct {
            (construct, false)
        } else {
            let type_name = self.context.get_type(target_type_id).name();
            self.error(
                SemanticErrorKind::MissingListBehavior,
                init_list.span,
                format!(
                    "Type '{}' does not have list_factory or list_construct behavior for initialization list syntax",
                    type_name
                ),
            );
            return None;
        };

        // Type check all elements and collect their types
        let mut element_types = Vec::with_capacity(init_list.elements.len());

        for element in init_list.elements {
            let elem_ctx = match element {
                InitElement::Expr(expr) => self.check_expr(expr)?,
                InitElement::InitList(nested) => self.check_init_list(nested)?,
            };
            element_types.push(elem_ctx.data_type);
        }

        // Elements are on the stack from check_expr calls above.
        // Push the count and call the list factory/constructor.
        //
        // Stack before: [elem0] [elem1] ... [elemN-1]
        // Stack after:  [elem0] [elem1] ... [elemN-1] [count]
        // List factory/constructor pops count + elements and pushes result.
        self.bytecode.emit(Instruction::PushInt(element_types.len() as i64));
        self.bytecode.emit(Instruction::CallConstructor {
            type_id: target_type_id.as_u32(),
            func_id: list_func_id.as_u32(),
        });

        // Return the appropriate type
        // Reference types return a handle, value types return the value directly
        if is_reference_type {
            Some(ExprContext::rvalue(DataType::with_handle(target_type_id, false)))
        } else {
            Some(ExprContext::rvalue(DataType::simple(target_type_id)))
        }
    }

    /// Type checks a parenthesized expression.
    /// Parentheses preserve the lvalue-ness of the inner expression.
    pub(super) fn check_paren(&mut self, paren: &ParenExpr<'ast>) -> Option<ExprContext> {
        self.check_expr(paren.expr)
    }
}
