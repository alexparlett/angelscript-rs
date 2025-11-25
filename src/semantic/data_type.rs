//! Data type representation with modifiers for the AngelScript type system.
//!
//! This module provides the `DataType` structure which represents a complete type
//! including all modifiers (const, handle, handle-to-const). This is distinct from
//! `TypeId` which only identifies the base type.
//!
//! # Example
//!
//! ```
//! use angelscript::semantic::{DataType, TypeId, INT32_TYPE};
//!
//! // Simple type: int
//! let simple = DataType::simple(INT32_TYPE);
//!
//! // Const type: const int
//! let const_type = DataType::with_const(INT32_TYPE);
//!
//! // Handle: int@
//! let handle = DataType::with_handle(INT32_TYPE, false);
//!
//! // Handle to const: const int@
//! let handle_to_const = DataType::with_handle(INT32_TYPE, true);
//! ```

use super::type_def::TypeId;

/// A complete type including all modifiers.
///
/// This represents the full type information for a value in AngelScript,
/// including the base type and all applied modifiers.
///
/// # Type Modifiers
///
/// - **const**: The value cannot be modified
/// - **handle (@)**: Reference semantics (handle to an object)
/// - **handle to const**: A handle to a const object
///
/// # Example
///
/// ```text
/// int              -> DataType { type_id: INT32_TYPE, is_const: false, is_handle: false, is_handle_to_const: false }
/// const int        -> DataType { type_id: INT32_TYPE, is_const: true, is_handle: false, is_handle_to_const: false }
/// int@             -> DataType { type_id: INT32_TYPE, is_const: false, is_handle: true, is_handle_to_const: false }
/// const int@       -> DataType { type_id: INT32_TYPE, is_const: false, is_handle: true, is_handle_to_const: true }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataType {
    /// The base type identifier
    pub type_id: TypeId,

    /// Whether the value is const (immutable)
    pub is_const: bool,

    /// Whether this is a handle (reference type)
    pub is_handle: bool,

    /// Whether this is a handle to a const value
    pub is_handle_to_const: bool,
}

impl DataType {
    /// Create a simple type with no modifiers.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, INT32_TYPE};
    ///
    /// let int_type = DataType::simple(INT32_TYPE);
    /// assert!(!int_type.is_const);
    /// assert!(!int_type.is_handle);
    /// ```
    #[inline]
    pub fn simple(type_id: TypeId) -> Self {
        Self {
            type_id,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
        }
    }

    /// Create a const type.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, INT32_TYPE};
    ///
    /// let const_int = DataType::with_const(INT32_TYPE);
    /// assert!(const_int.is_const);
    /// assert!(!const_int.is_handle);
    /// ```
    #[inline]
    pub fn with_const(type_id: TypeId) -> Self {
        Self {
            type_id,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
        }
    }

