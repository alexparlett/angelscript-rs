//! Operator overload resolution.
//!
//! This module handles resolution of binary and unary operators, determining
//! whether to use a primitive opcode or a user-defined operator method.

use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, TypeHash, primitives};
use angelscript_parser::ast::{BinaryOp, UnaryOp};

use crate::bytecode::OpCode;
use crate::context::CompilationContext;
use crate::conversion::{Conversion, find_conversion};

/// Result of operator resolution.
#[derive(Debug, Clone)]
pub enum OperatorResolution {
    /// Built-in primitive operation.
    Primitive {
        /// The opcode to use.
        opcode: OpCode,
        /// The result type of the operation.
        result_type: DataType,
    },
    /// User-defined operator method.
    Method {
        /// The method function hash.
        method_hash: TypeHash,
        /// Whether the method is on the left operand (true) or right (false for reverse ops).
        on_left: bool,
        /// Conversion needed for the argument (if any).
        arg_conversion: Option<Conversion>,
        /// The result type of the operation.
        result_type: DataType,
    },
}

/// Resolve a binary operator for given operand types.
///
/// Tries in order:
/// 1. Primitive operation (for built-in types)
/// 2. Left operand's operator method (e.g., `left.opAdd(right)`)
/// 3. Right operand's reverse operator method (e.g., `right.opAdd_r(left)`)
pub fn resolve_binary_operator(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // 1. Try primitive operation
    if let Some(resolution) = try_primitive_binary_op(left, right, op) {
        return Ok(resolution);
    }

    // 2. Try left.opXxx(right)
    let op_behavior = binary_op_to_behavior(op);
    if let Some(resolution) = try_operator_behavior(left, right, op_behavior, true, ctx) {
        return Ok(resolution);
    }

    // 3. Try right.opXxx_r(left) - reverse operator
    if let Some(reverse_behavior) = binary_op_to_reverse_behavior(op)
        && let Some(resolution) = try_operator_behavior(right, left, reverse_behavior, false, ctx)
    {
        return Ok(resolution);
    }

    Err(CompilationError::NoOperator {
        op: format!("{}", op),
        left: format_type(left.type_hash, ctx),
        right: format_type(right.type_hash, ctx),
        span,
    })
}

/// Resolve a unary operator for a given operand type.
pub fn resolve_unary_operator(
    operand: &DataType,
    op: UnaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try primitive operation
    if let Some(resolution) = try_primitive_unary_op(operand, op) {
        return Ok(resolution);
    }

    // Try operand.opXxx()
    let op_behavior = unary_op_to_behavior(op);
    if let Some(resolution) = try_unary_operator_behavior(operand, op_behavior, ctx) {
        return Ok(resolution);
    }

    Err(CompilationError::NoOperator {
        op: format!("{}", op),
        left: format_type(operand.type_hash, ctx),
        right: String::new(),
        span,
    })
}

