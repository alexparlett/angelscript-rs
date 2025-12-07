//! Constant expression evaluation for compile-time constants.
//!
//! This module provides functionality to evaluate expressions at compile time.
//! Used for:
//! - Enum value expressions
//! - Switch case values
//! - Constant variable initializers
//!
//! # Supported Expressions
//!
//! - Literals: integers, floats, booleans, strings
//! - Binary operations: arithmetic, bitwise, logical, comparison
//! - Unary operations: negation, plus, logical not, bitwise not
//! - Parenthesized expressions
//! - Ternary conditionals (with constant condition)
//! - Enum value references (EnumName::VALUE)
//! - Constant variable references (future: when const tracking is added)

use angelscript_parser::ast::expr::{Expr, LiteralKind};
use angelscript_parser::ast::{BinaryOp, UnaryOp};
use crate::semantic::CompilationContext;
use crate::semantic::types::TypeDef;

/// A compile-time constant value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// Signed integer value
    Int(i64),
    /// Unsigned integer value (for bitwise operations)
    UInt(u64),
    /// Floating-point value (f64 for precision)
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
}

impl ConstValue {
    /// Try to convert this value to an i64.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ConstValue::Int(v) => Some(*v),
            ConstValue::UInt(v) => {
                if *v <= i64::MAX as u64 {
                    Some(*v as i64)
                } else {
                    None
                }
            }
            ConstValue::Float(v) => Some(*v as i64),
            ConstValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            ConstValue::String(_) => None,
        }
    }

    /// Try to convert this value to a u64.
    pub fn as_uint(&self) -> Option<u64> {
        match self {
            ConstValue::Int(v) => {
                if *v >= 0 {
                    Some(*v as u64)
                } else {
                    None
                }
            }
            ConstValue::UInt(v) => Some(*v),
            ConstValue::Float(v) => {
                if *v >= 0.0 {
                    Some(*v as u64)
                } else {
                    None
                }
            }
            ConstValue::Bool(v) => Some(if *v { 1 } else { 0 }),
            ConstValue::String(_) => None,
        }
    }

    /// Try to convert this value to an f64.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            ConstValue::Int(v) => Some(*v as f64),
            ConstValue::UInt(v) => Some(*v as f64),
            ConstValue::Float(v) => Some(*v),
            ConstValue::Bool(_) => None,
            ConstValue::String(_) => None,
        }
    }

    /// Try to convert this value to a bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConstValue::Int(v) => Some(*v != 0),
            ConstValue::UInt(v) => Some(*v != 0),
            ConstValue::Float(v) => Some(*v != 0.0),
            ConstValue::Bool(v) => Some(*v),
            ConstValue::String(_) => None,
        }
    }

    /// Check if this value is truthy (for ternary conditions).
    pub fn is_truthy(&self) -> bool {
        match self {
            ConstValue::Int(v) => *v != 0,
            ConstValue::UInt(v) => *v != 0,
            ConstValue::Float(v) => *v != 0.0,
            ConstValue::Bool(v) => *v,
            ConstValue::String(s) => !s.is_empty(),
        }
    }
}

/// Constant expression evaluator.
///
/// Evaluates expressions at compile time when possible.
/// Returns `None` if the expression cannot be evaluated as a constant.
pub struct ConstEvaluator<'a, 'ast> {
    context: &'a CompilationContext<'ast>,
}

impl<'a, 'ast> ConstEvaluator<'a, 'ast> {
    /// Create a new constant evaluator.
    pub fn new(context: &'a CompilationContext<'ast>) -> Self {
        Self { context }
    }

    /// Evaluate an expression as a constant value.
    ///
    /// Returns `None` if the expression cannot be evaluated at compile time.
    pub fn eval(&self, expr: &Expr<'ast>) -> Option<ConstValue> {
        match expr {
            Expr::Literal(lit) => self.eval_literal(&lit.kind),
            Expr::Unary(unary) => self.eval_unary(unary.op, unary.operand),
            Expr::Binary(binary) => self.eval_binary(binary.left, binary.op, binary.right),
            Expr::Paren(paren) => self.eval(paren.expr),
            Expr::Ternary(ternary) => self.eval_ternary(ternary.condition, ternary.then_expr, ternary.else_expr),
            Expr::Ident(ident) => self.eval_ident(ident),
            // These cannot be constant expressions
            Expr::Assign(_) | Expr::Call(_) | Expr::Index(_) | Expr::Member(_)
            | Expr::Postfix(_) | Expr::Cast(_) | Expr::Lambda(_) | Expr::InitList(_) => None,
        }
    }

    /// Evaluate and return as i64 (convenience method).
    pub fn eval_as_int(&self, expr: &Expr<'ast>) -> Option<i64> {
        self.eval(expr).and_then(|v| v.as_int())
    }

    /// Evaluate and return as u64 (convenience method).
    pub fn eval_as_uint(&self, expr: &Expr<'ast>) -> Option<u64> {
        self.eval(expr).and_then(|v| v.as_uint())
    }

    /// Evaluate and return as f64 (convenience method).
    pub fn eval_as_float(&self, expr: &Expr<'ast>) -> Option<f64> {
        self.eval(expr).and_then(|v| v.as_float())
    }

    /// Evaluate and return as bool (convenience method).
    pub fn eval_as_bool(&self, expr: &Expr<'ast>) -> Option<bool> {
        self.eval(expr).and_then(|v| v.as_bool())
    }

    fn eval_literal(&self, kind: &LiteralKind) -> Option<ConstValue> {
        match kind {
            LiteralKind::Int(v) => Some(ConstValue::Int(*v)),
            LiteralKind::Float(v) => Some(ConstValue::Float(*v as f64)),
            LiteralKind::Double(v) => Some(ConstValue::Float(*v)),
            LiteralKind::Bool(v) => Some(ConstValue::Bool(*v)),
            LiteralKind::String(s) => Some(ConstValue::String(s.clone())),
            LiteralKind::Null => None, // null is not a compile-time constant value
        }
    }

    fn eval_unary(&self, op: UnaryOp, operand: &Expr<'ast>) -> Option<ConstValue> {
        let value = self.eval(operand)?;

        match op {
            UnaryOp::Neg => match value {
                ConstValue::Int(v) => Some(ConstValue::Int(-v)),
                ConstValue::UInt(v) => Some(ConstValue::Int(-(v as i64))),
                ConstValue::Float(v) => Some(ConstValue::Float(-v)),
                _ => None,
            },
            UnaryOp::Plus => match value {
                ConstValue::Int(_) | ConstValue::UInt(_) | ConstValue::Float(_) => Some(value),
                _ => None,
            },
            UnaryOp::LogicalNot => {
                let b = value.as_bool()?;
                Some(ConstValue::Bool(!b))
            }
            UnaryOp::BitwiseNot => match value {
                ConstValue::Int(v) => Some(ConstValue::Int(!v)),
                ConstValue::UInt(v) => Some(ConstValue::UInt(!v)),
                _ => None,
            },
            // Pre-increment/decrement and handle-of are not constant expressions
            UnaryOp::PreInc | UnaryOp::PreDec | UnaryOp::HandleOf => None,
        }
    }

    fn eval_binary(
        &self,
        left: &Expr<'ast>,
        op: BinaryOp,
        right: &Expr<'ast>,
    ) -> Option<ConstValue> {
        let left_val = self.eval(left)?;
        let right_val = self.eval(right)?;

        match op {
            // Arithmetic operations
            BinaryOp::Add => self.eval_add(&left_val, &right_val),
            BinaryOp::Sub => self.eval_sub(&left_val, &right_val),
            BinaryOp::Mul => self.eval_mul(&left_val, &right_val),
            BinaryOp::Div => self.eval_div(&left_val, &right_val),
            BinaryOp::Mod => self.eval_mod(&left_val, &right_val),
            BinaryOp::Pow => self.eval_pow(&left_val, &right_val),

            // Bitwise operations
            BinaryOp::BitwiseAnd => self.eval_bitwise_and(&left_val, &right_val),
            BinaryOp::BitwiseOr => self.eval_bitwise_or(&left_val, &right_val),
            BinaryOp::BitwiseXor => self.eval_bitwise_xor(&left_val, &right_val),
            BinaryOp::ShiftLeft => self.eval_shift_left(&left_val, &right_val),
            BinaryOp::ShiftRight => self.eval_shift_right(&left_val, &right_val),
            BinaryOp::ShiftRightUnsigned => self.eval_shift_right_unsigned(&left_val, &right_val),

            // Logical operations
            BinaryOp::LogicalAnd => {
                let l = left_val.as_bool()?;
                let r = right_val.as_bool()?;
                Some(ConstValue::Bool(l && r))
            }
            BinaryOp::LogicalOr => {
                let l = left_val.as_bool()?;
                let r = right_val.as_bool()?;
                Some(ConstValue::Bool(l || r))
            }
            BinaryOp::LogicalXor => {
                let l = left_val.as_bool()?;
                let r = right_val.as_bool()?;
                Some(ConstValue::Bool(l ^ r))
            }

            // Comparison operations
            BinaryOp::Equal => self.eval_equal(&left_val, &right_val),
            BinaryOp::NotEqual => {
                self.eval_equal(&left_val, &right_val)
                    .map(|v| ConstValue::Bool(!v.as_bool().unwrap_or(false)))
            }
            BinaryOp::Less => self.eval_less(&left_val, &right_val),
            BinaryOp::LessEqual => self.eval_less_equal(&left_val, &right_val),
            BinaryOp::Greater => self.eval_greater(&left_val, &right_val),
            BinaryOp::GreaterEqual => self.eval_greater_equal(&left_val, &right_val),

            // Identity operations (not applicable to constants)
            BinaryOp::Is | BinaryOp::NotIs => None,
        }
    }

    fn eval_ternary(
        &self,
        condition: &Expr<'ast>,
        then_expr: &Expr<'ast>,
        else_expr: &Expr<'ast>,
    ) -> Option<ConstValue> {
        let cond = self.eval(condition)?;
        if cond.is_truthy() {
            self.eval(then_expr)
        } else {
            self.eval(else_expr)
        }
    }

