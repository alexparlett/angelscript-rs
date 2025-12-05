//! FFI data type for deferred type resolution.
//!
//! This module provides `FfiDataType`, which allows type references to remain
//! unresolved during FFI registration and be resolved later when all types
//! are known.
//!
//! # Problem
//!
//! When registering FFI types, functions may reference types that haven't been
//! registered yet:
//!
//! ```ignore
//! module.register_function("void process(MyClass@ obj)")?;
//! // MyClass might not be registered yet!
//! ```
//!
//! # Solution
//!
//! `FfiDataType` defers resolution:
//! - Primitives (`int`, `float`, etc.) are `Resolved` immediately
//! - User types become `Unresolved("TypeName")` until install phase
//! - Template types use nested `FfiDataType` for arguments
//!
//! # Example
//!
//! ```ignore
//! // During registration - types are unresolved
//! let ffi_type = FfiDataType::unresolved("MyClass", false, true, false, RefModifier::None);
//!
//! // During install - resolve to concrete DataType
//! let data_type = ffi_type.resolve(
//!     |name| registry.get_type_id_by_name(name),
//!     |template_id, args| registry.instantiate_template(template_id, args),
//! )?;
//! ```

use crate::semantic::types::{DataType, RefModifier, TypeId};

/// The base type portion of a type reference (without modifiers).
///
/// This represents either a simple type name or a template instantiation
/// with type arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnresolvedBaseType {
    /// Simple type name (e.g., "MyClass", "int")
    Simple(String),

    /// Template instantiation (e.g., `array<string>`, `dictionary<string, int>`)
    Template {
        /// The template type name (e.g., "array", "dictionary")
        name: String,
        /// The type arguments, which may themselves be unresolved
        args: Vec<FfiDataType>,
    },
}

/// Type reference that may be unresolved during registration.
///
/// During FFI registration, types may reference other types that haven't been
/// registered yet. `FfiDataType` captures the type specification with all its
/// modifiers, deferring actual type resolution until the install phase.
///
/// # Variants
///
/// - `Resolved`: Already resolved to a concrete `DataType` (primitives, built-ins)
/// - `Unresolved`: Needs resolution during install phase
///
/// # Example
///
/// ```ignore
/// // Primitive - resolved immediately
/// let int_type = FfiDataType::Resolved(DataType::simple(INT32_TYPE));
///
/// // User type - unresolved until install
/// let my_class = FfiDataType::unresolved("MyClass", false, true, false, RefModifier::None);
///
/// // Template with unresolved arg
/// let array_of_class = FfiDataType::unresolved_template(
///     "array",
///     vec![FfiDataType::unresolved_simple("MyClass")],
///     false, false, false, RefModifier::None
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FfiDataType {
    /// Already resolved (primitives, already-registered types)
    Resolved(DataType),

    /// Needs resolution during install()
    Unresolved {
        /// The base type (simple name or template)
        base: UnresolvedBaseType,
        /// Whether the type is const
        is_const: bool,
        /// Whether this is a handle type (@)
        is_handle: bool,
        /// Whether this is a handle to a const value
        is_handle_to_const: bool,
        /// Reference modifier (&in, &out, &inout)
        ref_modifier: RefModifier,
    },
}

impl FfiDataType {
    /// Create an already-resolved type.
    #[inline]
    pub fn resolved(data_type: DataType) -> Self {
        FfiDataType::Resolved(data_type)
    }

    /// Create an unresolved simple type with all modifiers.
    pub fn unresolved(
        name: impl Into<String>,
        is_const: bool,
        is_handle: bool,
        is_handle_to_const: bool,
        ref_modifier: RefModifier,
    ) -> Self {
        FfiDataType::Unresolved {
            base: UnresolvedBaseType::Simple(name.into()),
            is_const,
            is_handle,
            is_handle_to_const,
            ref_modifier,
        }
    }

    /// Create an unresolved simple type with no modifiers.
    pub fn unresolved_simple(name: impl Into<String>) -> Self {
        Self::unresolved(name, false, false, false, RefModifier::None)
    }

