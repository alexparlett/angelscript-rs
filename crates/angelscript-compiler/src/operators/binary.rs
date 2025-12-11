//! Binary operator resolution.
//!
//! Resolves binary operators by trying:
//! 1. Primitive operations (direct opcodes)
//! 2. User-defined operators (method calls)
//! 3. Reverse operators on the right operand

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::BinaryOp;

use crate::context::CompilationContext;

use super::{OperatorResolution, primitive};

/// Resolve a binary operator.
///
/// Tries primitive operations first, then user-defined operators.
pub fn resolve_binary(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try primitive operation first
    if let Some(resolution) = primitive::try_primitive_binary(left, right, op) {
        return Ok(resolution);
    }

    // Try user-defined operators on left type
    if let Some(resolution) = try_user_defined_binary(left, right, op, ctx)? {
        return Ok(resolution);
    }

    Err(CompilationError::Other {
        message: format!(
            "No matching operator '{}' for types {:?} and {:?}",
            op, left.type_hash, right.type_hash
        ),
        span,
    })
}

/// Try to resolve a user-defined binary operator.
///
/// Attempts in order:
/// 1. Method on left operand (e.g., `left.opAdd(right)`)
/// 2. Reverse method on right operand (e.g., `right.opAddR(left)`)
fn try_user_defined_binary(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
) -> Result<Option<OperatorResolution>, CompilationError> {
    // Map BinaryOp to (primary behavior, reverse behavior)
    let (primary, reverse) = map_binary_op_to_behavior(op);

    // Try primary operator on left type
    if let Some(behavior) = primary
        && let Some(resolution) = try_operator_on_type(left, right, behavior, true, ctx)?
    {
        return Ok(Some(resolution));
    }

    // Try reverse operator on right type
    if let Some(behavior) = reverse
        && let Some(resolution) = try_operator_on_type(right, left, behavior, false, ctx)?
    {
        return Ok(Some(resolution));
    }

    Ok(None)
}

/// Map a BinaryOp to its corresponding operator behaviors (primary, reverse).
fn map_binary_op_to_behavior(
    op: BinaryOp,
) -> (
    Option<angelscript_core::OperatorBehavior>,
    Option<angelscript_core::OperatorBehavior>,
) {
    use angelscript_core::OperatorBehavior;

    match op {
        BinaryOp::Add => (
            Some(OperatorBehavior::OpAdd),
            Some(OperatorBehavior::OpAddR),
        ),
        BinaryOp::Sub => (
            Some(OperatorBehavior::OpSub),
            Some(OperatorBehavior::OpSubR),
        ),
        BinaryOp::Mul => (
            Some(OperatorBehavior::OpMul),
            Some(OperatorBehavior::OpMulR),
        ),
        BinaryOp::Div => (
            Some(OperatorBehavior::OpDiv),
            Some(OperatorBehavior::OpDivR),
        ),
        BinaryOp::Mod => (
            Some(OperatorBehavior::OpMod),
            Some(OperatorBehavior::OpModR),
        ),
        BinaryOp::Pow => (
            Some(OperatorBehavior::OpPow),
            Some(OperatorBehavior::OpPowR),
        ),
        BinaryOp::BitwiseAnd => (
            Some(OperatorBehavior::OpAnd),
            Some(OperatorBehavior::OpAndR),
        ),
        BinaryOp::BitwiseOr => (Some(OperatorBehavior::OpOr), Some(OperatorBehavior::OpOrR)),
        BinaryOp::BitwiseXor => (
            Some(OperatorBehavior::OpXor),
            Some(OperatorBehavior::OpXorR),
        ),
        BinaryOp::ShiftLeft => (
            Some(OperatorBehavior::OpShl),
            Some(OperatorBehavior::OpShlR),
        ),
        BinaryOp::ShiftRight => (
            Some(OperatorBehavior::OpShr),
            Some(OperatorBehavior::OpShrR),
        ),
        BinaryOp::ShiftRightUnsigned => (
            Some(OperatorBehavior::OpUShr),
            Some(OperatorBehavior::OpUShrR),
        ),
        BinaryOp::Equal => (Some(OperatorBehavior::OpEquals), None),
        BinaryOp::NotEqual => (Some(OperatorBehavior::OpEquals), None), // Use opEquals, then negate
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            (Some(OperatorBehavior::OpCmp), None)
        }
        BinaryOp::Is | BinaryOp::NotIs => (Some(OperatorBehavior::OpEquals), None), // Try opEquals for handles
        _ => (None, None),
    }
}

/// Try to find an operator method on a given type.
///
/// Returns a resolution if a suitable method is found, None if not found.
fn try_operator_on_type(
    obj_type: &DataType,
    arg_type: &DataType,
    behavior: angelscript_core::OperatorBehavior,
    on_left: bool,
    ctx: &CompilationContext<'_>,
) -> Result<Option<OperatorResolution>, CompilationError> {
    use angelscript_core::TypeEntry;

    // Get the type entry
    let type_entry = match ctx.get_type(obj_type.type_hash) {
        Some(entry) => entry,
        None => return Ok(None),
    };

    // Extract behaviors from ClassEntry (only classes have operator overloads)
    let behaviors = match type_entry {
        TypeEntry::Class(class) => &class.behaviors,
        _ => return Ok(None), // Only classes support operator overloads
    };

    let operator_overloads = match behaviors.get_operator(behavior) {
        Some(overloads) => overloads,
        None => return Ok(None),
    };

    // Try each overload
    for &method_hash in operator_overloads {
        let func_entry = match ctx.get_function(method_hash) {
            Some(entry) => entry,
            None => continue,
        };

        // Check if this overload matches
        // For binary operators: should have exactly 1 parameter
        if func_entry.def.params.len() != 1 {
            continue;
        }

        let param_type = &func_entry.def.params[0].data_type;

        // Check if argument type matches parameter (with implicit conversions only)
        if arg_type.type_hash != param_type.type_hash {
            // Check if implicit conversion is available
            match crate::conversion::find_conversion(arg_type, param_type, ctx) {
                Some(conv) if conv.is_implicit() => {
                    // Conversion is available and implicit - accept this overload
                }
                _ => continue, // No implicit conversion available, try next overload
            }
        }

        // For now, we don't emit the conversion here - that will be handled
        // by the bidirectional type checking in Step 3
        let conversion = None;

        // Const-correctness check
        // TODO: Implement const-correctness checks for methods and parameters

        let result_type = func_entry.def.return_type;

        return Ok(Some(if on_left {
            OperatorResolution::MethodOnLeft {
                method_hash,
                arg_conversion: conversion,
                result_type,
            }
        } else {
            OperatorResolution::MethodOnRight {
                method_hash,
                arg_conversion: conversion,
                result_type,
            }
        }));
    }

    Ok(None)
}