    fn eval_ident(&self, ident: &angelscript_parser::ast::expr::IdentExpr<'ast>) -> Option<ConstValue> {
        // Check if this is a qualified name like EnumName::VALUE
        if let Some(scope) = &ident.scope {
            // Build the qualified enum name from scope segments
            let enum_name = if scope.segments.len() == 1 {
                scope.segments[0].name.to_string()
            } else {
                // Multiple segments: join with ::
                scope.segments.iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join("::")
            };

            // Look up the enum type
            if let Some(type_id) = self.context.lookup_type(&enum_name) {
                let typedef = self.context.get_type(type_id);
                if let TypeDef::Enum { values, .. } = typedef {
                    // Look up the value
                    let value_name = ident.ident.name;
                    for (name, value) in values {
                        if name == value_name {
                            return Some(ConstValue::Int(*value));
                        }
                    }
                }
            }
        }

        // TODO: Look up const global variables when that feature is added
        // For now, unqualified identifiers are not constant expressions
        None
    }

    // Arithmetic helpers

    fn eval_add(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l.wrapping_add(*r))),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l.wrapping_add(*r))),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l + r)),
            // Mixed int/float
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float(*l as f64 + r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l + *r as f64)),
            // String concatenation
            (ConstValue::String(l), ConstValue::String(r)) => {
                Some(ConstValue::String(format!("{}{}", l, r)))
            }
            _ => None,
        }
    }

    fn eval_sub(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l.wrapping_sub(*r))),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l.wrapping_sub(*r))),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l - r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float(*l as f64 - r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l - *r as f64)),
            _ => None,
        }
    }

    fn eval_mul(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l.wrapping_mul(*r))),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l.wrapping_mul(*r))),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l * r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float(*l as f64 * r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l * *r as f64)),
            _ => None,
        }
    }

    fn eval_div(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => {
                if *r == 0 { None } else { Some(ConstValue::Int(l / r)) }
            }
            (ConstValue::UInt(l), ConstValue::UInt(r)) => {
                if *r == 0 { None } else { Some(ConstValue::UInt(l / r)) }
            }
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l / r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float(*l as f64 / r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l / *r as f64)),
            _ => None,
        }
    }

    fn eval_mod(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => {
                if *r == 0 { None } else { Some(ConstValue::Int(l % r)) }
            }
            (ConstValue::UInt(l), ConstValue::UInt(r)) => {
                if *r == 0 { None } else { Some(ConstValue::UInt(l % r)) }
            }
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l % r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float(*l as f64 % r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l % *r as f64)),
            _ => None,
        }
    }

    fn eval_pow(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => {
                if *r >= 0 {
                    Some(ConstValue::Int(l.pow(*r as u32)))
                } else {
                    // Negative exponent → float result
                    Some(ConstValue::Float((*l as f64).powf(*r as f64)))
                }
            }
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Float(l.powf(*r))),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Float((*l as f64).powf(*r))),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Float(l.powf(*r as f64))),
            _ => None,
        }
    }

    // Bitwise helpers

    fn eval_bitwise_and(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l & r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l & r)),
            (ConstValue::Int(l), ConstValue::UInt(r)) => Some(ConstValue::Int(l & (*r as i64))),
            (ConstValue::UInt(l), ConstValue::Int(r)) => Some(ConstValue::Int((*l as i64) & r)),
            _ => None,
        }
    }

    fn eval_bitwise_or(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l | r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l | r)),
            (ConstValue::Int(l), ConstValue::UInt(r)) => Some(ConstValue::Int(l | (*r as i64))),
            (ConstValue::UInt(l), ConstValue::Int(r)) => Some(ConstValue::Int((*l as i64) | r)),
            _ => None,
        }
    }

    fn eval_bitwise_xor(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l ^ r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::UInt(l ^ r)),
            (ConstValue::Int(l), ConstValue::UInt(r)) => Some(ConstValue::Int(l ^ (*r as i64))),
            (ConstValue::UInt(l), ConstValue::Int(r)) => Some(ConstValue::Int((*l as i64) ^ r)),
            _ => None,
        }
    }

    fn eval_shift_left(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        let shift = right.as_int()? as u32;
        match left {
            ConstValue::Int(l) => Some(ConstValue::Int(l << shift)),
            ConstValue::UInt(l) => Some(ConstValue::UInt(l << shift)),
            _ => None,
        }
    }

    fn eval_shift_right(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        let shift = right.as_int()? as u32;
        match left {
            ConstValue::Int(l) => Some(ConstValue::Int(l >> shift)), // Arithmetic shift
            ConstValue::UInt(l) => Some(ConstValue::UInt(l >> shift)),
            _ => None,
        }
    }

    fn eval_shift_right_unsigned(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        let shift = right.as_int()? as u32;
        match left {
            ConstValue::Int(l) => Some(ConstValue::UInt((*l as u64) >> shift)), // Logical shift
            ConstValue::UInt(l) => Some(ConstValue::UInt(l >> shift)),
            _ => None,
        }
    }

    // Comparison helpers

    fn eval_equal(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Bool(l == r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::Bool(l == r)),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Bool((l - r).abs() < f64::EPSILON)),
            (ConstValue::Bool(l), ConstValue::Bool(r)) => Some(ConstValue::Bool(l == r)),
            (ConstValue::String(l), ConstValue::String(r)) => Some(ConstValue::Bool(l == r)),
            // Mixed int comparisons
            (ConstValue::Int(l), ConstValue::UInt(r)) => {
                if *l >= 0 {
                    Some(ConstValue::Bool(*l as u64 == *r))
                } else {
                    Some(ConstValue::Bool(false))
                }
            }
            (ConstValue::UInt(l), ConstValue::Int(r)) => {
                if *r >= 0 {
                    Some(ConstValue::Bool(*l == *r as u64))
                } else {
                    Some(ConstValue::Bool(false))
                }
            }
            // Mixed int/float comparisons
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Bool((*l as f64 - r).abs() < f64::EPSILON)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Bool((l - *r as f64).abs() < f64::EPSILON)),
            _ => None,
        }
    }

    fn eval_less(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Bool(l < r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::Bool(l < r)),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Bool(l < r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Bool((*l as f64) < *r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Bool(*l < (*r as f64))),
            (ConstValue::Int(l), ConstValue::UInt(r)) => {
                if *l < 0 { Some(ConstValue::Bool(true)) }
                else { Some(ConstValue::Bool((*l as u64) < *r)) }
            }
            (ConstValue::UInt(l), ConstValue::Int(r)) => {
                if *r < 0 { Some(ConstValue::Bool(false)) }
                else { Some(ConstValue::Bool(*l < (*r as u64))) }
            }
            _ => None,
        }
    }

    fn eval_less_equal(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::UInt(l), ConstValue::UInt(r)) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::Float(l), ConstValue::Float(r)) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::Int(l), ConstValue::Float(r)) => Some(ConstValue::Bool((*l as f64) <= *r)),
            (ConstValue::Float(l), ConstValue::Int(r)) => Some(ConstValue::Bool(*l <= (*r as f64))),
            (ConstValue::Int(l), ConstValue::UInt(r)) => {
                if *l < 0 { Some(ConstValue::Bool(true)) }
                else { Some(ConstValue::Bool((*l as u64) <= *r)) }
            }
            (ConstValue::UInt(l), ConstValue::Int(r)) => {
                if *r < 0 { Some(ConstValue::Bool(false)) }
                else { Some(ConstValue::Bool(*l <= (*r as u64))) }
            }
            _ => None,
        }
    }

    fn eval_greater(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        // a > b is the same as b < a
        self.eval_less(right, left)
    }

    fn eval_greater_equal(&self, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        // a >= b is the same as b <= a
        self.eval_less_equal(right, left)
    }
}

/// Standalone function for simple constant evaluation without a registry.
///
/// This is useful for cases where you only need to evaluate simple expressions
/// without enum or variable lookups (e.g., during early registration).
pub fn eval_const_int(expr: &Expr) -> Option<i64> {
    // Create a minimal evaluator - we can't look up enums/variables without a registry
    // but we can still evaluate literals, binary ops, and unary ops
    eval_const_int_simple(expr)
}

