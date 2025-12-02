//! Global property definitions for FFI registration.
//!
//! Global properties allow scripts to read and write app-owned data.
//! The app owns the data; scripts access it via reference.

use std::any::Any;

use super::types::TypeSpec;

/// A global property definition for FFI registration.
///
/// Stores metadata about a global property that will be applied
/// to the Registry during `apply_to_registry()`.
///
/// The value is stored as a type-erased `&mut dyn Any` reference.
/// The `TypeSpec` provides the AngelScript type information.
pub struct GlobalPropertyDef<'app> {
    /// Property name (unqualified)
    pub name: String,
    /// Type specification (AngelScript type)
    pub type_spec: TypeSpec,
    /// Whether the property is const (read-only from script)
    pub is_const: bool,
    /// The actual value reference (type-erased)
    pub value: &'app mut dyn Any,
}

impl<'app> GlobalPropertyDef<'app> {
    /// Create a new global property definition.
    pub fn new<T: 'static>(
        name: impl Into<String>,
        type_spec: TypeSpec,
        is_const: bool,
        value: &'app mut T,
    ) -> Self {
        Self {
            name: name.into(),
            type_spec,
            is_const,
            value,
        }
    }

    /// Try to downcast to a concrete type (immutable).
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }

    /// Try to downcast to a concrete type (mutable).
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.value.downcast_mut::<T>()
    }

    /// Get the TypeId of the stored value.
    pub fn type_id(&self) -> std::any::TypeId {
        (*self.value).type_id()
    }
}

// Manual Debug implementation since dyn Any doesn't implement Debug
impl std::fmt::Debug for GlobalPropertyDef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GlobalPropertyDef")
            .field("name", &self.name)
            .field("type_spec", &self.type_spec)
            .field("is_const", &self.is_const)
            .field("value_type_id", &(*self.value).type_id())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_property_def_new() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new("score", TypeSpec::simple("int"), false, &mut value);

        assert_eq!(def.name, "score");
        assert_eq!(def.type_spec.type_name, "int");
        assert!(!def.is_const);
    }

    #[test]
    fn global_property_def_const() {
        let mut value = 3.14f64;
        let def = GlobalPropertyDef::new("PI", TypeSpec::simple("double"), true, &mut value);

        assert_eq!(def.name, "PI");
        assert!(def.is_const);
    }

    #[test]
    fn global_property_def_downcast_ref() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new("score", TypeSpec::simple("int"), false, &mut value);

        assert_eq!(def.downcast_ref::<i32>(), Some(&42));
        assert_eq!(def.downcast_ref::<i64>(), None);
    }

    #[test]
    fn global_property_def_downcast_mut() {
        let mut value = 42i32;
        let mut def = GlobalPropertyDef::new("score", TypeSpec::simple("int"), false, &mut value);

        if let Some(v) = def.downcast_mut::<i32>() {
            *v = 100;
        }

        assert_eq!(def.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn global_property_def_type_id() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new("score", TypeSpec::simple("int"), false, &mut value);

        assert_eq!(def.type_id(), std::any::TypeId::of::<i32>());
    }

    #[test]
    fn global_property_def_debug() {
        let mut value = 42i32;
        let def = GlobalPropertyDef::new("score", TypeSpec::simple("int"), false, &mut value);
        let debug = format!("{:?}", def);

        assert!(debug.contains("GlobalPropertyDef"));
        assert!(debug.contains("score"));
        assert!(debug.contains("int"));
    }

    #[test]
    fn global_property_def_string_type() {
        let mut value = String::from("hello");
        let def = GlobalPropertyDef::new("greeting", TypeSpec::simple("string"), false, &mut value);

        assert_eq!(def.downcast_ref::<String>(), Some(&String::from("hello")));
    }

    #[test]
    fn global_property_def_custom_type() {
        struct MyType {
            value: i32,
        }

        let mut val = MyType { value: 42 };
        let def = GlobalPropertyDef::new("obj", TypeSpec::simple("MyType"), false, &mut val);

        assert_eq!(def.type_id(), std::any::TypeId::of::<MyType>());
        assert!(def.downcast_ref::<MyType>().is_some());
    }
}
