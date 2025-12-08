//! Owned property definitions for FFI registry.
//!
//! This module provides `FfiPropertyDef`, an owned property definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.

use crate::NativeFn;
use angelscript_core::DataType;

/// A property definition with getter and optional setter.
///
/// This is an owned property definition that can be stored in `Arc<FfiRegistry>`
/// without arena lifetimes.
#[derive(Debug)]
pub struct FfiPropertyDef {
    /// Property name
    pub name: String,

    /// Property type (always resolved)
    pub data_type: DataType,

    /// Whether this property is read-only
    pub is_const: bool,

    /// Getter function
    pub getter: NativeFn,

    /// Setter function (if writable)
    pub setter: Option<NativeFn>,
}

impl FfiPropertyDef {
    /// Create a new read-only property.
    pub fn read_only(name: impl Into<String>, data_type: DataType, getter: NativeFn) -> Self {
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
        data_type: DataType,
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
    use crate::CallContext;
    use angelscript_core::primitives as primitive_hashes;

    fn dummy_native_fn() -> NativeFn {
        NativeFn::new(|_ctx: &mut CallContext| Ok(()))
    }

    #[test]
    fn read_only_property() {
        let prop = FfiPropertyDef::read_only(
            "value",
            DataType::simple(primitive_hashes::INT32),
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
            DataType::simple(primitive_hashes::INT32),
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
            DataType::simple(primitive_hashes::INT32),
            dummy_native_fn(),
        );
        let debug = format!("{:?}", prop);
        assert!(debug.contains("FfiPropertyDef"));
        assert!(debug.contains("test"));
    }
}
