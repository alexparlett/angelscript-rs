//! Conversion traits for FFI argument extraction and return value handling.
//!
//! This module provides traits for converting between Rust types and [`Dynamic`] values:
//! - [`FromDynamic`]: Extract a Rust value from a [`Dynamic`]
//! - [`IntoDynamic`]: Convert a Rust value into a [`Dynamic`]
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
//! let slot = Dynamic::Int(42);
//! let value: i32 = i32::from_dynamic(&slot)?;
//! let back: Dynamic = value.into_dynamic();
//! ```

use crate::native_error::ConversionError;
use crate::runtime::Dynamic;

/// Extract a value from a [`Dynamic`].
///
/// This trait is implemented for all primitive types that can be extracted
/// from the VM's dynamic value system.
pub trait FromDynamic: Sized {
    /// Extract a value from the given dynamic slot.
    ///
    /// Returns a `ConversionError` if the slot contains an incompatible type.
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError>;
}

/// Convert a value into a [`Dynamic`].
///
/// This trait is implemented for all primitive types that can be stored
/// in the VM's dynamic value system.
pub trait IntoDynamic {
    /// Convert this value into a Dynamic.
    fn into_dynamic(self) -> Dynamic;
}

// ============================================================================
// Integer implementations
// ============================================================================

macro_rules! impl_from_dynamic_int {
    ($($ty:ty),*) => {
        $(
            impl FromDynamic for $ty {
                fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
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

            impl IntoDynamic for $ty {
                fn into_dynamic(self) -> Dynamic {
                    Dynamic::Int(self as i64)
                }
            }
        )*
    };
}

impl_from_dynamic_int!(i8, i16, i32, i64);

// Unsigned integers need special handling for bounds checking
macro_rules! impl_from_dynamic_uint {
    ($($ty:ty),*) => {
        $(
            impl FromDynamic for $ty {
                fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
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

            impl IntoDynamic for $ty {
                fn into_dynamic(self) -> Dynamic {
                    Dynamic::Int(self as i64)
                }
            }
        )*
    };
}

impl_from_dynamic_uint!(u8, u16, u32);

// u64 needs special handling - values above i64::MAX need special treatment
impl FromDynamic for u64 {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
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

impl IntoDynamic for u64 {
    fn into_dynamic(self) -> Dynamic {
        // Reinterpret bits - this preserves full u64 range
        Dynamic::Int(self as i64)
    }
}

// ============================================================================
// Float implementations
// ============================================================================

impl FromDynamic for f32 {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
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

impl IntoDynamic for f32 {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::Float(self as f64)
    }
}

impl FromDynamic for f64 {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
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

impl IntoDynamic for f64 {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::Float(self)
    }
}

// ============================================================================
// Bool implementation
// ============================================================================

impl FromDynamic for bool {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Bool(v) => Ok(*v),
            _ => Err(ConversionError::TypeMismatch {
                expected: "bool",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoDynamic for bool {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::Bool(self)
    }
}

// ============================================================================
// Unit (void) implementation
// ============================================================================

impl FromDynamic for () {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::Void => Ok(()),
            _ => Err(ConversionError::TypeMismatch {
                expected: "void",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoDynamic for () {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::Void
    }
}

// ============================================================================
// String implementation
// ============================================================================

impl FromDynamic for String {
    fn from_dynamic(slot: &Dynamic) -> Result<Self, ConversionError> {
        match slot {
            Dynamic::String(s) => Ok(s.clone()),
            _ => Err(ConversionError::TypeMismatch {
                expected: "string",
                actual: slot.type_name(),
            }),
        }
    }
}

impl IntoDynamic for String {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::String(self)
    }
}

// Also support &str for convenience (returns owned String)
impl IntoDynamic for &str {
    fn into_dynamic(self) -> Dynamic {
        Dynamic::String(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // FromDynamic tests
    // ========================================================================

    #[test]
    fn from_dynamic_i8() {
        use crate::ConversionError;

        assert_eq!(i8::from_dynamic(&Dynamic::Int(42)).unwrap(), 42i8);
        assert_eq!(i8::from_dynamic(&Dynamic::Int(-128)).unwrap(), -128i8);
        assert_eq!(i8::from_dynamic(&Dynamic::Int(127)).unwrap(), 127i8);

        // Overflow checks
        assert!(matches!(
            i8::from_dynamic(&Dynamic::Int(128)),
            Err(ConversionError::IntegerOverflow { value: 128, .. })
        ));
        assert!(matches!(
            i8::from_dynamic(&Dynamic::Int(-129)),
            Err(ConversionError::IntegerOverflow { value: -129, .. })
        ));

        // Type mismatch
        assert!(matches!(
            i8::from_dynamic(&Dynamic::Bool(true)),
            Err(ConversionError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn from_dynamic_i16() {
        use crate::ConversionError;

        assert_eq!(i16::from_dynamic(&Dynamic::Int(1000)).unwrap(), 1000i16);
        assert!(matches!(
            i16::from_dynamic(&Dynamic::Int(40000)),
            Err(ConversionError::IntegerOverflow { value: 40000, .. })
        ));
    }

    #[test]
    fn from_dynamic_i32() {
        use crate::ConversionError;

        assert_eq!(i32::from_dynamic(&Dynamic::Int(100000)).unwrap(), 100000i32);
        assert!(matches!(
            i32::from_dynamic(&Dynamic::Int(i64::MAX)),
            Err(ConversionError::IntegerOverflow { .. })
        ));
    }

    #[test]
    fn from_dynamic_i64() {
        assert_eq!(
            i64::from_dynamic(&Dynamic::Int(i64::MAX)).unwrap(),
            i64::MAX
        );
        assert_eq!(
            i64::from_dynamic(&Dynamic::Int(i64::MIN)).unwrap(),
            i64::MIN
        );
    }

    #[test]
    fn from_dynamic_u8() {
        use crate::ConversionError;

        assert_eq!(u8::from_dynamic(&Dynamic::Int(255)).unwrap(), 255u8);
        assert!(matches!(
            u8::from_dynamic(&Dynamic::Int(-1)),
            Err(ConversionError::IntegerOverflow { value: -1, .. })
        ));
        assert!(matches!(
            u8::from_dynamic(&Dynamic::Int(256)),
            Err(ConversionError::IntegerOverflow { value: 256, .. })
        ));
    }

    #[test]
    fn from_dynamic_u16() {
        use crate::ConversionError;

        assert_eq!(u16::from_dynamic(&Dynamic::Int(65535)).unwrap(), 65535u16);
        assert!(matches!(
            u16::from_dynamic(&Dynamic::Int(-1)),
            Err(ConversionError::IntegerOverflow { value: -1, .. })
        ));
    }

    #[test]
    fn from_dynamic_u32() {
        use crate::ConversionError;

        assert_eq!(
            u32::from_dynamic(&Dynamic::Int(4294967295)).unwrap(),
            u32::MAX
        );
        assert!(matches!(
            u32::from_dynamic(&Dynamic::Int(-1)),
            Err(ConversionError::IntegerOverflow { value: -1, .. })
        ));
    }

    #[test]
    fn from_dynamic_u64() {
        // u64 uses bit reinterpretation for full range
        assert_eq!(u64::from_dynamic(&Dynamic::Int(0)).unwrap(), 0u64);
        // -1 as i64 becomes u64::MAX when reinterpreted
        assert_eq!(u64::from_dynamic(&Dynamic::Int(-1)).unwrap(), u64::MAX);
    }

    #[test]
    fn from_dynamic_f32() {
        use crate::ConversionError;

        assert_eq!(f32::from_dynamic(&Dynamic::Float(3.14)).unwrap(), 3.14f32);
        assert_eq!(f32::from_dynamic(&Dynamic::Int(42)).unwrap(), 42.0f32);
        assert!(
            f32::from_dynamic(&Dynamic::Float(f64::INFINITY))
                .unwrap()
                .is_infinite()
        );
        assert!(matches!(
            f32::from_dynamic(&Dynamic::Bool(true)),
            Err(ConversionError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn from_dynamic_f64() {
        assert_eq!(
            f64::from_dynamic(&Dynamic::Float(3.14159265358979)).unwrap(),
            3.14159265358979f64
        );
        assert_eq!(f64::from_dynamic(&Dynamic::Int(42)).unwrap(), 42.0f64);
    }

    #[test]
    fn from_dynamic_bool() {
        use crate::ConversionError;

        assert_eq!(bool::from_dynamic(&Dynamic::Bool(true)).unwrap(), true);
        assert_eq!(bool::from_dynamic(&Dynamic::Bool(false)).unwrap(), false);
        assert!(matches!(
            bool::from_dynamic(&Dynamic::Int(1)),
            Err(ConversionError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn from_dynamic_unit() {
        use crate::ConversionError;

        assert_eq!(<()>::from_dynamic(&Dynamic::Void).unwrap(), ());
        assert!(matches!(
            <()>::from_dynamic(&Dynamic::Int(0)),
            Err(ConversionError::TypeMismatch { .. })
        ));
    }

    // ========================================================================
    // IntoDynamic tests
    // ========================================================================

    #[test]
    fn into_dynamic_i8() {
        assert!(matches!(42i8.into_dynamic(), Dynamic::Int(42)));
        assert!(matches!((-128i8).into_dynamic(), Dynamic::Int(-128)));
    }

    #[test]
    fn into_dynamic_i16() {
        assert!(matches!(1000i16.into_dynamic(), Dynamic::Int(1000)));
    }

    #[test]
    fn into_dynamic_i32() {
        assert!(matches!(100000i32.into_dynamic(), Dynamic::Int(100000)));
    }

    #[test]
    fn into_dynamic_i64() {
        assert!(matches!(i64::MAX.into_dynamic(), Dynamic::Int(i64::MAX)));
    }

    #[test]
    fn into_dynamic_u8() {
        assert!(matches!(255u8.into_dynamic(), Dynamic::Int(255)));
    }

    #[test]
    fn into_dynamic_u16() {
        assert!(matches!(65535u16.into_dynamic(), Dynamic::Int(65535)));
    }

    #[test]
    fn into_dynamic_u32() {
        assert!(matches!(u32::MAX.into_dynamic(), Dynamic::Int(4294967295)));
    }

    #[test]
    fn into_dynamic_u64() {
        // u64::MAX becomes -1 when stored as i64
        let slot = u64::MAX.into_dynamic();
        assert!(matches!(slot, Dynamic::Int(-1)));
    }

    #[test]
    fn into_dynamic_f32() {
        match 3.14f32.into_dynamic() {
            Dynamic::Float(v) => assert!((v - 3.14).abs() < 0.001),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn into_dynamic_f64() {
        match 3.14159265358979f64.into_dynamic() {
            Dynamic::Float(v) => assert!((v - 3.14159265358979).abs() < 1e-10),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn into_dynamic_bool() {
        assert!(matches!(true.into_dynamic(), Dynamic::Bool(true)));
        assert!(matches!(false.into_dynamic(), Dynamic::Bool(false)));
    }

    #[test]
    fn into_dynamic_unit() {
        assert!(matches!(().into_dynamic(), Dynamic::Void));
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    #[test]
    fn roundtrip_i32() {
        let original = 42i32;
        let slot = original.into_dynamic();
        let recovered = i32::from_dynamic(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_f64() {
        let original = 3.14159265358979f64;
        let slot = original.into_dynamic();
        let recovered = f64::from_dynamic(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_bool() {
        let original = true;
        let slot = original.into_dynamic();
        let recovered = bool::from_dynamic(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn roundtrip_string() {
        let original = "hello world".to_string();
        let slot = original.clone().into_dynamic();
        let recovered = String::from_dynamic(&slot).unwrap();
        assert_eq!(original, recovered);
    }

    #[test]
    fn from_dynamic_string() {
        use crate::ConversionError;

        assert_eq!(
            String::from_dynamic(&Dynamic::String("test".into())).unwrap(),
            "test"
        );
        assert!(matches!(
            String::from_dynamic(&Dynamic::Int(42)),
            Err(ConversionError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn into_dynamic_string() {
        match "hello".to_string().into_dynamic() {
            Dynamic::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn into_dynamic_str() {
        match "hello".into_dynamic() {
            Dynamic::String(s) => assert_eq!(s, "hello"),
            _ => panic!("expected String"),
        }
    }
}
