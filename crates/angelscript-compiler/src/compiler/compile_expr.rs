//! Expression compilation — compiles AST expressions to bytecode.

use angelscript_core::opcode::OpCode;
use angelscript_core::{DataType, PrimitiveType};
use angelscript_parser::ast::{self, AssignOp, BinOp, ExprKind, PostfixOp, UnaryOp};

use super::{Compiler, ConstantValue, ExprResult, PrimCategory};

impl<'a> Compiler<'a> {
    /// Compile an expression, returning where the result lives.
    pub(crate) fn compile_expression(&mut self, expr: &ast::Expr) -> ExprResult {
        match &expr.kind {
            ExprKind::IntLiteral(val) => self.compile_int_literal(*val),
            ExprKind::FloatLiteral(val) => self.compile_float_literal(*val),
            ExprKind::DoubleLiteral(val) => self.compile_double_literal(*val),
            ExprKind::BoolLiteral(val) => self.compile_bool_literal(*val),
            ExprKind::Null => self.compile_null(),
            ExprKind::Identifier(id) => self.compile_identifier(id),
            ExprKind::BinaryOp { op, lhs, rhs } => self.compile_binary_op(*op, lhs, rhs),
            ExprKind::UnaryOp { op, operand } => self.compile_unary_op(*op, operand),
            ExprKind::PostfixOp { op, operand } => self.compile_postfix_op(*op, operand),
            ExprKind::Assign { target, op, value } => self.compile_assign(target, *op, value),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.compile_ternary(condition, then_expr, else_expr),
            ExprKind::FunctionCall { callee, args } => self.compile_function_call(callee, args),
            ExprKind::Void => ExprResult {
                data_type: DataType::void(),
                is_lvalue: false,
                is_constant: false,
                is_temporary: false,
                stack_offset: 0,
                constant_value: None,
            },
            // For now, emit a placeholder for unsupported expressions.
            _ => {
                self.error(format!("unsupported expression kind: {:?}", expr.kind));
                ExprResult {
                    data_type: DataType::void(),
                    is_lvalue: false,
                    is_constant: false,
                    is_temporary: false,
                    stack_offset: 0,
                    constant_value: None,
                }
            }
        }
    }

