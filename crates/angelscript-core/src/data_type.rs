//! DataType - represents a type with modifiers.
//!
//! This module provides the `DataType` structure which represents a complete type
//! including all modifiers (const, handle, handle-to-const, reference). This is
//! distinct from `TypeHash` which only identifies the base type.
//!
//! # Example
//!
//! ```
//! use angelscript_core::{DataType, RefModifier, primitives};
//!
//! // Simple type: int
//! let simple = DataType::simple(primitives::INT32);
//!
//! // Const type: const int
//! let const_type = DataType::with_const(primitives::INT32);
//!
//! // Handle: int@
//! let handle = DataType::with_handle(primitives::INT32, false);
//!
//! // Reference parameter: int &in
//! let ref_in = DataType::with_ref_in(primitives::INT32);
//! assert_eq!(ref_in.ref_modifier, RefModifier::In);
//! ```

use std::fmt::{self, Display, Formatter};

use crate::TypeHash;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RefModifier {
    /// No reference modifier
    #[default]
    None,
    /// &in - Read-only reference, accepts any value
    In,
    /// &out - Write-only reference, requires mutable lvalue
    Out,
    /// &inout - Read-write reference, requires mutable lvalue
    InOut,
}

impl Display for RefModifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RefModifier::None => Ok(()),
            RefModifier::In => write!(f, "&in"),
            RefModifier::Out => write!(f, "&out"),
            RefModifier::InOut => write!(f, "&inout"),
        }
    }
}

