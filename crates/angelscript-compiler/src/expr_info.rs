//! Expression type information for the compiler.
//!
//! `ExprInfo` captures the result of type-checking an expression,
//! including whether it's an lvalue and its mutability.

use angelscript_core::DataType;

/// The source/storage class of an expression value.
///
/// This is used to validate reference returns - we cannot return
/// references to local variables or parameters since they are
/// cleaned up when the function exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValueSource {
    /// A temporary value (rvalue, literals, expression results).
    #[default]
    Temporary,
    /// A local variable or function parameter.
    Local,
    /// A global variable.
    Global,
    /// A class member (field or property).
    Member,
    /// The 'this' pointer.
    This,
}

impl ValueSource {
    /// Check if this source is safe to return by reference.
    ///
    /// Local variables and parameters cannot be returned by reference
    /// since they are destroyed when the function exits.
    pub fn is_safe_for_ref_return(&self) -> bool {
        match self {
            ValueSource::Temporary => false, // Can't return reference to temporary
            ValueSource::Local => false,     // Can't return reference to local
            ValueSource::Global => true,     // Globals persist
            ValueSource::Member => true,     // Members persist (via this)
            ValueSource::This => true,       // this is valid for method duration
        }
    }
}

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
    /// The source/storage class of this value.
    pub source: ValueSource,
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
            source: ValueSource::Temporary,
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
            source: ValueSource::Temporary, // Default, caller should set if needed
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
            source: ValueSource::Temporary, // Default, caller should set if needed
        }
    }

    /// Create an lvalue for a local variable.
    pub fn local(data_type: DataType, is_const: bool) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: !is_const,
            source: ValueSource::Local,
        }
    }

    /// Create an lvalue for a global variable.
    pub fn global(data_type: DataType, is_const: bool) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: !is_const,
            source: ValueSource::Global,
        }
    }

    /// Create an lvalue for a class member.
    pub fn member(data_type: DataType, is_const: bool) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: !is_const,
            source: ValueSource::Member,
        }
    }

    /// Create an lvalue for 'this'.
    pub fn this_ptr(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: false, // 'this' itself cannot be reassigned
            source: ValueSource::This,
        }
    }

    /// Set the value source.
    pub fn with_source(mut self, source: ValueSource) -> Self {
        self.source = source;
        self
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
    /// Preserves the source for reference return validation.
    pub fn to_rvalue(self) -> Self {
        Self {
            data_type: self.data_type,
            is_lvalue: false,
            is_mutable: false,
            source: self.source, // Preserve source for ref return validation
        }
    }

    /// Check if this value is safe to return by reference.
    pub fn is_safe_for_ref_return(&self) -> bool {
        self.source.is_safe_for_ref_return()
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

    #[test]
    fn local_not_safe_for_ref_return() {
        let info = ExprInfo::local(DataType::simple(primitives::INT32), false);
        assert!(!info.is_safe_for_ref_return());
    }

    #[test]
    fn global_safe_for_ref_return() {
        let info = ExprInfo::global(DataType::simple(primitives::INT32), false);
        assert!(info.is_safe_for_ref_return());
    }

    #[test]
    fn member_safe_for_ref_return() {
        let info = ExprInfo::member(DataType::simple(primitives::INT32), false);
        assert!(info.is_safe_for_ref_return());
    }

    #[test]
    fn this_safe_for_ref_return() {
        let info = ExprInfo::this_ptr(DataType::simple(primitives::INT32));
        assert!(info.is_safe_for_ref_return());
    }

    #[test]
    fn temporary_not_safe_for_ref_return() {
        let info = ExprInfo::rvalue(DataType::simple(primitives::INT32));
        assert!(!info.is_safe_for_ref_return());
    }

    #[test]
    fn to_rvalue_preserves_source() {
        let local = ExprInfo::local(DataType::simple(primitives::INT32), false);
        let rvalue = local.to_rvalue();
        assert_eq!(rvalue.source, ValueSource::Local);
        assert!(!rvalue.is_safe_for_ref_return());

        let global = ExprInfo::global(DataType::simple(primitives::INT32), false);
        let rvalue = global.to_rvalue();
        assert_eq!(rvalue.source, ValueSource::Global);
        assert!(rvalue.is_safe_for_ref_return());
    }
}
