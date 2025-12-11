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
    _ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<UnaryResolution, CompilationError> {
    // Try primitive operation first
    if let Some(resolution) = primitive::try_primitive_unary(operand, op) {
        return Ok(resolution);
    }

    // TODO: Try user-defined operators

    Err(CompilationError::Other {
        message: format!(
            "No matching operator '{}' for type {:?}",
            op, operand.type_hash
        ),
        span,
    })
}