/// Try to resolve as a primitive binary operation.
fn try_primitive_binary_op(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
) -> Option<OperatorResolution> {
    // Both operands must be the same primitive type for most operations
    if left.type_hash != right.type_hash {
        return None;
    }

    let type_hash = left.type_hash;
    let opcode = match (type_hash, op) {
        // i32 arithmetic
        (h, BinaryOp::Add) if h == primitives::INT32 => OpCode::AddI32,
        (h, BinaryOp::Sub) if h == primitives::INT32 => OpCode::SubI32,
        (h, BinaryOp::Mul) if h == primitives::INT32 => OpCode::MulI32,
        (h, BinaryOp::Div) if h == primitives::INT32 => OpCode::DivI32,
        (h, BinaryOp::Mod) if h == primitives::INT32 => OpCode::ModI32,

        // i32 comparisons
        (h, BinaryOp::Less) if h == primitives::INT32 => OpCode::LtI32,
        (h, BinaryOp::LessEqual) if h == primitives::INT32 => OpCode::LeI32,
        (h, BinaryOp::Greater) if h == primitives::INT32 => OpCode::GtI32,
        (h, BinaryOp::GreaterEqual) if h == primitives::INT32 => OpCode::GeI32,
        (h, BinaryOp::Equal) if h == primitives::INT32 => OpCode::EqI32,

        // i64 arithmetic
        (h, BinaryOp::Add) if h == primitives::INT64 => OpCode::AddI64,
        (h, BinaryOp::Sub) if h == primitives::INT64 => OpCode::SubI64,
        (h, BinaryOp::Mul) if h == primitives::INT64 => OpCode::MulI64,
        (h, BinaryOp::Div) if h == primitives::INT64 => OpCode::DivI64,
        (h, BinaryOp::Mod) if h == primitives::INT64 => OpCode::ModI64,

        // i64 comparisons
        (h, BinaryOp::Less) if h == primitives::INT64 => OpCode::LtI64,
        (h, BinaryOp::LessEqual) if h == primitives::INT64 => OpCode::LeI64,
        (h, BinaryOp::Greater) if h == primitives::INT64 => OpCode::GtI64,
        (h, BinaryOp::GreaterEqual) if h == primitives::INT64 => OpCode::GeI64,
        (h, BinaryOp::Equal) if h == primitives::INT64 => OpCode::EqI64,

        // f32 arithmetic
        (h, BinaryOp::Add) if h == primitives::FLOAT => OpCode::AddF32,
        (h, BinaryOp::Sub) if h == primitives::FLOAT => OpCode::SubF32,
        (h, BinaryOp::Mul) if h == primitives::FLOAT => OpCode::MulF32,
        (h, BinaryOp::Div) if h == primitives::FLOAT => OpCode::DivF32,

        // f32 comparisons
        (h, BinaryOp::Less) if h == primitives::FLOAT => OpCode::LtF32,
        (h, BinaryOp::LessEqual) if h == primitives::FLOAT => OpCode::LeF32,
        (h, BinaryOp::Greater) if h == primitives::FLOAT => OpCode::GtF32,
        (h, BinaryOp::GreaterEqual) if h == primitives::FLOAT => OpCode::GeF32,
        (h, BinaryOp::Equal) if h == primitives::FLOAT => OpCode::EqF32,

        // f64 arithmetic
        (h, BinaryOp::Add) if h == primitives::DOUBLE => OpCode::AddF64,
        (h, BinaryOp::Sub) if h == primitives::DOUBLE => OpCode::SubF64,
        (h, BinaryOp::Mul) if h == primitives::DOUBLE => OpCode::MulF64,
        (h, BinaryOp::Div) if h == primitives::DOUBLE => OpCode::DivF64,

        // f64 comparisons
        (h, BinaryOp::Less) if h == primitives::DOUBLE => OpCode::LtF64,
        (h, BinaryOp::LessEqual) if h == primitives::DOUBLE => OpCode::LeF64,
        (h, BinaryOp::Greater) if h == primitives::DOUBLE => OpCode::GtF64,
        (h, BinaryOp::GreaterEqual) if h == primitives::DOUBLE => OpCode::GeF64,
        (h, BinaryOp::Equal) if h == primitives::DOUBLE => OpCode::EqF64,

        // Bitwise (i32)
        (h, BinaryOp::BitwiseAnd) if h == primitives::INT32 => OpCode::BitAnd,
        (h, BinaryOp::BitwiseOr) if h == primitives::INT32 => OpCode::BitOr,
        (h, BinaryOp::BitwiseXor) if h == primitives::INT32 => OpCode::BitXor,
        (h, BinaryOp::ShiftLeft) if h == primitives::INT32 => OpCode::Shl,
        (h, BinaryOp::ShiftRight) if h == primitives::INT32 => OpCode::Shr,
        (h, BinaryOp::ShiftRightUnsigned) if h == primitives::INT32 => OpCode::Ushr,

        // Boolean
        (h, BinaryOp::Equal) if h == primitives::BOOL => OpCode::EqBool,

        _ => return None,
    };

    // Determine result type
    let result_type = match op {
        BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual
        | BinaryOp::Equal
        | BinaryOp::NotEqual => DataType::simple(primitives::BOOL),
        _ => *left, // Arithmetic produces same type as operands
    };

    Some(OperatorResolution::Primitive {
        opcode,
        result_type,
    })
}