    /// Create an unresolved handle type.
    pub fn unresolved_handle(name: impl Into<String>, is_handle_to_const: bool) -> Self {
        Self::unresolved(name, false, true, is_handle_to_const, RefModifier::None)
    }

    /// Create an unresolved const type.
    pub fn unresolved_const(name: impl Into<String>) -> Self {
        Self::unresolved(name, true, false, false, RefModifier::None)
    }

    /// Create an unresolved template type with all modifiers.
    pub fn unresolved_template(
        name: impl Into<String>,
        args: Vec<FfiDataType>,
        is_const: bool,
        is_handle: bool,
        is_handle_to_const: bool,
        ref_modifier: RefModifier,
    ) -> Self {
        FfiDataType::Unresolved {
            base: UnresolvedBaseType::Template {
                name: name.into(),
                args,
            },
            is_const,
            is_handle,
            is_handle_to_const,
            ref_modifier,
        }
    }

    /// Create an unresolved template type with no modifiers.
    pub fn unresolved_template_simple(name: impl Into<String>, args: Vec<FfiDataType>) -> Self {
        Self::unresolved_template(name, args, false, false, false, RefModifier::None)
    }

    /// Resolve this type to a concrete `DataType`.
    ///
    /// # Arguments
    ///
    /// * `lookup` - Function to look up a type ID by name
    /// * `instantiate` - Function to instantiate a template with type arguments
    ///
    /// # Returns
    ///
    /// The resolved `DataType`, or an error if resolution fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let resolved = ffi_type.resolve(
    ///     |name| registry.get_type_id_by_name(name),
    ///     |template_id, args| registry.instantiate_template(template_id, args),
    /// )?;
    /// ```
    pub fn resolve<L, I>(&self, lookup: &L, instantiate: &mut I) -> Result<DataType, String>
    where
        L: Fn(&str) -> Option<TypeId>,
        I: FnMut(TypeId, Vec<DataType>) -> Result<TypeId, String>,
    {
        match self {
            FfiDataType::Resolved(dt) => Ok(dt.clone()),

            FfiDataType::Unresolved {
                base,
                is_const,
                is_handle,
                is_handle_to_const,
                ref_modifier,
            } => {
                let type_id = match base {
                    UnresolvedBaseType::Simple(name) => lookup(name)
                        .ok_or_else(|| format!("Unknown type: {}", name))?,

                    UnresolvedBaseType::Template { name, args } => {
                        // Look up the template type
                        let template_id = lookup(name)
                            .ok_or_else(|| format!("Unknown template type: {}", name))?;

                        // Recursively resolve all type arguments
                        let resolved_args: Vec<DataType> = args
                            .iter()
                            .map(|arg| arg.resolve(lookup, instantiate))
                            .collect::<Result<_, _>>()?;

                        // Instantiate the template
                        instantiate(template_id, resolved_args)?
                    }
                };

                Ok(DataType {
                    type_id,
                    is_const: *is_const,
                    is_handle: *is_handle,
                    is_handle_to_const: *is_handle_to_const,
                    ref_modifier: *ref_modifier,
                })
            }
        }
    }

    /// Check if this type is already resolved.
    pub fn is_resolved(&self) -> bool {
        matches!(self, FfiDataType::Resolved(_))
    }

    /// Check if this type needs resolution.
    pub fn is_unresolved(&self) -> bool {
        matches!(self, FfiDataType::Unresolved { .. })
    }