    /// Create a handle type with optional const.
    ///
    /// # Arguments
    ///
    /// * `type_id` - The base type identifier
    /// * `is_const` - Whether this is a handle to a const value
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, INT32_TYPE};
    ///
    /// // int@
    /// let handle = DataType::with_handle(INT32_TYPE, false);
    /// assert!(!handle.is_const);
    /// assert!(handle.is_handle);
    /// assert!(!handle.is_handle_to_const);
    ///
    /// // const int@
    /// let const_handle = DataType::with_handle(INT32_TYPE, true);
    /// assert!(!const_handle.is_const);
    /// assert!(const_handle.is_handle);
    /// assert!(const_handle.is_handle_to_const);
    /// ```
    #[inline]
    pub fn with_handle(type_id: TypeId, is_const: bool) -> Self {
        Self {
            type_id,
            is_const: false,
            is_handle: true,
            is_handle_to_const: is_const,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::type_def::{INT32_TYPE, BOOL_TYPE, FLOAT_TYPE, STRING_TYPE};
    use std::collections::HashSet;

    #[test]
    fn simple_type_creation() {
        let dt = DataType::simple(INT32_TYPE);
        assert_eq!(dt.type_id, INT32_TYPE);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn const_type_creation() {
        let dt = DataType::with_const(INT32_TYPE);
        assert_eq!(dt.type_id, INT32_TYPE);
        assert!(dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_type_creation() {
        let dt = DataType::with_handle(INT32_TYPE, false);
        assert_eq!(dt.type_id, INT32_TYPE);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_to_const_creation() {
        let dt = DataType::with_handle(INT32_TYPE, true);
        assert_eq!(dt.type_id, INT32_TYPE);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(dt.is_handle_to_const);
    }

    #[test]
    fn different_type_ids() {
        let int_type = DataType::simple(INT32_TYPE);
        let bool_type = DataType::simple(BOOL_TYPE);
        let float_type = DataType::simple(FLOAT_TYPE);

        assert_ne!(int_type, bool_type);
        assert_ne!(int_type, float_type);
        assert_ne!(bool_type, float_type);
    }

    #[test]
    fn equality_simple_types() {
        let dt1 = DataType::simple(INT32_TYPE);
        let dt2 = DataType::simple(INT32_TYPE);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_const_types() {
        let dt1 = DataType::with_const(INT32_TYPE);
        let dt2 = DataType::with_const(INT32_TYPE);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_handle_types() {
        let dt1 = DataType::with_handle(INT32_TYPE, false);
        let dt2 = DataType::with_handle(INT32_TYPE, false);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_handle_to_const_types() {
        let dt1 = DataType::with_handle(INT32_TYPE, true);
        let dt2 = DataType::with_handle(INT32_TYPE, true);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn inequality_const_vs_non_const() {
        let simple = DataType::simple(INT32_TYPE);
        let const_type = DataType::with_const(INT32_TYPE);
        assert_ne!(simple, const_type);
    }

    #[test]
    fn inequality_handle_vs_non_handle() {
        let simple = DataType::simple(INT32_TYPE);
        let handle = DataType::with_handle(INT32_TYPE, false);
        assert_ne!(simple, handle);
    }

    #[test]
    fn inequality_handle_const_vs_non_const() {
        let handle = DataType::with_handle(INT32_TYPE, false);
        let const_handle = DataType::with_handle(INT32_TYPE, true);
        assert_ne!(handle, const_handle);
    }

    #[test]
    fn clone_simple_type() {
        let dt1 = DataType::simple(INT32_TYPE);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_const_type() {
        let dt1 = DataType::with_const(INT32_TYPE);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_handle_type() {
        let dt1 = DataType::with_handle(INT32_TYPE, false);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_handle_to_const_type() {
        let dt1 = DataType::with_handle(INT32_TYPE, true);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::simple(INT32_TYPE);
        let dt2 = DataType::simple(INT32_TYPE);

        let mut hasher1 = DefaultHasher::new();
        dt1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        dt2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_different_for_different_modifiers() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let simple = DataType::simple(INT32_TYPE);
        let const_type = DataType::with_const(INT32_TYPE);

        let mut hasher1 = DefaultHasher::new();
        simple.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        const_type.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn can_use_in_hashset() {
        let mut set = HashSet::new();

        set.insert(DataType::simple(INT32_TYPE));
        set.insert(DataType::with_const(INT32_TYPE));
        set.insert(DataType::with_handle(INT32_TYPE, false));
        set.insert(DataType::with_handle(INT32_TYPE, true));

        assert_eq!(set.len(), 4);
        assert!(set.contains(&DataType::simple(INT32_TYPE)));
        assert!(set.contains(&DataType::with_const(INT32_TYPE)));
        assert!(set.contains(&DataType::with_handle(INT32_TYPE, false)));
        assert!(set.contains(&DataType::with_handle(INT32_TYPE, true)));
    }

    #[test]
    fn hashset_no_duplicates() {
        let mut set = HashSet::new();

        set.insert(DataType::simple(INT32_TYPE));
        set.insert(DataType::simple(INT32_TYPE));
        set.insert(DataType::simple(INT32_TYPE));

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn debug_output_simple() {
        let dt = DataType::simple(INT32_TYPE);
        let debug = format!("{:?}", dt);
        assert!(debug.contains("DataType"));
        assert!(debug.contains("type_id"));
    }

    #[test]
    fn debug_output_with_modifiers() {
        let dt = DataType::with_handle(INT32_TYPE, true);
        let debug = format!("{:?}", dt);
        assert!(debug.contains("DataType"));
        assert!(debug.contains("is_handle: true"));
        assert!(debug.contains("is_handle_to_const: true"));
    }

    #[test]
    fn all_modifier_combinations() {
        // Test all 8 possible combinations of the 3 boolean flags
        let type_id = INT32_TYPE;

        // 000: simple
        let dt = DataType { type_id, is_const: false, is_handle: false, is_handle_to_const: false };
        assert_eq!(dt, DataType::simple(type_id));

        // 100: const
        let dt = DataType { type_id, is_const: true, is_handle: false, is_handle_to_const: false };
        assert_eq!(dt, DataType::with_const(type_id));

        // 010: handle
        let dt = DataType { type_id, is_const: false, is_handle: true, is_handle_to_const: false };
        assert_eq!(dt, DataType::with_handle(type_id, false));

        // 011: handle to const
        let dt = DataType { type_id, is_const: false, is_handle: true, is_handle_to_const: true };
        assert_eq!(dt, DataType::with_handle(type_id, true));

        // Other combinations (110, 101, 111, 001) might be invalid in AngelScript
        // but DataType should still be able to represent them
        let dt = DataType { type_id, is_const: true, is_handle: true, is_handle_to_const: false };
        assert!(dt.is_const && dt.is_handle);

        let dt = DataType { type_id, is_const: true, is_handle: false, is_handle_to_const: true };
        assert!(dt.is_const && dt.is_handle_to_const);
    }

    #[test]
    fn works_with_string_type() {
        let dt = DataType::simple(STRING_TYPE);
        assert_eq!(dt.type_id, STRING_TYPE);
    }

    #[test]
    fn works_with_bool_type() {
        let dt = DataType::with_const(BOOL_TYPE);
        assert_eq!(dt.type_id, BOOL_TYPE);
        assert!(dt.is_const);
    }
}