    fn compile_int_literal(&mut self, val: i64) -> ExprResult {
        if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
            let dt = DataType::primitive(PrimitiveType::Int32);
            let offset = self.variables.alloc_temp(dt.clone());
            // SetV4: w_arg0 = variable offset, dw_arg = value
            self.bytecode
                .emit_w_dw(OpCode::SetV4, offset, val as i32 as u32);
            ExprResult {
                data_type: DataType::primitive(PrimitiveType::Int32),
                is_lvalue: false,
                is_constant: true,
                is_temporary: true,
                stack_offset: offset,
                constant_value: Some(ConstantValue::Int32(val as i32)),
            }
        } else {
            let dt = DataType::primitive(PrimitiveType::Int64);
            let offset = self.variables.alloc_temp(dt.clone());
            // SetV8: w_arg0 = variable offset, qw_arg = value
            self.bytecode.emit_w_qw(OpCode::SetV8, offset, val as u64);
            ExprResult {
                data_type: DataType::primitive(PrimitiveType::Int64),
                is_lvalue: false,
                is_constant: true,
                is_temporary: true,
                stack_offset: offset,
                constant_value: Some(ConstantValue::Int64(val)),
            }
        }
    }

    fn compile_float_literal(&mut self, val: f64) -> ExprResult {
        let dt = DataType::primitive(PrimitiveType::Float);
        let offset = self.variables.alloc_temp(dt.clone());
        let bits = (val as f32).to_bits();
        // SetV4: w_arg0 = variable offset, dw_arg = value
        self.bytecode.emit_w_dw(OpCode::SetV4, offset, bits);
        ExprResult {
            data_type: DataType::primitive(PrimitiveType::Float),
            is_lvalue: false,
            is_constant: true,
            is_temporary: true,
            stack_offset: offset,
            constant_value: Some(ConstantValue::Float(val as f32)),
        }
    }

    fn compile_double_literal(&mut self, val: f64) -> ExprResult {
        let dt = DataType::primitive(PrimitiveType::Double);
        let offset = self.variables.alloc_temp(dt.clone());
        let bits = val.to_bits();
        // SetV8: w_arg0 = variable offset, qw_arg = value
        self.bytecode.emit_w_qw(OpCode::SetV8, offset, bits);
        ExprResult {
            data_type: DataType::primitive(PrimitiveType::Double),
            is_lvalue: false,
            is_constant: true,
            is_temporary: true,
            stack_offset: offset,
            constant_value: Some(ConstantValue::Double(val)),
        }
    }

    fn compile_bool_literal(&mut self, val: bool) -> ExprResult {
        let dt = DataType::primitive(PrimitiveType::Bool);
        let offset = self.variables.alloc_temp(dt.clone());
        // SetV4: w_arg0 = variable offset, dw_arg = value
        self.bytecode
            .emit_w_dw(OpCode::SetV4, offset, if val { 1 } else { 0 });
        ExprResult {
            data_type: DataType::primitive(PrimitiveType::Bool),
            is_lvalue: false,
            is_constant: true,
            is_temporary: true,
            stack_offset: offset,
            constant_value: Some(ConstantValue::Bool(val)),
        }
    }

    fn compile_null(&mut self) -> ExprResult {
        let dt = DataType::void();
        let offset = self
            .variables
            .alloc_temp(DataType::primitive(PrimitiveType::UInt32));
        self.bytecode.emit_w(OpCode::ClrVPtr, offset);
        ExprResult {
            data_type: dt,
            is_lvalue: false,
            is_constant: true,
            is_temporary: true,
            stack_offset: offset,
            constant_value: None,
        }
    }

    fn compile_identifier(&mut self, id: &ast::ScopedIdentifier) -> ExprResult {
        // Look up as a local variable first.
        if id.scopes.is_empty() && !id.is_global {
            if let Some(var) = self.variables.lookup(&id.name) {
                return ExprResult {
                    data_type: var.data_type.clone(),
                    is_lvalue: true,
                    is_constant: false,
                    is_temporary: false,
                    stack_offset: var.offset,
                    constant_value: None,
                };
            }
        }

        // Look up as a global variable.
        let qn = if id.is_global || !id.scopes.is_empty() {
            angelscript_core::QualifiedName::new(id.scopes.clone(), &id.name)
        } else {
            // Try to resolve in current namespace context.
            match self.registry.resolve_name(&id.name, &self.namespace) {
                Some(resolved) => resolved,
                None => angelscript_core::QualifiedName::global(&id.name),
            }
        };

        // Check for enum values.
        // TODO: look up enum values in registry.

        // Check for global variables.
        if let Some(global) = self.registry.get_global(&qn) {
            let dt = global.data_type.clone();
            let offset = self.variables.alloc_temp(dt.clone());
            // Load global address into register, read through it.
            // For now emit a placeholder.
            self.bytecode.emit_dw(OpCode::LDG, qn.type_id().0 as u32);
            self.bytecode.emit_w(OpCode::RDR4, offset);
            return ExprResult {
                data_type: dt,
                is_lvalue: true,
                is_constant: false,
                is_temporary: false,
                stack_offset: offset,
                constant_value: None,
            };
        }

        self.error(format!("undefined identifier: '{}'", id.name));
        ExprResult {
            data_type: DataType::void(),
            is_lvalue: false,
            is_constant: false,
            is_temporary: false,
            stack_offset: 0,
            constant_value: None,
        }
    }

    fn compile_binary_op(&mut self, op: BinOp, lhs: &ast::Expr, rhs: &ast::Expr) -> ExprResult {
        let left = self.compile_expression(lhs);
        let right = self.compile_expression(rhs);

        match op {
            // Comparison operators produce a bool via CMPx + test.
            BinOp::Eq
            | BinOp::NotEq
            | BinOp::Less
            | BinOp::LessEq
            | BinOp::Greater
            | BinOp::GreaterEq => {
                return self.compile_comparison(op, &left, &right);
            }
            // Logical operators.
            BinOp::LogicAnd | BinOp::LogicOr => {
                return self.compile_logical_op(op, &left, &right);
            }
            _ => {}
        }

        // Determine the common type for arithmetic.
        let result_type = self.common_type(&left.data_type, &right.data_type);
        let prim = result_type.as_primitive().unwrap_or(PrimitiveType::Int32);
        let category = Self::primitive_category(prim);

        // Convert operands if needed.
        self.emit_conversion(&left.data_type, &result_type, left.stack_offset);
        self.emit_conversion(&right.data_type, &result_type, right.stack_offset);

        let result_offset = self.variables.alloc_temp(result_type.clone());

        // Copy the left operand to the result temp first, because arithmetic
        // ops mutate the destination in-place: OP dst, src → dst = dst OP src.
        let copy_op = match category {
            PrimCategory::Int64 | PrimCategory::Double => OpCode::CpyVtoV8,
            _ => OpCode::CpyVtoV4,
        };
        self.bytecode
            .emit_ww(copy_op, result_offset, left.stack_offset);

        match op {
            BinOp::Add => {
                let opc = Self::arith_op(
                    category,
                    OpCode::ADDi,
                    OpCode::ADDf,
                    OpCode::ADDd,
                    OpCode::ADDi64,
                );
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::Sub => {
                let opc = Self::arith_op(
                    category,
                    OpCode::SUBi,
                    OpCode::SUBf,
                    OpCode::SUBd,
                    OpCode::SUBi64,
                );
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::Mul => {
                let opc = Self::arith_op(
                    category,
                    OpCode::MULi,
                    OpCode::MULf,
                    OpCode::MULd,
                    OpCode::MULi64,
                );
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::Div => {
                let opc = Self::arith_op(
                    category,
                    OpCode::DIVi,
                    OpCode::DIVf,
                    OpCode::DIVd,
                    OpCode::DIVi64,
                );
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::Mod => {
                let opc = Self::arith_op(
                    category,
                    OpCode::MODi,
                    OpCode::MODf,
                    OpCode::MODd,
                    OpCode::MODi64,
                );
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::Pow => {
                let opc = match category {
                    PrimCategory::Int32 => OpCode::POWi,
                    PrimCategory::Float => OpCode::POWf,
                    PrimCategory::Double => OpCode::POWd,
                    PrimCategory::Int64 => OpCode::POWi64,
                    PrimCategory::Void => OpCode::POWi,
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::BitAnd => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BAND64
                } else {
                    OpCode::BAND
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::BitOr => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BOR64
                } else {
                    OpCode::BOR
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::BitXor => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BXOR64
                } else {
                    OpCode::BXOR
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::ShiftLeft => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BSLL64
                } else {
                    OpCode::BSLL
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::ShiftRight => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BSRL64
                } else {
                    OpCode::BSRL
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::ShiftRightArith => {
                let opc = if category == PrimCategory::Int64 {
                    OpCode::BSRA64
                } else {
                    OpCode::BSRA
                };
                self.bytecode
                    .emit_ww(opc, result_offset, right.stack_offset);
            }
            BinOp::LogicXor => {
                // XOR of booleans: (a != 0) ^ (b != 0)
                self.bytecode
                    .emit_ww(OpCode::BXOR, result_offset, right.stack_offset);
            }
            // Other operators already handled above.
            _ => {
                self.error(format!("unsupported binary operator: {:?}", op));
            }
        }

        // Use the right copy instruction based on result size.
        ExprResult {
            data_type: result_type,
            is_lvalue: false,
            is_constant: false,
            is_temporary: true,
            stack_offset: result_offset,
            constant_value: None,
        }
    }

    fn compile_comparison(
        &mut self,
        op: BinOp,
        left: &ExprResult,
        right: &ExprResult,
    ) -> ExprResult {
        let common = self.common_type(&left.data_type, &right.data_type);
        let prim = common.as_primitive().unwrap_or(PrimitiveType::Int32);
        let category = Self::primitive_category(prim);

        self.emit_conversion(&left.data_type, &common, left.stack_offset);
        self.emit_conversion(&right.data_type, &common, right.stack_offset);

        // Emit comparison instruction.
        let cmp = Self::cmp_op(category);
        self.bytecode
            .emit_ww(cmp, left.stack_offset, right.stack_offset);

        // Emit the test instruction based on comparison kind.
        let test_op = match op {
            BinOp::Eq => OpCode::TZ,         // CMPx returns 0 when equal
            BinOp::NotEq => OpCode::TNZ,     // CMPx returns non-zero when not equal
            BinOp::Less => OpCode::TS,       // CMPx returns negative when less
            BinOp::GreaterEq => OpCode::TNS, // not negative = >=
            BinOp::Greater => OpCode::TP,    // CMPx returns positive when greater
            BinOp::LessEq => OpCode::TNP,    // not positive = <=
            _ => OpCode::TZ,
        };
        self.bytecode.emit_ww(test_op, 0, 0);

        let result_offset = self
            .variables
            .alloc_temp(DataType::primitive(PrimitiveType::Bool));
        self.bytecode.emit_w(OpCode::CpyRtoV4, result_offset);

        ExprResult {
            data_type: DataType::primitive(PrimitiveType::Bool),
            is_lvalue: false,
            is_constant: false,
            is_temporary: true,
            stack_offset: result_offset,
            constant_value: None,
        }
    }

    fn compile_logical_op(
        &mut self,
        op: BinOp,
        left: &ExprResult,
        right: &ExprResult,
    ) -> ExprResult {
        // For short-circuit evaluation, we should evaluate the left side
        // then conditionally evaluate right. For now we evaluate both eagerly.
        let result_offset = self
            .variables
            .alloc_temp(DataType::primitive(PrimitiveType::Bool));

        // Copy left to result first (in-place ops modify dst).
        self.bytecode
            .emit_ww(OpCode::CpyVtoV4, result_offset, left.stack_offset);

        match op {
            BinOp::LogicAnd => {
                // result = left & right (bitwise works for bools)
                self.bytecode
                    .emit_ww(OpCode::BAND, result_offset, right.stack_offset);
            }
            BinOp::LogicOr => {
                self.bytecode
                    .emit_ww(OpCode::BOR, result_offset, right.stack_offset);
            }
            _ => unreachable!(),
        }

        ExprResult {
            data_type: DataType::primitive(PrimitiveType::Bool),
            is_lvalue: false,
            is_constant: false,
            is_temporary: true,
            stack_offset: result_offset,
            constant_value: None,
        }
    }

    fn compile_unary_op(&mut self, op: UnaryOp, operand: &ast::Expr) -> ExprResult {
        let val = self.compile_expression(operand);

        match op {
            UnaryOp::Neg => {
                let prim = val.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);
                let neg_op = match category {
                    PrimCategory::Int32 => OpCode::NEGi,
                    PrimCategory::Float => OpCode::NEGf,
                    PrimCategory::Double => OpCode::NEGd,
                    PrimCategory::Int64 => OpCode::NEGi64,
                    PrimCategory::Void => OpCode::NEGi,
                };
                let result_offset = self.variables.alloc_temp(val.data_type.clone());
                // Copy value to result temp, then negate in-place.
                let copy_op = match category {
                    PrimCategory::Int64 | PrimCategory::Double => OpCode::CpyVtoV8,
                    _ => OpCode::CpyVtoV4,
                };
                self.bytecode
                    .emit_ww(copy_op, result_offset, val.stack_offset);
                self.bytecode.emit_w(neg_op, result_offset);
                ExprResult {
                    data_type: val.data_type,
                    is_lvalue: false,
                    is_constant: false,
                    is_temporary: true,
                    stack_offset: result_offset,
                    constant_value: None,
                }
            }
            UnaryOp::Pos => {
                // No-op for positive.
                val
            }
            UnaryOp::LogicNot => {
                let result_offset = self
                    .variables
                    .alloc_temp(DataType::primitive(PrimitiveType::Bool));
                // NOT operates on the register, so load value first.
                self.bytecode.emit_w(OpCode::CpyVtoR4, val.stack_offset);
                self.bytecode.emit_op(OpCode::NOT);
                self.bytecode.emit_w(OpCode::CpyRtoV4, result_offset);
                ExprResult {
                    data_type: DataType::primitive(PrimitiveType::Bool),
                    is_lvalue: false,
                    is_constant: false,
                    is_temporary: true,
                    stack_offset: result_offset,
                    constant_value: None,
                }
            }
            UnaryOp::BitNot => {
                let prim = val.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);
                let not_op = if category == PrimCategory::Int64 {
                    OpCode::BNOT64
                } else {
                    OpCode::BNOT
                };
                let result_offset = self.variables.alloc_temp(val.data_type.clone());
                // Copy value to result temp, then NOT in-place.
                let copy_op = if category == PrimCategory::Int64 {
                    OpCode::CpyVtoV8
                } else {
                    OpCode::CpyVtoV4
                };
                self.bytecode
                    .emit_ww(copy_op, result_offset, val.stack_offset);
                self.bytecode.emit_w(not_op, result_offset);
                ExprResult {
                    data_type: val.data_type,
                    is_lvalue: false,
                    is_constant: false,
                    is_temporary: true,
                    stack_offset: result_offset,
                    constant_value: None,
                }
            }
            UnaryOp::PreInc | UnaryOp::PreDec => {
                if !val.is_lvalue {
                    self.error("pre-increment/decrement requires an lvalue");
                    return val;
                }
                let prim = val.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);
                let op_code = match (op, category) {
                    (UnaryOp::PreInc, PrimCategory::Int32) => OpCode::IncVi,
                    (UnaryOp::PreDec, PrimCategory::Int32) => OpCode::DecVi,
                    _ => {
                        self.error("pre-inc/dec only supported for int32 variables");
                        return val;
                    }
                };
                self.bytecode.emit_w(op_code, val.stack_offset);
                val
            }
            UnaryOp::HandleOf => {
                // @ operator — for now just pass through.
                val
            }
        }
    }

    fn compile_postfix_op(&mut self, op: PostfixOp, operand: &ast::Expr) -> ExprResult {
        let val = self.compile_expression(operand);

        if !val.is_lvalue {
            self.error("postfix increment/decrement requires an lvalue");
            return val;
        }

        // Save the pre-increment value.
        let result_offset = self.variables.alloc_temp(val.data_type.clone());
        self.bytecode
            .emit_ww(OpCode::CpyVtoV4, result_offset, val.stack_offset);

        let prim = val.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
        let category = Self::primitive_category(prim);

        match (op, category) {
            (PostfixOp::Inc, PrimCategory::Int32) => {
                self.bytecode.emit_w(OpCode::IncVi, val.stack_offset);
            }
            (PostfixOp::Dec, PrimCategory::Int32) => {
                self.bytecode.emit_w(OpCode::DecVi, val.stack_offset);
            }
            _ => {
                self.error("postfix inc/dec only supported for int32 variables");
            }
        }

        ExprResult {
            data_type: val.data_type,
            is_lvalue: false,
            is_constant: false,
            is_temporary: true,
            stack_offset: result_offset,
            constant_value: None,
        }
    }

    fn compile_assign(
        &mut self,
        target: &ast::Expr,
        op: AssignOp,
        value: &ast::Expr,
    ) -> ExprResult {
        let lhs = self.compile_expression(target);

        if !lhs.is_lvalue {
            self.error("assignment target is not an lvalue");
        }

        let rhs = self.compile_expression(value);

        match op {
            AssignOp::Assign => {
                // Simple copy.
                let prim = lhs.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);
                let cpy_op = match category {
                    PrimCategory::Int64 | PrimCategory::Double => OpCode::CpyVtoV8,
                    _ => OpCode::CpyVtoV4,
                };
                self.emit_conversion(&rhs.data_type, &lhs.data_type, rhs.stack_offset);
                self.bytecode
                    .emit_ww(cpy_op, lhs.stack_offset, rhs.stack_offset);
            }
            AssignOp::AddAssign
            | AssignOp::SubAssign
            | AssignOp::MulAssign
            | AssignOp::DivAssign
            | AssignOp::ModAssign => {
                let prim = lhs.data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);

                self.emit_conversion(&rhs.data_type, &lhs.data_type, rhs.stack_offset);

                let arith_op = match op {
                    AssignOp::AddAssign => Self::arith_op(
                        category,
                        OpCode::ADDi,
                        OpCode::ADDf,
                        OpCode::ADDd,
                        OpCode::ADDi64,
                    ),
                    AssignOp::SubAssign => Self::arith_op(
                        category,
                        OpCode::SUBi,
                        OpCode::SUBf,
                        OpCode::SUBd,
                        OpCode::SUBi64,
                    ),
                    AssignOp::MulAssign => Self::arith_op(
                        category,
                        OpCode::MULi,
                        OpCode::MULf,
                        OpCode::MULd,
                        OpCode::MULi64,
                    ),
                    AssignOp::DivAssign => Self::arith_op(
                        category,
                        OpCode::DIVi,
                        OpCode::DIVf,
                        OpCode::DIVd,
                        OpCode::DIVi64,
                    ),
                    AssignOp::ModAssign => Self::arith_op(
                        category,
                        OpCode::MODi,
                        OpCode::MODf,
                        OpCode::MODd,
                        OpCode::MODi64,
                    ),
                    _ => unreachable!(),
                };
                // Compound assignment: OP modifies lhs in-place.
                self.bytecode
                    .emit_ww(arith_op, lhs.stack_offset, rhs.stack_offset);
            }
            _ => {
                self.error(format!("unsupported compound assignment: {:?}", op));
            }
        }

        ExprResult {
            data_type: lhs.data_type,
            is_lvalue: true,
            is_constant: false,
            is_temporary: false,
            stack_offset: lhs.stack_offset,
            constant_value: None,
        }
    }

    fn compile_ternary(
        &mut self,
        condition: &ast::Expr,
        then_expr: &ast::Expr,
        else_expr: &ast::Expr,
    ) -> ExprResult {
        let cond = self.compile_expression(condition);

        // Load condition into register and test.
        self.bytecode.emit_w(OpCode::CpyVtoR4, cond.stack_offset);

        let else_label = self.bytecode.new_label();
        let end_label = self.bytecode.new_label();

        // Jump to else if condition is zero (false).
        self.bytecode.emit_jump(OpCode::JZ, else_label);

        // Then branch.
        let then_result = self.compile_expression(then_expr);
        let result_offset = self.variables.alloc_temp(then_result.data_type.clone());
        let prim = then_result
            .data_type
            .as_primitive()
            .unwrap_or(PrimitiveType::Int32);
        let category = Self::primitive_category(prim);
        let cpy_op = match category {
            PrimCategory::Int64 | PrimCategory::Double => OpCode::CpyVtoV8,
            _ => OpCode::CpyVtoV4,
        };
        self.bytecode
            .emit_ww(cpy_op, result_offset, then_result.stack_offset);
        self.bytecode.emit_jump(OpCode::JMP, end_label);

        // Else branch.
        self.bytecode.place_label(else_label);
        let else_result = self.compile_expression(else_expr);
        self.emit_conversion(
            &else_result.data_type,
            &then_result.data_type,
            else_result.stack_offset,
        );
        self.bytecode
            .emit_ww(cpy_op, result_offset, else_result.stack_offset);

        self.bytecode.place_label(end_label);

        ExprResult {
            data_type: then_result.data_type,
            is_lvalue: false,
            is_constant: false,
            is_temporary: true,
            stack_offset: result_offset,
            constant_value: None,
        }
    }

    fn compile_function_call(&mut self, callee: &ast::Expr, args: &[ast::FuncArg]) -> ExprResult {
        // For now, handle simple identifier calls.
        let func_name = match &callee.kind {
            ExprKind::Identifier(id) => {
                if id.is_global || !id.scopes.is_empty() {
                    angelscript_core::QualifiedName::new(id.scopes.clone(), &id.name)
                } else {
                    angelscript_core::QualifiedName::global(&id.name)
                }
            }
            _ => {
                self.error("only simple function calls are currently supported");
                return ExprResult {
                    data_type: DataType::void(),
                    is_lvalue: false,
                    is_constant: false,
                    is_temporary: false,
                    stack_offset: 0,
                    constant_value: None,
                };
            }
        };

        // Look up function in registry.
        let func_ids = self.registry.get_functions_by_name(&func_name);
        if func_ids.is_empty() {
            self.error(format!("undefined function: '{}'", func_name));
            return ExprResult {
                data_type: DataType::void(),
                is_lvalue: false,
                is_constant: false,
                is_temporary: false,
                stack_offset: 0,
                constant_value: None,
            };
        }

        // For now, just use the first overload.
        let func_id = func_ids[0];
        let return_type = self
            .registry
            .get_function(func_id)
            .map(|f| f.signature.return_type.clone())
            .unwrap_or_else(DataType::void);

        // Compile arguments and push them.
        for arg in args {
            let result = self.compile_expression(&arg.expr);
            // Push the argument value onto the stack.
            let prim = result
                .data_type
                .as_primitive()
                .unwrap_or(PrimitiveType::Int32);
            let category = Self::primitive_category(prim);
            match category {
                PrimCategory::Int64 | PrimCategory::Double => {
                    self.bytecode.emit_w(OpCode::PshV8, result.stack_offset);
                }
                _ => {
                    self.bytecode.emit_w(OpCode::PshV4, result.stack_offset);
                }
            }
        }

        // Emit the CALL instruction.
        self.bytecode.emit_dw(OpCode::CALL, func_id.0);

        // Allocate result.
        if !return_type.is_void() {
            let result_offset = self.variables.alloc_temp(return_type.clone());
            self.bytecode.emit_w(OpCode::CpyRtoV4, result_offset);
            ExprResult {
                data_type: return_type,
                is_lvalue: false,
                is_constant: false,
                is_temporary: true,
                stack_offset: result_offset,
                constant_value: None,
            }
        } else {
            ExprResult {
                data_type: DataType::void(),
                is_lvalue: false,
                is_constant: false,
                is_temporary: false,
                stack_offset: 0,
                constant_value: None,
            }
        }
    }

    /// Determine the common type for binary operations (type promotion).
    fn common_type(&self, a: &DataType, b: &DataType) -> DataType {
        let a_prim = a.as_primitive().unwrap_or(PrimitiveType::Int32);
        let b_prim = b.as_primitive().unwrap_or(PrimitiveType::Int32);

        if a_prim == b_prim {
            return a.clone();
        }

        let a_cat = Self::primitive_category(a_prim);
        let b_cat = Self::primitive_category(b_prim);

        // Promotion order: Int32 < Float < Int64 < Double
        let result_cat = match (a_cat, b_cat) {
            (PrimCategory::Double, _) | (_, PrimCategory::Double) => PrimCategory::Double,
            (PrimCategory::Int64, PrimCategory::Float)
            | (PrimCategory::Float, PrimCategory::Int64) => PrimCategory::Double,
            (PrimCategory::Int64, _) | (_, PrimCategory::Int64) => PrimCategory::Int64,
            (PrimCategory::Float, _) | (_, PrimCategory::Float) => PrimCategory::Float,
            _ => PrimCategory::Int32,
        };

        match result_cat {
            PrimCategory::Int32 => DataType::primitive(PrimitiveType::Int32),
            PrimCategory::Int64 => DataType::primitive(PrimitiveType::Int64),
            PrimCategory::Float => DataType::primitive(PrimitiveType::Float),
            PrimCategory::Double => DataType::primitive(PrimitiveType::Double),
            PrimCategory::Void => DataType::void(),
        }
    }
}
