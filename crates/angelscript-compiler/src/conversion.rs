//! Type conversion tracking for overload resolution.
//!
//! This module defines types for tracking implicit and explicit type
//! conversions, along with their costs for overload resolution.

use angelscript_core::TypeHash;

/// A type conversion with its cost for overload resolution.
///
/// When resolving overloaded functions, the compiler needs to track
/// what conversions are required for each argument and their relative
/// costs to choose the best match.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    /// The kind of conversion being performed.
    pub kind: ConversionKind,
    /// The cost of this conversion (lower is better).
    pub cost: u32,
    /// Whether this conversion can be applied implicitly.
    pub is_implicit: bool,
}

/// The kind of conversion being performed.
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// No conversion needed (exact match).
    Identity,

    /// Primitive type conversion (int -> float, etc.).
    Primitive {
        /// Source type hash.
        from: TypeHash,
        /// Target type hash.
        to: TypeHash,
    },

    /// Null literal to handle type.
    NullToHandle,

    /// Handle to const handle.
    HandleToConst,

    /// Derived class to base class.
    DerivedToBase {
        /// The base class type hash.
        base: TypeHash,
    },

    /// Class to interface it implements.
    ClassToInterface {
        /// The interface type hash.
        interface: TypeHash,
    },

    /// Implicit conversion via constructor.
    ConstructorConversion {
        /// The constructor function hash.
        constructor: TypeHash,
    },

    /// Implicit conversion via opImplConv method.
    ImplicitConvMethod {
        /// The conversion method hash.
        method: TypeHash,
    },

    /// Explicit cast via opCast method.
    ExplicitCastMethod {
        /// The cast method hash.
        method: TypeHash,
    },

    /// Value type to handle (@value).
    ValueToHandle,

    /// Enum to underlying integer type.
    EnumToInt,

    /// Integer to enum type.
    IntToEnum {
        /// The enum type hash.
        enum_type: TypeHash,
    },
}

impl Conversion {
    /// Cost for exact match (identity conversion).
    pub const COST_EXACT: u32 = 0;
    /// Cost for adding const qualifier.
    pub const COST_CONST_ADDITION: u32 = 1;
    /// Cost for primitive widening (int8 -> int32, float -> double).
    pub const COST_PRIMITIVE_WIDENING: u32 = 2;
    /// Cost for primitive narrowing (int32 -> int8, double -> float).
    pub const COST_PRIMITIVE_NARROWING: u32 = 4;
    /// Cost for derived-to-base conversion.
    pub const COST_DERIVED_TO_BASE: u32 = 5;
    /// Cost for class-to-interface conversion.
    pub const COST_CLASS_TO_INTERFACE: u32 = 6;
    /// Cost for user-defined implicit conversion.
    pub const COST_USER_IMPLICIT: u32 = 10;
    /// Cost marker for explicit-only conversions (not usable implicitly).
    pub const COST_EXPLICIT_ONLY: u32 = 100;

    /// Create an identity conversion (no conversion needed).
    pub fn identity() -> Self {
        Self {
            kind: ConversionKind::Identity,
            cost: Self::COST_EXACT,
            is_implicit: true,
        }
    }

    /// Create a primitive widening conversion.
    pub fn primitive_widening(from: TypeHash, to: TypeHash) -> Self {
        Self {
            kind: ConversionKind::Primitive { from, to },
            cost: Self::COST_PRIMITIVE_WIDENING,
            is_implicit: true,
        }
    }

    /// Create a primitive narrowing conversion.
    pub fn primitive_narrowing(from: TypeHash, to: TypeHash) -> Self {
        Self {
            kind: ConversionKind::Primitive { from, to },
            cost: Self::COST_PRIMITIVE_NARROWING,
            is_implicit: false, // Narrowing requires explicit cast
        }
    }

    /// Create a null-to-handle conversion.
    pub fn null_to_handle() -> Self {
        Self {
            kind: ConversionKind::NullToHandle,
            cost: Self::COST_EXACT,
            is_implicit: true,
        }
    }

    /// Create a handle-to-const conversion.
    pub fn handle_to_const() -> Self {
        Self {
            kind: ConversionKind::HandleToConst,
            cost: Self::COST_CONST_ADDITION,
            is_implicit: true,
        }
    }

    /// Create a derived-to-base conversion.
    pub fn derived_to_base(base: TypeHash) -> Self {
        Self {
            kind: ConversionKind::DerivedToBase { base },
            cost: Self::COST_DERIVED_TO_BASE,
            is_implicit: true,
        }
    }

    /// Create a class-to-interface conversion.
    pub fn class_to_interface(interface: TypeHash) -> Self {
        Self {
            kind: ConversionKind::ClassToInterface { interface },
            cost: Self::COST_CLASS_TO_INTERFACE,
            is_implicit: true,
        }
    }

    /// Check if this conversion can be used implicitly.
    pub fn is_implicit(&self) -> bool {
        self.is_implicit
    }

    /// Check if this is an exact match (no conversion).
    pub fn is_exact(&self) -> bool {
        matches!(self.kind, ConversionKind::Identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn identity_conversion() {
        let conv = Conversion::identity();
        assert!(conv.is_exact());
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_EXACT);
    }

    #[test]
    fn primitive_widening() {
        let conv = Conversion::primitive_widening(primitives::INT32, primitives::INT64);
        assert!(!conv.is_exact());
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_WIDENING);
    }

    #[test]
    fn primitive_narrowing_is_explicit() {
        let conv = Conversion::primitive_narrowing(primitives::INT64, primitives::INT32);
        assert!(!conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_PRIMITIVE_NARROWING);
    }

    #[test]
    fn null_to_handle() {
        let conv = Conversion::null_to_handle();
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_EXACT);
    }

    #[test]
    fn derived_to_base() {
        let base = TypeHash::from_name("Base");
        let conv = Conversion::derived_to_base(base);
        assert!(conv.is_implicit());
        assert_eq!(conv.cost, Conversion::COST_DERIVED_TO_BASE);
        assert!(matches!(conv.kind, ConversionKind::DerivedToBase { base: b } if b == base));
    }
}