/// Try to resolve as a primitive unary operation.
fn try_primitive_unary_op(operand: &DataType, op: UnaryOp) -> Option<OperatorResolution> {
    let type_hash = operand.type_hash;

    let opcode = match (type_hash, op) {
        // Negation
        (h, UnaryOp::Neg) if h == primitives::INT32 => OpCode::NegI32,
        (h, UnaryOp::Neg) if h == primitives::INT64 => OpCode::NegI64,
        (h, UnaryOp::Neg) if h == primitives::FLOAT => OpCode::NegF32,
        (h, UnaryOp::Neg) if h == primitives::DOUBLE => OpCode::NegF64,

        // Bitwise NOT
        (h, UnaryOp::BitwiseNot) if h == primitives::INT32 => OpCode::BitNot,

        // Logical NOT
        (h, UnaryOp::LogicalNot) if h == primitives::BOOL => OpCode::Not,

        // Unary plus is a no-op for numeric types
        (h, UnaryOp::Plus)
            if h == primitives::INT32
                || h == primitives::INT64
                || h == primitives::FLOAT
                || h == primitives::DOUBLE =>
        {
            // No opcode needed, just return identity
            return Some(OperatorResolution::Primitive {
                opcode: OpCode::Dup, // Placeholder - actually no-op
                result_type: *operand,
            });
        }

        _ => return None,
    };

    Some(OperatorResolution::Primitive {
        opcode,
        result_type: *operand,
    })
}

/// Try to resolve a binary operator via TypeBehaviors.
fn try_operator_behavior(
    object: &DataType,
    arg: &DataType,
    op: OperatorBehavior,
    on_left: bool,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    // Get the type entry and its behaviors
    let type_entry = ctx.get_type(object.type_hash)?;
    let class = type_entry.as_class()?;
    let operators = class.behaviors.get_operator(op)?;

    for &method_hash in operators {
        let func = ctx.get_function(method_hash)?;
        let def = &func.def;

        // Must have exactly one parameter
        if def.params.len() != 1 {
            continue;
        }

        // Check if argument can convert to parameter
        if let Some(conv) = find_conversion(arg, &def.params[0].data_type, ctx)
            && conv.is_implicit
        {
            return Some(OperatorResolution::Method {
                method_hash,
                on_left,
                arg_conversion: Some(conv),
                result_type: def.return_type,
            });
        }
    }

    None
}

/// Try to resolve a unary operator via TypeBehaviors.
fn try_unary_operator_behavior(
    operand: &DataType,
    op: OperatorBehavior,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    // Get the type entry and its behaviors
    let type_entry = ctx.get_type(operand.type_hash)?;
    let class = type_entry.as_class()?;
    let operators = class.behaviors.get_operator(op)?;

    for &method_hash in operators {
        let func = ctx.get_function(method_hash)?;
        let def = &func.def;

        // Must have no parameters (besides implicit 'this')
        if !def.params.is_empty() {
            continue;
        }

        return Some(OperatorResolution::Method {
            method_hash,
            on_left: true,
            arg_conversion: None,
            result_type: def.return_type,
        });
    }

    None
}

