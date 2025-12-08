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
//!     vec![DataType::simple(primitives::INT32)],
//!     &ffi_registry,
//!     &mut script_registry,
//!     &mut type_by_name,
//! )?;
//! ```

use rustc_hash::FxHashMap;

use angelscript_ffi::{FfiRegistry, TemplateInstanceInfo};
use angelscript_parser::lexer::Span;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use crate::semantic::types::registry::{FunctionDef, ScriptParam, ScriptRegistry};
use crate::semantic::types::type_def::{OperatorBehavior, TypeDef, Visibility};
use crate::semantic::types::DataType;
use angelscript_core::{primitives, TypeHash};

/// Handles template instantiation with caching.
///
/// This struct is responsible for creating concrete types from generic templates.
/// It maintains a cache to avoid creating duplicate instantiations.
#[derive(Debug, Default)]
pub struct TemplateInstantiator {
    /// Cache: (template_type_id, arg_type_ids) → instance_type_id
    cache: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
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
    pub fn get_cached(&self, template_id: TypeHash, args: &[DataType]) -> Option<TypeHash> {
        let key = (template_id, args.iter().map(|a| a.type_hash).collect());
        self.cache.get(&key).copied()
    }

    /// Substitute template parameters in a DataType with concrete type arguments.
    ///
    /// If the type is a template parameter (matching one in `template_params`),
    /// it will be replaced with the corresponding type from `args`.
    /// If the type is primitives::SELF and `instance_id` is provided, it will be replaced
    /// with the instance type.
    fn substitute_type(
        data_type: &DataType,
        template_params: &[TypeHash],
        args: &[DataType],
        ffi: &FfiRegistry,
        instance_id: Option<TypeHash>,
    ) -> DataType {
        // Check for primitives::SELF - substitute with the instance type
        if data_type.type_hash == primitives::SELF
            && let Some(inst_id) = instance_id {
                let mut substituted = DataType::simple(inst_id);
                // Preserve modifiers from the original type
                substituted.is_const = data_type.is_const;
                substituted.is_handle = data_type.is_handle;
                substituted.is_handle_to_const = data_type.is_handle_to_const;
                substituted.ref_modifier = data_type.ref_modifier;
                return substituted;
            }

        // Check if this type is a template parameter
        if let Some(typedef) = ffi.get_type(data_type.type_hash)
            && let TypeDef::TemplateParam { index, .. } = typedef {
                // This is a template parameter - substitute it
                if *index < args.len() {
                    let mut substituted = args[*index];
                    // Preserve modifiers from the original type
                    substituted.is_const = data_type.is_const;
                    substituted.ref_modifier = data_type.ref_modifier;
                    return substituted;
                }
            }

        // Also check if the type_id is directly in template_params
        for (i, &param_id) in template_params.iter().enumerate() {
            if data_type.type_hash == param_id && i < args.len() {
                let mut substituted = args[i];
                // Preserve modifiers from the original type
                substituted.is_const = data_type.is_const;
                substituted.ref_modifier = data_type.ref_modifier;
                return substituted;
            }
        }

        // Not a template parameter - return as-is
        *data_type
    }

    /// Instantiate a template type with the given type arguments.
    ///
    /// This creates a new concrete type from a template (e.g., `array<int>` from `array<T>`).
    /// Template instances are cached to avoid duplicate instantiations.
    ///
    /// # Arguments
    /// - `template_id`: The TypeHash of the template type (must have template_params)
    /// - `args`: The concrete type arguments to substitute for template parameters
    /// - `ffi`: The FFI registry (for looking up FFI templates)
    /// - `script`: The script registry (for registering instances and looking up script templates)
    /// - `type_by_name`: The unified name→TypeHash map (updated with instance name)
    ///
    /// # Returns
    /// - `Ok(TypeHash)` - The TypeHash of the instantiated type
    /// - `Err(SemanticError)` - If the template is invalid or validation fails
    pub fn instantiate(
        &mut self,
        template_id: TypeHash,
        args: Vec<DataType>,
        ffi: &FfiRegistry,
        script: &mut ScriptRegistry<'_>,
        type_by_name: &mut FxHashMap<String, TypeHash>,
    ) -> Result<TypeHash, SemanticError> {
        // Create cache key from type IDs
        let cache_key = (template_id, args.iter().map(|a| a.type_hash).collect::<Vec<_>>());

        // Check cache first
        if let Some(&instance_id) = self.cache.get(&cache_key) {
            return Ok(instance_id);
        }

        // Get the template definition (try FFI first, then Script)
        let template_def = ffi.get_type(template_id)
            .or_else(|| script.get_type_by_hash(template_id))
            .ok_or_else(|| {
                SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    Span::default(),
                    format!("Template type {:?} not found", template_id),
                )
            })?;

