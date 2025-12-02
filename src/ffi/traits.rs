//! Core traits for FFI type conversion.
//!
//! These traits define how Rust types convert to/from VM slots.
//! Type information comes from parsed declaration strings (AST primitives),
//! not from these traits.

use super::error::ConversionError;
use super::native_fn::VmSlot;

/// Convert from VM slot to Rust type (for extracting arguments).
///
/// Implement this trait for types that can be received from AngelScript.
///
/// # Example
///
/// ```ignore
/// impl FromScript for i32 {
///     fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
///         match slot {
///             VmSlot::Int(v) => Ok(*v as i32),
///             _ => Err(ConversionError::TypeMismatch {
///                 expected: "int",
///                 actual: slot.type_name(),
///             }),
///         }
///     }
/// }
/// ```
pub trait FromScript: Sized {
    /// Convert from a VM slot to this Rust type.
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError>;
}

/// Convert from Rust type to VM slot (for setting return values).
///
/// Implement this trait for types that can be returned to AngelScript.
///
/// # Example
///
/// ```ignore
/// impl ToScript for i32 {
///     fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
///         *slot = VmSlot::Int(self as i64);
///         Ok(())
///     }
/// }
/// ```
pub trait ToScript {
    /// Convert this Rust type to a VM slot.
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError>;
}

/// Marker trait for types that can be registered as native types.
///
/// Types implementing this trait can be used with `ClassBuilder`.
///
/// # Example
///
/// ```ignore
/// struct Vec3 { x: f32, y: f32, z: f32 }
///
/// impl NativeType for Vec3 {
///     const NAME: &'static str = "Vec3";
/// }
/// ```
pub trait NativeType: 'static + Send + Sync {
    /// The name of this type in AngelScript.
    const NAME: &'static str;
}

// =============================================================================
// FromScript implementations for primitive types
// =============================================================================

impl FromScript for () {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Void => Ok(()),
            _ => Err(ConversionError::TypeMismatch {
                expected: "void",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for bool {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Bool(v) => Ok(*v),
            _ => Err(ConversionError::TypeMismatch {
                expected: "bool",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for i8 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => i8::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "int8",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "int8",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for i16 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => i16::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "int16",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "int16",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for i32 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => i32::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "int",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "int",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for i64 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => Ok(*v),
            _ => Err(ConversionError::TypeMismatch {
                expected: "int64",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for u8 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => u8::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "uint8",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "uint8",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for u16 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => u16::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "uint16",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "uint16",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for u32 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => u32::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "uint",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "uint",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for u64 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Int(v) => u64::try_from(*v).map_err(|_| ConversionError::IntegerOverflow {
                value: *v,
                target_type: "uint64",
            }),
            _ => Err(ConversionError::TypeMismatch {
                expected: "uint64",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for f32 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Float(v) => Ok(*v as f32),
            _ => Err(ConversionError::TypeMismatch {
                expected: "float",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for f64 {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::Float(v) => Ok(*v),
            _ => Err(ConversionError::TypeMismatch {
                expected: "double",
                actual: slot.type_name(),
            }),
        }
    }
}

impl FromScript for String {
    fn from_vm(slot: &VmSlot) -> Result<Self, ConversionError> {
        match slot {
            VmSlot::String(s) => Ok(s.clone()),
            _ => Err(ConversionError::TypeMismatch {
                expected: "string",
                actual: slot.type_name(),
            }),
        }
    }
}

// Note: &str cannot implement FromScript because it requires a lifetime
// that ties to the VmSlot. Instead, use String and borrow from it.

// =============================================================================
// ToScript implementations for primitive types
// =============================================================================

impl ToScript for () {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Void;
        Ok(())
    }
}

impl ToScript for bool {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Bool(self);
        Ok(())
    }
}

impl ToScript for i8 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for i16 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for i32 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for i64 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self);
        Ok(())
    }
}

impl ToScript for u8 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for u16 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for u32 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for u64 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        // Note: This may lose precision for values > i64::MAX
        *slot = VmSlot::Int(self as i64);
        Ok(())
    }
}

impl ToScript for f32 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Float(self as f64);
        Ok(())
    }
}

impl ToScript for f64 {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::Float(self);
        Ok(())
    }
}

impl ToScript for String {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::String(self);
        Ok(())
    }
}