/// Map a binary operator to its OperatorBehavior enum.
fn binary_op_to_behavior(op: BinaryOp) -> OperatorBehavior {
    match op {
        BinaryOp::Add => OperatorBehavior::OpAdd,
        BinaryOp::Sub => OperatorBehavior::OpSub,
        BinaryOp::Mul => OperatorBehavior::OpMul,
        BinaryOp::Div => OperatorBehavior::OpDiv,
        BinaryOp::Mod => OperatorBehavior::OpMod,
        BinaryOp::Pow => OperatorBehavior::OpPow,
        BinaryOp::Equal | BinaryOp::NotEqual => OperatorBehavior::OpEquals,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            OperatorBehavior::OpCmp
        }
        BinaryOp::BitwiseAnd => OperatorBehavior::OpAnd,
        BinaryOp::BitwiseOr => OperatorBehavior::OpOr,
        BinaryOp::BitwiseXor => OperatorBehavior::OpXor,
        BinaryOp::ShiftLeft => OperatorBehavior::OpShl,
        BinaryOp::ShiftRight => OperatorBehavior::OpShr,
        BinaryOp::ShiftRightUnsigned => OperatorBehavior::OpUShr,
        BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => {
            // Logical operators don't have method overloads in AngelScript
            // They use short-circuit evaluation on bool values
            OperatorBehavior::OpEquals // Placeholder - won't match
        }
        BinaryOp::Is | BinaryOp::NotIs => OperatorBehavior::OpEquals, // Identity is built-in
    }
}

/// Map a binary operator to its reverse OperatorBehavior enum.
fn binary_op_to_reverse_behavior(op: BinaryOp) -> Option<OperatorBehavior> {
    Some(match op {
        BinaryOp::Add => OperatorBehavior::OpAddR,
        BinaryOp::Sub => OperatorBehavior::OpSubR,
        BinaryOp::Mul => OperatorBehavior::OpMulR,
        BinaryOp::Div => OperatorBehavior::OpDivR,
        BinaryOp::Mod => OperatorBehavior::OpModR,
        BinaryOp::Pow => OperatorBehavior::OpPowR,
        BinaryOp::BitwiseAnd => OperatorBehavior::OpAndR,
        BinaryOp::BitwiseOr => OperatorBehavior::OpOrR,
        BinaryOp::BitwiseXor => OperatorBehavior::OpXorR,
        BinaryOp::ShiftLeft => OperatorBehavior::OpShlR,
        BinaryOp::ShiftRight => OperatorBehavior::OpShrR,
        BinaryOp::ShiftRightUnsigned => OperatorBehavior::OpUShrR,
        // Comparison and logical ops don't have reverse
        _ => return None,
    })
}

/// Map a unary operator to its OperatorBehavior enum.
fn unary_op_to_behavior(op: UnaryOp) -> OperatorBehavior {
    match op {
        UnaryOp::Neg => OperatorBehavior::OpNeg,
        UnaryOp::Plus => OperatorBehavior::OpNeg, // No OpPos - unary plus is identity
        UnaryOp::LogicalNot => OperatorBehavior::OpNeg, // No OpNot for logical not
        UnaryOp::BitwiseNot => OperatorBehavior::OpCom,
        UnaryOp::PreInc => OperatorBehavior::OpPreInc,
        UnaryOp::PreDec => OperatorBehavior::OpPreDec,
        UnaryOp::HandleOf => OperatorBehavior::OpNeg, // HandleOf is built-in
    }
}

