//! Conversion traits for FFI argument extraction and return value handling.
//!
//! This module provides traits for converting between Rust types and VM slot values:
//! - [`FromSlot`]: Extract a Rust value from a [`VmSlot`]
//! - [`IntoSlot`]: Convert a Rust value into a [`VmSlot`]
//!
//! ## Supported Primitive Types
//!
//! - Integers: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`
//! - Floats: `f32`, `f64`
//! - Boolean: `bool`
//! - Unit: `()` (void)
//!
//! ## Example
//!
//! ```ignore
//! let slot = VmSlot::Int(42);
//! let value: i32 = i32::from_slot(&slot)?;
//! let back: VmSlot = value.into_slot();
//! ```

use crate::native_error::ConversionError;
use crate::native_fn::Dynamic;

/// Extract a value from a Dynamic slot.
///
/// This trait is implemented for all primitive types that can be extracted
/// from the VM's slot-based value system.
pub trait FromSlot: Sized {
    /// Extract a value from the given slot.
    ///
    /// Returns a `ConversionError` if the slot contains an incompatible type.
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError>;
}

/// Convert a value into a Dynamic slot.
///
/// This trait is implemented for all primitive types that can be stored
/// in the VM's slot-based value system.
pub trait IntoSlot {
    /// Convert this value into a Dynamic slot.
    fn into_slot(self) -> Dynamic;
}

// ============================================================================
// Integer implementations
// ============================================================================

macro_rules! impl_from_slot_int {
    ($($ty:ty),*) => {
        $(
            impl FromSlot for $ty {
                fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
                    match slot {
                        Dynamic::Int(v) => {
                            // Check bounds for narrowing conversions
                            if *v >= Self::MIN as i64 && *v <= Self::MAX as i64 {
                                Ok(*v as Self)
                            } else {
                                Err(ConversionError::IntegerOverflow {
                                    value: *v,
                                    target_type: stringify!($ty),
                                })
                            }
                        }
                        _ => Err(ConversionError::TypeMismatch {
                            expected: "int",
                            actual: slot.type_name(),
                        }),
                    }
                }
            }

            impl IntoSlot for $ty {
                fn into_slot(self) -> Dynamic {
                    Dynamic::Int(self as i64)
                }
            }
        )*
    };
}

impl_from_slot_int!(i8, i16, i32, i64);

// Unsigned integers need special handling for bounds checking
macro_rules! impl_from_slot_uint {
    ($($ty:ty),*) => {
        $(
            impl FromSlot for $ty {
                fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
                    match slot {
                        Dynamic::Int(v) => {
                            // For unsigned types, we need to handle negative values
                            if *v >= 0 && *v <= Self::MAX as i64 {
                                Ok(*v as Self)
                            } else {
                                Err(ConversionError::IntegerOverflow {
                                    value: *v,
                                    target_type: stringify!($ty),
                                })
                            }
                        }
                        _ => Err(ConversionError::TypeMismatch {
                            expected: "int",
                            actual: slot.type_name(),
                        }),
                    }
                }
            }

            impl IntoSlot for $ty {
                fn into_slot(self) -> Dynamic {
                    Dynamic::Int(self as i64)
                }
            }
        )*
    };
}

impl_from_slot_uint!(u8, u16, u32);

