//! Operator resolution for expression compilation.
//!
//! This module determines how to compile operators given operand types:
//! - Primitive operations use direct opcodes (AddI32, MulF64, etc.)
//! - User-defined operators use method calls (opAdd, opEquals, etc.)

mod binary;
mod primitive;
mod unary;

pub use binary::resolve_binary;
pub use unary::resolve_unary;

use angelscript_core::{DataType, TypeHash};

use crate::bytecode::OpCode;

/// Result of operator resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum OperatorResolution {
    /// Use a primitive opcode.
    Primitive {
        /// The opcode to emit.
        opcode: OpCode,
        /// Conversion for left operand (if types don't match exactly).
        left_conv: Option<OpCode>,
        /// Conversion for right operand (if types don't match exactly).
        right_conv: Option<OpCode>,
        /// Result type of the operation.
        result_type: DataType,
    },
    /// Call a method on the left operand (e.g., `left.opAdd(right)`).
    MethodOnLeft {
        /// Function hash of the method to call.
        method_hash: TypeHash,
        /// Conversion needed for the argument.
        arg_conversion: Option<OpCode>,
        /// Result type of the operation.
        result_type: DataType,
    },
    /// Call a reverse method on the right operand (e.g., `right.opAddR(left)`).
    MethodOnRight {
        /// Function hash of the reverse method to call.
        method_hash: TypeHash,
        /// Conversion needed for the argument.
        arg_conversion: Option<OpCode>,
        /// Result type of the operation.
        result_type: DataType,
    },
    /// Handle/pointer comparison (default for `is`/`!is`).
    HandleComparison {
        /// True for `!is`, false for `is`.
        negate: bool,
    },
}