impl ToScript for &str {
    fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
        *slot = VmSlot::String(self.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vm_void() {
        let slot = VmSlot::Void;
        assert!(<() as FromScript>::from_vm(&slot).is_ok());
        let _: () = <() as FromScript>::from_vm(&slot).unwrap();
    }

    #[test]
    fn from_vm_bool() {
        let slot = VmSlot::Bool(true);
        let result: bool = FromScript::from_vm(&slot).unwrap();
        assert!(result);
    }

    #[test]
    fn from_vm_i32() {
        let slot = VmSlot::Int(42);
        let result: i32 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn from_vm_i32_overflow() {
        let slot = VmSlot::Int(i64::MAX);
        let result: Result<i32, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_i64() {
        let slot = VmSlot::Int(i64::MAX);
        let result: i64 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn from_vm_u32() {
        let slot = VmSlot::Int(100);
        let result: u32 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn from_vm_u32_negative() {
        let slot = VmSlot::Int(-1);
        let result: Result<u32, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_f32() {
        let slot = VmSlot::Float(3.14);
        let result: f32 = FromScript::from_vm(&slot).unwrap();
        assert!((result - 3.14).abs() < 0.001);
    }

    #[test]
    fn from_vm_f64() {
        let slot = VmSlot::Float(std::f64::consts::PI);
        let result: f64 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, std::f64::consts::PI);
    }

    #[test]
    fn from_vm_string() {
        let slot = VmSlot::String("hello".to_string());
        let result: String = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn from_vm_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<String, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn to_vm_void() {
        let mut slot = VmSlot::Int(42);
        ().to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Void));
    }

    #[test]
    fn to_vm_bool() {
        let mut slot = VmSlot::Void;
        true.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Bool(true)));
    }

    #[test]
    fn to_vm_i32() {
        let mut slot = VmSlot::Void;
        42i32.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(42)));
    }

    #[test]
    fn to_vm_f64() {
        let mut slot = VmSlot::Void;
        3.14f64.to_vm(&mut slot).unwrap();
        if let VmSlot::Float(v) = slot {
            assert_eq!(v, 3.14);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn to_vm_string() {
        let mut slot = VmSlot::Void;
        "hello".to_string().to_vm(&mut slot).unwrap();
        if let VmSlot::String(s) = slot {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn to_vm_str() {
        let mut slot = VmSlot::Void;
        "world".to_vm(&mut slot).unwrap();
        if let VmSlot::String(s) = slot {
            assert_eq!(s, "world");
        } else {
            panic!("Expected String");
        }
    }

    // Additional tests for remaining integer types
    #[test]
    fn from_vm_i8() {
        let slot = VmSlot::Int(42);
        let result: i8 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn from_vm_i8_overflow() {
        let slot = VmSlot::Int(200);
        let result: Result<i8, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_i16() {
        let slot = VmSlot::Int(1000);
        let result: i16 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn from_vm_i16_overflow() {
        let slot = VmSlot::Int(40000);
        let result: Result<i16, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_u8() {
        let slot = VmSlot::Int(200);
        let result: u8 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn from_vm_u8_overflow() {
        let slot = VmSlot::Int(300);
        let result: Result<u8, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_u16() {
        let slot = VmSlot::Int(50000);
        let result: u16 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 50000);
    }

    #[test]
    fn from_vm_u16_overflow() {
        let slot = VmSlot::Int(70000);
        let result: Result<u16, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_vm_u64() {
        let slot = VmSlot::Int(100);
        let result: u64 = FromScript::from_vm(&slot).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn from_vm_u64_negative() {
        let slot = VmSlot::Int(-1);
        let result: Result<u64, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    // ToScript tests for remaining types
    #[test]
    fn to_vm_i8() {
        let mut slot = VmSlot::Void;
        42i8.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(42)));
    }

    #[test]
    fn to_vm_i16() {
        let mut slot = VmSlot::Void;
        1000i16.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(1000)));
    }

    #[test]
    fn to_vm_i64() {
        let mut slot = VmSlot::Void;
        i64::MAX.to_vm(&mut slot).unwrap();
        if let VmSlot::Int(v) = slot {
            assert_eq!(v, i64::MAX);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn to_vm_u8() {
        let mut slot = VmSlot::Void;
        200u8.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(200)));
    }

    #[test]
    fn to_vm_u16() {
        let mut slot = VmSlot::Void;
        50000u16.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(50000)));
    }

    #[test]
    fn to_vm_u32() {
        let mut slot = VmSlot::Void;
        100000u32.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(100000)));
    }

    #[test]
    fn to_vm_u64() {
        let mut slot = VmSlot::Void;
        100u64.to_vm(&mut slot).unwrap();
        assert!(matches!(slot, VmSlot::Int(100)));
    }

    #[test]
    fn to_vm_f32() {
        let mut slot = VmSlot::Void;
        3.14f32.to_vm(&mut slot).unwrap();
        if let VmSlot::Float(v) = slot {
            assert!((v - 3.14).abs() < 0.01);
        } else {
            panic!("Expected Float");
        }
    }

    // Type mismatch tests for each type
    #[test]
    fn from_vm_bool_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<bool, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_void_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<(), _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_f32_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<f32, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_f64_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<f64, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_i8_type_mismatch() {
        let slot = VmSlot::String("hello".into());
        let result: Result<i8, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_i16_type_mismatch() {
        let slot = VmSlot::Bool(true);
        let result: Result<i16, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_i64_type_mismatch() {
        let slot = VmSlot::Float(3.14);
        let result: Result<i64, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_u8_type_mismatch() {
        let slot = VmSlot::Void;
        let result: Result<u8, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_u16_type_mismatch() {
        let slot = VmSlot::NullHandle;
        let result: Result<u16, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_vm_u64_type_mismatch() {
        let slot = VmSlot::String("test".into());
        let result: Result<u64, _> = FromScript::from_vm(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }
}
