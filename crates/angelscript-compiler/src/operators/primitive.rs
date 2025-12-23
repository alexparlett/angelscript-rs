//! Primitive type operator resolution.
//!
//! Handles operator resolution for built-in primitive types (int, float, etc.)
//! using direct VM opcodes. Enums are treated as int32 for operator purposes.

use angelscript_core::{DataType, TypeHash, primitives};
use angelscript_parser::ast::{BinaryOp, UnaryOp};

use super::{OperatorResolution, UnaryResolution};
use crate::bytecode::OpCode;

/// Normalize a type for operator resolution.
/// Enums are treated as int32 since they're just integers under the hood.
#[inline]
fn normalize_type_hash(data_type: &DataType) -> TypeHash {
    if data_type.is_enum {
        primitives::INT32
    } else {
        data_type.type_hash
    }
}

/// Try to resolve a binary operator for primitive types.
///
/// Returns `Some(resolution)` if both operands are primitives (or enums) and the operator
/// is supported, `None` otherwise.
pub fn try_primitive_binary(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
) -> Option<OperatorResolution> {
    // Get the underlying type hashes, treating enums as int32
    let left_hash = normalize_type_hash(left);
    let right_hash = normalize_type_hash(right);

    // Both must be numeric or bool primitives
    if !is_numeric_primitive(left_hash) && left_hash != primitives::BOOL {
        return None;
    }
    if !is_numeric_primitive(right_hash) && right_hash != primitives::BOOL {
        return None;
    }

    match op {
        // Arithmetic operators - numeric only
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
            resolve_arithmetic(left_hash, right_hash, op)
        }
        BinaryOp::Mod => resolve_modulo(left_hash, right_hash),

        // Comparison operators
        BinaryOp::Equal | BinaryOp::NotEqual => resolve_equality(left_hash, right_hash, op),
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            resolve_comparison(left_hash, right_hash, op)
        }

        // Bitwise operators - integer only
        BinaryOp::BitwiseAnd
        | BinaryOp::BitwiseOr
        | BinaryOp::BitwiseXor
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight
        | BinaryOp::ShiftRightUnsigned => resolve_bitwise(left_hash, right_hash, op),

        // Exponentiation
        BinaryOp::Pow => resolve_pow(left_hash, right_hash),

        // Logical operators are NOT primitive operations (need short-circuit evaluation)
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => None,

        // Identity operators handled elsewhere (handle comparison)
        BinaryOp::Is | BinaryOp::NotIs => None,
    }
}