    /// Get the resolved DataType if already resolved.
    pub fn as_resolved(&self) -> Option<&DataType> {
        match self {
            FfiDataType::Resolved(dt) => Some(dt),
            FfiDataType::Unresolved { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::type_def::INT32_TYPE;

    // Use a fixed TypeId for string in tests since STRING_TYPE may not be available
    const TEST_STRING_TYPE: TypeId = TypeId(50);

    /// Helper to create a simple type lookup
    fn make_lookup<'a>(types: &'a [(&'a str, TypeId)]) -> impl Fn(&str) -> Option<TypeId> + 'a {
        move |name| types.iter().find(|(n, _)| *n == name).map(|(_, id)| *id)
    }

    /// Helper instantiate function that just returns an error (no templates)
    fn no_templates(_: TypeId, _: Vec<DataType>) -> Result<TypeId, String> {
        Err("Templates not supported in this test".to_string())
    }

    #[test]
    fn resolved_type_passes_through() {
        let original = DataType::simple(INT32_TYPE);
        let ffi = FfiDataType::resolved(original.clone());

        assert!(ffi.is_resolved());
        assert!(!ffi.is_unresolved());
        assert_eq!(ffi.as_resolved(), Some(&original));

        let types = [];
        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();
        assert_eq!(resolved, original);
    }

    #[test]
    fn simple_unresolved_type_resolves() {
        let ffi = FfiDataType::unresolved_simple("MyClass");

        assert!(!ffi.is_resolved());
        assert!(ffi.is_unresolved());

        // Create a mock type ID for MyClass
        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();

        assert_eq!(resolved.type_id, my_class_id);
        assert!(!resolved.is_const);
        assert!(!resolved.is_handle);
        assert!(!resolved.is_handle_to_const);
        assert_eq!(resolved.ref_modifier, RefModifier::None);
    }

    #[test]
    fn unresolved_with_modifiers_preserves_modifiers() {
        let ffi = FfiDataType::unresolved("MyClass", true, true, true, RefModifier::In);

        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();

        assert_eq!(resolved.type_id, my_class_id);
        assert!(resolved.is_const);
        assert!(resolved.is_handle);
        assert!(resolved.is_handle_to_const);
        assert_eq!(resolved.ref_modifier, RefModifier::In);
    }

    #[test]
    fn unresolved_handle_type() {
        let ffi = FfiDataType::unresolved_handle("MyClass", false);

        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();

        assert_eq!(resolved.type_id, my_class_id);
        assert!(!resolved.is_const);
        assert!(resolved.is_handle);
        assert!(!resolved.is_handle_to_const);
    }

    #[test]
    fn unresolved_handle_to_const() {
        let ffi = FfiDataType::unresolved_handle("MyClass", true);

        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();

        assert!(resolved.is_handle);
        assert!(resolved.is_handle_to_const);
    }

    #[test]
    fn unresolved_const_type() {
        let ffi = FfiDataType::unresolved_const("MyClass");

        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();

        assert!(resolved.is_const);
        assert!(!resolved.is_handle);
    }

    #[test]
    fn unknown_type_returns_error() {
        let ffi = FfiDataType::unresolved_simple("UnknownType");

        let types: [(& str, TypeId); 0] = [];
        let result = ffi.resolve(&make_lookup(&types), &mut no_templates);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown type: UnknownType"));
    }

    #[test]
    fn template_type_with_resolved_args() {
        // array<int> where int is already resolved in the arg
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::resolved(DataType::simple(INT32_TYPE))],
        );

        let array_template_id = TypeId(200);
        let array_int_instance_id = TypeId(201);

        let types = [("array", array_template_id)];

        // Track instantiation calls
        let mut instantiate_calls: Vec<(TypeId, Vec<DataType>)> = Vec::new();
        let mut instantiate = |template_id: TypeId, args: Vec<DataType>| -> Result<TypeId, String> {
            instantiate_calls.push((template_id, args.clone()));
            if template_id == array_template_id {
                Ok(array_int_instance_id)
            } else {
                Err("Unknown template".to_string())
            }
        };

        let resolved = ffi.resolve(&make_lookup(&types), &mut instantiate).unwrap();

        assert_eq!(resolved.type_id, array_int_instance_id);
        assert_eq!(instantiate_calls.len(), 1);
        assert_eq!(instantiate_calls[0].0, array_template_id);
        assert_eq!(instantiate_calls[0].1.len(), 1);
        assert_eq!(instantiate_calls[0].1[0].type_id, INT32_TYPE);
    }

    #[test]
    fn template_type_with_unresolved_args() {
        // array<MyClass> where MyClass needs resolution
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("MyClass")],
        );

        let array_template_id = TypeId(200);
        let my_class_id = TypeId(100);
        let array_myclass_instance_id = TypeId(202);

        let types = [("array", array_template_id), ("MyClass", my_class_id)];

        let mut instantiate = |template_id: TypeId, args: Vec<DataType>| -> Result<TypeId, String> {
            if template_id == array_template_id && args.len() == 1 && args[0].type_id == my_class_id {
                Ok(array_myclass_instance_id)
            } else {
                Err("Unexpected template instantiation".to_string())
            }
        };

        let resolved = ffi.resolve(&make_lookup(&types), &mut instantiate).unwrap();

        assert_eq!(resolved.type_id, array_myclass_instance_id);
    }

    #[test]
    fn nested_template_resolution() {
        // array<array<int>> - nested templates
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_template_simple(
                "array",
                vec![FfiDataType::resolved(DataType::simple(INT32_TYPE))],
            )],
        );

