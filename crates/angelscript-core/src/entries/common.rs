//! Common entry types shared across registry entries.
//!
//! This module provides smaller structures used within the main entry types
//! like `ClassEntry`, `EnumEntry`, etc.

use crate::{DataType, TypeHash, Visibility};

/// A property entry for virtual properties in classes.
///
/// Virtual properties are accessed like fields but are backed by
/// getter/setter methods (e.g., `get_length()`, `set_length()`).
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyEntry {
    /// Property name.
    pub name: String,
    /// Property type.
    pub data_type: DataType,
    /// Property visibility.
    pub visibility: Visibility,
    /// Getter function hash (const method returning the property type).
    pub getter: Option<TypeHash>,
    /// Setter function hash (method taking the property type).
    pub setter: Option<TypeHash>,
}

impl PropertyEntry {
    /// Create a new property entry.
    pub fn new(
        name: impl Into<String>,
        data_type: DataType,
        visibility: Visibility,
        getter: Option<TypeHash>,
        setter: Option<TypeHash>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility,
            getter,
            setter,
        }
    }

    /// Create a read-only property.
    pub fn read_only(
        name: impl Into<String>,
        data_type: DataType,
        getter: TypeHash,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility: Visibility::Public,
            getter: Some(getter),
            setter: None,
        }
    }

    /// Create a read-write property.
    pub fn read_write(
        name: impl Into<String>,
        data_type: DataType,
        getter: TypeHash,
        setter: TypeHash,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility: Visibility::Public,
            getter: Some(getter),
            setter: Some(setter),
        }
    }

    /// Check if this property is read-only.
    pub fn is_read_only(&self) -> bool {
        self.getter.is_some() && self.setter.is_none()
    }

    /// Check if this property is write-only.
    pub fn is_write_only(&self) -> bool {
        self.getter.is_none() && self.setter.is_some()
    }

    /// Check if this property is read-write.
    pub fn is_read_write(&self) -> bool {
        self.getter.is_some() && self.setter.is_some()
    }
}

/// An enum value entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValue {
    /// Value name.
    pub name: String,
    /// Integer value.
    pub value: i64,
}

impl EnumValue {
    /// Create a new enum value.
    pub fn new(name: impl Into<String>, value: i64) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn property_entry_read_only() {
        let getter = TypeHash::from_name("get_length");
        let prop = PropertyEntry::read_only("length", DataType::simple(primitives::INT32), getter);
        assert!(prop.is_read_only());
        assert!(!prop.is_write_only());
        assert!(!prop.is_read_write());
        assert_eq!(prop.getter, Some(getter));
        assert_eq!(prop.setter, None);
    }

    #[test]
    fn property_entry_read_write() {
        let getter = TypeHash::from_name("get_name");
        let setter = TypeHash::from_name("set_name");
        let prop = PropertyEntry::read_write(
            "name",
            DataType::simple(primitives::STRING),
            getter,
            setter,
        );
        assert!(prop.is_read_write());
        assert!(!prop.is_read_only());
        assert!(!prop.is_write_only());
    }

    #[test]
    fn enum_value_creation() {
        let value = EnumValue::new("Red", 0);
        assert_eq!(value.name, "Red");
        assert_eq!(value.value, 0);
    }
}