/// Format a type hash as a readable name.
fn format_type(hash: TypeHash, ctx: &CompilationContext<'_>) -> String {
    ctx.get_type(hash)
        .map(|e| e.qualified_name().to_string())
        .unwrap_or_else(|| format!("{:?}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_registry::SymbolRegistry;

    #[test]
    fn primitive_int_add() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Add, &ctx, Span::default());

        assert!(result.is_ok());
        match result.unwrap() {
            OperatorResolution::Primitive {
                opcode,
                result_type,
            } => {
                assert_eq!(opcode, OpCode::AddI32);
                assert_eq!(result_type.type_hash, primitives::INT32);
            }
            _ => panic!("Expected primitive resolution"),
        }
    }

    #[test]
    fn primitive_float_mul() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::FLOAT);
        let right = DataType::simple(primitives::FLOAT);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Mul, &ctx, Span::default());

        assert!(result.is_ok());
        match result.unwrap() {
            OperatorResolution::Primitive { opcode, .. } => {
                assert_eq!(opcode, OpCode::MulF32);
            }
            _ => panic!("Expected primitive resolution"),
        }
    }

    #[test]
    fn primitive_comparison_returns_bool() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Less, &ctx, Span::default());

        assert!(result.is_ok());
        match result.unwrap() {
            OperatorResolution::Primitive {
                opcode,
                result_type,
            } => {
                assert_eq!(opcode, OpCode::LtI32);
                assert_eq!(result_type.type_hash, primitives::BOOL);
            }
            _ => panic!("Expected primitive resolution"),
        }
    }

    #[test]
    fn mismatched_types_fails() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::FLOAT);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Add, &ctx, Span::default());

        // Should fail because no automatic promotion and no method
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::NoOperator { .. }
        ));
    }

    #[test]
    fn unary_negation_int() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::INT32);

        let result = resolve_unary_operator(&operand, UnaryOp::Neg, &ctx, Span::default());

        assert!(result.is_ok());
        match result.unwrap() {
            OperatorResolution::Primitive {
                opcode,
                result_type,
            } => {
                assert_eq!(opcode, OpCode::NegI32);
                assert_eq!(result_type.type_hash, primitives::INT32);
            }
            _ => panic!("Expected primitive resolution"),
        }
    }

    #[test]
    fn unary_logical_not_bool() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::BOOL);

        let result = resolve_unary_operator(&operand, UnaryOp::LogicalNot, &ctx, Span::default());

        assert!(result.is_ok());
        match result.unwrap() {
            OperatorResolution::Primitive {
                opcode,
                result_type,
            } => {
                assert_eq!(opcode, OpCode::Not);
                assert_eq!(result_type.type_hash, primitives::BOOL);
            }
            _ => panic!("Expected primitive resolution"),
        }
    }

    #[test]
    fn bitwise_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        // AND
        let result =
            resolve_binary_operator(&left, &right, BinaryOp::BitwiseAnd, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::BitAnd,
                ..
            }
        ));

        // OR
        let result =
            resolve_binary_operator(&left, &right, BinaryOp::BitwiseOr, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::BitOr,
                ..
            }
        ));

        // XOR
        let result =
            resolve_binary_operator(&left, &right, BinaryOp::BitwiseXor, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::BitXor,
                ..
            }
        ));

        // Shift left
        let result =
            resolve_binary_operator(&left, &right, BinaryOp::ShiftLeft, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::Shl,
                ..
            }
        ));

        // Shift right
        let result =
            resolve_binary_operator(&left, &right, BinaryOp::ShiftRight, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::Shr,
                ..
            }
        ));

        // Unsigned shift right
        let result = resolve_binary_operator(
            &left,
            &right,
            BinaryOp::ShiftRightUnsigned,
            &ctx,
            Span::default(),
        );
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::Ushr,
                ..
            }
        ));
    }

    #[test]
    fn i64_arithmetic_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT64);
        let right = DataType::simple(primitives::INT64);

        // Add
        let result = resolve_binary_operator(&left, &right, BinaryOp::Add, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::AddI64,
                ..
            }
        ));

        // Sub
        let result = resolve_binary_operator(&left, &right, BinaryOp::Sub, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::SubI64,
                ..
            }
        ));

        // Mul
        let result = resolve_binary_operator(&left, &right, BinaryOp::Mul, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::MulI64,
                ..
            }
        ));

        // Div
        let result = resolve_binary_operator(&left, &right, BinaryOp::Div, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::DivI64,
                ..
            }
        ));

        // Mod
        let result = resolve_binary_operator(&left, &right, BinaryOp::Mod, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::ModI64,
                ..
            }
        ));
    }

    #[test]
    fn i64_comparison_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT64);
        let right = DataType::simple(primitives::INT64);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Less, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LtI64,
                result_type,
            } if result_type.type_hash == primitives::BOOL
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::LessEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LeI64,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::Greater, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GtI64,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::GreaterEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GeI64,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Equal, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::EqI64,
                ..
            }
        ));
    }

    #[test]
    fn f64_arithmetic_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::DOUBLE);
        let right = DataType::simple(primitives::DOUBLE);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Add, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::AddF64,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Sub, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::SubF64,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Mul, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::MulF64,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Div, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::DivF64,
                ..
            }
        ));
    }

    #[test]
    fn f64_comparison_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::DOUBLE);
        let right = DataType::simple(primitives::DOUBLE);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Less, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LtF64,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::LessEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LeF64,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::Greater, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GtF64,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::GreaterEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GeF64,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Equal, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::EqF64,
                ..
            }
        ));
    }

    #[test]
    fn f32_comparison_operations() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::FLOAT);
        let right = DataType::simple(primitives::FLOAT);

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::LessEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LeF32,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::Greater, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GtF32,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::GreaterEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GeF32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Equal, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::EqF32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Less, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LtF32,
                ..
            }
        ));
    }

    #[test]
    fn f32_arithmetic_sub_div() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::FLOAT);
        let right = DataType::simple(primitives::FLOAT);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Sub, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::SubF32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Div, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::DivF32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Add, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::AddF32,
                ..
            }
        ));
    }

    #[test]
    fn i32_remaining_arithmetic() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Sub, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::SubI32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Mul, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::MulI32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Div, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::DivI32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Mod, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::ModI32,
                ..
            }
        ));
    }

    #[test]
    fn i32_remaining_comparisons() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::INT32);
        let right = DataType::simple(primitives::INT32);

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::LessEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::LeI32,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::Greater, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GtI32,
                ..
            }
        ));

        let result =
            resolve_binary_operator(&left, &right, BinaryOp::GreaterEqual, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::GeI32,
                ..
            }
        ));

        let result = resolve_binary_operator(&left, &right, BinaryOp::Equal, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::EqI32,
                ..
            }
        ));
    }

    #[test]
    fn bool_equality() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let left = DataType::simple(primitives::BOOL);
        let right = DataType::simple(primitives::BOOL);

        let result = resolve_binary_operator(&left, &right, BinaryOp::Equal, &ctx, Span::default());
        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::EqBool,
                ..
            }
        ));
    }

    #[test]
    fn unary_negation_i64() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::INT64);
        let result = resolve_unary_operator(&operand, UnaryOp::Neg, &ctx, Span::default());

        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::NegI64,
                ..
            }
        ));
    }

    #[test]
    fn unary_negation_f32() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::FLOAT);
        let result = resolve_unary_operator(&operand, UnaryOp::Neg, &ctx, Span::default());

        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::NegF32,
                ..
            }
        ));
    }

    #[test]
    fn unary_negation_f64() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::DOUBLE);
        let result = resolve_unary_operator(&operand, UnaryOp::Neg, &ctx, Span::default());

        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::NegF64,
                ..
            }
        ));
    }

    #[test]
    fn unary_bitwise_not() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let operand = DataType::simple(primitives::INT32);
        let result = resolve_unary_operator(&operand, UnaryOp::BitwiseNot, &ctx, Span::default());

        assert!(matches!(
            result.unwrap(),
            OperatorResolution::Primitive {
                opcode: OpCode::BitNot,
                ..
            }
        ));
    }

    #[test]
    fn unary_plus_is_noop() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        // Test all numeric types
        for type_hash in [
            primitives::INT32,
            primitives::INT64,
            primitives::FLOAT,
            primitives::DOUBLE,
        ] {
            let operand = DataType::simple(type_hash);
            let result = resolve_unary_operator(&operand, UnaryOp::Plus, &ctx, Span::default());
            assert!(result.is_ok());
            match result.unwrap() {
                OperatorResolution::Primitive { result_type, .. } => {
                    assert_eq!(result_type.type_hash, type_hash);
                }
                _ => panic!("Expected primitive resolution"),
            }
        }
    }

    #[test]
    fn unary_operator_on_unsupported_type_fails() {
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        // Try negation on bool - should fail
        let operand = DataType::simple(primitives::BOOL);
        let result = resolve_unary_operator(&operand, UnaryOp::Neg, &ctx, Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::NoOperator { .. }
        ));
    }
}