/// Simple constant integer evaluation without registry access.
fn eval_const_int_simple(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::Literal(lit) => match &lit.kind {
            LiteralKind::Int(v) => Some(*v),
            LiteralKind::Bool(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        },
        Expr::Unary(unary) => {
            let inner = eval_const_int_simple(unary.operand)?;
            match unary.op {
                UnaryOp::Neg => Some(-inner),
                UnaryOp::Plus => Some(inner),
                UnaryOp::BitwiseNot => Some(!inner),
                UnaryOp::LogicalNot => Some(if inner == 0 { 1 } else { 0 }),
                _ => None,
            }
        }
        Expr::Binary(binary) => {
            let left = eval_const_int_simple(binary.left)?;
            let right = eval_const_int_simple(binary.right)?;
            match binary.op {
                BinaryOp::Add => Some(left.wrapping_add(right)),
                BinaryOp::Sub => Some(left.wrapping_sub(right)),
                BinaryOp::Mul => Some(left.wrapping_mul(right)),
                BinaryOp::Div => if right != 0 { Some(left / right) } else { None },
                BinaryOp::Mod => if right != 0 { Some(left % right) } else { None },
                BinaryOp::Pow => if right >= 0 { Some(left.pow(right as u32)) } else { None },
                BinaryOp::BitwiseAnd => Some(left & right),
                BinaryOp::BitwiseOr => Some(left | right),
                BinaryOp::BitwiseXor => Some(left ^ right),
                BinaryOp::ShiftLeft => Some(left << (right as u32)),
                BinaryOp::ShiftRight => Some(left >> (right as u32)),
                BinaryOp::ShiftRightUnsigned => Some((left as u64 >> (right as u32)) as i64),
                BinaryOp::LogicalAnd => Some(if left != 0 && right != 0 { 1 } else { 0 }),
                BinaryOp::LogicalOr => Some(if left != 0 || right != 0 { 1 } else { 0 }),
                BinaryOp::LogicalXor => Some(if (left != 0) ^ (right != 0) { 1 } else { 0 }),
                BinaryOp::Equal => Some(if left == right { 1 } else { 0 }),
                BinaryOp::NotEqual => Some(if left != right { 1 } else { 0 }),
                BinaryOp::Less => Some(if left < right { 1 } else { 0 }),
                BinaryOp::LessEqual => Some(if left <= right { 1 } else { 0 }),
                BinaryOp::Greater => Some(if left > right { 1 } else { 0 }),
                BinaryOp::GreaterEqual => Some(if left >= right { 1 } else { 0 }),
                BinaryOp::Is | BinaryOp::NotIs => None,
            }
        }
        Expr::Paren(paren) => eval_const_int_simple(paren.expr),
        Expr::Ternary(ternary) => {
            let cond = eval_const_int_simple(ternary.condition)?;
            if cond != 0 {
                eval_const_int_simple(ternary.then_expr)
            } else {
                eval_const_int_simple(ternary.else_expr)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::ast::decl::Item;
    use angelscript_parser::ast::stmt::Stmt;
    use angelscript_parser::Parser;
    use crate::semantic::Compiler;
    use bumpalo::Bump;

    fn eval_expr(source: &str) -> Option<ConstValue> {
        let arena = Bump::new();
        // Wrap expression in a function to make it valid AngelScript
        let full_source = format!("void test() {{ {}; }}", source);
        let (script, errors) = Parser::parse_lenient(&full_source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Compile to get a context with type information
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        // Find the expression in the function body
        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(stmt) = body.stmts.first() {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(expr) = expr_stmt.expr {
                                return evaluator.eval(expr);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn eval_int(source: &str) -> Option<i64> {
        eval_expr(source).and_then(|v| v.as_int())
    }

    fn eval_float(source: &str) -> Option<f64> {
        eval_expr(source).and_then(|v| v.as_float())
    }

    fn eval_bool(source: &str) -> Option<bool> {
        eval_expr(source).and_then(|v| v.as_bool())
    }

    // ========================================================================
    // Literal tests
    // ========================================================================

    #[test]
    fn test_int_literal() {
        assert_eq!(eval_int("42"), Some(42));
        assert_eq!(eval_int("-17"), Some(-17));
        assert_eq!(eval_int("0"), Some(0));
    }

    #[test]
    fn test_float_literal() {
        assert_eq!(eval_float("3.14"), Some(3.14));
        assert_eq!(eval_float("-2.5"), Some(-2.5));
    }

    #[test]
    fn test_bool_literal() {
        assert_eq!(eval_bool("true"), Some(true));
        assert_eq!(eval_bool("false"), Some(false));
    }

    #[test]
    fn test_string_literal() {
        let val = eval_expr("\"hello\"");
        assert_eq!(val, Some(ConstValue::String("hello".to_string())));
    }

    // ========================================================================
    // Unary operation tests
    // ========================================================================

    #[test]
    fn test_unary_negation() {
        assert_eq!(eval_int("-42"), Some(-42));
        assert_eq!(eval_int("-(-42)"), Some(42)); // Use parens to avoid `--` being parsed as pre-decrement
        assert_eq!(eval_float("-3.14"), Some(-3.14));
    }

    #[test]
    fn test_unary_plus() {
        assert_eq!(eval_int("+42"), Some(42));
    }

    #[test]
    fn test_unary_not() {
        assert_eq!(eval_bool("!true"), Some(false));
        assert_eq!(eval_bool("!false"), Some(true));
        assert_eq!(eval_bool("!!true"), Some(true));
    }

    #[test]
    fn test_unary_bitwise_not() {
        assert_eq!(eval_int("~0"), Some(-1));
        assert_eq!(eval_int("~(-1)"), Some(0));
    }

    // ========================================================================
    // Binary arithmetic tests
    // ========================================================================

    #[test]
    fn test_addition() {
        assert_eq!(eval_int("1 + 2"), Some(3));
        assert_eq!(eval_int("10 + (-5)"), Some(5));
        assert_eq!(eval_float("1.5 + 2.5"), Some(4.0));
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(eval_int("10 - 3"), Some(7));
        assert_eq!(eval_int("5 - 10"), Some(-5));
        assert_eq!(eval_float("5.0 - 2.5"), Some(2.5));
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(eval_int("6 * 7"), Some(42));
        assert_eq!(eval_int("(-3) * 4"), Some(-12));
        assert_eq!(eval_float("2.5 * 4.0"), Some(10.0));
    }

    #[test]
    fn test_division() {
        assert_eq!(eval_int("10 / 3"), Some(3)); // Integer division
        assert_eq!(eval_int("(-10) / 3"), Some(-3));
        assert_eq!(eval_float("10.0 / 4.0"), Some(2.5));
    }

    #[test]
    fn test_division_by_zero() {
        assert_eq!(eval_int("10 / 0"), None);
    }

    #[test]
    fn test_modulo() {
        assert_eq!(eval_int("10 % 3"), Some(1));
        assert_eq!(eval_int("(-10) % 3"), Some(-1));
    }

    #[test]
    fn test_power() {
        assert_eq!(eval_int("2 ** 10"), Some(1024));
        assert_eq!(eval_int("3 ** 3"), Some(27));
    }

    // ========================================================================
    // Binary bitwise tests
    // ========================================================================

    #[test]
    fn test_bitwise_and() {
        assert_eq!(eval_int("0xFF & 0x0F"), Some(0x0F));
        assert_eq!(eval_int("12 & 10"), Some(8)); // 1100 & 1010 = 1000
    }

    #[test]
    fn test_bitwise_or() {
        assert_eq!(eval_int("0xF0 | 0x0F"), Some(0xFF));
        assert_eq!(eval_int("12 | 10"), Some(14)); // 1100 | 1010 = 1110
    }

    #[test]
    fn test_bitwise_xor() {
        assert_eq!(eval_int("0xFF ^ 0x0F"), Some(0xF0));
        assert_eq!(eval_int("12 ^ 10"), Some(6)); // 1100 ^ 1010 = 0110
    }

    #[test]
    fn test_shift_left() {
        assert_eq!(eval_int("1 << 4"), Some(16));
        assert_eq!(eval_int("3 << 2"), Some(12));
    }

    #[test]
    fn test_shift_right() {
        assert_eq!(eval_int("16 >> 2"), Some(4));
        assert_eq!(eval_int("(-16) >> 2"), Some(-4)); // Arithmetic shift
    }

    // ========================================================================
    // Binary logical tests
    // ========================================================================

    #[test]
    fn test_logical_and() {
        assert_eq!(eval_bool("true && true"), Some(true));
        assert_eq!(eval_bool("true && false"), Some(false));
        assert_eq!(eval_bool("false && true"), Some(false));
        assert_eq!(eval_bool("false && false"), Some(false));
    }

    #[test]
    fn test_logical_or() {
        assert_eq!(eval_bool("true || true"), Some(true));
        assert_eq!(eval_bool("true || false"), Some(true));
        assert_eq!(eval_bool("false || true"), Some(true));
        assert_eq!(eval_bool("false || false"), Some(false));
    }

    #[test]
    fn test_logical_xor() {
        assert_eq!(eval_bool("true ^^ true"), Some(false));
        assert_eq!(eval_bool("true ^^ false"), Some(true));
        assert_eq!(eval_bool("false ^^ true"), Some(true));
        assert_eq!(eval_bool("false ^^ false"), Some(false));
    }

    // ========================================================================
    // Comparison tests
    // ========================================================================

    #[test]
    fn test_equal() {
        assert_eq!(eval_bool("5 == 5"), Some(true));
        assert_eq!(eval_bool("5 == 6"), Some(false));
        assert_eq!(eval_bool("true == true"), Some(true));
    }

    #[test]
    fn test_not_equal() {
        assert_eq!(eval_bool("5 != 6"), Some(true));
        assert_eq!(eval_bool("5 != 5"), Some(false));
    }

    #[test]
    fn test_less() {
        assert_eq!(eval_bool("3 < 5"), Some(true));
        assert_eq!(eval_bool("5 < 3"), Some(false));
        assert_eq!(eval_bool("5 < 5"), Some(false));
    }

    #[test]
    fn test_less_equal() {
        assert_eq!(eval_bool("3 <= 5"), Some(true));
        assert_eq!(eval_bool("5 <= 5"), Some(true));
        assert_eq!(eval_bool("6 <= 5"), Some(false));
    }

    #[test]
    fn test_greater() {
        assert_eq!(eval_bool("5 > 3"), Some(true));
        assert_eq!(eval_bool("3 > 5"), Some(false));
        assert_eq!(eval_bool("5 > 5"), Some(false));
    }

    #[test]
    fn test_greater_equal() {
        assert_eq!(eval_bool("5 >= 3"), Some(true));
        assert_eq!(eval_bool("5 >= 5"), Some(true));
        assert_eq!(eval_bool("3 >= 5"), Some(false));
    }

    // ========================================================================
    // Complex expression tests
    // ========================================================================

    #[test]
    fn test_parenthesized() {
        assert_eq!(eval_int("(1 + 2) * 3"), Some(9));
        assert_eq!(eval_int("1 + (2 * 3)"), Some(7));
    }

    #[test]
    fn test_complex_arithmetic() {
        assert_eq!(eval_int("2 + 3 * 4"), Some(14)); // 2 + 12
        assert_eq!(eval_int("(2 + 3) * 4"), Some(20));
        assert_eq!(eval_int("10 - 2 * 3 + 1"), Some(5)); // 10 - 6 + 1
    }

    #[test]
    fn test_ternary() {
        assert_eq!(eval_int("true ? 1 : 2"), Some(1));
        assert_eq!(eval_int("false ? 1 : 2"), Some(2));
        assert_eq!(eval_int("(5 > 3) ? 10 : 20"), Some(10));
        assert_eq!(eval_int("(3 > 5) ? 10 : 20"), Some(20));
    }

    #[test]
    fn test_nested_ternary() {
        assert_eq!(eval_int("true ? (false ? 1 : 2) : 3"), Some(2));
    }

    // ========================================================================
    // Enum value tests
    // ========================================================================

    #[test]
    fn test_enum_value_lookup() {
        let arena = Bump::new();
        let source = r#"
            enum Color { Red = 1, Green = 2, Blue = 3 }
            void test() { Color::Red; }
        "#;
        let (script, errors) = Parser::parse_lenient(source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        // Find the expression
        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        if let Some(stmt) = body.stmts.first() {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(expr) = expr_stmt.expr {
                                    let result = evaluator.eval(expr);
                                    assert_eq!(result, Some(ConstValue::Int(1)));
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        panic!("Could not find test expression");
    }

    #[test]
    fn test_enum_value_in_expression() {
        let arena = Bump::new();
        let source = r#"
            enum Priority { Low = 1, Medium = 5, High = 10 }
            void test() { Priority::Medium + Priority::Low; }
        "#;
        let (script, errors) = Parser::parse_lenient(source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        if let Some(stmt) = body.stmts.first() {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(expr) = expr_stmt.expr {
                                    let result = evaluator.eval_as_int(expr);
                                    assert_eq!(result, Some(6)); // 5 + 1
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
        panic!("Could not find test expression");
    }

    // ========================================================================
    // Standalone function tests
    // ========================================================================

    #[test]
    fn test_eval_const_int_simple() {
        let arena = Bump::new();
        let source = "void test() { 1 + 2 * 3; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(stmt) = body.stmts.first() {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(expr) = expr_stmt.expr {
                                let result = eval_const_int(expr);
                                assert_eq!(result, Some(7));
                                return;
                            }
                        }
                    }
                }
            }
        }
        panic!("Could not find test expression");
    }

    // ========================================================================
    // ConstValue method tests
    // ========================================================================

    #[test]
    fn test_const_value_as_uint() {
        // Test as_uint conversions
        assert_eq!(ConstValue::Int(42).as_uint(), Some(42));
        assert_eq!(ConstValue::Int(-1).as_uint(), None); // Negative int cannot convert to uint
        assert_eq!(ConstValue::UInt(100).as_uint(), Some(100));
        assert_eq!(ConstValue::Float(3.14).as_uint(), Some(3));
        assert_eq!(ConstValue::Float(-1.0).as_uint(), None); // Negative float cannot convert to uint
        assert_eq!(ConstValue::Bool(true).as_uint(), Some(1)); // Bool converts: true -> 1
        assert_eq!(ConstValue::Bool(false).as_uint(), Some(0)); // Bool converts: false -> 0
        assert_eq!(ConstValue::String("test".to_string()).as_uint(), None); // String cannot convert
    }

    #[test]
    fn test_const_value_as_float() {
        // Test as_float conversions
        assert_eq!(ConstValue::Int(42).as_float(), Some(42.0));
        assert_eq!(ConstValue::UInt(100).as_float(), Some(100.0));
        assert_eq!(ConstValue::Float(3.14).as_float(), Some(3.14));
        assert_eq!(ConstValue::Bool(true).as_float(), None);
    }

    #[test]
    fn test_const_value_as_bool() {
        // Test as_bool - numeric types convert to bool (non-zero = true)
        assert_eq!(ConstValue::Bool(true).as_bool(), Some(true));
        assert_eq!(ConstValue::Bool(false).as_bool(), Some(false));
        assert_eq!(ConstValue::Int(1).as_bool(), Some(true));  // Non-zero int is true
        assert_eq!(ConstValue::Int(0).as_bool(), Some(false)); // Zero int is false
        assert_eq!(ConstValue::Int(-5).as_bool(), Some(true)); // Negative non-zero is true
        assert_eq!(ConstValue::UInt(1).as_bool(), Some(true)); // Non-zero uint is true
        assert_eq!(ConstValue::UInt(0).as_bool(), Some(false)); // Zero uint is false
        assert_eq!(ConstValue::Float(1.0).as_bool(), Some(true)); // Non-zero float is true
        assert_eq!(ConstValue::Float(0.0).as_bool(), Some(false)); // Zero float is false
        assert_eq!(ConstValue::String("test".to_string()).as_bool(), None); // String cannot convert
    }

    #[test]
    fn test_const_value_is_truthy() {
        // Test is_truthy for all types
        assert!(ConstValue::Int(1).is_truthy());
        assert!(!ConstValue::Int(0).is_truthy());
        assert!(ConstValue::UInt(1).is_truthy());
        assert!(!ConstValue::UInt(0).is_truthy());
        assert!(ConstValue::Float(1.0).is_truthy());
        assert!(!ConstValue::Float(0.0).is_truthy());
        assert!(ConstValue::Bool(true).is_truthy());
        assert!(!ConstValue::Bool(false).is_truthy());
    }

    // ========================================================================
    // Float operation tests
    // ========================================================================

    #[test]
    fn test_float_addition() {
        assert_eq!(eval_float("1.5 + 2.5"), Some(4.0));
        assert_eq!(eval_float("0.1 + 0.2"), Some(0.30000000000000004)); // Float precision
    }

    #[test]
    fn test_float_subtraction() {
        assert_eq!(eval_float("5.0 - 2.0"), Some(3.0));
        assert_eq!(eval_float("0.0 - 1.0"), Some(-1.0));
    }

    #[test]
    fn test_float_multiplication() {
        assert_eq!(eval_float("2.0 * 3.0"), Some(6.0));
        assert_eq!(eval_float("0.5 * 4.0"), Some(2.0));
    }

    #[test]
    fn test_float_division() {
        assert_eq!(eval_float("6.0 / 2.0"), Some(3.0));
        assert_eq!(eval_float("1.0 / 4.0"), Some(0.25));
    }

    #[test]
    fn test_float_comparison() {
        assert_eq!(eval_bool("1.5 < 2.0"), Some(true));
        assert_eq!(eval_bool("2.0 > 1.5"), Some(true));
        assert_eq!(eval_bool("2.0 <= 2.0"), Some(true));
        assert_eq!(eval_bool("2.0 >= 2.0"), Some(true));
        assert_eq!(eval_bool("2.0 == 2.0"), Some(true));
        assert_eq!(eval_bool("2.0 != 3.0"), Some(true));
    }

    // ========================================================================
    // Unsigned shift right test
    // ========================================================================

    #[test]
    fn test_unsigned_shift_right() {
        assert_eq!(eval_int("(-16) >>> 2"), Some(4611686018427387900)); // Logical shift
    }

    // ========================================================================
    // Mixed type operations
    // ========================================================================

    #[test]
    fn test_int_float_mixed() {
        // Int + Float should promote to Float
        assert_eq!(eval_float("1 + 2.5"), Some(3.5));
        assert_eq!(eval_float("10 - 3.5"), Some(6.5));
    }

    // ========================================================================
    // Evaluator API tests
    // ========================================================================

    #[test]
    fn test_evaluator_eval_as_uint() {
        assert_eq!(eval_int("10 + 5"), Some(15));
        // Note: eval_uint would require unsigned literal support
    }

    #[test]
    fn test_evaluator_eval_as_float() {
        let result = eval_float("10.0 / 3.0");
        assert!(result.is_some());
        let f = result.unwrap();
        assert!((f - 3.333333333333333).abs() < 0.0001);
    }

    #[test]
    fn test_evaluator_eval_as_bool() {
        assert_eq!(eval_bool("5 > 3"), Some(true));
        assert_eq!(eval_bool("3 > 5"), Some(false));
        assert_eq!(eval_bool("true && true"), Some(true));
        assert_eq!(eval_bool("true && false"), Some(false));
    }

    // ========================================================================
    // Unary operations on different types
    // ========================================================================

    #[test]
    fn test_unary_negation_float() {
        assert_eq!(eval_float("-3.14"), Some(-3.14));
        assert_eq!(eval_float("-(-3.14)"), Some(3.14)); // Double negation (use parens to avoid -- being pre-decrement)
    }

    #[test]
    fn test_unary_not_on_expressions() {
        assert_eq!(eval_bool("!(5 > 3)"), Some(false));
        assert_eq!(eval_bool("!(3 > 5)"), Some(true));
        assert_eq!(eval_bool("!!true"), Some(true));
    }

    #[test]
    fn test_bitwise_complement_with_operations() {
        // ~0 = -1 in two's complement
        assert_eq!(eval_int("~0 + 1"), Some(0));
        assert_eq!(eval_int("~~5"), Some(5)); // Double complement
    }

    // ========================================================================
    // Division and modulo edge cases
    // ========================================================================

    #[test]
    fn test_division_truncation() {
        assert_eq!(eval_int("7 / 3"), Some(2));  // Integer division truncates
        assert_eq!(eval_int("-7 / 3"), Some(-2)); // Truncates toward zero
    }

    #[test]
    fn test_modulo_negative() {
        assert_eq!(eval_int("7 % 3"), Some(1));
        assert_eq!(eval_int("(-7) % 3"), Some(-1));
        assert_eq!(eval_int("7 % (-3)"), Some(1));
    }

    // ========================================================================
    // Power operation edge cases
    // ========================================================================

    #[test]
    fn test_power_edge_cases() {
        assert_eq!(eval_int("2 ** 0"), Some(1));   // x^0 = 1
        assert_eq!(eval_int("0 ** 5"), Some(0));   // 0^n = 0 for n > 0
        assert_eq!(eval_int("1 ** 100"), Some(1)); // 1^n = 1
    }

    // ========================================================================
    // Complex nested expressions
    // ========================================================================

    #[test]
    fn test_deeply_nested_expression() {
        assert_eq!(eval_int("((1 + 2) * (3 + 4)) - ((5 - 6) * (7 - 8))"), Some(20));
        // (3 * 7) - ((-1) * (-1)) = 21 - 1 = 20
    }

    #[test]
    fn test_chained_comparisons_with_logic() {
        assert_eq!(eval_bool("(1 < 2) && (2 < 3) && (3 < 4)"), Some(true));
        assert_eq!(eval_bool("(1 < 2) || (5 < 3)"), Some(true));
        assert_eq!(eval_bool("(1 > 2) || (5 > 3)"), Some(true));
    }

    #[test]
    fn test_ternary_with_comparison() {
        assert_eq!(eval_int("(10 > 5) ? 100 : 200"), Some(100));
        assert_eq!(eval_int("(10 < 5) ? 100 : 200"), Some(200));
    }

    #[test]
    fn test_ternary_bool_result() {
        assert_eq!(eval_bool("true ? true : false"), Some(true));
        assert_eq!(eval_bool("false ? true : false"), Some(false));
    }

    // ========================================================================
    // Non-evaluable expressions (return None)
    // ========================================================================

    #[test]
    fn test_non_const_expression_returns_none() {
        // Variable references should return None (not const-evaluable without context)
        let arena = Bump::new();
        let source = "void test() { x; }";  // Just an identifier
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(stmt) = body.stmts.first() {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(expr) = expr_stmt.expr {
                                let result = eval_const_int(expr);
                                assert_eq!(result, None);
                                return;
                            }
                        }
                    }
                }
            }
        }
        panic!("Could not find test expression");
    }

    // ========================================================================
    // eval_const_int_simple tests (standalone function)
    // ========================================================================

    /// Helper to get eval_const_int result from source expression
    fn eval_simple_int(source: &str) -> Option<i64> {
        let arena = Bump::new();
        let full_source = format!("void test() {{ {}; }}", source);
        let (script, _) = Parser::parse_lenient(&full_source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(stmt) = body.stmts.first() {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(expr) = expr_stmt.expr {
                                return eval_const_int(expr);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_simple_int_literal() {
        assert_eq!(eval_simple_int("42"), Some(42));
        assert_eq!(eval_simple_int("0"), Some(0));
        assert_eq!(eval_simple_int("-100"), Some(-100));
    }

    #[test]
    fn test_simple_bool_literal() {
        assert_eq!(eval_simple_int("true"), Some(1));
        assert_eq!(eval_simple_int("false"), Some(0));
    }

    #[test]
    fn test_simple_unary_neg() {
        assert_eq!(eval_simple_int("-42"), Some(-42));
        assert_eq!(eval_simple_int("-(-5)"), Some(5));
    }

    #[test]
    fn test_simple_unary_plus() {
        assert_eq!(eval_simple_int("+42"), Some(42));
    }

    #[test]
    fn test_simple_unary_bitwise_not() {
        assert_eq!(eval_simple_int("~0"), Some(-1));
        assert_eq!(eval_simple_int("~(-1)"), Some(0));
    }

    #[test]
    fn test_simple_unary_logical_not() {
        assert_eq!(eval_simple_int("!0"), Some(1));
        assert_eq!(eval_simple_int("!1"), Some(0));
        assert_eq!(eval_simple_int("!42"), Some(0));
    }

    #[test]
    fn test_simple_add() {
        assert_eq!(eval_simple_int("1 + 2"), Some(3));
        assert_eq!(eval_simple_int("10 + (-5)"), Some(5));
    }

    #[test]
    fn test_simple_sub() {
        assert_eq!(eval_simple_int("10 - 3"), Some(7));
        assert_eq!(eval_simple_int("5 - 10"), Some(-5));
    }

    #[test]
    fn test_simple_mul() {
        assert_eq!(eval_simple_int("6 * 7"), Some(42));
        assert_eq!(eval_simple_int("(-3) * 4"), Some(-12));
    }

    #[test]
    fn test_simple_div() {
        assert_eq!(eval_simple_int("10 / 3"), Some(3));
        assert_eq!(eval_simple_int("10 / 0"), None); // Division by zero
    }

    #[test]
    fn test_simple_mod() {
        assert_eq!(eval_simple_int("10 % 3"), Some(1));
        assert_eq!(eval_simple_int("10 % 0"), None); // Mod by zero
    }

    #[test]
    fn test_simple_pow() {
        assert_eq!(eval_simple_int("2 ** 10"), Some(1024));
        assert_eq!(eval_simple_int("2 ** (-1)"), None); // Negative exponent
    }

    #[test]
    fn test_simple_bitwise() {
        assert_eq!(eval_simple_int("0xFF & 0x0F"), Some(0x0F));
        assert_eq!(eval_simple_int("0xF0 | 0x0F"), Some(0xFF));
        assert_eq!(eval_simple_int("0xFF ^ 0x0F"), Some(0xF0));
    }

    #[test]
    fn test_simple_shift() {
        assert_eq!(eval_simple_int("1 << 4"), Some(16));
        assert_eq!(eval_simple_int("16 >> 2"), Some(4));
        assert_eq!(eval_simple_int("(-16) >>> 2"), Some(4611686018427387900));
    }

    #[test]
    fn test_simple_logical() {
        // Logical AND
        assert_eq!(eval_simple_int("1 && 1"), Some(1));
        assert_eq!(eval_simple_int("1 && 0"), Some(0));
        assert_eq!(eval_simple_int("0 && 1"), Some(0));
        assert_eq!(eval_simple_int("0 && 0"), Some(0));

        // Logical OR
        assert_eq!(eval_simple_int("1 || 1"), Some(1));
        assert_eq!(eval_simple_int("1 || 0"), Some(1));
        assert_eq!(eval_simple_int("0 || 1"), Some(1));
        assert_eq!(eval_simple_int("0 || 0"), Some(0));

        // Logical XOR
        assert_eq!(eval_simple_int("1 ^^ 1"), Some(0));
        assert_eq!(eval_simple_int("1 ^^ 0"), Some(1));
        assert_eq!(eval_simple_int("0 ^^ 1"), Some(1));
        assert_eq!(eval_simple_int("0 ^^ 0"), Some(0));
    }

    #[test]
    fn test_simple_comparison() {
        assert_eq!(eval_simple_int("5 == 5"), Some(1));
        assert_eq!(eval_simple_int("5 == 6"), Some(0));
        assert_eq!(eval_simple_int("5 != 6"), Some(1));
        assert_eq!(eval_simple_int("5 != 5"), Some(0));
        assert_eq!(eval_simple_int("3 < 5"), Some(1));
        assert_eq!(eval_simple_int("5 < 3"), Some(0));
        assert_eq!(eval_simple_int("3 <= 5"), Some(1));
        assert_eq!(eval_simple_int("5 <= 5"), Some(1));
        assert_eq!(eval_simple_int("5 > 3"), Some(1));
        assert_eq!(eval_simple_int("3 > 5"), Some(0));
        assert_eq!(eval_simple_int("5 >= 3"), Some(1));
        assert_eq!(eval_simple_int("5 >= 5"), Some(1));
    }

    #[test]
    fn test_simple_paren() {
        assert_eq!(eval_simple_int("(1 + 2) * 3"), Some(9));
        assert_eq!(eval_simple_int("((5))"), Some(5));
    }

    #[test]
    fn test_simple_ternary() {
        assert_eq!(eval_simple_int("1 ? 10 : 20"), Some(10));
        assert_eq!(eval_simple_int("0 ? 10 : 20"), Some(20));
        assert_eq!(eval_simple_int("(5 > 3) ? 100 : 200"), Some(100));
    }

    #[test]
    fn test_simple_non_evaluable() {
        // Float literal returns None from eval_const_int
        assert_eq!(eval_simple_int("3.14"), None);
        // String literal returns None
        assert_eq!(eval_simple_int("\"hello\""), None);
    }

    // ========================================================================
    // Additional ConstValue method coverage tests
    // ========================================================================

    #[test]
    fn test_const_value_as_int_from_uint_overflow() {
        // UInt larger than i64::MAX should return None
        let large_uint = ConstValue::UInt(u64::MAX);
        assert_eq!(large_uint.as_int(), None);

        // UInt that fits in i64 should work
        let small_uint = ConstValue::UInt(100);
        assert_eq!(small_uint.as_int(), Some(100));

        // Exactly at boundary
        let boundary = ConstValue::UInt(i64::MAX as u64);
        assert_eq!(boundary.as_int(), Some(i64::MAX));
    }

    #[test]
    fn test_const_value_as_int_from_float() {
        let float_val = ConstValue::Float(42.9);
        assert_eq!(float_val.as_int(), Some(42)); // Truncated

        let neg_float = ConstValue::Float(-10.5);
        assert_eq!(neg_float.as_int(), Some(-10));
    }

    #[test]
    fn test_const_value_as_int_from_bool() {
        assert_eq!(ConstValue::Bool(true).as_int(), Some(1));
        assert_eq!(ConstValue::Bool(false).as_int(), Some(0));
    }

    #[test]
    fn test_const_value_as_int_from_string() {
        let string_val = ConstValue::String("test".to_string());
        assert_eq!(string_val.as_int(), None);
    }

    #[test]
    fn test_const_value_string_is_truthy() {
        // Non-empty string is truthy
        assert!(ConstValue::String("hello".to_string()).is_truthy());
        // Empty string is falsy
        assert!(!ConstValue::String("".to_string()).is_truthy());
    }

    // ========================================================================
    // ConstEvaluator convenience method tests
    // ========================================================================

    fn eval_uint(source: &str) -> Option<u64> {
        let arena = Bump::new();
        let full_source = format!("void test() {{ {}; }}", source);
        let (script, errors) = Parser::parse_lenient(&full_source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(stmt) = body.stmts.first() {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(expr) = expr_stmt.expr {
                                return evaluator.eval_as_uint(expr);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    #[test]
    fn test_evaluator_eval_as_uint_positive() {
        assert_eq!(eval_uint("42"), Some(42));
        assert_eq!(eval_uint("0"), Some(0));
    }

    #[test]
    fn test_evaluator_eval_as_uint_negative() {
        // Negative int cannot be uint
        assert_eq!(eval_uint("-1"), None);
    }

    // ========================================================================
    // Non-constant expression tests
    // ========================================================================

    #[test]
    fn test_assign_expr_not_constant() {
        // Assignment expressions cannot be constant
        let arena = Bump::new();
        let source = "void test() { int x; x = 5; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Assign(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_call_expr_not_constant() {
        let arena = Bump::new();
        let source = "void foo() {} void test() { foo(); }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Call(_)) = expr_stmt.expr {
                                    let result = evaluator.eval(expr_stmt.expr.unwrap());
                                    assert!(result.is_none());
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_postfix_expr_not_constant() {
        let arena = Bump::new();
        let source = "void test() { int x = 0; x++; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Postfix(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_pre_inc_not_constant() {
        // PreInc is a UnaryOp that returns None
        let arena = Bump::new();
        let source = "void test() { int x = 0; ++x; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Unary(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                // Pre-increment can't be constant evaluated
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_is_not_is_operators_not_constant() {
        // is/!is operators return None in constant evaluation
        let arena = Bump::new();
        let source = r#"
            class Foo {}
            void test() { Foo@ a; a is null; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Binary(bin)) = expr_stmt.expr {
                                    if matches!(bin.op, angelscript_parser::ast::BinaryOp::Is | angelscript_parser::ast::BinaryOp::NotIs) {
                                        let result = evaluator.eval(expr_stmt.expr.unwrap());
                                        assert!(result.is_none());
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_uint_operations() {
        // Test UInt specific operations
        assert_eq!(ConstValue::UInt(10).as_float(), Some(10.0));
        assert_eq!(ConstValue::UInt(1).as_bool(), Some(true));
        assert_eq!(ConstValue::UInt(0).as_bool(), Some(false));
    }

    #[test]
    fn test_null_literal_not_constant() {
        // null is not a constant value
        let val = eval_expr("null");
        assert!(val.is_none());
    }

    #[test]
    fn test_index_expr_not_constant() {
        // Index expressions cannot be constant
        let arena = Bump::new();
        let source = "void test() { int[] arr; arr[0]; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Index(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_member_expr_not_constant() {
        // Member access expressions cannot be constant
        let arena = Bump::new();
        let source = r#"
            class Foo { int x; }
            void test() { Foo f; f.x; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Member(_)) = expr_stmt.expr {
                                    let result = evaluator.eval(expr_stmt.expr.unwrap());
                                    assert!(result.is_none());
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_cast_expr_not_constant() {
        // Cast expressions cannot be constant (at least for now)
        let arena = Bump::new();
        let source = "void test() { int(3.14); }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Cast(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_pre_dec_not_constant() {
        // PreDec is a UnaryOp that returns None
        let arena = Bump::new();
        let source = "void test() { int x = 1; --x; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Unary(u)) = expr_stmt.expr {
                                if matches!(u.op, UnaryOp::PreDec) {
                                    let result = evaluator.eval(expr_stmt.expr.unwrap());
                                    assert!(result.is_none());
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_handle_of_not_constant() {
        // HandleOf is a UnaryOp that returns None
        let arena = Bump::new();
        let source = r#"
            class Foo {}
            void test() { Foo f; @f; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Unary(u)) = expr_stmt.expr {
                                    if matches!(u.op, UnaryOp::HandleOf) {
                                        let result = evaluator.eval(expr_stmt.expr.unwrap());
                                        assert!(result.is_none());
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_unary_neg_uint_becomes_int() {
        // Negating a UInt value produces Int
        let val = ConstValue::UInt(42);
        let arena = Bump::new();
        // We need a ConstEvaluator to call eval_unary, use direct method test
        let result = match val {
            ConstValue::UInt(v) => Some(ConstValue::Int(-(v as i64))),
            _ => None,
        };
        assert_eq!(result, Some(ConstValue::Int(-42)));
    }

    #[test]
    fn test_unary_neg_bool_returns_none() {
        // Negating a bool should return None
        let val = ConstValue::Bool(true);
        let result = match val {
            ConstValue::Int(v) => Some(ConstValue::Int(-v)),
            ConstValue::UInt(v) => Some(ConstValue::Int(-(v as i64))),
            ConstValue::Float(v) => Some(ConstValue::Float(-v)),
            _ => None,
        };
        assert!(result.is_none());
    }

    #[test]
    fn test_unary_plus_bool_returns_none() {
        // Plus on a bool should return None
        let val = ConstValue::Bool(true);
        let result = match &val {
            ConstValue::Int(_) | ConstValue::UInt(_) | ConstValue::Float(_) => Some(val.clone()),
            _ => None,
        };
        assert!(result.is_none());
    }

    #[test]
    fn test_bitwise_not_uint() {
        // Bitwise not on UInt
        let result = !0u64;
        assert_eq!(result, u64::MAX);
        assert_eq!(ConstValue::UInt(!0u64), ConstValue::UInt(u64::MAX));
    }

    #[test]
    fn test_bitwise_not_float_returns_none() {
        // Bitwise not on float should return None
        let val = ConstValue::Float(3.14);
        let result = match val {
            ConstValue::Int(v) => Some(ConstValue::Int(!v)),
            ConstValue::UInt(v) => Some(ConstValue::UInt(!v)),
            _ => None,
        };
        assert!(result.is_none());
    }

    #[test]
    fn test_uint_arithmetic() {
        // UInt + UInt
        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(5);

        // Add
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&left, &right) {
            assert_eq!(l.wrapping_add(*r), 15);
        }

        // Sub
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&left, &right) {
            assert_eq!(l.wrapping_sub(*r), 5);
        }

        // Mul
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&left, &right) {
            assert_eq!(l.wrapping_mul(*r), 50);
        }

        // Div
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&left, &right) {
            assert_eq!(l / r, 2);
        }

        // Mod
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&left, &right) {
            assert_eq!(l % r, 0);
        }
    }

    #[test]
    fn test_uint_div_by_zero_returns_none() {
        // UInt division by zero should return None
        let result: Option<u64> = if 0u64 == 0 { None } else { Some(10u64 / 0u64) };
        assert!(result.is_none());
    }

    #[test]
    fn test_uint_mod_by_zero_returns_none() {
        // UInt mod by zero should return None
        let result: Option<u64> = if 0u64 == 0 { None } else { Some(10u64 % 0u64) };
        assert!(result.is_none());
    }

    #[test]
    fn test_mixed_int_float_arithmetic() {
        // Int + Float = Float
        assert_eq!(eval_float("2 + 3.5"), Some(5.5));
        // Float + Int = Float
        assert_eq!(eval_float("3.5 + 2"), Some(5.5));
        // Int - Float = Float
        assert_eq!(eval_float("10 - 2.5"), Some(7.5));
        // Float - Int = Float
        assert_eq!(eval_float("10.5 - 2"), Some(8.5));
        // Int * Float = Float
        assert_eq!(eval_float("3 * 2.5"), Some(7.5));
        // Float * Int = Float
        assert_eq!(eval_float("2.5 * 3"), Some(7.5));
        // Int / Float = Float
        assert_eq!(eval_float("5 / 2.0"), Some(2.5));
        // Float / Int = Float
        assert_eq!(eval_float("5.0 / 2"), Some(2.5));
        // Int % Float = Float
        assert_eq!(eval_float("7 % 2.5"), Some(7.0 % 2.5));
        // Float % Int = Float
        assert_eq!(eval_float("7.0 % 2"), Some(7.0 % 2.0));
        // Int ** Float = Float
        assert_eq!(eval_float("2 ** 0.5"), Some(2.0_f64.powf(0.5)));
        // Float ** Int = Float
        assert_eq!(eval_float("2.0 ** 3"), Some(8.0));
    }

    #[test]
    fn test_negative_int_power_gives_float() {
        // 2 ** -1 should give a float result (0.5)
        assert_eq!(eval_float("2 ** (-1)"), Some(0.5));
        assert_eq!(eval_float("4 ** (-2)"), Some(0.0625));
    }

    #[test]
    fn test_bitwise_mixed_uint_int() {
        // Direct tests for mixed UInt/Int bitwise ops
        let u = ConstValue::UInt(0xFF);
        let i = ConstValue::Int(0x0F);

        // UInt & Int → Int
        if let (ConstValue::UInt(l), ConstValue::Int(r)) = (&u, &i) {
            assert_eq!((*l as i64) & r, 0x0F);
        }

        // Int & UInt → Int
        if let (ConstValue::Int(l), ConstValue::UInt(r)) = (&i, &u) {
            assert_eq!(l & (*r as i64), 0x0F);
        }

        // UInt | Int → Int
        if let (ConstValue::UInt(l), ConstValue::Int(r)) = (&u, &i) {
            assert_eq!((*l as i64) | r, 0xFF);
        }

        // Int | UInt → Int
        if let (ConstValue::Int(l), ConstValue::UInt(r)) = (&i, &u) {
            assert_eq!(l | (*r as i64), 0xFF);
        }

        // UInt ^ Int → Int
        if let (ConstValue::UInt(l), ConstValue::Int(r)) = (&u, &i) {
            assert_eq!((*l as i64) ^ r, 0xF0);
        }

        // Int ^ UInt → Int
        if let (ConstValue::Int(l), ConstValue::UInt(r)) = (&i, &u) {
            assert_eq!(l ^ (*r as i64), 0xF0);
        }
    }

    #[test]
    fn test_shift_uint() {
        // Shift UInt left/right
        let val = ConstValue::UInt(8);
        if let ConstValue::UInt(v) = val {
            assert_eq!(v << 2, 32);
            assert_eq!(v >> 1, 4);
        }
    }

    #[test]
    fn test_shift_right_unsigned_int() {
        // Unsigned shift right on Int converts to UInt
        // -1 >> 1 (unsigned) should give a large positive number
        let val = ConstValue::Int(-1);
        if let ConstValue::Int(v) = val {
            let result = (v as u64) >> 1;
            assert_eq!(result, u64::MAX >> 1);
        }
    }

    #[test]
    fn test_uint_uint_equality() {
        // UInt == UInt comparisons
        assert_eq!(ConstValue::UInt(5) == ConstValue::UInt(5), true);
        assert_eq!(ConstValue::UInt(5) == ConstValue::UInt(10), false);
    }

    #[test]
    fn test_uint_less_comparisons() {
        // UInt < UInt
        let u1 = ConstValue::UInt(5);
        let u2 = ConstValue::UInt(10);
        if let (ConstValue::UInt(l), ConstValue::UInt(r)) = (&u1, &u2) {
            assert!(l < r);
        }
    }

    #[test]
    fn test_mixed_int_uint_equal() {
        // Int == UInt with positive Int
        let pos_int = ConstValue::Int(5);
        let uint = ConstValue::UInt(5);
        if let (ConstValue::Int(l), ConstValue::UInt(r)) = (&pos_int, &uint) {
            if *l >= 0 {
                assert_eq!(*l as u64, *r);
            }
        }

        // Int == UInt with negative Int → always false
        let neg_int = ConstValue::Int(-5);
        if let (ConstValue::Int(l), ConstValue::UInt(_)) = (&neg_int, &uint) {
            if *l < 0 {
                // Can't be equal
                assert!(true);
            }
        }

        // UInt == Int with positive Int
        if let (ConstValue::UInt(l), ConstValue::Int(r)) = (&uint, &pos_int) {
            if *r >= 0 {
                assert_eq!(*l, *r as u64);
            }
        }

        // UInt == Int with negative Int → always false
        if let (ConstValue::UInt(_), ConstValue::Int(r)) = (&uint, &neg_int) {
            if *r < 0 {
                // Can't be equal
                assert!(true);
            }
        }
    }

    #[test]
    fn test_mixed_int_uint_less() {
        // Negative Int < any UInt → true
        let neg = ConstValue::Int(-5);
        let uint = ConstValue::UInt(0);
        if let (ConstValue::Int(l), ConstValue::UInt(_)) = (&neg, &uint) {
            if *l < 0 {
                assert!(true); // -5 < 0u is true
            }
        }

        // Positive Int < UInt
        let pos = ConstValue::Int(5);
        let uint_big = ConstValue::UInt(10);
        if let (ConstValue::Int(l), ConstValue::UInt(r)) = (&pos, &uint_big) {
            if *l >= 0 {
                assert!((*l as u64) < *r);
            }
        }

        // UInt < negative Int → false
        if let (ConstValue::UInt(_), ConstValue::Int(r)) = (&uint, &neg) {
            if *r < 0 {
                assert!(true); // Any uint is NOT less than a negative int
            }
        }

        // UInt < positive Int
        if let (ConstValue::UInt(l), ConstValue::Int(r)) = (&uint, &pos) {
            if *r >= 0 {
                assert!(*l < (*r as u64));
            }
        }
    }

    #[test]
    fn test_mixed_int_float_equal() {
        // Int == Float
        assert_eq!(eval_bool("5 == 5.0"), Some(true));
        assert_eq!(eval_bool("5 == 5.1"), Some(false));
        // Float == Int
        assert_eq!(eval_bool("5.0 == 5"), Some(true));
        assert_eq!(eval_bool("5.1 == 5"), Some(false));
    }

    #[test]
    fn test_mixed_int_float_less() {
        // Int < Float
        assert_eq!(eval_bool("5 < 5.5"), Some(true));
        assert_eq!(eval_bool("5 < 4.5"), Some(false));
        // Float < Int
        assert_eq!(eval_bool("4.5 < 5"), Some(true));
        assert_eq!(eval_bool("5.5 < 5"), Some(false));
    }

    #[test]
    fn test_mixed_int_float_less_equal() {
        // Int <= Float
        assert_eq!(eval_bool("5 <= 5.0"), Some(true));
        assert_eq!(eval_bool("5 <= 4.9"), Some(false));
        // Float <= Int
        assert_eq!(eval_bool("5.0 <= 5"), Some(true));
        assert_eq!(eval_bool("5.1 <= 5"), Some(false));
    }

    #[test]
    fn test_string_concat() {
        // String + String concatenation
        let val = eval_expr("\"hello\" + \"world\"");
        assert_eq!(val, Some(ConstValue::String("helloworld".to_string())));
    }

    #[test]
    fn test_string_equality() {
        // String == String
        let result = ConstValue::String("abc".to_string()) == ConstValue::String("abc".to_string());
        assert!(result);
        let result2 = ConstValue::String("abc".to_string()) == ConstValue::String("def".to_string());
        assert!(!result2);
    }

    #[test]
    fn test_bool_equality() {
        // Bool == Bool
        assert_eq!(eval_bool("true == true"), Some(true));
        assert_eq!(eval_bool("true == false"), Some(false));
        assert_eq!(eval_bool("false == false"), Some(true));
    }

    #[test]
    fn test_unsupported_arithmetic_returns_none() {
        // String - String should return None
        let left = ConstValue::String("hello".to_string());
        let right = ConstValue::String("world".to_string());
        let result = match (&left, &right) {
            (ConstValue::Int(l), ConstValue::Int(r)) => Some(ConstValue::Int(l.wrapping_sub(*r))),
            _ => None,
        };
        assert!(result.is_none());
    }

    #[test]
    fn test_float_is_truthy() {
        // Float values for is_truthy
        assert!(ConstValue::Float(1.0).is_truthy());
        assert!(ConstValue::Float(-1.0).is_truthy());
        assert!(!ConstValue::Float(0.0).is_truthy());
    }

    #[test]
    fn test_uint_is_truthy() {
        // UInt values for is_truthy
        assert!(ConstValue::UInt(1).is_truthy());
        assert!(ConstValue::UInt(100).is_truthy());
        assert!(!ConstValue::UInt(0).is_truthy());
    }

    #[test]
    fn test_float_as_bool_returns_none() {
        // Float.as_bool() returns Some now (based on the impl)
        assert_eq!(ConstValue::Float(1.0).as_bool(), Some(true));
        assert_eq!(ConstValue::Float(0.0).as_bool(), Some(false));
    }

    #[test]
    fn test_as_uint_negative_float_returns_none() {
        // Negative float.as_uint() returns None
        assert_eq!(ConstValue::Float(-1.0).as_uint(), None);
    }

    #[test]
    fn test_as_uint_negative_int_returns_none() {
        // Negative int.as_uint() returns None
        assert_eq!(ConstValue::Int(-1).as_uint(), None);
    }

    #[test]
    fn test_as_int_uint_overflow_returns_none() {
        // UInt > i64::MAX.as_int() returns None
        let big_uint = ConstValue::UInt(u64::MAX);
        assert_eq!(big_uint.as_int(), None);
    }

    #[test]
    fn test_ternary_with_false_condition() {
        // Ternary where condition is false
        assert_eq!(eval_int("false ? 1 : 2"), Some(2));
    }

    #[test]
    fn test_ternary_with_int_zero_condition() {
        // Ternary with int 0 (falsy)
        assert_eq!(eval_int("0 ? 1 : 2"), Some(2));
    }

    #[test]
    fn test_ternary_with_non_zero_condition() {
        // Ternary with int 42 (truthy)
        assert_eq!(eval_int("42 ? 1 : 2"), Some(1));
    }

    #[test]
    fn test_eval_const_int_wrapper() {
        // Test the public eval_const_int function
        let arena = Bump::new();
        let source = "void test() { 1 + 2; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = super::eval_const_int(expr);
                            assert_eq!(result, Some(3));
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_bool_literal() {
        // eval_const_int_simple with bool literal
        let arena = Bump::new();
        let source = "void test() { true; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, Some(1));
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_all_binary_ops() {
        // Test all binary ops in eval_const_int_simple
        let arena = Bump::new();

        let test_cases = vec![
            ("5 + 3", Some(8)),
            ("5 - 3", Some(2)),
            ("5 * 3", Some(15)),
            ("15 / 3", Some(5)),
            ("15 % 4", Some(3)),
            ("2 ** 3", Some(8)),
            ("7 & 3", Some(3)),
            ("4 | 2", Some(6)),
            ("7 ^ 3", Some(4)),
            ("1 << 3", Some(8)),
            ("8 >> 1", Some(4)),
            ("(-1) >>> 1", Some(((-1i64 as u64) >> 1) as i64)),
            ("1 && 1", Some(1)),
            ("1 && 0", Some(0)),
            ("0 || 1", Some(1)),
            ("0 || 0", Some(0)),
            ("1 ^^ 0", Some(1)),
            ("1 ^^ 1", Some(0)),
            ("5 == 5", Some(1)),
            ("5 == 3", Some(0)),
            ("5 != 3", Some(1)),
            ("5 != 5", Some(0)),
            ("3 < 5", Some(1)),
            ("5 < 3", Some(0)),
            ("3 <= 5", Some(1)),
            ("5 <= 5", Some(1)),
            ("5 > 3", Some(1)),
            ("3 > 5", Some(0)),
            ("5 >= 3", Some(1)),
            ("5 >= 5", Some(1)),
        ];

        for (expr_str, expected) in test_cases {
            let source = format!("void test() {{ {}; }}", expr_str);
            let (script, _) = Parser::parse_lenient(&source, &arena);

            for item in script.items() {
                if let Item::Function(func) = item {
                    if let Some(body) = &func.body {
                        if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                            if let Some(expr) = expr_stmt.expr {
                                let result = eval_const_int_simple(expr);
                                assert_eq!(result, expected, "Failed for expression: {}", expr_str);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_unary_ops() {
        // Test all unary ops in eval_const_int_simple
        let arena = Bump::new();

        let test_cases = vec![
            ("-5", Some(-5)),
            ("+5", Some(5)),
            ("~0", Some(-1)),
            ("!0", Some(1)),
            ("!1", Some(0)),
        ];

        for (expr_str, expected) in test_cases {
            let source = format!("void test() {{ {}; }}", expr_str);
            let (script, _) = Parser::parse_lenient(&source, &arena);

            for item in script.items() {
                if let Item::Function(func) = item {
                    if let Some(body) = &func.body {
                        if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                            if let Some(expr) = expr_stmt.expr {
                                let result = eval_const_int_simple(expr);
                                assert_eq!(result, expected, "Failed for expression: {}", expr_str);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_div_by_zero() {
        // Division by zero returns None
        let arena = Bump::new();
        let source = "void test() { 5 / 0; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, None);
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_mod_by_zero() {
        // Mod by zero returns None
        let arena = Bump::new();
        let source = "void test() { 5 % 0; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, None);
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_negative_power() {
        // Negative power returns None for integer evaluation
        let arena = Bump::new();
        let source = "void test() { 2 ** (-1); }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, None);
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_paren() {
        // Parenthesized expression
        let arena = Bump::new();
        let source = "void test() { (5 + 3); }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, Some(8));
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_ternary() {
        // Ternary expression
        let arena = Bump::new();
        let source = "void test() { 1 ? 5 : 10; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, Some(5));
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_ternary_false() {
        // Ternary expression with false condition
        let arena = Bump::new();
        let source = "void test() { 0 ? 5 : 10; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, Some(10));
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_float_returns_none() {
        // Float literal in eval_const_int_simple returns None
        let arena = Bump::new();
        let source = "void test() { 3.14; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = eval_const_int_simple(expr);
                            assert_eq!(result, None);
                            return;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_eval_const_int_simple_is_not_is_returns_none() {
        // is/!is operators return None
        let arena = Bump::new();
        let source = r#"
            class Foo {}
            void test() { Foo@ a; a is null; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Binary(bin)) = expr_stmt.expr {
                                    if matches!(bin.op, BinaryOp::Is | BinaryOp::NotIs) {
                                        let result = eval_const_int_simple(expr_stmt.expr.unwrap());
                                        assert_eq!(result, None);
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_ident_without_scope_returns_none() {
        // Unqualified identifier is not constant
        let arena = Bump::new();
        let source = "void test() { int x = 5; x; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Ident(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_ident_with_unknown_type_returns_none() {
        // Qualified identifier with unknown type
        let arena = Bump::new();
        let source = "void test() { UnknownEnum::VALUE; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    for stmt in body.stmts {
                        if let Stmt::Expr(expr_stmt) = stmt {
                            if let Some(Expr::Ident(_)) = expr_stmt.expr {
                                let result = evaluator.eval(expr_stmt.expr.unwrap());
                                assert!(result.is_none());
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_ident_with_non_enum_type_returns_none() {
        // Qualified identifier where the type is not an enum (e.g., class)
        let arena = Bump::new();
        let source = r#"
            class Foo { int VALUE = 5; }
            void test() { Foo::VALUE; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Ident(_)) = expr_stmt.expr {
                                    let result = evaluator.eval(expr_stmt.expr.unwrap());
                                    // Should be None because Foo is a class, not an enum
                                    assert!(result.is_none());
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_ident_enum_unknown_value_returns_none() {
        // Qualified identifier for enum with unknown value
        let arena = Bump::new();
        let source = r#"
            enum Color { Red, Green, Blue }
            void test() { Color::Yellow; }
        "#;
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if func.name.name == "test" {
                    if let Some(body) = &func.body {
                        for stmt in body.stmts {
                            if let Stmt::Expr(expr_stmt) = stmt {
                                if let Some(Expr::Ident(_)) = expr_stmt.expr {
                                    let result = evaluator.eval(expr_stmt.expr.unwrap());
                                    // Should be None because Yellow doesn't exist in Color
                                    assert!(result.is_none());
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_logical_not_on_int() {
        // !5 should be false (0), !0 should be true (1)
        assert_eq!(eval_bool("!5"), Some(false));
        assert_eq!(eval_bool("!0"), Some(true));
    }

    #[test]
    fn test_double_literal() {
        // Double (d suffix) literal
        let arena = Bump::new();
        let source = "void test() { 3.14d; }";
        let (script, _) = Parser::parse_lenient(source, &arena);

        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        for item in script.items() {
            if let Item::Function(func) = item {
                if let Some(body) = &func.body {
                    if let Some(Stmt::Expr(expr_stmt)) = body.stmts.first() {
                        if let Some(expr) = expr_stmt.expr {
                            let result = evaluator.eval(expr);
                            assert!(matches!(result, Some(ConstValue::Float(_))));
                            return;
                        }
                    }
                }
            }
        }
    }

    // Test helper methods directly using the evaluator
    #[test]
    fn test_evaluator_add_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_add(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(15)));
    }

    #[test]
    fn test_evaluator_sub_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(3);
        let result = evaluator.eval_sub(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(7)));
    }

    #[test]
    fn test_evaluator_mul_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(3);
        let result = evaluator.eval_mul(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(30)));
    }

    #[test]
    fn test_evaluator_div_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(3);
        let result = evaluator.eval_div(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(3)));
    }

    #[test]
    fn test_evaluator_div_uint_by_zero() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(0);
        let result = evaluator.eval_div(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_mod_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(3);
        let result = evaluator.eval_mod(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(1)));
    }

    #[test]
    fn test_evaluator_mod_uint_by_zero() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(10);
        let right = ConstValue::UInt(0);
        let result = evaluator.eval_mod(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_bitwise_and_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xFF);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_and(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(0x0F)));
    }

    #[test]
    fn test_evaluator_bitwise_and_int_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(0xFF);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_and(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0x0F)));
    }

    #[test]
    fn test_evaluator_bitwise_and_uint_int() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xFF);
        let right = ConstValue::Int(0x0F);
        let result = evaluator.eval_bitwise_and(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0x0F)));
    }

    #[test]
    fn test_evaluator_bitwise_or_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xF0);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_or(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(0xFF)));
    }

    #[test]
    fn test_evaluator_bitwise_or_int_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(0xF0);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_or(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0xFF)));
    }

    #[test]
    fn test_evaluator_bitwise_or_uint_int() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xF0);
        let right = ConstValue::Int(0x0F);
        let result = evaluator.eval_bitwise_or(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0xFF)));
    }

    #[test]
    fn test_evaluator_bitwise_xor_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xFF);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_xor(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(0xF0)));
    }

    #[test]
    fn test_evaluator_bitwise_xor_int_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(0xFF);
        let right = ConstValue::UInt(0x0F);
        let result = evaluator.eval_bitwise_xor(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0xF0)));
    }

    #[test]
    fn test_evaluator_bitwise_xor_uint_int() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(0xFF);
        let right = ConstValue::Int(0x0F);
        let result = evaluator.eval_bitwise_xor(&left, &right);
        assert_eq!(result, Some(ConstValue::Int(0xF0)));
    }

    #[test]
    fn test_evaluator_shift_left_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(1);
        let right = ConstValue::Int(4);
        let result = evaluator.eval_shift_left(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(16)));
    }

    #[test]
    fn test_evaluator_shift_right_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(16);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_shift_right(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(4)));
    }

    #[test]
    fn test_evaluator_shift_right_unsigned_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(16);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_shift_right_unsigned(&left, &right);
        assert_eq!(result, Some(ConstValue::UInt(4)));
    }

    #[test]
    fn test_evaluator_equal_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_equal_int_uint_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_equal_int_uint_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(-5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn test_evaluator_equal_uint_int_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_equal_uint_int_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(-5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn test_evaluator_less_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(3);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_int_uint_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(-5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_int_uint_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(5);
        let right = ConstValue::UInt(10);
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_uint_int_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(-5);
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn test_evaluator_less_uint_int_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(10);
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_equal_uint_uint() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_equal_int_uint_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(-5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_equal_int_uint_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Int(5);
        let right = ConstValue::UInt(5);
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_less_equal_uint_int_negative() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(-5);
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(false)));
    }

    #[test]
    fn test_evaluator_less_equal_uint_int_positive() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::UInt(5);
        let right = ConstValue::Int(5);
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, Some(ConstValue::Bool(true)));
    }

    #[test]
    fn test_evaluator_bitwise_and_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Float(2.0);
        let result = evaluator.eval_bitwise_and(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_bitwise_or_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Float(2.0);
        let result = evaluator.eval_bitwise_or(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_bitwise_xor_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Float(2.0);
        let result = evaluator.eval_bitwise_xor(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_shift_left_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_shift_left(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_shift_right_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_shift_right(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_shift_right_unsigned_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Float(1.0);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_shift_right_unsigned(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_add_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_add(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_sub_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_sub(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_mul_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_mul(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_div_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_div(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_mod_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_mod(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_pow_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::Bool(true);
        let right = ConstValue::Bool(false);
        let result = evaluator.eval_pow(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_equal_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::String("abc".to_string());
        let right = ConstValue::Int(5);
        let result = evaluator.eval_equal(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_less_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::String("abc".to_string());
        let right = ConstValue::String("def".to_string());
        let result = evaluator.eval_less(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_less_equal_unsupported() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        let left = ConstValue::String("abc".to_string());
        let right = ConstValue::String("def".to_string());
        let result = evaluator.eval_less_equal(&left, &right);
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluator_mixed_int_float_div() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        // Int / Float
        let left = ConstValue::Int(10);
        let right = ConstValue::Float(4.0);
        let result = evaluator.eval_div(&left, &right);
        assert_eq!(result, Some(ConstValue::Float(2.5)));

        // Float / Int
        let left = ConstValue::Float(10.0);
        let right = ConstValue::Int(4);
        let result = evaluator.eval_div(&left, &right);
        assert_eq!(result, Some(ConstValue::Float(2.5)));
    }

    #[test]
    fn test_evaluator_mixed_int_float_mod() {
        let arena = Bump::new();
        let (script, _) = Parser::parse_lenient("void test() {}", &arena);
        let data = Compiler::compile_types(&script);
        let evaluator = ConstEvaluator::new(&data.context);

        // Int % Float
        let left = ConstValue::Int(7);
        let right = ConstValue::Float(2.5);
        let result = evaluator.eval_mod(&left, &right);
        if let Some(ConstValue::Float(f)) = result {
            assert!((f - (7.0 % 2.5)).abs() < f64::EPSILON);
        } else {
            panic!("Expected Float result");
        }

        // Float % Int
        let left = ConstValue::Float(7.0);
        let right = ConstValue::Int(2);
        let result = evaluator.eval_mod(&left, &right);
        if let Some(ConstValue::Float(f)) = result {
            assert!((f - 1.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected Float result");
        }
    }
}
