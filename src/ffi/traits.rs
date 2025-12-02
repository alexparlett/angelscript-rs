//! Core traits for FFI type conversion.
//!
//! These traits define how Rust types map to AngelScript types for
//! function parameters and return values.

use super::error::ConversionError;
use super::native_fn::VmSlot;
use super::types::TypeSpec;
use crate::semantic::types::data_type::RefModifier;

/// Maps Rust types to AngelScript types for function parameters.
///
/// Implement this trait for types that can be received from AngelScript.
///
/// # Example
///
/// ```ignore
/// impl FromScript for i32 {
///     fn script_type() -> TypeSpec {
///         TypeSpec::simple("int")
///     }
///
///     fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    /// The AngelScript type specification for this Rust type.
    fn script_type() -> TypeSpec;

    /// Convert from a VM slot to this Rust type.
    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError>;
}

/// Maps Rust types to AngelScript types for function return values.
///
/// Implement this trait for types that can be returned to AngelScript.
///
/// # Example
///
/// ```ignore
/// impl ToScript for i32 {
///     fn script_type() -> TypeSpec {
///         TypeSpec::simple("int")
///     }
///
///     fn to_slot(self) -> Result<VmSlot, ConversionError> {
///         Ok(VmSlot::Int(self as i64))
///     }
/// }
/// ```
pub trait ToScript {
    /// The AngelScript type specification for this Rust type.
    fn script_type() -> TypeSpec;

    /// Convert this Rust type to a VM slot.
    fn to_slot(self) -> Result<VmSlot, ConversionError>;
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
    fn script_type() -> TypeSpec {
        TypeSpec::void()
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("bool")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int8")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int16")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int64")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint8")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint16")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint64")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("float")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("double")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
    fn script_type() -> TypeSpec {
        TypeSpec::simple("string")
    }

    fn from_slot(slot: &VmSlot) -> Result<Self, ConversionError> {
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
// that ties to the VmSlot. Instead, we provide a special case for &str
// parameters that uses `const string &in` semantics.

// =============================================================================
// ToScript implementations for primitive types
// =============================================================================

impl ToScript for () {
    fn script_type() -> TypeSpec {
        TypeSpec::void()
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Void)
    }
}

impl ToScript for bool {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("bool")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Bool(self))
    }
}

impl ToScript for i8 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int8")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for i16 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int16")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for i32 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for i64 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("int64")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self))
    }
}

impl ToScript for u8 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint8")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for u16 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint16")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for u32 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for u64 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("uint64")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        // Note: This may lose precision for values > i64::MAX
        Ok(VmSlot::Int(self as i64))
    }
}

impl ToScript for f32 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("float")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Float(self as f64))
    }
}

impl ToScript for f64 {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("double")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::Float(self))
    }
}

impl ToScript for String {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("string")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::String(self))
    }
}

impl ToScript for &str {
    fn script_type() -> TypeSpec {
        TypeSpec::simple("string")
    }

    fn to_slot(self) -> Result<VmSlot, ConversionError> {
        Ok(VmSlot::String(self.to_string()))
    }
}

// =============================================================================
// Special case: &str as parameter (const string &in)
// =============================================================================

/// Helper struct for `&str` parameter type info.
/// This is used internally by the function builder to handle `&str` parameters
/// which map to `const string &in` in AngelScript.
pub struct StrRef;

impl StrRef {
    /// Get the TypeSpec for `&str` parameters (maps to `const string &in`).
    pub fn script_type() -> TypeSpec {
        TypeSpec::new("string").with_const().with_ref(RefModifier::In)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_script_void() {
        let slot = VmSlot::Void;
        assert!(<() as FromScript>::from_slot(&slot).is_ok());
        let _: () = <() as FromScript>::from_slot(&slot).unwrap();
    }

    #[test]
    fn from_script_bool() {
        let slot = VmSlot::Bool(true);
        let result: bool = FromScript::from_slot(&slot).unwrap();
        assert!(result);
    }

    #[test]
    fn from_script_i32() {
        let slot = VmSlot::Int(42);
        let result: i32 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn from_script_i32_overflow() {
        let slot = VmSlot::Int(i64::MAX);
        let result: Result<i32, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_i64() {
        let slot = VmSlot::Int(i64::MAX);
        let result: i64 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn from_script_u32() {
        let slot = VmSlot::Int(100);
        let result: u32 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn from_script_u32_negative() {
        let slot = VmSlot::Int(-1);
        let result: Result<u32, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_f32() {
        let slot = VmSlot::Float(3.14);
        let result: f32 = FromScript::from_slot(&slot).unwrap();
        assert!((result - 3.14).abs() < 0.001);
    }

    #[test]
    fn from_script_f64() {
        let slot = VmSlot::Float(std::f64::consts::PI);
        let result: f64 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, std::f64::consts::PI);
    }

    #[test]
    fn from_script_string() {
        let slot = VmSlot::String("hello".to_string());
        let result: String = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn from_script_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<String, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn to_script_void() {
        let slot = ().to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Void));
    }

    #[test]
    fn to_script_bool() {
        let slot = true.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Bool(true)));
    }

    #[test]
    fn to_script_i32() {
        let slot = 42i32.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(42)));
    }

