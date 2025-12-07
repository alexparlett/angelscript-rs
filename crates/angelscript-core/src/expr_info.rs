//! ExprInfo - expression type checking result.
//!
//! This module provides [`ExprInfo`], the result of type-checking an expression.
//! It contains the expression's data type along with lvalue/mutability information.

use crate::DataType;

/// Result of expression type checking.
///
/// After type-checking an expression, `ExprInfo` captures:
/// - The expression's [`DataType`]
/// - Whether it's an lvalue (can appear on left side of assignment)
/// - Whether it's mutable (can be modified)
///
/// # Lvalue vs Rvalue
///
/// - **Lvalue**: Has a memory location, can be assigned to (variables, fields, array elements)
/// - **Rvalue**: Temporary value, cannot be assigned to (literals, function returns, arithmetic results)
///
/// # Mutability
///
/// Only lvalues can be mutable. An lvalue is mutable if:
/// - It's a non-const variable
/// - It's a field accessed through a non-const reference
/// - It's not a `const&` parameter
///
/// # Examples
///
/// ```
/// use angelscript_core::{ExprInfo, DataType, primitives};
///
/// // Integer literal: rvalue (temporary, cannot assign to it)
/// let literal = ExprInfo::rvalue(DataType::simple(primitives::INT32));
/// assert!(!literal.is_lvalue);
/// assert!(!literal.is_mutable);
///
/// // Local variable: mutable lvalue
/// let var = ExprInfo::lvalue(DataType::simple(primitives::INT32), true);
/// assert!(var.is_lvalue);
/// assert!(var.is_mutable);
///
/// // Const variable: immutable lvalue
/// let const_var = ExprInfo::const_lvalue(DataType::simple(primitives::INT32));
/// assert!(const_var.is_lvalue);
/// assert!(!const_var.is_mutable);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprInfo {
    /// The data type of this expression.
    pub data_type: DataType,

    /// Whether this expression is an lvalue (can be assigned to).
    ///
    /// Lvalues have a memory location and can appear on the left side of an assignment.
    /// Examples: variables, fields, array elements.
    ///
    /// Rvalues are temporary values that cannot be assigned to.
    /// Examples: literals, function return values, arithmetic expressions.
    pub is_lvalue: bool,

    /// Whether this lvalue is mutable (can be modified).
    ///
    /// - Always `false` for rvalues (temporaries are inherently immutable)
    /// - `true` for non-const lvalues (variables, mutable fields, etc.)
    /// - `false` for const lvalues (const variables, `const&` parameters, etc.)
    pub is_mutable: bool,
}

