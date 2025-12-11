//! Unary operator resolution.
//!
//! Resolves unary operators by trying:
//! 1. Primitive operations (direct opcodes)
//! 2. User-defined operators (method calls)

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::UnaryOp;

use crate::context::CompilationContext;

use super::{UnaryResolution, primitive};

/// Resolve a unary operator.
///
/// Tries primitive operations first, then user-defined operators.
pub fn resolve_unary(
    operand: &DataType,
    op: UnaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<UnaryResolution, CompilationError> {
    // Try primitive operation first
    if let Some(resolution) = primitive::try_primitive_unary(operand, op) {
        return Ok(resolution);
    }

    // Try user-defined operators
    if let Some(resolution) = try_user_defined_unary(operand, op, ctx)? {
        return Ok(resolution);
    }

    Err(CompilationError::Other {
        message: format!(
            "No matching operator '{}' for type {:?}",
            op, operand.type_hash
        ),
        span,
    })
}

/// Try to resolve a user-defined unary operator.
///
/// Looks for an operator method on the operand type (e.g., `operand.opNeg()`).
fn try_user_defined_unary(
    operand: &DataType,
    op: UnaryOp,
    ctx: &CompilationContext<'_>,
) -> Result<Option<UnaryResolution>, CompilationError> {
    use angelscript_core::TypeEntry;

    // Map UnaryOp to OperatorBehavior
    let behavior = match map_unary_op_to_behavior(op) {
        Some(b) => b,
        None => return Ok(None),
    };

    // Get the type entry
    let type_entry = match ctx.get_type(operand.type_hash) {
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
        // For unary operators: should have no parameters (operates on self)
        if !func_entry.def.params.is_empty() {
            continue;
        }

        // Const-correctness check
        // TODO: Implement const-correctness checks for methods

        let result_type = func_entry.def.return_type;

        return Ok(Some(UnaryResolution::Method {
            method_hash,
            result_type,
        }));
    }

    Ok(None)
}

/// Map a UnaryOp to its corresponding operator behavior.
fn map_unary_op_to_behavior(op: UnaryOp) -> Option<angelscript_core::OperatorBehavior> {
    use angelscript_core::OperatorBehavior;

    match op {
        UnaryOp::Neg => Some(OperatorBehavior::OpNeg),
        UnaryOp::BitwiseNot => Some(OperatorBehavior::OpCom),
        UnaryOp::LogicalNot => None, // No user-defined operator for logical not
        UnaryOp::PreInc => Some(OperatorBehavior::OpPreInc),
        UnaryOp::PreDec => Some(OperatorBehavior::OpPreDec),
        UnaryOp::Plus => None,     // Unary plus has no user-defined operator
        UnaryOp::HandleOf => None, // Handle-of has no user-defined operator
    }
}