/// Try to resolve a unary operator for primitive types.
///
/// Returns `Some(resolution)` if the operand is a primitive (or enum) and the operator
/// is supported, `None` otherwise.
pub fn try_primitive_unary(operand: &DataType, op: UnaryOp) -> Option<UnaryResolution> {
    // Normalize type hash - enums become int32
    let type_hash = normalize_type_hash(operand);

    match op {
        UnaryOp::Neg => resolve_negation(type_hash),
        UnaryOp::Plus => resolve_plus(type_hash),
        UnaryOp::LogicalNot => {
            if type_hash == primitives::BOOL {
                Some(UnaryResolution::Primitive {
                    opcode: OpCode::Not,
                    result_type: DataType::simple(primitives::BOOL),
                })
            } else {
                None
            }
        }
        UnaryOp::BitwiseNot => {
            if is_integer_primitive(type_hash) {
                Some(UnaryResolution::Primitive {
                    opcode: OpCode::BitNot,
                    result_type: DataType::simple(type_hash),
                })
            } else {
                None
            }
        }
        // Pre increment/decrement handled at a higher level (requires lvalue)
        UnaryOp::PreInc | UnaryOp::PreDec => None,
        // Handle-of is not a primitive operation
        UnaryOp::HandleOf => None,
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn is_numeric_primitive(hash: TypeHash) -> bool {
    is_integer_primitive(hash) || is_float_primitive(hash)
}

fn is_integer_primitive(hash: TypeHash) -> bool {
    matches!(
        hash,
        h if h == primitives::INT8
            || h == primitives::INT16
            || h == primitives::INT32
            || h == primitives::INT64
            || h == primitives::UINT8
            || h == primitives::UINT16
            || h == primitives::UINT32
            || h == primitives::UINT64
    )
}

fn is_float_primitive(hash: TypeHash) -> bool {
    hash == primitives::FLOAT || hash == primitives::DOUBLE
}

/// Type promotion rank for arithmetic. Higher rank = wider type.
/// Returns None for non-promotable types.
fn promotion_rank(hash: TypeHash) -> Option<u8> {
    match hash {
        h if h == primitives::INT8 => Some(1),
        h if h == primitives::UINT8 => Some(2),
        h if h == primitives::INT16 => Some(3),
        h if h == primitives::UINT16 => Some(4),
        h if h == primitives::INT32 => Some(5),
        h if h == primitives::UINT32 => Some(6),
        h if h == primitives::INT64 => Some(7),
        h if h == primitives::UINT64 => Some(8),
        h if h == primitives::FLOAT => Some(9),
        h if h == primitives::DOUBLE => Some(10),
        _ => None,
    }
}

/// Get the promoted type for two types.
/// Both types are promoted to the wider of the two.
fn promote_types(left: TypeHash, right: TypeHash) -> Option<TypeHash> {
    let left_rank = promotion_rank(left)?;
    let right_rank = promotion_rank(right)?;

    if left_rank >= right_rank {
        Some(left)
    } else {
        Some(right)
    }
}

/// Get conversion opcode to convert `from` type to `to` type.
fn conversion_opcode(from: TypeHash, to: TypeHash) -> Option<OpCode> {
    if from == to {
        return None;
    }

    // Common cases for type promotion
    match (from, to) {
        // int32 promotions
        (f, t) if f == primitives::INT32 && t == primitives::INT64 => Some(OpCode::I32toI64),
        (f, t) if f == primitives::INT32 && t == primitives::FLOAT => Some(OpCode::I32toF32),
        (f, t) if f == primitives::INT32 && t == primitives::DOUBLE => Some(OpCode::I32toF64),

        // int64 promotions
        (f, t) if f == primitives::INT64 && t == primitives::FLOAT => Some(OpCode::I64toF32),
        (f, t) if f == primitives::INT64 && t == primitives::DOUBLE => Some(OpCode::I64toF64),

        // float promotions
        (f, t) if f == primitives::FLOAT && t == primitives::DOUBLE => Some(OpCode::F32toF64),

        // int8 promotions
        (f, t) if f == primitives::INT8 && t == primitives::INT16 => Some(OpCode::I8toI16),
        (f, t) if f == primitives::INT8 && t == primitives::INT32 => Some(OpCode::I8toI32),
        (f, t) if f == primitives::INT8 && t == primitives::INT64 => Some(OpCode::I8toI64),

        // int16 promotions
        (f, t) if f == primitives::INT16 && t == primitives::INT32 => Some(OpCode::I16toI32),
        (f, t) if f == primitives::INT16 && t == primitives::INT64 => Some(OpCode::I16toI64),

        // uint8 promotions
        (f, t) if f == primitives::UINT8 && t == primitives::UINT16 => Some(OpCode::U8toU16),
        (f, t) if f == primitives::UINT8 && t == primitives::UINT32 => Some(OpCode::U8toU32),
        (f, t) if f == primitives::UINT8 && t == primitives::UINT64 => Some(OpCode::U8toU64),

        // uint16 promotions
        (f, t) if f == primitives::UINT16 && t == primitives::UINT32 => Some(OpCode::U16toU32),
        (f, t) if f == primitives::UINT16 && t == primitives::UINT64 => Some(OpCode::U16toU64),

        // uint32 promotions
        (f, t) if f == primitives::UINT32 && t == primitives::UINT64 => Some(OpCode::U32toU64),

        // Cross integer width promotions (promote smaller ints to larger types)
        (f, t) if f == primitives::INT8 && t == primitives::FLOAT => Some(OpCode::I32toF32),
        (f, t) if f == primitives::INT8 && t == primitives::DOUBLE => Some(OpCode::I32toF64),
        (f, t) if f == primitives::INT16 && t == primitives::FLOAT => Some(OpCode::I32toF32),
        (f, t) if f == primitives::INT16 && t == primitives::DOUBLE => Some(OpCode::I32toF64),

        _ => None,
    }
}

/// Get the arithmetic opcode for the given operation.
/// Now uses generic opcodes - VM determines type from stack values.
fn arithmetic_opcode(op: BinaryOp) -> Option<OpCode> {
    match op {
        BinaryOp::Add => Some(OpCode::Add),
        BinaryOp::Sub => Some(OpCode::Sub),
        BinaryOp::Mul => Some(OpCode::Mul),
        BinaryOp::Div => Some(OpCode::Div),
        _ => None,
    }
}

fn resolve_arithmetic(left: TypeHash, right: TypeHash, op: BinaryOp) -> Option<OperatorResolution> {
    // Only numeric types can do arithmetic
    if !is_numeric_primitive(left) || !is_numeric_primitive(right) {
        return None;
    }

    let promoted = promote_types(left, right)?;
    let opcode = arithmetic_opcode(op)?;
    let left_conv = conversion_opcode(left, promoted);
    let right_conv = conversion_opcode(right, promoted);

    Some(OperatorResolution::Primitive {
        opcode,
        left_conv,
        right_conv,
        result_type: DataType::simple(promoted),
    })
}

fn resolve_modulo(left: TypeHash, right: TypeHash) -> Option<OperatorResolution> {
    // Modulo only works on integers
    if !is_integer_primitive(left) || !is_integer_primitive(right) {
        return None;
    }

    let promoted = promote_types(left, right)?;

    // For smaller integer types, promote to int32
    let result_type = if promoted == primitives::INT64 || promoted == primitives::UINT64 {
        promoted
    } else {
        primitives::INT32
    };

    let left_conv = if left != result_type {
        conversion_opcode(left, result_type)
    } else {
        None
    };

    let right_conv = if right != result_type {
        conversion_opcode(right, result_type)
    } else {
        None
    };

    Some(OperatorResolution::Primitive {
        opcode: OpCode::Mod,
        left_conv,
        right_conv,
        result_type: DataType::simple(result_type),
    })
}

fn resolve_equality(left: TypeHash, right: TypeHash, op: BinaryOp) -> Option<OperatorResolution> {
    // Bool can be compared with bool
    if left == primitives::BOOL && right == primitives::BOOL {
        return Some(OperatorResolution::Primitive {
            opcode: OpCode::Eq,
            left_conv: None,
            right_conv: None,
            result_type: DataType::simple(primitives::BOOL),
        });
    }

    // Numeric types
    if !is_numeric_primitive(left) || !is_numeric_primitive(right) {
        return None;
    }

    let promoted = promote_types(left, right)?;
    let left_conv = conversion_opcode(left, promoted);
    let right_conv = conversion_opcode(right, promoted);

    // Note: NotEqual is handled by emitting Eq followed by Not at a higher level
    let _ = op; // Both Equal and NotEqual use same base opcode

    Some(OperatorResolution::Primitive {
        opcode: OpCode::Eq,
        left_conv,
        right_conv,
        result_type: DataType::simple(primitives::BOOL),
    })
}

fn resolve_comparison(left: TypeHash, right: TypeHash, op: BinaryOp) -> Option<OperatorResolution> {
    // Only numeric types can be compared with < > <= >=
    if !is_numeric_primitive(left) || !is_numeric_primitive(right) {
        return None;
    }

    let promoted = promote_types(left, right)?;

    let opcode = match op {
        BinaryOp::Less => OpCode::Lt,
        BinaryOp::LessEqual => OpCode::Le,
        BinaryOp::Greater => OpCode::Gt,
        BinaryOp::GreaterEqual => OpCode::Ge,
        _ => return None,
    };

    let left_conv = conversion_opcode(left, promoted);
    let right_conv = conversion_opcode(right, promoted);

    Some(OperatorResolution::Primitive {
        opcode,
        left_conv,
        right_conv,
        result_type: DataType::simple(primitives::BOOL),
    })
}

fn resolve_bitwise(left: TypeHash, right: TypeHash, op: BinaryOp) -> Option<OperatorResolution> {
    // Bitwise operators only work on integers
    if !is_integer_primitive(left) || !is_integer_primitive(right) {
        return None;
    }

    // For bitwise ops, we use int32 as the common type for now
    // (Shifts use left operand type, other ops use promoted type)
    let result_type = primitives::INT32;

    let opcode = match op {
        BinaryOp::BitwiseAnd => OpCode::BitAnd,
        BinaryOp::BitwiseOr => OpCode::BitOr,
        BinaryOp::BitwiseXor => OpCode::BitXor,
        BinaryOp::ShiftLeft => OpCode::Shl,
        BinaryOp::ShiftRight => OpCode::Shr,
        BinaryOp::ShiftRightUnsigned => OpCode::Ushr,
        _ => return None,
    };

    let left_conv = conversion_opcode(left, result_type);
    let right_conv = conversion_opcode(right, result_type);

    Some(OperatorResolution::Primitive {
        opcode,
        left_conv,
        right_conv,
        result_type: DataType::simple(result_type),
    })
}

fn resolve_negation(type_hash: TypeHash) -> Option<UnaryResolution> {
    // Negation works on all numeric types
    if !is_numeric_primitive(type_hash) {
        return None;
    }

    // For smaller types, the result is promoted to int32
    let result_type = match type_hash {
        t if t == primitives::INT8 || t == primitives::INT16 => primitives::INT32,
        _ => type_hash,
    };

    Some(UnaryResolution::Primitive {
        opcode: OpCode::Neg,
        result_type: DataType::simple(result_type),
    })
}

fn resolve_plus(type_hash: TypeHash) -> Option<UnaryResolution> {
    // Unary plus is a no-op for numeric types
    if is_numeric_primitive(type_hash) {
        Some(UnaryResolution::NoOp {
            result_type: DataType::simple(type_hash),
        })
    } else {
        None
    }
}

fn resolve_pow(left: TypeHash, right: TypeHash) -> Option<OperatorResolution> {
    // Only numeric types can do exponentiation
    if !is_numeric_primitive(left) || !is_numeric_primitive(right) {
        return None;
    }

    // For floats: base ** exp where both are promoted to same float type
    if is_float_primitive(left) || is_float_primitive(right) {
        let promoted = promote_types(left, right)?;
        let left_conv = conversion_opcode(left, promoted);
        let right_conv = conversion_opcode(right, promoted);

        return Some(OperatorResolution::Primitive {
            opcode: OpCode::Pow,
            left_conv,
            right_conv,
            result_type: DataType::simple(promoted),
        });
    }

    // For integers: base ** exp where exp is converted to u32
    // Result type is the promoted integer type
    let promoted = promote_types(left, right)?;

    // For integer pow, left operand type determines result type
    // Right operand (exponent) is converted to u32
    let result_type = match promoted {
        t if t == primitives::INT64 || t == primitives::UINT64 => promoted,
        _ => primitives::INT32,
    };

    let left_conv = conversion_opcode(left, result_type);
    // Exponent is always converted to u32 for Rust's pow
    let right_conv = if right != primitives::UINT32 {
        conversion_opcode(right, primitives::UINT32)
    } else {
        None
    };

    Some(OperatorResolution::Primitive {
        opcode: OpCode::Pow,
        left_conv,
        right_conv,
        result_type: DataType::simple(result_type),
    })
}
