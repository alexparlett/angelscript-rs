//! ExprInfo - expression type checking result.

use super::DataType;

/// Result of expression type checking.
#[derive(Debug, Clone)]
pub struct ExprInfo {
    pub data_type: DataType,
    pub is_lvalue: bool,
    pub is_constant: bool,
}