impl ExprInfo {
    /// Create a new rvalue (temporary value, cannot be assigned to).
    ///
    /// Rvalues are the result of expressions that produce temporary values:
    /// - Literals (`42`, `"hello"`, `true`)
    /// - Arithmetic expressions (`a + b`)
    /// - Function calls that return by value
    /// - Casts and conversions
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript_core::{ExprInfo, DataType, primitives};
    ///
    /// // The expression `1 + 2` produces an rvalue
    /// let result = ExprInfo::rvalue(DataType::simple(primitives::INT32));
    /// assert!(!result.is_lvalue);
    /// assert!(!result.is_mutable);
    /// ```
    #[inline]
    pub fn rvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }

    /// Create a new lvalue (has memory location, can be assigned to).
    ///
    /// Lvalues have a stable memory location and can appear on the left side
    /// of an assignment. The `is_mutable` parameter controls whether the
    /// lvalue can be modified.
    ///
    /// # Parameters
    ///
    /// - `data_type`: The type of the expression
    /// - `is_mutable`: Whether this lvalue can be modified
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript_core::{ExprInfo, DataType, primitives};
    ///
    /// // A mutable local variable `int x`
    /// let mutable_var = ExprInfo::lvalue(DataType::simple(primitives::INT32), true);
    /// assert!(mutable_var.is_lvalue);
    /// assert!(mutable_var.is_mutable);
    ///
    /// // A const parameter `const int& x`
    /// let const_param = ExprInfo::lvalue(DataType::simple(primitives::INT32), false);
    /// assert!(const_param.is_lvalue);
    /// assert!(!const_param.is_mutable);
    /// ```
    #[inline]
    pub fn lvalue(data_type: DataType, is_mutable: bool) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable,
        }
    }

    /// Create an immutable lvalue (can be read but not written).
    ///
    /// This is a convenience method equivalent to `lvalue(data_type, false)`.
    /// Use this for:
    /// - Const variables
    /// - `const&` parameters
    /// - Fields accessed through a const reference
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript_core::{ExprInfo, DataType, primitives};
    ///
    /// // A const variable `const int x = 42`
    /// let const_var = ExprInfo::const_lvalue(DataType::simple(primitives::INT32));
    /// assert!(const_var.is_lvalue);
    /// assert!(!const_var.is_mutable);
    /// ```
    #[inline]
    pub fn const_lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: false,
        }
    }

    /// Returns `true` if this expression can be modified.
    ///
    /// An expression is modifiable if it's a mutable lvalue.
    /// This is equivalent to checking `is_lvalue && is_mutable`.
    #[inline]
    pub fn is_modifiable(&self) -> bool {
        self.is_lvalue && self.is_mutable
    }

    /// Returns `true` if this expression is a temporary (rvalue).
    ///
    /// This is the opposite of `is_lvalue`.
    #[inline]
    pub fn is_rvalue(&self) -> bool {
        !self.is_lvalue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives, TypeHash};

    fn int_type() -> DataType {
        DataType::simple(primitives::INT32)
    }

    fn string_type() -> DataType {
        DataType::simple(primitives::STRING)
    }

    #[test]
    fn test_rvalue_creation() {
        let expr = ExprInfo::rvalue(int_type());

        assert_eq!(expr.data_type.type_hash, primitives::INT32);
        assert!(!expr.is_lvalue);
        assert!(!expr.is_mutable);
        assert!(expr.is_rvalue());
        assert!(!expr.is_modifiable());
    }

    #[test]
    fn test_mutable_lvalue_creation() {
        let expr = ExprInfo::lvalue(int_type(), true);

        assert_eq!(expr.data_type.type_hash, primitives::INT32);
        assert!(expr.is_lvalue);
        assert!(expr.is_mutable);
        assert!(!expr.is_rvalue());
        assert!(expr.is_modifiable());
    }

    #[test]
    fn test_immutable_lvalue_creation() {
        let expr = ExprInfo::lvalue(int_type(), false);

        assert_eq!(expr.data_type.type_hash, primitives::INT32);
        assert!(expr.is_lvalue);
        assert!(!expr.is_mutable);
        assert!(!expr.is_rvalue());
        assert!(!expr.is_modifiable());
    }

    #[test]
    fn test_const_lvalue_creation() {
        let expr = ExprInfo::const_lvalue(string_type());

        assert!(expr.is_lvalue);
        assert!(!expr.is_mutable);
        assert!(!expr.is_rvalue());
        assert!(!expr.is_modifiable());
    }

    #[test]
    fn test_const_lvalue_equals_immutable_lvalue() {
        let const_lvalue = ExprInfo::const_lvalue(int_type());
        let immutable_lvalue = ExprInfo::lvalue(int_type(), false);

        assert_eq!(const_lvalue, immutable_lvalue);
    }

    #[test]
    fn test_copy_semantics() {
        let original = ExprInfo::rvalue(int_type());
        let copied = original; // Copy, not move

        // Both should be usable (Copy trait)
        assert_eq!(original.data_type, copied.data_type);
        assert_eq!(original.is_lvalue, copied.is_lvalue);
        assert_eq!(original.is_mutable, copied.is_mutable);
    }

    #[test]
    fn test_with_handle_type() {
        let handle_type = DataType {
            type_hash: TypeHash::from_name("Player"),
            is_const: false,
            is_handle: true,
            is_handle_to_const: false,
            ref_modifier: crate::RefModifier::None,
        };

        // Handle variable is a mutable lvalue
        let expr = ExprInfo::lvalue(handle_type, true);

        assert!(expr.is_lvalue);
        assert!(expr.is_mutable);
        assert!(expr.data_type.is_handle);
    }

    #[test]
    fn test_with_const_ref_type() {
        let const_ref = DataType {
            type_hash: TypeHash::from_name("Vector3"),
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: crate::RefModifier::In,
        };

        // const& parameter is an immutable lvalue
        let expr = ExprInfo::const_lvalue(const_ref);

        assert!(expr.is_lvalue);
        assert!(!expr.is_mutable);
        assert!(expr.data_type.is_const);
    }
}
