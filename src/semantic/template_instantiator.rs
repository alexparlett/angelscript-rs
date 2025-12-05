//! Template instantiation logic for creating concrete types from templates.
//!
//! This module provides `TemplateInstantiator`, a single-responsibility struct
//! that handles template type instantiation with caching. It's used by
//! `CompilationContext` to instantiate templates like `array<T>` into concrete
//! types like `array<int>`.
//!
//! # Architecture
//!
//! Template instantiation involves:
//! 1. Looking up the template definition (can be FFI or Script)
//! 2. Validating the type arguments match template parameters
//! 3. Running optional validation callbacks (FFI templates)
//! 4. Creating a new concrete TypeDef with substituted type args
//! 5. Registering the instance as a new Script type
//! 6. Caching the instance to avoid duplicates
//!
//! # Example
//!
//! ```ignore
//! let mut instantiator = TemplateInstantiator::new();
//!
//! // Instantiate array<int> from array<T>
//! let array_int_id = instantiator.instantiate(
//!     array_template_id,
//!     vec![DataType::simple(INT32_TYPE)],
//!     &ffi_registry,
//!     &mut script_registry,
//!     &mut type_by_name,
//! )?;
//! ```

use rustc_hash::FxHashMap;

use crate::ffi::{FfiRegistry, TemplateInstanceInfo};
use crate::lexer::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::types::behaviors::TypeBehaviors;
use crate::semantic::types::registry::ScriptRegistry;
use crate::semantic::types::type_def::{TypeDef, TypeId};
use crate::semantic::types::DataType;

/// Handles template instantiation with caching.
///
/// This struct is responsible for creating concrete types from generic templates.
/// It maintains a cache to avoid creating duplicate instantiations.
#[derive(Debug, Default)]
pub struct TemplateInstantiator {
    /// Cache: (template_type_id, arg_type_ids) → instance_type_id
    cache: FxHashMap<(TypeId, Vec<TypeId>), TypeId>,
}

impl TemplateInstantiator {
    /// Create a new template instantiator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the number of cached template instances.
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Check if an instantiation is already cached.
    pub fn get_cached(&self, template_id: TypeId, args: &[DataType]) -> Option<TypeId> {
        let key = (template_id, args.iter().map(|a| a.type_id).collect());
        self.cache.get(&key).copied()
    }