        // Verify it's a template and extract info
        let (template_name, template_hash, template_params, template_kind, template_operator_methods, template_methods) =
            match template_def {
                TypeDef::Class {
                    name,
                    type_hash,
                    template_params,
                    type_kind,
                    operator_methods,
                    methods,
                    ..
                } => {
                    if template_params.is_empty() {
                        return Err(SemanticError::new(
                            SemanticErrorKind::NotATemplate,
                            Span::default(),
                            format!("{} is not a template type", name),
                        ));
                    }
                    (
                        name.clone(),
                        *type_hash,
                        template_params.clone(),
                        type_kind.clone(),
                        operator_methods.clone(),
                        methods.clone(),
                    )
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

        // Build the instance name (e.g., "array<int>")
        let type_arg_names: Vec<String> = args
            .iter()
            .map(|arg| {
                // Try FFI first, then Script
                ffi.get_type(arg.type_hash)
                    .or_else(|| script.get_type_by_hash(arg.type_hash))
                    .map(|t| t.name().to_string())
                    .unwrap_or_else(|| format!("{:?}", arg.type_hash))
            })
            .collect();
        let instance_name = format!("{}<{}>", template_name, type_arg_names.join(", "));

        // Compute instance hash from template hash + type argument hashes
        // This is done early so we can use it for method hash computation
        // Use the type_hash directly from args - they already have the correct hash
        let arg_hashes: Vec<TypeHash> = args.iter().map(|a| a.type_hash).collect();
        let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);

        // Create specialized operator methods with substituted types
        let mut specialized_operator_methods: FxHashMap<OperatorBehavior, Vec<TypeHash>> =
            FxHashMap::default();

        for (operator, func_ids) in &template_operator_methods {
            let mut specialized_ids = Vec::with_capacity(func_ids.len());

            for &func_id in func_ids {
                // Get the original FFI function
                if let Some(ffi_func) = ffi.get_function(func_id) {
                    // Create specialized parameters with substituted types
                    // Use None for instance_id since we don't have the instance registered yet
                    let specialized_params: Vec<ScriptParam<'_>> = ffi_func
                        .params
                        .iter()
                        .map(|p| {
                            let substituted_type = Self::substitute_type(
                                &p.data_type,
                                &template_params,
                                &args,
                                ffi,
                                None,
                            );
                            ScriptParam::new(p.name.clone(), substituted_type)
                        })
                        .collect();

                    // Substitute return type
                    let specialized_return_type = Self::substitute_type(
                        &ffi_func.return_type,
                        &template_params,
                        &args,
                        ffi,
                        None,
                    );

                    // Compute func_hash for the specialized method
                    // Use the type_hash directly from the substituted params - they already have the correct hash
                    let param_hashes: Vec<TypeHash> = specialized_params.iter()
                        .map(|p| p.data_type.type_hash)
                        .collect();
                    let is_const = ffi_func.traits.is_const;
                    let return_is_const = specialized_return_type.is_const;
                    let func_hash = if ffi_func.traits.is_constructor {
                        TypeHash::from_constructor(instance_hash, &param_hashes)
                    } else {
                        TypeHash::from_method(instance_hash, &ffi_func.name, &param_hashes, is_const, return_is_const)
                    };

                    // Create a new script function with specialized types
                    let specialized_func = FunctionDef {
                        func_hash,
                        name: ffi_func.name.clone(),
                        namespace: Vec::new(),
                        params: specialized_params,
                        return_type: specialized_return_type,
                        object_type: None, // Will be set after type is registered
                        traits: ffi_func.traits,
                        is_native: true, // Still backed by native FFI function
                        visibility: Visibility::Public,
                        signature_filled: true,
                    };

                    let specialized_id = script.register_function(specialized_func);
                    specialized_ids.push(specialized_id);
                } else {
                    // Function not found in FFI - keep original ID (shouldn't happen)
                    specialized_ids.push(func_id);
                }
            }

            specialized_operator_methods.insert(*operator, specialized_ids);
        }

        // Create the instance TypeDef first (with empty methods)
        // We need the instance_id before we can specialize methods that use primitives::SELF
        // Template instances are always Script types (created per-compilation)
        let instance_def = TypeDef::Class {
            name: instance_name.clone(),
            qualified_name: instance_name.clone(),
            type_hash: instance_hash,
            fields: Vec::new(), // TODO: Copy and substitute fields from template
            methods: Vec::new(), // Methods will be added after registration
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: specialized_operator_methods, // Use specialized operator functions
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

        // Now specialize methods with substituted types (including primitives::SELF substitution)
        for &func_id in &template_methods {
            // Get the original FFI function
            if let Some(ffi_func) = ffi.get_function(func_id) {
                // Create specialized parameters with substituted types
                // Pass instance_id for primitives::SELF substitution
                let specialized_params: Vec<ScriptParam<'_>> = ffi_func
                    .params
                    .iter()
                    .map(|p| {
                        let substituted_type = Self::substitute_type(
                            &p.data_type,
                            &template_params,
                            &args,
                            ffi,
                            Some(instance_id),
                        );
                        ScriptParam::new(p.name.clone(), substituted_type)
                    })
                    .collect();

                // Substitute return type
                let specialized_return_type = Self::substitute_type(
                    &ffi_func.return_type,
                    &template_params,
                    &args,
                    ffi,
                    Some(instance_id),
                );

                // Compute func_hash for the specialized method
                // Use the type_hash directly from the substituted params - they already have the correct hash
                let param_hashes: Vec<TypeHash> = specialized_params.iter()
                    .map(|p| p.data_type.type_hash)
                    .collect();
                let is_const = ffi_func.traits.is_const;
                let return_is_const = specialized_return_type.is_const;
                let func_hash = if ffi_func.traits.is_constructor {
                    TypeHash::from_constructor(instance_hash, &param_hashes)
                } else {
                    TypeHash::from_method(instance_hash, &ffi_func.name, &param_hashes, is_const, return_is_const)
                };

                // Create a new script function with specialized types
                let specialized_func = FunctionDef {
                    func_hash,
                    name: ffi_func.name.clone(),
                    namespace: Vec::new(),
                    params: specialized_params,
                    return_type: specialized_return_type,
                    object_type: Some(instance_id),
                    traits: ffi_func.traits,
                    is_native: true, // Still backed by native FFI function
                    visibility: Visibility::Public,
                    signature_filled: true,
                };

                let specialized_id = script.register_function(specialized_func);
                script.add_method_to_class(instance_id, specialized_id);
            }
            // If FFI function not found, skip it (shouldn't happen for valid templates)
        }

        // Add to unified name map
        type_by_name.insert(instance_name, instance_id);

        // Copy behaviors from template to instance (try FFI first, then Script)
        let template_behaviors = ffi.get_behaviors(template_id)
            .or_else(|| script.get_behaviors(template_id))
            .cloned();

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
    use angelscript_ffi::FfiRegistryBuilder;
    use crate::semantic::types::behaviors::TypeBehaviors;
    use angelscript_core::primitives;
    use angelscript_core::TypeKind;

    fn create_test_ffi_with_array() -> (FfiRegistry, TypeHash) {
        let mut builder = FfiRegistryBuilder::new();

        // Template ID is based on the template name
        let template_id = TypeHash::from_name("array");

        // Template param hash is computed from template + param index
        let t_param = TypeHash::from_template_instance(template_id, &[TypeHash(0)]);

        builder.register_type_with_id(
            t_param,
            TypeDef::TemplateParam {
                name: "T".to_string(),
                index: 0,
                owner: template_id,
                type_hash: t_param,
            },
            None,
        );

        // Register array template
        let array_typedef = TypeDef::Class {
            name: "array".to_string(),
            qualified_name: "array".to_string(),
            type_hash: template_id,
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
        behaviors.list_factory = Some(TypeHash(9999));
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
                vec![DataType::simple(primitives::INT32)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        // Instance should be created successfully
        assert!(!instance_id.is_empty());

        // Should be in name map
        assert_eq!(type_by_name.get("array<int>"), Some(&instance_id));

        // Should be cached
        assert_eq!(instantiator.cache_size(), 1);
        assert_eq!(
            instantiator.get_cached(array_template, &[DataType::simple(primitives::INT32)]),
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
                vec![DataType::simple(primitives::INT32)],
                &ffi,
                &mut script,
                &mut type_by_name,
            )
            .unwrap();

        let instance_id2 = instantiator
            .instantiate(
                array_template,
                vec![DataType::simple(primitives::INT32)],
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

        // primitives::INT32 is a primitive, not a template
        let result = instantiator.instantiate(
            primitives::INT32,
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
                vec![DataType::simple(primitives::INT32)],
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
