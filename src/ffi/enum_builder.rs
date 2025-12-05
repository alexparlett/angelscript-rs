//! EnumBuilder for registering native enum types with the FFI system.
//!
//! EnumBuilder provides a fluent API for registering native enum types
//! with explicit or auto-incremented values.
//!
//! # Example
//!
//! ```ignore
//! // Basic enum with explicit values
//! module.register_enum("Color")
//!     .value("Red", 0)?
//!     .value("Green", 1)?
//!     .value("Blue", 2)?
//!     .build()?;
//!
//! // Auto-numbered enum
//! module.register_enum("Direction")
//!     .auto_value("North")?
//!     .auto_value("East")?
//!     .auto_value("South")?
//!     .auto_value("West")?
//!     .build()?;
//! ```

use crate::module::{FfiModuleError, Module};
use crate::semantic::types::type_def::TypeId;
use crate::types::FfiEnumDef;

/// Builder for registering native enum types.
///
/// Created by calling `Module::register_enum(name)`.
///
/// # Type Parameters
///
/// - `'m`: Lifetime of the mutable borrow of the Module
/// - `'app`: Application lifetime for global property references
#[derive(Debug)]
pub struct EnumBuilder<'m, 'app> {
    /// Reference to the module where the enum will be registered
    module: &'m mut Module<'app>,
    /// Enum name
    name: String,
    /// Enum values (name, value)
    values: Vec<(String, i64)>,
    /// Next auto-increment value
    next_value: i64,
}

impl<'m, 'app> EnumBuilder<'m, 'app> {
    /// Create a new EnumBuilder for the given enum name.
    ///
    /// This is called internally by `Module::register_enum()`.
    pub(crate) fn new(module: &'m mut Module<'app>, name: String) -> Self {
        Self {
            module,
            name,
            values: Vec::new(),
            next_value: 0,
        }
    }

    /// Add an enum value with an explicit integer value.
    ///
    /// The next auto-increment value will be set to `value + 1`.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the enum value (e.g., `"Red"`)
    /// - `value`: The integer value for this enum entry
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_enum("FileFlags")
    ///     .value("None", 0)?
    ///     .value("Read", 1)?
    ///     .value("Write", 2)?
    ///     .value("Execute", 4)?
    ///     .value("All", 7)?
    ///     .build()?;
    /// ```
    pub fn value(mut self, name: &str, value: i64) -> Result<Self, FfiModuleError> {
        // Check for duplicate value names
        if self.values.iter().any(|(n, _)| n == name) {
            return Err(FfiModuleError::DuplicateEnumValue {
                enum_name: self.name.clone(),
                value_name: name.to_string(),
            });
        }

        self.values.push((name.to_string(), value));
        self.next_value = value + 1;
        Ok(self)
    }

    /// Add an enum value with an auto-incremented value.
    ///
    /// The value will be the next value in the auto-increment sequence,
    /// starting from 0 or from the last explicit value + 1.
    ///
    /// # Parameters
    ///
    /// - `name`: The name of the enum value (e.g., `"North"`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Values: North=0, East=1, South=2, West=3
    /// module.register_enum("Direction")
    ///     .auto_value("North")?
    ///     .auto_value("East")?
    ///     .auto_value("South")?
    ///     .auto_value("West")?
    ///     .build()?;
    /// ```
    pub fn auto_value(mut self, name: &str) -> Result<Self, FfiModuleError> {
        // Check for duplicate value names
        if self.values.iter().any(|(n, _)| n == name) {
            return Err(FfiModuleError::DuplicateEnumValue {
                enum_name: self.name.clone(),
                value_name: name.to_string(),
            });
        }

        self.values.push((name.to_string(), self.next_value));
        self.next_value += 1;
        Ok(self)
    }