    /// Instantiate a template type with the given type arguments.
    ///
    /// This creates a new concrete type from a template (e.g., `array<int>` from `array<T>`).
    /// Template instances are cached to avoid duplicate instantiations.
    ///
    /// # Arguments
    /// - `template_id`: The TypeId of the template type (must have template_params)
    /// - `args`: The concrete type arguments to substitute for template parameters
    /// - `ffi`: The FFI registry (for looking up FFI templates)
    /// - `script`: The script registry (for registering instances and looking up script templates)
    /// - `type_by_name`: The unified name→TypeId map (updated with instance name)
    ///
    /// # Returns
    /// - `Ok(TypeId)` - The TypeId of the instantiated type
    /// - `Err(SemanticError)` - If the template is invalid or validation fails
    pub fn instantiate(
        &mut self,
        template_id: TypeId,
        args: Vec<DataType>,
        ffi: &FfiRegistry,
        script: &mut ScriptRegistry<'_>,
        type_by_name: &mut FxHashMap<String, TypeId>,
    ) -> Result<TypeId, SemanticError> {
        // Create cache key from type IDs
        let cache_key = (template_id, args.iter().map(|a| a.type_id).collect::<Vec<_>>());

        // Check cache first
        if let Some(&instance_id) = self.cache.get(&cache_key) {
            return Ok(instance_id);
        }

        // Get the template definition
        let template_def = if template_id.is_ffi() {
            ffi.get_type(template_id).ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    Span::default(),
                    format!("Template type {:?} not found", template_id),
                )
            })?
        } else {
            script.get_type(template_id)
        };

        // Verify it's a template and extract info
        let (template_name, template_params, template_kind) = match template_def {
            TypeDef::Class {
                name,
                template_params,
                type_kind,
                ..
            } => {
                if template_params.is_empty() {
                    return Err(SemanticError::new(
                        SemanticErrorKind::NotATemplate,
                        Span::default(),
                        format!("{} is not a template type", name),
                    ));
                }
                (name.clone(), template_params.clone(), type_kind.clone())
            }
            _ => {
                return Err(SemanticError::new(
                    SemanticErrorKind::NotATemplate,
                    Span::default(),
                    "Only class types can be templates".to_string(),
                ));
            }
        };

        // Verify argument count matches parameter count
        if args.len() != template_params.len() {
            return Err(SemanticError::new(
                SemanticErrorKind::WrongTemplateArgCount,
                Span::default(),
                format!(
                    "Template {} expects {} type arguments, got {}",
                    template_name,
                    template_params.len(),
                    args.len()
                ),
            ));
        }

        // Run validation callback if present (FFI templates only)
        if template_id.is_ffi() {
            if let Some(callback) = ffi.get_template_callback(template_id) {
                let info = TemplateInstanceInfo::new(template_name.clone(), args.clone());
                let validation = callback(&info);
                if !validation.is_valid {
                    return Err(SemanticError::new(
                        SemanticErrorKind::InvalidTemplateInstantiation,
                        Span::default(),
                        validation
                            .error
                            .unwrap_or_else(|| "Template validation failed".to_string()),
                    ));
                }
            }
        }

        // Build the instance name (e.g., "array<int>")
        let type_arg_names: Vec<String> = args
            .iter()
            .map(|arg| {
                if arg.type_id.is_ffi() {
                    ffi.get_type(arg.type_id)
                        .map(|t| t.name().to_string())
                        .unwrap_or_else(|| format!("{:?}", arg.type_id))
                } else {
                    script.get_type(arg.type_id).name().to_string()
                }
            })
            .collect();
        let instance_name = format!("{}<{}>", template_name, type_arg_names.join(", "));

        // Create the instance TypeDef
        // Template instances are always Script types (created per-compilation)
        let instance_def = TypeDef::Class {
            name: instance_name.clone(),
            qualified_name: instance_name.clone(),
            fields: Vec::new(), // TODO: Copy and substitute fields from template
            methods: Vec::new(), // Methods will be added via instantiate_template_methods
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(), // Instance has no params
            template: Some(template_id),
            type_args: args.clone(),
            type_kind: template_kind,
        };

        // Register the instance (always as a script type)
        let instance_id = script.register_type(instance_def, Some(&instance_name));

        // Add to unified name map
        type_by_name.insert(instance_name, instance_id);

        // Copy behaviors from template to instance
        let template_behaviors = if template_id.is_ffi() {
            ffi.get_behaviors(template_id).cloned()
        } else {
            script.get_behaviors(template_id).cloned()
        };

        if let Some(behaviors) = template_behaviors {
            script.set_behaviors(instance_id, behaviors);
        }

        // Cache the instance
        self.cache.insert(cache_key, instance_id);

        Ok(instance_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::FfiRegistryBuilder;
    use crate::semantic::types::type_def::INT32_TYPE;
    use crate::types::TypeKind;

    fn create_test_ffi_with_array() -> (FfiRegistry, TypeId) {
        let mut builder = FfiRegistryBuilder::new();

        // Register template param T first
        let t_param = TypeId::next_ffi();
        let template_id = TypeId::next_ffi();

        builder.register_type_with_id(
            t_param,
            TypeDef::TemplateParam {
                name: "T".to_string(),
                index: 0,
                owner: template_id,
            },
            None,
        );

        // Register array template
        let array_typedef = TypeDef::Class {
            name: "array".to_string(),
            qualified_name: "array".to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![t_param],
            template: None,
            type_args: Vec::new(),
            type_kind: TypeKind::reference(),
        };

        builder.register_type_with_id(template_id, array_typedef, Some("array"));

        // Register list_factory behavior for array template
        let mut behaviors = TypeBehaviors::default();
        behaviors.list_factory = Some(crate::semantic::FunctionId::new(9999));
        builder.set_behaviors(template_id, behaviors);

        (builder.build().unwrap(), template_id)
    }

    #[test]
    fn instantiate_basic() {
        let (ffi, array_template) = create_test_ffi_with_array();
        let mut script = ScriptRegistry::new();
        let mut type_by_name = ffi.type_by_name().clone();
        let mut instantiator = TemplateInstantiator::new();

        let instance_id = instantiator
            .instantiate(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        // Should be a script type
        assert!(instance_id.is_script());

        // Should be in name map
        assert_eq!(type_by_name.get("array<int>"), Some(&instance_id));

        // Should be cached
        assert_eq!(instantiator.cache_size(), 1);
        assert_eq!(
            instantiator.get_cached(array_template, &[DataType::simple(INT32_TYPE)]),
            Some(instance_id)
        );
    }

    #[test]
    fn instantiate_cached() {
        let (ffi, array_template) = create_test_ffi_with_array();
        let mut script = ScriptRegistry::new();
        let mut type_by_name = ffi.type_by_name().clone();
        let mut instantiator = TemplateInstantiator::new();

        let instance_id1 = instantiator
            .instantiate(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        let instance_id2 = instantiator
            .instantiate(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        // Should return same ID
        assert_eq!(instance_id1, instance_id2);

        // Should only be cached once
        assert_eq!(instantiator.cache_size(), 1);
    }

    #[test]
    fn instantiate_wrong_arg_count() {
        let (ffi, array_template) = create_test_ffi_with_array();
        let mut script = ScriptRegistry::new();
        let mut type_by_name = ffi.type_by_name().clone();
        let mut instantiator = TemplateInstantiator::new();

        let result = instantiator.instantiate(
            array_template,
            vec![], // Wrong count - array<T> needs 1 arg
            &ffi,
            &mut script,
            &mut type_by_name,
        );

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("expects 1 type arguments"));
    }

    #[test]
    fn instantiate_non_template_fails() {
        let ffi = FfiRegistryBuilder::new().build().unwrap();
        let mut script = ScriptRegistry::new();
        let mut type_by_name = ffi.type_by_name().clone();
        let mut instantiator = TemplateInstantiator::new();

        // INT32_TYPE is a primitive, not a template
        let result = instantiator.instantiate(
            INT32_TYPE,
            vec![],
            &ffi,
            &mut script,
            &mut type_by_name,
        );

        assert!(result.is_err());
    }

    #[test]
    fn behaviors_copied_to_instance() {
        let (ffi, array_template) = create_test_ffi_with_array();
        let mut script = ScriptRegistry::new();
        let mut type_by_name = ffi.type_by_name().clone();
        let mut instantiator = TemplateInstantiator::new();

        let instance_id = instantiator
            .instantiate(
                array_template,
                vec![DataType::simple(INT32_TYPE)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        // Instance should have behaviors copied from template
        let behaviors = script.get_behaviors(instance_id);
        assert!(behaviors.is_some());
        assert!(behaviors.unwrap().list_factory.is_some());
    }
}