// u64 needs special handling - values above i64::MAX need special treatment
impl FromSlot for u64 {
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Int(v) => {
                // Reinterpret the bits - this allows full u64 range via i64
                Ok(*v as u64)
            }
            _ => Err(ConversionError::TypeMismatch {
                expected: "int",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoSlot for u64 {
    fn into_slot(self) -> Dynamic {
        // Reinterpret bits - this preserves full u64 range
        Dynamic::Int(self as i64)
    }
}

// ============================================================================
// Float implementations
// ============================================================================

impl FromSlot for f32 {
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Float(v) => {
                // Check if the value can be represented as f32
                if v.is_finite() && (*v <= f32::MAX as f64) && (*v >= f32::MIN as f64) {
                    Ok(*v as f32)
                } else if !v.is_finite() {
                    // Preserve infinities and NaN
                    Ok(*v as f32)
                } else {
                    Err(ConversionError::FloatConversion {
                        value: *v,
                        target_type: "f32",
                    })
                }
            }
            Dynamic::Int(v) => Ok(*v as f32),
            _ => Err(ConversionError::TypeMismatch {
                expected: "float",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoSlot for f32 {
    fn into_slot(self) -> Dynamic {
        Dynamic::Float(self as f64)
    }
}

impl FromSlot for f64 {
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Float(v) => Ok(*v),
            Dynamic::Int(v) => Ok(*v as f64),
            _ => Err(ConversionError::TypeMismatch {
                expected: "float",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoSlot for f64 {
    fn into_slot(self) -> Dynamic {
        Dynamic::Float(self)
    }
}

// ============================================================================
// Bool implementation
// ============================================================================

impl FromSlot for bool {
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Bool(v) => Ok(*v),
            _ => Err(ConversionError::TypeMismatch {
                expected: "bool",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoSlot for bool {
    fn into_slot(self) -> Dynamic {
        Dynamic::Bool(self)
    }
}

// ============================================================================
// Unit (void) implementation
// ============================================================================

impl FromSlot for () {
    fn from_slot(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Void => Ok(()),
            _ => Err(ConversionError::TypeMismatch {
                expected: "void",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoSlot for () {
    fn into_slot(self) -> Dynamic {
        Dynamic::Void
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // FromSlot tests
    // ========================================================================

    #[test]
    fn from_slot_i8() {
        assert_eq!(i8::from_slot(&Dynamic::Int(42)).unwrap(), 42i8);
        assert_eq!(i8::from_slot(&Dynamic::Int(-128)).unwrap(), -128i8);
        assert_eq!(i8::from_slot(&Dynamic::Int(127)).unwrap(), 127i8);
        assert!(i8::from_slot(&Dynamic::Int(128)).is_err());
        assert!(i8::from_slot(&Dynamic::Int(-129)).is_err());
        assert!(i8::from_slot(&Dynamic::Bool(true)).is_err());
    }

    #[test]
    fn from_slot_i16() {
        assert_eq!(i16::from_slot(&Dynamic::Int(1000)).unwrap(), 1000i16);
        assert!(i16::from_slot(&Dynamic::Int(40000)).is_err());
    }

    #[test]
    fn from_slot_i32() {
        assert_eq!(i32::from_slot(&Dynamic::Int(100000)).unwrap(), 100000i32);
        assert!(i32::from_slot(&Dynamic::Int(i64::MAX)).is_err());
    }

    #[test]
    fn from_slot_i64() {
        assert_eq!(i64::from_slot(&Dynamic::Int(i64::MAX)).unwrap(), i64::MAX);
        assert_eq!(i64::from_slot(&Dynamic::Int(i64::MIN)).unwrap(), i64::MIN);
    }

    #[test]
    fn from_slot_u8() {
        assert_eq!(u8::from_slot(&Dynamic::Int(255)).unwrap(), 255u8);
        assert!(u8::from_slot(&Dynamic::Int(-1)).is_err());
        assert!(u8::from_slot(&Dynamic::Int(256)).is_err());
    }

    #[test]
    fn from_slot_u16() {
        assert_eq!(u16::from_slot(&Dynamic::Int(65535)).unwrap(), 65535u16);
        assert!(u16::from_slot(&Dynamic::Int(-1)).is_err());
    }

    #[test]
    fn from_slot_u32() {
        assert_eq!(u32::from_slot(&Dynamic::Int(4294967295)).unwrap(), u32::MAX);
        assert!(u32::from_slot(&Dynamic::Int(-1)).is_err());
    }

    #[test]
    fn from_slot_u64() {
        // u64 uses bit reinterpretation for full range
        assert_eq!(u64::from_slot(&Dynamic::Int(0)).unwrap(), 0u64);
        // -1 as i64 becomes u64::MAX when reinterpreted
        assert_eq!(u64::from_slot(&Dynamic::Int(-1)).unwrap(), u64::MAX);
    }

    #[test]
    fn from_slot_f32() {
        assert_eq!(f32::from_slot(&Dynamic::Float(3.14)).unwrap(), 3.14f32);
        assert_eq!(f32::from_slot(&Dynamic::Int(42)).unwrap(), 42.0f32);
        assert!(
            f32::from_slot(&Dynamic::Float(f64::INFINITY))
                .unwrap()
                .is_infinite()
        );
        assert!(f32::from_slot(&Dynamic::Bool(true)).is_err());
    }

    #[test]
    fn from_slot_f64() {
        assert_eq!(
            f64::from_slot(&Dynamic::Float(3.14159265358979)).unwrap(),
            3.14159265358979f64
        );
        assert_eq!(f64::from_slot(&Dynamic::Int(42)).unwrap(), 42.0f64);
    }

    #[test]
    fn from_slot_bool() {
        assert_eq!(bool::from_slot(&Dynamic::Bool(true)).unwrap(), true);
        assert_eq!(bool::from_slot(&Dynamic::Bool(false)).unwrap(), false);
        assert!(bool::from_slot(&Dynamic::Int(1)).is_err());
    }

    #[test]
    fn from_slot_unit() {
        assert_eq!(<()>::from_slot(&Dynamic::Void).unwrap(), ());
        assert!(<()>::from_slot(&Dynamic::Int(0)).is_err());
    }

    // ========================================================================
    // IntoSlot tests
    // ========================================================================

    #[test]
    fn into_slot_i8() {
        assert!(matches!(42i8.into_slot(), Dynamic::Int(42)));
        assert!(matches!((-128i8).into_slot(), Dynamic::Int(-128)));
    }

    #[test]
    fn into_slot_i16() {
        assert!(matches!(1000i16.into_slot(), Dynamic::Int(1000)));
    }

    #[test]
    fn into_slot_i32() {
        assert!(matches!(100000i32.into_slot(), Dynamic::Int(100000)));
    }

    #[test]
    fn into_slot_i64() {
        assert!(matches!(i64::MAX.into_slot(), Dynamic::Int(i64::MAX)));
    }

    #[test]
    fn into_slot_u8() {
        assert!(matches!(255u8.into_slot(), Dynamic::Int(255)));
    }

    #[test]
    fn into_slot_u16() {
        assert!(matches!(65535u16.into_slot(), Dynamic::Int(65535)));
    }

    #[test]
    fn into_slot_u32() {
        assert!(matches!(u32::MAX.into_slot(), Dynamic::Int(4294967295)));
    }

    #[test]
    fn into_slot_u64() {
        // u64::MAX becomes -1 when stored as i64
        let slot = u64::MAX.into_slot();
        assert!(matches!(slot, Dynamic::Int(-1)));
    }

    #[test]
    fn into_slot_f32() {
        match 3.14f32.into_slot() {
            Dynamic::Float(v) => assert!((v - 3.14).abs() < 0.001),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn into_slot_f64() {
        match 3.14159265358979f64.into_slot() {
            Dynamic::Float(v) => assert!((v - 3.14159265358979).abs() < 1e-10),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn into_slot_bool() {
        assert!(matches!(true.into_slot(), Dynamic::Bool(true)));
        assert!(matches!(false.into_slot(), Dynamic::Bool(false)));
    }

    #[test]
    fn into_slot_unit() {
        assert!(matches!(().into_slot(), Dynamic::Void));
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    #[test]
    fn roundtrip_i32() {
        let original = 42i32;
        let slot = original.into_slot();
        let recovered = i32::from_slot(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_f64() {
        let original = 3.14159265358979f64;
        let slot = original.into_slot();
        let recovered = f64::from_slot(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_bool() {
        let original = true;
        let slot = original.into_slot();
        let recovered = bool::from_slot(&slot).unwrap();
        assert_eq!(original, recovered);
    }
}