    /// Finish building and register the enum with the module.
    ///
    /// This consumes the builder and adds the enum definition to the module.
    ///
    /// # Errors
    ///
    /// Returns an error if the enum has no values.
    pub fn build(self) -> Result<(), FfiModuleError> {
        if self.values.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                format!("enum '{}' has no values", self.name),
            ));
        }

        let enum_def = FfiEnumDef::new(TypeId::next(), self.name, self.values);

        self.module.add_enum(enum_def);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_builder_explicit_values() {
        let mut module = Module::root();
        module
            .register_enum("Color")
            .value("Red", 0)
            .unwrap()
            .value("Green", 1)
            .unwrap()
            .value("Blue", 2)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].name, "Color");
        assert_eq!(module.enums()[0].values.len(), 3);
        assert_eq!(module.enums()[0].values[0], ("Red".to_string(), 0));
        assert_eq!(module.enums()[0].values[1], ("Green".to_string(), 1));
        assert_eq!(module.enums()[0].values[2], ("Blue".to_string(), 2));
    }

    #[test]
    fn enum_builder_auto_values() {
        let mut module = Module::root();
        module
            .register_enum("Direction")
            .auto_value("North")
            .unwrap()
            .auto_value("East")
            .unwrap()
            .auto_value("South")
            .unwrap()
            .auto_value("West")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].values.len(), 4);
        assert_eq!(module.enums()[0].values[0], ("North".to_string(), 0));
        assert_eq!(module.enums()[0].values[1], ("East".to_string(), 1));
        assert_eq!(module.enums()[0].values[2], ("South".to_string(), 2));
        assert_eq!(module.enums()[0].values[3], ("West".to_string(), 3));
    }

    #[test]
    fn enum_builder_mixed_values() {
        let mut module = Module::root();
        module
            .register_enum("Status")
            .value("Pending", 0)
            .unwrap()
            .auto_value("Running")
            .unwrap()
            .auto_value("Completed")
            .unwrap()
            .value("Failed", -1)
            .unwrap()
            .value("Cancelled", -2)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].values.len(), 5);
        assert_eq!(module.enums()[0].values[0], ("Pending".to_string(), 0));
        assert_eq!(module.enums()[0].values[1], ("Running".to_string(), 1));
        assert_eq!(module.enums()[0].values[2], ("Completed".to_string(), 2));
        assert_eq!(module.enums()[0].values[3], ("Failed".to_string(), -1));
        assert_eq!(module.enums()[0].values[4], ("Cancelled".to_string(), -2));
    }

    #[test]
    fn enum_builder_flags() {
        let mut module = Module::root();
        module
            .register_enum("FileFlags")
            .value("None", 0)
            .unwrap()
            .value("Read", 1)
            .unwrap()
            .value("Write", 2)
            .unwrap()
            .value("Execute", 4)
            .unwrap()
            .value("All", 7)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].values[4], ("All".to_string(), 7));
    }

    #[test]
    fn enum_builder_duplicate_value_name() {
        let mut module = Module::root();
        let result = module
            .register_enum("Color")
            .value("Red", 0)
            .unwrap()
            .value("Red", 1); // Duplicate!

        assert!(result.is_err());
        match result.unwrap_err() {
            FfiModuleError::DuplicateEnumValue { enum_name, value_name } => {
                assert_eq!(enum_name, "Color");
                assert_eq!(value_name, "Red");
            }
            _ => panic!("Expected DuplicateEnumValue error"),
        }
    }

    #[test]
    fn enum_builder_empty_enum() {
        let mut module = Module::root();
        let result = module.register_enum("Empty").build();

        assert!(result.is_err());
        match result.unwrap_err() {
            FfiModuleError::InvalidDeclaration(msg) => {
                assert!(msg.contains("Empty"));
                assert!(msg.contains("no values"));
            }
            _ => panic!("Expected InvalidDeclaration error"),
        }
    }

    #[test]
    fn enum_builder_negative_values() {
        let mut module = Module::root();
        module
            .register_enum("ErrorCode")
            .value("Success", 0)
            .unwrap()
            .value("NotFound", -1)
            .unwrap()
            .value("AccessDenied", -2)
            .unwrap()
            .value("Timeout", -3)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].values[1], ("NotFound".to_string(), -1));
        assert_eq!(module.enums()[0].values[3], ("Timeout".to_string(), -3));
    }

    #[test]
    fn enum_builder_auto_after_explicit() {
        let mut module = Module::root();
        module
            .register_enum("Level")
            .value("Debug", 10)
            .unwrap()
            .auto_value("Info")     // Should be 11
            .unwrap()
            .auto_value("Warning")  // Should be 12
            .unwrap()
            .value("Error", 100)
            .unwrap()
            .auto_value("Critical") // Should be 101
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.enums()[0].values[0], ("Debug".to_string(), 10));
        assert_eq!(module.enums()[0].values[1], ("Info".to_string(), 11));
        assert_eq!(module.enums()[0].values[2], ("Warning".to_string(), 12));
        assert_eq!(module.enums()[0].values[3], ("Error".to_string(), 100));
        assert_eq!(module.enums()[0].values[4], ("Critical".to_string(), 101));
    }
}