        let array_template_id = TypeId(200);
        let array_int_id = TypeId(201);
        let array_array_int_id = TypeId(202);

        let types = [("array", array_template_id)];

        let mut instantiate = |template_id: TypeId, args: Vec<DataType>| -> Result<TypeId, String> {
            if template_id == array_template_id {
                if args.len() == 1 && args[0].type_id == INT32_TYPE {
                    Ok(array_int_id)
                } else if args.len() == 1 && args[0].type_id == array_int_id {
                    Ok(array_array_int_id)
                } else {
                    Err("Unexpected args".to_string())
                }
            } else {
                Err("Unknown template".to_string())
            }
        };

        let resolved = ffi.resolve(&make_lookup(&types), &mut instantiate).unwrap();

        assert_eq!(resolved.type_id, array_array_int_id);
    }

    #[test]
    fn template_with_multiple_args() {
        // dictionary<string, MyClass>
        let ffi = FfiDataType::unresolved_template_simple(
            "dictionary",
            vec![
                FfiDataType::resolved(DataType::simple(TEST_STRING_TYPE)),
                FfiDataType::unresolved_simple("MyClass"),
            ],
        );

        let dict_template_id = TypeId(300);
        let my_class_id = TypeId(100);
        let dict_instance_id = TypeId(301);

        let types = [("dictionary", dict_template_id), ("MyClass", my_class_id)];

        let mut instantiate = |template_id: TypeId, args: Vec<DataType>| -> Result<TypeId, String> {
            if template_id == dict_template_id
                && args.len() == 2
                && args[0].type_id == TEST_STRING_TYPE
                && args[1].type_id == my_class_id
            {
                Ok(dict_instance_id)
            } else {
                Err("Unexpected instantiation".to_string())
            }
        };

        let resolved = ffi.resolve(&make_lookup(&types), &mut instantiate).unwrap();

        assert_eq!(resolved.type_id, dict_instance_id);
    }

    #[test]
    fn unknown_template_returns_error() {
        let ffi = FfiDataType::unresolved_template_simple(
            "UnknownTemplate",
            vec![FfiDataType::resolved(DataType::simple(INT32_TYPE))],
        );

        let types: [(&str, TypeId); 0] = [];
        let result = ffi.resolve(&make_lookup(&types), &mut no_templates);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown template type: UnknownTemplate"));
    }

    #[test]
    fn error_propagates_from_nested_resolution() {
        // array<UnknownType> - the inner type resolution should fail
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("UnknownType")],
        );

        let array_template_id = TypeId(200);
        let types = [("array", array_template_id)];

        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Ok(TypeId(999)) // Should never be called
        };

        let result = ffi.resolve(&make_lookup(&types), &mut instantiate);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown type: UnknownType"));
    }

    #[test]
    fn template_instantiation_error_propagates() {
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::resolved(DataType::simple(INT32_TYPE))],
        );

        let array_template_id = TypeId(200);
        let types = [("array", array_template_id)];

        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Err("Template instantiation failed: invalid type argument".to_string())
        };

        let result = ffi.resolve(&make_lookup(&types), &mut instantiate);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Template instantiation failed"));
    }

    #[test]
    fn template_with_handle_modifier() {
        // array<MyClass>@ - handle to array
        let ffi = FfiDataType::unresolved_template(
            "array",
            vec![FfiDataType::unresolved_simple("MyClass")],
            false, // is_const
            true,  // is_handle
            false, // is_handle_to_const
            RefModifier::None,
        );

        let array_template_id = TypeId(200);
        let my_class_id = TypeId(100);
        let array_instance_id = TypeId(201);

        let types = [("array", array_template_id), ("MyClass", my_class_id)];

        let mut instantiate = |_: TypeId, _: Vec<DataType>| -> Result<TypeId, String> {
            Ok(array_instance_id)
        };

        let resolved = ffi.resolve(&make_lookup(&types), &mut instantiate).unwrap();

        assert_eq!(resolved.type_id, array_instance_id);
        assert!(resolved.is_handle);
        assert!(!resolved.is_const);
    }

    #[test]
    fn ref_modifiers_preserved() {
        let test_cases = [
            (RefModifier::None, "None"),
            (RefModifier::In, "In"),
            (RefModifier::Out, "Out"),
            (RefModifier::InOut, "InOut"),
        ];

        let my_class_id = TypeId(100);
        let types = [("MyClass", my_class_id)];

        for (ref_mod, _name) in test_cases {
            let ffi = FfiDataType::unresolved("MyClass", false, false, false, ref_mod);
            let resolved = ffi.resolve(&make_lookup(&types), &mut no_templates).unwrap();
            assert_eq!(resolved.ref_modifier, ref_mod);
        }
    }

    #[test]
    fn equality_resolved() {
        let dt = DataType::simple(INT32_TYPE);
        let ffi1 = FfiDataType::resolved(dt.clone());
        let ffi2 = FfiDataType::resolved(dt);

        assert_eq!(ffi1, ffi2);
    }

    #[test]
    fn equality_unresolved_simple() {
        let ffi1 = FfiDataType::unresolved_simple("MyClass");
        let ffi2 = FfiDataType::unresolved_simple("MyClass");
        let ffi3 = FfiDataType::unresolved_simple("OtherClass");

        assert_eq!(ffi1, ffi2);
        assert_ne!(ffi1, ffi3);
    }

    #[test]
    fn equality_unresolved_with_modifiers() {
        let ffi1 = FfiDataType::unresolved("MyClass", true, false, false, RefModifier::In);
        let ffi2 = FfiDataType::unresolved("MyClass", true, false, false, RefModifier::In);
        let ffi3 = FfiDataType::unresolved("MyClass", false, false, false, RefModifier::In);

        assert_eq!(ffi1, ffi2);
        assert_ne!(ffi1, ffi3);
    }

    #[test]
    fn equality_template() {
        let ffi1 = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("MyClass")],
        );
        let ffi2 = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("MyClass")],
        );
        let ffi3 = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("OtherClass")],
        );

        assert_eq!(ffi1, ffi2);
        assert_ne!(ffi1, ffi3);
    }

    #[test]
    fn debug_output() {
        let ffi = FfiDataType::unresolved_simple("MyClass");
        let debug = format!("{:?}", ffi);
        assert!(debug.contains("Unresolved"));
        assert!(debug.contains("MyClass"));
    }

    #[test]
    fn clone_resolved() {
        let ffi = FfiDataType::resolved(DataType::simple(INT32_TYPE));
        let cloned = ffi.clone();
        assert_eq!(ffi, cloned);
    }

    #[test]
    fn clone_unresolved() {
        let ffi = FfiDataType::unresolved_template_simple(
            "array",
            vec![FfiDataType::unresolved_simple("MyClass")],
        );
        let cloned = ffi.clone();
        assert_eq!(ffi, cloned);
    }
}