/// A complete type including all modifiers.
///
/// This represents the full type information for a value in AngelScript,
/// including the base type and all applied modifiers. This struct is `Copy`
/// for efficient passing without allocation.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// use angelscript_core::{DataType, primitives};
    ///
    /// let int_type = DataType::simple(primitives::INT32);
    /// assert!(!int_type.is_const);
    /// assert!(!int_type.is_handle);
    /// ```
    #[inline]
    pub const fn simple(type_hash: TypeHash) -> Self {
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
    /// use angelscript_core::{DataType, primitives};
    ///
    /// let const_int = DataType::with_const(primitives::INT32);
    /// assert!(const_int.is_const);
    /// assert!(!const_int.is_handle);
    /// ```
    #[inline]
    pub const fn with_const(type_hash: TypeHash) -> Self {
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
    /// use angelscript_core::{DataType, primitives};
    ///
    /// // int@ - mutable handle to mutable object
    /// let handle = DataType::with_handle(primitives::INT32, false);
    /// assert!(!handle.is_const);
    /// assert!(handle.is_handle);
    /// assert!(!handle.is_handle_to_const);
    ///
    /// // int@ const - mutable handle to const object
    /// let handle_to_const = DataType::with_handle(primitives::INT32, true);
    /// assert!(!handle_to_const.is_const);
    /// assert!(handle_to_const.is_handle);
    /// assert!(handle_to_const.is_handle_to_const);
    /// ```
    #[inline]
    pub const fn with_handle(type_hash: TypeHash, is_handle_to_const: bool) -> Self {
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
    /// use angelscript_core::{DataType, primitives};
    ///
    /// // const int@ - read-only handle to mutable object
    /// let const_handle = DataType::const_handle(primitives::INT32, false);
    /// assert!(const_handle.is_const);
    /// assert!(const_handle.is_handle);
    /// assert!(!const_handle.is_handle_to_const);
    ///
    /// // const int@ const - read-only handle to const object
    /// let const_handle_to_const = DataType::const_handle(primitives::INT32, true);
    /// assert!(const_handle_to_const.is_const);
    /// assert!(const_handle_to_const.is_handle);
    /// assert!(const_handle_to_const.is_handle_to_const);
    /// ```
    #[inline]
    pub const fn const_handle(type_hash: TypeHash, is_handle_to_const: bool) -> Self {
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
    /// use angelscript_core::{DataType, RefModifier, primitives};
    ///
    /// let ref_in = DataType::with_ref_in(primitives::INT32);
    /// assert_eq!(ref_in.ref_modifier, RefModifier::In);
    /// ```
    #[inline]
    pub const fn with_ref_in(type_hash: TypeHash) -> Self {
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
    /// use angelscript_core::{DataType, RefModifier, primitives};
    ///
    /// let ref_out = DataType::with_ref_out(primitives::INT32);
    /// assert_eq!(ref_out.ref_modifier, RefModifier::Out);
    /// ```
    #[inline]
    pub const fn with_ref_out(type_hash: TypeHash) -> Self {
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
    /// use angelscript_core::{DataType, RefModifier, primitives};
    ///
    /// let ref_inout = DataType::with_ref_inout(primitives::INT32);
    /// assert_eq!(ref_inout.ref_modifier, RefModifier::InOut);
    /// ```
    #[inline]
    pub const fn with_ref_inout(type_hash: TypeHash) -> Self {
        Self {
            type_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::InOut,
        }
    }

    /// Create a void type.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript_core::{DataType, primitives};
    ///
    /// let void_type = DataType::void();
    /// assert_eq!(void_type.type_hash, primitives::VOID);
    /// ```
    #[inline]
    pub const fn void() -> Self {
        Self::simple(crate::primitives::VOID)
    }

    /// Create a null literal type.
    ///
    /// Null literals use NULL hash and are compatible with any handle type
    /// through implicit conversion.
    ///
    /// # Example
    ///
    /// ```
    /// use angelscript_core::{DataType, primitives};
    ///
    /// let null_lit = DataType::null_literal();
    /// assert_eq!(null_lit.type_hash, primitives::NULL);
    /// ```
    #[inline]
    pub const fn null_literal() -> Self {
        Self::simple(crate::primitives::NULL)
    }

    /// Returns true if this type has any reference modifier.
    #[inline]
    pub const fn is_reference(&self) -> bool {
        !matches!(self.ref_modifier, RefModifier::None)
    }

    /// Returns a copy of this type with the const modifier set.
    #[inline]
    pub const fn as_const(self) -> Self {
        Self {
            is_const: true,
            ..self
        }
    }

    /// Returns a copy of this type without the const modifier.
    #[inline]
    pub const fn without_const(self) -> Self {
        Self {
            is_const: false,
            ..self
        }
    }

    /// Returns a copy of this type without any reference modifier.
    #[inline]
    pub const fn without_ref(self) -> Self {
        Self {
            ref_modifier: RefModifier::None,
            ..self
        }
    }

    /// Returns true if this type is void.
    #[inline]
    pub const fn is_void(&self) -> bool {
        self.type_hash.0 == crate::primitives::VOID.0
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Format: [const] <type>[@] [const] [&ref]
        // Examples: "int", "const int", "Player@", "const Player@ const", "int &in"

        if self.is_const && !self.is_handle {
            write!(f, "const ")?;
        }

        // Write type hash (will show as hex for now, could be improved with type name lookup)
        write!(f, "{}", self.type_hash)?;

        if self.is_handle {
            write!(f, "@")?;
            if self.is_handle_to_const {
                write!(f, " const")?;
            }
        }

        if self.ref_modifier != RefModifier::None {
            write!(f, " {}", self.ref_modifier)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;
    use std::collections::HashSet;

    #[test]
    fn simple_type_creation() {
        let dt = DataType::simple(primitives::INT32);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
        assert_eq!(dt.ref_modifier, RefModifier::None);
    }

    #[test]
    fn const_type_creation() {
        let dt = DataType::with_const(primitives::INT32);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(dt.is_const);
        assert!(!dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_type_creation() {
        let dt = DataType::with_handle(primitives::INT32, false);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(!dt.is_handle_to_const);
    }

    #[test]
    fn handle_to_const_creation() {
        let dt = DataType::with_handle(primitives::INT32, true);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert!(!dt.is_const);
        assert!(dt.is_handle);
        assert!(dt.is_handle_to_const);
    }

    #[test]
    fn const_handle_creation() {
        let dt = DataType::const_handle(primitives::INT32, false);
        assert!(dt.is_const);
        assert!(dt.is_handle);
        assert!(!dt.is_handle_to_const);

        let dt2 = DataType::const_handle(primitives::INT32, true);
        assert!(dt2.is_const);
        assert!(dt2.is_handle);
        assert!(dt2.is_handle_to_const);
    }

    #[test]
    fn ref_in_creation() {
        let dt = DataType::with_ref_in(primitives::INT32);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::In);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn ref_out_creation() {
        let dt = DataType::with_ref_out(primitives::FLOAT);
        assert_eq!(dt.type_hash, primitives::FLOAT);
        assert_eq!(dt.ref_modifier, RefModifier::Out);
    }

    #[test]
    fn ref_inout_creation() {
        let dt = DataType::with_ref_inout(primitives::INT32);
        assert_eq!(dt.type_hash, primitives::INT32);
        assert_eq!(dt.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn void_type() {
        let dt = DataType::void();
        assert_eq!(dt.type_hash, primitives::VOID);
        assert!(!dt.is_const);
        assert!(!dt.is_handle);
    }

    #[test]
    fn null_literal_type() {
        let dt = DataType::null_literal();
        assert_eq!(dt.type_hash, primitives::NULL);
    }

    #[test]
    fn is_reference() {
        assert!(!DataType::simple(primitives::INT32).is_reference());
        assert!(DataType::with_ref_in(primitives::INT32).is_reference());
        assert!(DataType::with_ref_out(primitives::INT32).is_reference());
        assert!(DataType::with_ref_inout(primitives::INT32).is_reference());
    }

    #[test]
    fn as_const() {
        let dt = DataType::simple(primitives::INT32);
        assert!(!dt.is_const);
        let const_dt = dt.as_const();
        assert!(const_dt.is_const);
        assert_eq!(const_dt.type_hash, primitives::INT32);
    }

    #[test]
    fn without_const() {
        let dt = DataType::with_const(primitives::INT32);
        assert!(dt.is_const);
        let non_const = dt.without_const();
        assert!(!non_const.is_const);
    }

    #[test]
    fn without_ref() {
        let dt = DataType::with_ref_in(primitives::INT32);
        assert!(dt.is_reference());
        let no_ref = dt.without_ref();
        assert!(!no_ref.is_reference());
        assert_eq!(no_ref.ref_modifier, RefModifier::None);
    }

    #[test]
    fn equality() {
        let dt1 = DataType::simple(primitives::INT32);
        let dt2 = DataType::simple(primitives::INT32);
        assert_eq!(dt1, dt2);

        let dt3 = DataType::with_const(primitives::INT32);
        assert_ne!(dt1, dt3);
    }

    #[test]
    fn copy_semantics() {
        let dt1 = DataType::simple(primitives::INT32);
        let dt2 = dt1; // Copy, not move
        assert_eq!(dt1, dt2); // dt1 still usable
    }

    #[test]
    fn hash_in_set() {
        let mut set = HashSet::new();
        set.insert(DataType::simple(primitives::INT32));
        set.insert(DataType::with_const(primitives::INT32));
        set.insert(DataType::with_handle(primitives::INT32, false));
        set.insert(DataType::with_ref_in(primitives::INT32));

        assert_eq!(set.len(), 4);
        assert!(set.contains(&DataType::simple(primitives::INT32)));
    }

    #[test]
    fn ref_modifier_display() {
        assert_eq!(format!("{}", RefModifier::None), "");
        assert_eq!(format!("{}", RefModifier::In), "&in");
        assert_eq!(format!("{}", RefModifier::Out), "&out");
        assert_eq!(format!("{}", RefModifier::InOut), "&inout");
    }

    #[test]
    fn ref_modifier_default() {
        assert_eq!(RefModifier::default(), RefModifier::None);
    }

    #[test]
    fn data_type_display_simple() {
        let dt = DataType::simple(primitives::INT32);
        let s = format!("{}", dt);
        // Should contain the type hash representation
        assert!(!s.is_empty());
    }

    #[test]
    fn data_type_display_const() {
        let dt = DataType::with_const(primitives::INT32);
        let s = format!("{}", dt);
        assert!(s.starts_with("const "));
    }

    #[test]
    fn data_type_display_handle() {
        let dt = DataType::with_handle(primitives::INT32, false);
        let s = format!("{}", dt);
        assert!(s.contains("@"));
    }

    #[test]
    fn data_type_display_handle_to_const() {
        let dt = DataType::with_handle(primitives::INT32, true);
        let s = format!("{}", dt);
        assert!(s.contains("@"));
        assert!(s.contains("const"));
    }

    #[test]
    fn data_type_display_ref() {
        let dt = DataType::with_ref_in(primitives::INT32);
        let s = format!("{}", dt);
        assert!(s.contains("&in"));
    }
}
