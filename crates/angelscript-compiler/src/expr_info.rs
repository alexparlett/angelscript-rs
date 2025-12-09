//! Expression type information for the compiler.
//!
//! `ExprInfo` captures the result of type-checking an expression,
//! including whether it's an lvalue and its mutability.

use angelscript_core::DataType;

/// Result of type-checking an expression.
///
/// Contains the type and lvalue/mutability information needed
/// for assignment checking and code generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExprInfo {
    /// The type of the expression.
    pub data_type: DataType,
    /// Whether this is an lvalue (can appear on left side of assignment).
    pub is_lvalue: bool,
    /// Whether this lvalue can be modified (false for const).
    pub is_mutable: bool,
}

impl ExprInfo {
    /// Create an rvalue (temporary, cannot be assigned to).
    ///
    /// # Example
    /// ```ignore
    /// // The result of `1 + 2` is an rvalue
    /// let info = ExprInfo::rvalue(DataType::int32());
    /// assert!(!info.is_assignable());
    /// ```
    pub fn rvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }

    /// Create a mutable lvalue (can be assigned to).
    ///
    /// # Example
    /// ```ignore
    /// // A local variable `int x` is a mutable lvalue
    /// let info = ExprInfo::lvalue(DataType::int32());
    /// assert!(info.is_assignable());
    /// ```
    pub fn lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: true,
        }
    }

    /// Create a const lvalue (can be read but not assigned).
    ///
    /// # Example
    /// ```ignore
    /// // A `const int x` is a const lvalue
    /// let info = ExprInfo::const_lvalue(DataType::int32());
    /// assert!(info.is_lvalue);
    /// assert!(!info.is_assignable());
    /// ```
    pub fn const_lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: false,
        }
    }

    /// Check if this can be assigned to.
    ///
    /// Returns true only if this is both an lvalue and mutable.
    pub fn is_assignable(&self) -> bool {
        self.is_lvalue && self.is_mutable
    }

    /// Convert to rvalue (for reading).
    ///
    /// This is used when an lvalue is used in a context that
    /// requires a value (e.g., right side of assignment).
    pub fn to_rvalue(self) -> Self {
        Self {
            data_type: self.data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn rvalue_not_assignable() {
        let info = ExprInfo::rvalue(DataType::simple(primitives::INT32));
        assert!(!info.is_lvalue);
        assert!(!info.is_mutable);
        assert!(!info.is_assignable());
    }

    #[test]
    fn lvalue_is_assignable() {
        let info = ExprInfo::lvalue(DataType::simple(primitives::INT32));
        assert!(info.is_lvalue);
        assert!(info.is_mutable);
        assert!(info.is_assignable());
    }

    #[test]
    fn const_lvalue_not_assignable() {
        let info = ExprInfo::const_lvalue(DataType::simple(primitives::INT32));
        assert!(info.is_lvalue);
        assert!(!info.is_mutable);
        assert!(!info.is_assignable());
    }

    #[test]
    fn to_rvalue_converts() {
        let lvalue = ExprInfo::lvalue(DataType::simple(primitives::DOUBLE));
        let rvalue = lvalue.to_rvalue();

        assert!(!rvalue.is_lvalue);
        assert!(!rvalue.is_mutable);
        assert_eq!(rvalue.data_type, lvalue.data_type);
    }
}