/// Result of unary operator resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryResolution {
    /// Use a primitive opcode.
    Primitive {
        /// The opcode to emit.
        opcode: OpCode,
        /// Result type of the operation.
        result_type: DataType,
    },
    /// No operation needed (e.g., unary `+` on numeric types).
    NoOp {
        /// Result type (same as operand type).
        result_type: DataType,
    },
    /// Call a method on the operand (e.g., `obj.opNeg()`).
    Method {
        /// Function hash of the method to call.
        method_hash: TypeHash,
        /// Result type of the operation.
        result_type: DataType,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;
    use angelscript_parser::ast::{BinaryOp, UnaryOp};

    // =========================================================================
    // Primitive Binary Operator Tests
    // =========================================================================

    #[test]
    fn resolve_i32_add() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i64_add() {
        let left = DataType::simple(primitives::INT64);
        let right = DataType::simple(primitives::INT64);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddI64,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT64),
            })
        );
    }

    #[test]
    fn resolve_f32_add() {
        let left = DataType::simple(primitives::FLOAT);
        let right = DataType::simple(primitives::FLOAT);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddF32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::FLOAT),
            })
        );
    }

    #[test]
    fn resolve_f64_add() {
        let left = DataType::simple(primitives::DOUBLE);
        let right = DataType::simple(primitives::DOUBLE);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddF64,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::DOUBLE),
            })
        );
    }

    #[test]
    fn resolve_i32_sub() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Sub);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::SubI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_mul() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Mul);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::MulI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_div() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Div);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::DivI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_mod() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Mod);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::ModI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    // =========================================================================
    // Comparison Operator Tests
    // =========================================================================

    #[test]
    fn resolve_i32_equal() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Equal);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::EqI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_i32_less() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Less);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::LtI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_i32_less_equal() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::LessEqual);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::LeI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_i32_greater() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Greater);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::GtI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_i32_greater_equal() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::GreaterEqual);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::GeI32,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_bool_equal() {
        let left = DataType::simple(primitives::BOOL);
        let right = DataType::simple(primitives::BOOL);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Equal);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::EqBool,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    // =========================================================================
    // Bitwise Operator Tests
    // =========================================================================

    #[test]
    fn resolve_i32_bitwise_and() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::BitwiseAnd);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::BitAnd,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_bitwise_or() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::BitwiseOr);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::BitOr,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_bitwise_xor() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::BitwiseXor);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::BitXor,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_shift_left() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::ShiftLeft);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::Shl,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_shift_right() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::ShiftRight);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::Shr,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i32_shift_right_unsigned() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::ShiftRightUnsigned);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::Ushr,
                left_conv: None,
                right_conv: None,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    // =========================================================================
    // Type Promotion Tests
    // =========================================================================

    #[test]
    fn resolve_i32_f64_add_promotes_to_f64() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::DOUBLE);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddF64,
                left_conv: Some(OpCode::I32toF64),
                right_conv: None,
                result_type: DataType::simple(primitives::DOUBLE),
            })
        );
    }

    #[test]
    fn resolve_f64_i32_add_promotes_to_f64() {
        let left = DataType::simple(primitives::DOUBLE);
        let right = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddF64,
                left_conv: None,
                right_conv: Some(OpCode::I32toF64),
                result_type: DataType::simple(primitives::DOUBLE),
            })
        );
    }

    #[test]
    fn resolve_i32_i64_add_promotes_to_i64() {
        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT64);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddI64,
                left_conv: Some(OpCode::I32toI64),
                right_conv: None,
                result_type: DataType::simple(primitives::INT64),
            })
        );
    }

    #[test]
    fn resolve_f32_f64_add_promotes_to_f64() {
        let left = DataType::simple(primitives::FLOAT);
        let right = DataType::simple(primitives::DOUBLE);

        let result = primitive::try_primitive_binary(&left, &right, BinaryOp::Add);

        assert_eq!(
            result,
            Some(OperatorResolution::Primitive {
                opcode: OpCode::AddF64,
                left_conv: Some(OpCode::F32toF64),
                right_conv: None,
                result_type: DataType::simple(primitives::DOUBLE),
            })
        );
    }

    // =========================================================================
    // Unary Operator Tests
    // =========================================================================

    #[test]
    fn resolve_i32_neg() {
        let operand = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::Neg);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::NegI32,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_i64_neg() {
        let operand = DataType::simple(primitives::INT64);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::Neg);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::NegI64,
                result_type: DataType::simple(primitives::INT64),
            })
        );
    }

    #[test]
    fn resolve_f32_neg() {
        let operand = DataType::simple(primitives::FLOAT);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::Neg);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::NegF32,
                result_type: DataType::simple(primitives::FLOAT),
            })
        );
    }

    #[test]
    fn resolve_f64_neg() {
        let operand = DataType::simple(primitives::DOUBLE);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::Neg);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::NegF64,
                result_type: DataType::simple(primitives::DOUBLE),
            })
        );
    }

    #[test]
    fn resolve_bool_logical_not() {
        let operand = DataType::simple(primitives::BOOL);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::LogicalNot);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::Not,
                result_type: DataType::simple(primitives::BOOL),
            })
        );
    }

    #[test]
    fn resolve_i32_bitwise_not() {
        let operand = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::BitwiseNot);

        assert_eq!(
            result,
            Some(UnaryResolution::Primitive {
                opcode: OpCode::BitNot,
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    #[test]
    fn resolve_numeric_plus_is_noop() {
        let operand = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_unary(&operand, UnaryOp::Plus);

        assert_eq!(
            result,
            Some(UnaryResolution::NoOp {
                result_type: DataType::simple(primitives::INT32),
            })
        );
    }

    // =========================================================================
    // Non-Primitive Type Tests
    // =========================================================================

    #[test]
    fn non_primitive_returns_none() {
        let class_type = DataType::simple(TypeHash::from_name("MyClass"));
        let int_type = DataType::simple(primitives::INT32);

        let result = primitive::try_primitive_binary(&class_type, &int_type, BinaryOp::Add);

        assert_eq!(result, None);
    }

    #[test]
    fn non_primitive_unary_returns_none() {
        let class_type = DataType::simple(TypeHash::from_name("MyClass"));

        let result = primitive::try_primitive_unary(&class_type, UnaryOp::Neg);

        assert_eq!(result, None);
    }
}