    #[test]
    fn to_script_f64() {
        let slot = 3.14f64.to_slot().unwrap();
        if let VmSlot::Float(v) = slot {
            assert_eq!(v, 3.14);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn to_script_string() {
        let slot = "hello".to_string().to_slot().unwrap();
        if let VmSlot::String(s) = slot {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn to_script_str() {
        let slot = "world".to_slot().unwrap();
        if let VmSlot::String(s) = slot {
            assert_eq!(s, "world");
        } else {
            panic!("Expected String");
        }
    }

    #[test]
    fn script_type_void() {
        let spec = <() as FromScript>::script_type();
        assert!(spec.is_void());
    }

    #[test]
    fn script_type_int() {
        let spec = <i32 as FromScript>::script_type();
        assert_eq!(spec.type_name, "int");
    }

    #[test]
    fn script_type_str_ref() {
        let spec = StrRef::script_type();
        assert_eq!(spec.type_name, "string");
        assert!(spec.is_const);
        assert_eq!(spec.ref_modifier, RefModifier::In);
    }

    // Additional tests for remaining integer types
    #[test]
    fn from_script_i8() {
        let slot = VmSlot::Int(42);
        let result: i8 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn from_script_i8_overflow() {
        let slot = VmSlot::Int(200);
        let result: Result<i8, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_i16() {
        let slot = VmSlot::Int(1000);
        let result: i16 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 1000);
    }

    #[test]
    fn from_script_i16_overflow() {
        let slot = VmSlot::Int(40000);
        let result: Result<i16, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_u8() {
        let slot = VmSlot::Int(200);
        let result: u8 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn from_script_u8_overflow() {
        let slot = VmSlot::Int(300);
        let result: Result<u8, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_u16() {
        let slot = VmSlot::Int(50000);
        let result: u16 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 50000);
    }

    #[test]
    fn from_script_u16_overflow() {
        let slot = VmSlot::Int(70000);
        let result: Result<u16, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    #[test]
    fn from_script_u64() {
        let slot = VmSlot::Int(100);
        let result: u64 = FromScript::from_slot(&slot).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn from_script_u64_negative() {
        let slot = VmSlot::Int(-1);
        let result: Result<u64, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::IntegerOverflow { .. })));
    }

    // ToScript tests for remaining types
    #[test]
    fn to_script_i8() {
        let slot = 42i8.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(42)));
    }

    #[test]
    fn to_script_i16() {
        let slot = 1000i16.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(1000)));
    }

    #[test]
    fn to_script_i64() {
        let slot = i64::MAX.to_slot().unwrap();
        if let VmSlot::Int(v) = slot {
            assert_eq!(v, i64::MAX);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn to_script_u8() {
        let slot = 200u8.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(200)));
    }

    #[test]
    fn to_script_u16() {
        let slot = 50000u16.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(50000)));
    }

    #[test]
    fn to_script_u32() {
        let slot = 100000u32.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(100000)));
    }

    #[test]
    fn to_script_u64() {
        let slot = 100u64.to_slot().unwrap();
        assert!(matches!(slot, VmSlot::Int(100)));
    }

    #[test]
    fn to_script_f32() {
        let slot = 3.14f32.to_slot().unwrap();
        if let VmSlot::Float(v) = slot {
            assert!((v - 3.14).abs() < 0.01);
        } else {
            panic!("Expected Float");
        }
    }

    // Script type tests for remaining types
    #[test]
    fn script_type_i8() {
        let spec = <i8 as FromScript>::script_type();
        assert_eq!(spec.type_name, "int8");
    }

    #[test]
    fn script_type_i16() {
        let spec = <i16 as FromScript>::script_type();
        assert_eq!(spec.type_name, "int16");
    }

    #[test]
    fn script_type_i64() {
        let spec = <i64 as FromScript>::script_type();
        assert_eq!(spec.type_name, "int64");
    }

    #[test]
    fn script_type_u8() {
        let spec = <u8 as FromScript>::script_type();
        assert_eq!(spec.type_name, "uint8");
    }

    #[test]
    fn script_type_u16() {
        let spec = <u16 as FromScript>::script_type();
        assert_eq!(spec.type_name, "uint16");
    }

    #[test]
    fn script_type_u32() {
        let spec = <u32 as FromScript>::script_type();
        assert_eq!(spec.type_name, "uint");
    }

    #[test]
    fn script_type_u64() {
        let spec = <u64 as FromScript>::script_type();
        assert_eq!(spec.type_name, "uint64");
    }

    #[test]
    fn script_type_f32() {
        let spec = <f32 as FromScript>::script_type();
        assert_eq!(spec.type_name, "float");
    }

    #[test]
    fn script_type_f64() {
        let spec = <f64 as FromScript>::script_type();
        assert_eq!(spec.type_name, "double");
    }

    #[test]
    fn script_type_bool() {
        let spec = <bool as FromScript>::script_type();
        assert_eq!(spec.type_name, "bool");
    }

    #[test]
    fn script_type_string() {
        let spec = <String as FromScript>::script_type();
        assert_eq!(spec.type_name, "string");
    }

    // Type mismatch tests for each type
    #[test]
    fn from_script_bool_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<bool, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_void_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<(), _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_f32_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<f32, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_f64_type_mismatch() {
        let slot = VmSlot::Int(42);
        let result: Result<f64, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_i8_type_mismatch() {
        let slot = VmSlot::String("hello".into());
        let result: Result<i8, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_i16_type_mismatch() {
        let slot = VmSlot::Bool(true);
        let result: Result<i16, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_i64_type_mismatch() {
        let slot = VmSlot::Float(3.14);
        let result: Result<i64, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_u8_type_mismatch() {
        let slot = VmSlot::Void;
        let result: Result<u8, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_u16_type_mismatch() {
        let slot = VmSlot::NullHandle;
        let result: Result<u16, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }

    #[test]
    fn from_script_u64_type_mismatch() {
        let slot = VmSlot::String("test".into());
        let result: Result<u64, _> = FromScript::from_slot(&slot);
        assert!(matches!(result, Err(ConversionError::TypeMismatch { .. })));
    }
}
