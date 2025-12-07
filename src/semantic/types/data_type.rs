//! Data type representation with modifiers for the AngelScript type system.
//!
//! This module provides the `DataType` structure which represents a complete type
//! including all modifiers (const, handle, handle-to-const). This is distinct from
//! `TypeHash` which only identifies the base type.
//!
//! # Example
//!
//! ```
//! use angelscript::semantic::DataType;
//! use angelscript::types::primitive_hashes;
//!
//! // Simple type: int
//! let simple = DataType::simple(primitive_hashes::INT32);
//!
//! // Const type: const int
//! let const_type = DataType::with_const(primitive_hashes::INT32);
//!
//! // Handle: int@
//! let handle = DataType::with_handle(primitive_hashes::INT32, false);
//!
//! // Handle to const: const int@
//! let handle_to_const = DataType::with_handle(primitive_hashes::INT32, true);
//! ```

use crate::types::{primitive_hashes, TypeHash};

/// Reference modifier for parameters.
///
/// In AngelScript, parameters can be passed by reference with different access modes:
/// - `&in`: Read-only reference, accepts any value (can create temps)
/// - `&out`: Write-only reference, requires mutable lvalue, uninitialized on entry
/// - `&inout`: Read-write reference, requires mutable lvalue, must be initialized
///
/// # Examples
///
/// ```angelscript
/// void foo(int &in x)           // Pass by reference (read-only)
/// void bar(int &out y)          // Pass by reference (write-only)
/// void baz(int &inout z)        // Pass by reference (read-write)
/// MyClass(const MyClass &inout other)  // Copy constructor
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefModifier {
    /// No reference modifier
    None,
    /// &in - Read-only reference, accepts any value
    In,
    /// &out - Write-only reference, requires mutable lvalue
    Out,
    /// &inout - Read-write reference, requires mutable lvalue
    InOut,
}

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
/// - **reference (&in, &out, &inout)**: Parameter passing mode
///
/// # Example
///
/// ```text
/// int              -> DataType { type_hash: INT32, is_const: false, is_handle: false, is_handle_to_const: false, ref_modifier: None }
/// const int        -> DataType { type_hash: INT32, is_const: true, is_handle: false, is_handle_to_const: false, ref_modifier: None }
/// int@             -> DataType { type_hash: INT32, is_const: false, is_handle: true, is_handle_to_const: false, ref_modifier: None }
/// const int@       -> DataType { type_hash: INT32, is_const: false, is_handle: true, is_handle_to_const: true, ref_modifier: None }
/// int &in          -> DataType { type_hash: INT32, is_const: false, is_handle: false, is_handle_to_const: false, ref_modifier: In }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataType {
    /// The base type hash (deterministic hash computed from type name)
    pub type_hash: TypeHash,

    /// Whether the value is const (immutable)
    pub is_const: bool,

    /// Whether this is a handle (reference type)
    pub is_handle: bool,

    /// Whether this is a handle to a const value
    pub is_handle_to_const: bool,

    /// Reference modifier for parameters
    pub ref_modifier: RefModifier,
}

