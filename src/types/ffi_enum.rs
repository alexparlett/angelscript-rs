//! Owned enum definitions for FFI registry.
//!
//! This module provides `FfiEnumDef`, an owned enum definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.

use crate::semantic::types::type_def::TypeId;

/// A native enum type definition.
///
/// This is an owned enum definition that can be stored in `Arc<FfiRegistry>`
/// without arena lifetimes.
///
/// # Example
///
/// ```ignore
/// let enum_def = FfiEnumDef::new(
///     TypeId::next_ffi(),
///     "Color",
///     vec![
///         ("Red".to_string(), 0),
///         ("Green".to_string(), 1),
///         ("Blue".to_string(), 2),
///     ],
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FfiEnumDef {
    /// Unique FFI type ID (assigned at registration via TypeId::next_ffi())
    pub id: TypeId,

    /// Enum name
    pub name: String,

    /// Enum values (name -> value)
    pub values: Vec<(String, i64)>,
}

impl FfiEnumDef {
    /// Create a new enum definition.
    pub fn new(id: TypeId, name: impl Into<String>, values: Vec<(String, i64)>) -> Self {
        Self {
            id,
            name: name.into(),
            values,
        }
    }

    /// Get the enum name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the enum values.
    pub fn values(&self) -> &[(String, i64)] {
        &self.values
    }

    /// Get the number of values.
    pub fn value_count(&self) -> usize {
        self.values.len()
    }

    /// Look up a value by name.
    pub fn get_value(&self, name: &str) -> Option<i64> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| *v)
    }

    /// Look up a name by value.
    pub fn get_name(&self, value: i64) -> Option<&str> {
        self.values
            .iter()
            .find(|(_, v)| *v == value)
            .map(|(n, _)| n.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_creation() {
        let enum_def = FfiEnumDef::new(
            TypeId::next_ffi(),
            "Color",
            vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        );

        assert_eq!(enum_def.name(), "Color");
        assert_eq!(enum_def.value_count(), 3);
    }

    #[test]
    fn get_value() {
        let enum_def = FfiEnumDef::new(
            TypeId::next_ffi(),
            "Color",
            vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        );

        assert_eq!(enum_def.get_value("Red"), Some(0));
        assert_eq!(enum_def.get_value("Green"), Some(1));
        assert_eq!(enum_def.get_value("Blue"), Some(2));
        assert_eq!(enum_def.get_value("Yellow"), None);
    }

    #[test]
    fn get_name() {
        let enum_def = FfiEnumDef::new(
            TypeId::next_ffi(),
            "Color",
            vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        );

        assert_eq!(enum_def.get_name(0), Some("Red"));
        assert_eq!(enum_def.get_name(1), Some("Green"));
        assert_eq!(enum_def.get_name(2), Some("Blue"));
        assert_eq!(enum_def.get_name(99), None);
    }

    #[test]
    fn negative_values() {
        let enum_def = FfiEnumDef::new(
            TypeId::next_ffi(),
            "ErrorCode",
            vec![
                ("Success".to_string(), 0),
                ("NotFound".to_string(), -1),
                ("AccessDenied".to_string(), -2),
            ],
        );

        assert_eq!(enum_def.get_value("NotFound"), Some(-1));
        assert_eq!(enum_def.get_name(-2), Some("AccessDenied"));
    }

    #[test]
    fn debug_output() {
        let enum_def = FfiEnumDef::new(TypeId::next_ffi(), "Test", vec![]);
        let debug = format!("{:?}", enum_def);
        assert!(debug.contains("FfiEnumDef"));
        assert!(debug.contains("Test"));
    }

    #[test]
    fn clone() {
        let original = FfiEnumDef::new(
            TypeId::next_ffi(),
            "Direction",
            vec![
                ("North".to_string(), 0),
                ("East".to_string(), 1),
            ],
        );

        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        assert_eq!(cloned.value_count(), original.value_count());
    }
}
