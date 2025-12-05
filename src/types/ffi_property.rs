//! Owned property definitions for FFI registry.
//!
//! This module provides `FfiPropertyDef`, an owned property definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.

use crate::ffi::NativeFn;
use crate::types::FfiDataType;

/// A property definition with getter and optional setter.
///
/// This is the FFI equivalent of `NativePropertyDef<'ast>`, but fully owned
/// so it can be stored in `Arc<FfiRegistry>`.
#[derive(Debug)]
pub struct FfiPropertyDef {
    /// Property name
    pub name: String,

    /// Property type (may be unresolved during registration)
    pub data_type: FfiDataType,

    /// Whether this property is read-only
    pub is_const: bool,

    /// Getter function
    pub getter: NativeFn,

    /// Setter function (if writable)
    pub setter: Option<NativeFn>,
}

impl FfiPropertyDef {
    /// Create a new read-only property.
    pub fn read_only(name: impl Into<String>, data_type: FfiDataType, getter: NativeFn) -> Self {
        Self {
            name: name.into(),
            data_type,
            is_const: true,
            getter,
            setter: None,
        }
    }

    /// Create a new read-write property.
    pub fn read_write(
        name: impl Into<String>,
        data_type: FfiDataType,
        getter: NativeFn,
        setter: NativeFn,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            is_const: false,
            getter,
            setter: Some(setter),
        }
    }

    /// Check if this property is read-only.
    pub fn is_read_only(&self) -> bool {
        self.is_const || self.setter.is_none()
    }

    /// Check if this property is writable.
    pub fn is_writable(&self) -> bool {
        self.setter.is_some() && !self.is_const
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::CallContext;
    use crate::semantic::types::type_def::INT32_TYPE;
    use crate::semantic::types::DataType;

    fn dummy_native_fn() -> NativeFn {
        NativeFn::new(|_ctx: &mut CallContext| Ok(()))
    }

    #[test]
    fn read_only_property() {
        let prop = FfiPropertyDef::read_only(
            "value",
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            dummy_native_fn(),
        );

        assert_eq!(prop.name, "value");
        assert!(prop.is_const);
        assert!(prop.is_read_only());
        assert!(!prop.is_writable());
        assert!(prop.setter.is_none());
    }

    #[test]
    fn read_write_property() {
        let prop = FfiPropertyDef::read_write(
            "value",
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            dummy_native_fn(),
            dummy_native_fn(),
        );

        assert_eq!(prop.name, "value");
        assert!(!prop.is_const);
        assert!(!prop.is_read_only());
        assert!(prop.is_writable());
        assert!(prop.setter.is_some());
    }

    #[test]
    fn debug_output() {
        let prop = FfiPropertyDef::read_only(
            "test",
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            dummy_native_fn(),
        );
        let debug = format!("{:?}", prop);
        assert!(debug.contains("FfiPropertyDef"));
        assert!(debug.contains("test"));
    }
}
