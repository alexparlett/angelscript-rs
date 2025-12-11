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
    _ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try primitive operation first
    if let Some(resolution) = primitive::try_primitive_binary(left, right, op) {
        return Ok(resolution);
    }

    // TODO: Try user-defined operators

    Err(CompilationError::Other {
        message: format!(
            "No matching operator '{}' for types {:?} and {:?}",
            op, left.type_hash, right.type_hash
        ),
        span,
    })
}