impl DataType {
    /// Create a simple type with no modifiers.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::DataType;
    /// use angelscript::types::primitive_hashes;
    ///
    /// let int_type = DataType::simple(primitive_hashes::INT32);
    /// assert!(!int_type.is_const);
    /// assert!(!int_type.is_handle);
    /// ```
    #[inline]
    pub fn simple(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
        }
    }

    /// Create a const type.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::DataType;
    /// use angelscript::types::primitive_hashes;
    ///
    /// let const_int = DataType::with_const(primitive_hashes::INT32);
    /// assert!(const_int.is_const);
    /// assert!(!const_int.is_handle);
    /// ```
    #[inline]
    pub fn with_const(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
        }
    }

    /// Create a handle type with optional const modifiers.
    ///
    /// AngelScript supports two independent const modifiers for handles:
    /// - `const T@` - Read-only handle (can't reassign the handle)
    /// - `T@ const` - Handle to a const object (can't modify the object)
    /// - `const T@ const` - Both restrictions
    ///
    /// # Arguments
    ///
    /// * `type_hash` - The base type hash
    /// * `is_handle_to_const` - Whether this is a handle to a const value (T@ const)
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::DataType;
    /// use angelscript::types::primitive_hashes;
    ///
    /// // int@ - mutable handle to mutable object
    /// let handle = DataType::with_handle(primitive_hashes::INT32, false);
    /// assert!(!handle.is_const);
    /// assert!(handle.is_handle);
    /// assert!(!handle.is_handle_to_const);
    ///
    /// // int@ const - mutable handle to const object
    /// let handle_to_const = DataType::with_handle(primitive_hashes::INT32, true);
    /// assert!(!handle_to_const.is_const);
    /// assert!(handle_to_const.is_handle);
    /// assert!(handle_to_const.is_handle_to_const);
    /// ```
    #[inline]
    pub fn with_handle(type_hash: TypeHash, is_handle_to_const: bool) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: true,
            is_handle_to_const,
            ref_modifier: RefModifier::None,
        }
    }

    /// Create a const handle type (const T@).
    ///
    /// This creates a read-only handle - the handle itself cannot be reassigned.
    /// Optionally, the object can also be const (const T@ const).
    ///
    /// # Arguments
    ///
    /// * `type_hash` - The base type hash
    /// * `is_handle_to_const` - Whether the object is also const (const T@ const)
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::DataType;
    /// use angelscript::types::primitive_hashes;
    ///
    /// // const int@ - read-only handle to mutable object
    /// let const_handle = DataType::const_handle(primitive_hashes::INT32, false);
    /// assert!(const_handle.is_const);
    /// assert!(const_handle.is_handle);
    /// assert!(!const_handle.is_handle_to_const);
    ///
    /// // const int@ const - read-only handle to const object
    /// let const_handle_to_const = DataType::const_handle(primitive_hashes::INT32, true);
    /// assert!(const_handle_to_const.is_const);
    /// assert!(const_handle_to_const.is_handle);
    /// assert!(const_handle_to_const.is_handle_to_const);
    /// ```
    #[inline]
    pub fn const_handle(type_hash: TypeHash, is_handle_to_const: bool) -> Self {
        Self {
            type_hash,
            is_const: true,
            is_handle: true,
            is_handle_to_const,
            ref_modifier: RefModifier::None,
        }
    }

    /// Create a reference type with &in modifier.
    ///
    /// `&in` parameters are read-only references that can accept any value (including temps).
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, RefModifier};
    /// use angelscript::types::primitive_hashes;
    ///
    /// let ref_in = DataType::with_ref_in(primitive_hashes::INT32);
    /// assert_eq!(ref_in.ref_modifier, RefModifier::In);
    /// ```
    #[inline]
    pub fn with_ref_in(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::In,
        }
    }

    /// Create a reference type with &out modifier.
    ///
    /// `&out` parameters are write-only references that require mutable lvalues.
    /// The parameter is uninitialized on entry.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, RefModifier};
    /// use angelscript::types::primitive_hashes;
    ///
    /// let ref_out = DataType::with_ref_out(primitive_hashes::INT32);
    /// assert_eq!(ref_out.ref_modifier, RefModifier::Out);
    /// ```
    #[inline]
    pub fn with_ref_out(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::Out,
        }
    }

    /// Create a reference type with &inout modifier.
    ///
    /// `&inout` parameters are read-write references that require mutable lvalues.
    /// The parameter must be initialized on entry.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::{DataType, RefModifier};
    /// use angelscript::types::primitive_hashes;
    ///
    /// let ref_inout = DataType::with_ref_inout(primitive_hashes::INT32);
    /// assert_eq!(ref_inout.ref_modifier, RefModifier::InOut);
    /// ```
    #[inline]
    pub fn with_ref_inout(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::InOut,
        }
    }

    /// Create a null literal type.
    ///
    /// Null literals use NULL hash and are compatible with any handle type through implicit conversion.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript::semantic::DataType;
    /// use angelscript::types::primitive_hashes;
    ///
    /// let null_lit = DataType::null_literal();
    /// assert_eq!(null_lit.type_hash, primitive_hashes::NULL);
    /// ```
    #[inline]
    pub fn null_literal() -> Self {
        Self::simple(primitive_hashes::NULL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn simple_type_creation() {
        let dt = DataType::simple(primitive_hashes::INT32);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn const_type_creation() {
        let dt = DataType::with_const(primitive_hashes::INT32);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_type_creation() {
        let dt = DataType::with_handle(primitive_hashes::INT32, false);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_to_const_creation() {
        let dt = DataType::with_handle(primitive_hashes::INT32, true);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(dt.is_handle_to_const);
    }

    #[test]
    fn different_type_hashes() {
        let int_type = DataType::simple(primitive_hashes::INT32);
        let bool_type = DataType::simple(primitive_hashes::BOOL);
        let float_type = DataType::simple(primitive_hashes::FLOAT);

        assert_ne!(int_type, bool_type);
        assert_ne!(int_type, float_type);
        assert_ne!(bool_type, float_type);
    }

    #[test]
    fn equality_simple_types() {
        let dt1 = DataType::simple(primitive_hashes::INT32);
        let dt2 = DataType::simple(primitive_hashes::INT32);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_const_types() {
        let dt1 = DataType::with_const(primitive_hashes::INT32);
        let dt2 = DataType::with_const(primitive_hashes::INT32);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_handle_types() {
        let dt1 = DataType::with_handle(primitive_hashes::INT32, false);
        let dt2 = DataType::with_handle(primitive_hashes::INT32, false);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn equality_handle_to_const_types() {
        let dt1 = DataType::with_handle(primitive_hashes::INT32, true);
        let dt2 = DataType::with_handle(primitive_hashes::INT32, true);
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn inequality_const_vs_non_const() {
        let simple = DataType::simple(primitive_hashes::INT32);
        let const_type = DataType::with_const(primitive_hashes::INT32);
        assert_ne!(simple, const_type);
    }

    #[test]
    fn inequality_handle_vs_non_handle() {
        let simple = DataType::simple(primitive_hashes::INT32);
        let handle = DataType::with_handle(primitive_hashes::INT32, false);
        assert_ne!(simple, handle);
    }

    #[test]
    fn inequality_handle_const_vs_non_const() {
        let handle = DataType::with_handle(primitive_hashes::INT32, false);
        let const_handle = DataType::with_handle(primitive_hashes::INT32, true);
        assert_ne!(handle, const_handle);
    }

    #[test]
    fn clone_simple_type() {
        let dt1 = DataType::simple(primitive_hashes::INT32);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_const_type() {
        let dt1 = DataType::with_const(primitive_hashes::INT32);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_handle_type() {
        let dt1 = DataType::with_handle(primitive_hashes::INT32, false);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn clone_handle_to_const_type() {
        let dt1 = DataType::with_handle(primitive_hashes::INT32, true);
        let dt2 = dt1.clone();
        assert_eq!(dt1, dt2);
    }

    #[test]
    fn hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let dt1 = DataType::simple(primitive_hashes::INT32);
        let dt2 = DataType::simple(primitive_hashes::INT32);

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

        let simple = DataType::simple(primitive_hashes::INT32);
        let const_type = DataType::with_const(primitive_hashes::INT32);

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

        set.insert(DataType::simple(primitive_hashes::INT32));
        set.insert(DataType::with_const(primitive_hashes::INT32));
        set.insert(DataType::with_handle(primitive_hashes::INT32, false));
        set.insert(DataType::with_handle(primitive_hashes::INT32, true));

        assert_eq!(set.len(), 4);
        assert!(set.contains(&DataType::simple(primitive_hashes::INT32)));
        assert!(set.contains(&DataType::with_const(primitive_hashes::INT32)));
        assert!(set.contains(&DataType::with_handle(primitive_hashes::INT32, false)));
        assert!(set.contains(&DataType::with_handle(primitive_hashes::INT32, true)));
    }

    #[test]
    fn hashset_no_duplicates() {
        let mut set = HashSet::new();

        set.insert(DataType::simple(primitive_hashes::INT32));
        set.insert(DataType::simple(primitive_hashes::INT32));
        set.insert(DataType::simple(primitive_hashes::INT32));

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn debug_output_simple() {
        let dt = DataType::simple(primitive_hashes::INT32);
        let debug = format!("{:?}", dt);
        assert!(debug.contains("DataType"));
        assert!(debug.contains("type_hash"));
    }

    #[test]
    fn debug_output_with_modifiers() {
        let dt = DataType::with_handle(primitive_hashes::INT32, true);
        let debug = format!("{:?}", dt);
        assert!(debug.contains("DataType"));
        assert!(debug.contains("is_handle: true"));
        assert!(debug.contains("is_handle_to_const: true"));
    }

    #[test]
    fn all_modifier_combinations() {
        // Test all 8 possible combinations of the 3 boolean flags
        let type_hash = primitive_hashes::INT32;

        // 000: simple
        let dt = DataType { type_hash, is_const: false, is_handle: false, is_handle_to_const: false, ref_modifier: RefModifier::None };
        assert_eq!(dt, DataType::simple(type_hash));

        // 100: const
        let dt = DataType { type_hash, is_const: true, is_handle: false, is_handle_to_const: false, ref_modifier: RefModifier::None };
        assert_eq!(dt, DataType::with_const(type_hash));

        // 010: handle
        let dt = DataType { type_hash, is_const: false, is_handle: true, is_handle_to_const: false, ref_modifier: RefModifier::None };
        assert_eq!(dt, DataType::with_handle(type_hash, false));

        // 011: handle to const
        let dt = DataType { type_hash, is_const: false, is_handle: true, is_handle_to_const: true, ref_modifier: RefModifier::None };
        assert_eq!(dt, DataType::with_handle(type_hash, true));

        // Other combinations (110, 101, 111, 001) might be invalid in AngelScript
        // but DataType should still be able to represent them
        let dt = DataType { type_hash, is_const: true, is_handle: true, is_handle_to_const: false, ref_modifier: RefModifier::None };
        assert!(dt.is_const && dt.is_handle);

        let dt = DataType { type_hash, is_const: true, is_handle: false, is_handle_to_const: true, ref_modifier: RefModifier::None };
        assert!(dt.is_const && dt.is_handle_to_const);
    }

    #[test]
    fn works_with_bool_type() {
        let dt = DataType::with_const(primitive_hashes::BOOL);
        assert_eq!(dt.type_hash, primitive_hashes::BOOL);
        assert!(dt.is_const);
    }

    #[test]
    fn ref_in_creation() {
        let dt = DataType::with_ref_in(primitive_hashes::INT32);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::In);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn ref_out_creation() {
        let dt = DataType::with_ref_out(primitive_hashes::FLOAT);
        assert_eq!(dt.type_hash, primitive_hashes::FLOAT);
        assert_eq!(dt.ref_modifier, RefModifier::Out);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn ref_inout_creation() {
        let dt = DataType::with_ref_inout(primitive_hashes::INT32);
        assert_eq!(dt.type_hash, primitive_hashes::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::InOut);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn ref_modifier_none_by_default() {
        let dt = DataType::simple(primitive_hashes::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::None);

        let dt = DataType::with_const(primitive_hashes::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::None);

        let dt = DataType::with_handle(primitive_hashes::INT32, false);
        assert_eq!(dt.ref_modifier, RefModifier::None);
    }

    #[test]
    fn ref_modifier_equality() {
        let ref_in1 = DataType::with_ref_in(primitive_hashes::INT32);
        let ref_in2 = DataType::with_ref_in(primitive_hashes::INT32);
        assert_eq!(ref_in1, ref_in2);

        let ref_out = DataType::with_ref_out(primitive_hashes::INT32);
        assert_ne!(ref_in1, ref_out);

        let no_ref = DataType::simple(primitive_hashes::INT32);
        assert_ne!(ref_in1, no_ref);
    }

    #[test]
    fn ref_modifier_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(DataType::with_ref_in(primitive_hashes::INT32));
        set.insert(DataType::with_ref_out(primitive_hashes::INT32));
        set.insert(DataType::with_ref_inout(primitive_hashes::INT32));
        set.insert(DataType::simple(primitive_hashes::INT32));

        assert_eq!(set.len(), 4);
    }
}
