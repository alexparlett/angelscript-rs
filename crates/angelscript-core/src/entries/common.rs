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

/// A field entry for class member variables.
///
/// Fields are direct data members in a class, unlike properties which
/// are backed by accessor methods.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEntry {
    /// Field name.
    pub name: String,
    /// Field type.
    pub data_type: DataType,
    /// Field visibility.
    pub visibility: Visibility,
    /// Byte offset within the object (for native types).
    pub offset: usize,
}

impl FieldEntry {
    /// Create a new field entry.
    pub fn new(
        name: impl Into<String>,
        data_type: DataType,
        visibility: Visibility,
        offset: usize,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility,
            offset,
        }
    }

    /// Create a public field at offset 0.
    pub fn public(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility: Visibility::Public,
            offset: 0,
        }
    }

    /// Create a private field at offset 0.
    pub fn private(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility: Visibility::Private,
            offset: 0,
        }
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
    fn field_entry_public() {
        let field = FieldEntry::public("health", DataType::simple(primitives::INT32));
        assert_eq!(field.name, "health");
        assert_eq!(field.visibility, Visibility::Public);
        assert_eq!(field.offset, 0);
    }

    #[test]
    fn field_entry_private() {
        let field = FieldEntry::private("secret", DataType::simple(primitives::INT32));
        assert_eq!(field.visibility, Visibility::Private);
    }

    #[test]
    fn field_entry_with_offset() {
        let field = FieldEntry::new(
            "value",
            DataType::simple(primitives::FLOAT),
            Visibility::Public,
            8,
        );
        assert_eq!(field.offset, 8);
    }

    #[test]
    fn enum_value_creation() {
        let value = EnumValue::new("Red", 0);
        assert_eq!(value.name, "Red");
        assert_eq!(value.value, 0);
    }
}
