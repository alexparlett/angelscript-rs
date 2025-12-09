//! Enum type entry.
//!
//! This module provides `EnumEntry` for enumeration types.

use crate::TypeHash;

use super::{EnumValue, TypeSource};

/// Registry entry for an enumeration type.
///
/// Enums in AngelScript are integer-backed named constants.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumEntry {
    /// Unqualified name.
    pub name: String,
    /// Namespace path (e.g., `["Game", "Types"]`).
    pub namespace: Vec<String>,
    /// Fully qualified name (with namespace).
    pub qualified_name: String,
    /// Type hash for identity.
    pub type_hash: TypeHash,
    /// Source (FFI or script).
    pub source: TypeSource,
    /// Enum values.
    pub values: Vec<EnumValue>,
}

impl EnumEntry {
    /// Create a new enum entry.
    pub fn new(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        source: TypeSource,
    ) -> Self {
        Self {
            name: name.into(),
            namespace,
            qualified_name: qualified_name.into(),
            type_hash,
            source,
            values: Vec::new(),
        }
    }

    /// Create an FFI enum entry in the global namespace.
    pub fn ffi(name: impl Into<String>) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        Self {
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name,
            type_hash,
            source: TypeSource::ffi_untyped(),
            values: Vec::new(),
        }
    }

    /// Add a value to the enum.
    pub fn with_value(mut self, name: impl Into<String>, value: i64) -> Self {
        self.values.push(EnumValue::new(name, value));
        self
    }

    /// Add multiple values to the enum.
    pub fn with_values(mut self, values: impl IntoIterator<Item = (String, i64)>) -> Self {
        for (name, value) in values {
            self.values.push(EnumValue::new(name, value));
        }
        self
    }

    /// Look up a value by name.
    pub fn get_value(&self, name: &str) -> Option<i64> {
        self.values.iter().find(|v| v.name == name).map(|v| v.value)
    }

    /// Look up a name by value.
    pub fn get_name(&self, value: i64) -> Option<&str> {
        self.values
            .iter()
            .find(|v| v.value == value)
            .map(|v| v.name.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_entry_creation() {
        let entry = EnumEntry::ffi("Color")
            .with_value("Red", 0)
            .with_value("Green", 1)
            .with_value("Blue", 2);

        assert_eq!(entry.name, "Color");
        assert_eq!(entry.qualified_name, "Color");
        assert_eq!(entry.values.len(), 3);
        assert!(entry.source.is_ffi());
    }

    #[test]
    fn enum_entry_get_value() {
        let entry = EnumEntry::ffi("Status")
            .with_value("Pending", 0)
            .with_value("Active", 1)
            .with_value("Complete", 2);

        assert_eq!(entry.get_value("Pending"), Some(0));
        assert_eq!(entry.get_value("Active"), Some(1));
        assert_eq!(entry.get_value("Unknown"), None);
    }

    #[test]
    fn enum_entry_get_name() {
        let entry = EnumEntry::ffi("Direction")
            .with_value("North", 0)
            .with_value("South", 1);

        assert_eq!(entry.get_name(0), Some("North"));
        assert_eq!(entry.get_name(1), Some("South"));
        assert_eq!(entry.get_name(99), None);
    }

    #[test]
    fn enum_entry_with_values() {
        let values = vec![
            ("A".to_string(), 1),
            ("B".to_string(), 2),
            ("C".to_string(), 3),
        ];
        let entry = EnumEntry::ffi("Letters").with_values(values);

        assert_eq!(entry.values.len(), 3);
        assert_eq!(entry.get_value("B"), Some(2));
    }
}
