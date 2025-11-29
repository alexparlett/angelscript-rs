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

use crate::ast::expr::{Expr, LiteralKind};
use crate::ast::{BinaryOp, UnaryOp};
use crate::semantic::types::{Registry, TypeDef};

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
pub struct ConstEvaluator<'a, 'src, 'ast> {
    registry: &'a Registry<'src, 'ast>,
}

impl<'a, 'src, 'ast> ConstEvaluator<'a, 'src, 'ast> {
    /// Create a new constant evaluator.
    pub fn new(registry: &'a Registry<'src, 'ast>) -> Self {
        Self { registry }
    }

    /// Evaluate an expression as a constant value.
    ///
    /// Returns `None` if the expression cannot be evaluated at compile time.
    pub fn eval(&self, expr: &Expr<'src, 'ast>) -> Option<ConstValue> {
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
    pub fn eval_as_int(&self, expr: &Expr<'src, 'ast>) -> Option<i64> {
        self.eval(expr).and_then(|v| v.as_int())
    }

    /// Evaluate and return as u64 (convenience method).
    pub fn eval_as_uint(&self, expr: &Expr<'src, 'ast>) -> Option<u64> {
        self.eval(expr).and_then(|v| v.as_uint())
    }

    /// Evaluate and return as f64 (convenience method).
    pub fn eval_as_float(&self, expr: &Expr<'src, 'ast>) -> Option<f64> {
        self.eval(expr).and_then(|v| v.as_float())
    }

    /// Evaluate and return as bool (convenience method).
    pub fn eval_as_bool(&self, expr: &Expr<'src, 'ast>) -> Option<bool> {
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

    fn eval_unary(&self, op: UnaryOp, operand: &Expr<'src, 'ast>) -> Option<ConstValue> {
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
        left: &Expr<'src, 'ast>,
        op: BinaryOp,
        right: &Expr<'src, 'ast>,
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
        condition: &Expr<'src, 'ast>,
        then_expr: &Expr<'src, 'ast>,
        else_expr: &Expr<'src, 'ast>,
    ) -> Option<ConstValue> {
        let cond = self.eval(condition)?;
        if cond.is_truthy() {
            self.eval(then_expr)
        } else {
            self.eval(else_expr)
        }
    }

    fn eval_ident(&self, ident: &crate::ast::expr::IdentExpr<'src, 'ast>) -> Option<ConstValue> {
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
            if let Some(type_id) = self.registry.lookup_type(&enum_name) {
                let typedef = self.registry.get_type(type_id);
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
    use crate::ast::decl::Item;
    use crate::ast::stmt::Stmt;
    use crate::parse_lenient;
    use crate::semantic::Registrar;
    use bumpalo::Bump;

    fn eval_expr(source: &str) -> Option<ConstValue> {
        let arena = Bump::new();
        // Wrap expression in a function to make it valid AngelScript
        let full_source = format!("void test() {{ {}; }}", source);
        let (script, errors) = parse_lenient(&full_source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        // Register to get a registry
        let data = Registrar::register(&script);
        let evaluator = ConstEvaluator::new(&data.registry);

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
        let (script, errors) = parse_lenient(source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        let data = Registrar::register(&script);
        let evaluator = ConstEvaluator::new(&data.registry);

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
        let (script, errors) = parse_lenient(source, &arena);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        let data = Registrar::register(&script);
        let evaluator = ConstEvaluator::new(&data.registry);

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
        let (script, _) = parse_lenient(source, &arena);

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
}
