//! Template instantiation logic.
//!
//! Provides functions to instantiate template types and functions with
//! concrete type arguments.

use angelscript_core::{
    ClassEntry, CompilationError, DataType, FuncdefEntry, FunctionDef, FunctionEntry, FunctionImpl,
    RegistrationError, Span, TemplateInstanceInfo, TypeHash,
};
use angelscript_registry::SymbolRegistry;

use super::cache::TemplateInstanceCache;
use super::substitution::{build_substitution_map, substitute_params, substitute_type};
use super::validation::TemplateCallback;

/// Instantiate a template type with concrete type arguments.
///
/// Returns the hash of the instantiated type.
/// Uses cache to avoid duplicate instantiation and respect FFI specializations.
///
/// # Arguments
/// * `template_hash` - Hash of the template type (e.g., hash of "array")
/// * `type_args` - Concrete types to substitute (e.g., [int])
/// * `span` - Source location for error reporting
/// * `cache` - Template instance cache
/// * `registry` - Registry to look up types and register instances
/// * `callbacks` - Template validation callbacks
pub fn instantiate_template_type<T: TemplateCallback>(
    template_hash: TypeHash,
    type_args: &[DataType],
    span: Span,
    cache: &mut TemplateInstanceCache,
    registry: &mut SymbolRegistry,
    global_registry: &SymbolRegistry,
    callbacks: &T,
) -> Result<TypeHash, CompilationError> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);

    // 2. Check cache - includes FFI specializations and previous instantiations
    if let Some(cached) = cache.get_type_instance(template_hash, &arg_hashes) {
        return Ok(cached);
    }

    // 3. Check if already in registry (FFI specialization not in cache)
    if registry.contains_type(instance_hash) || global_registry.contains_type(instance_hash) {
        cache.cache_type_instance(template_hash, arg_hashes, instance_hash);
        return Ok(instance_hash);
    }

    // 4. Get template definition
    let template = registry
        .get(template_hash)
        .or_else(|| global_registry.get(template_hash))
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompilationError::UnknownType {
            name: format!("{:?}", template_hash),
            span,
        })?;

    if !template.is_template() {
        return Err(CompilationError::NotATemplate {
            name: template.name.clone(),
            span,
        });
    }

    // Clone data we need before mutable borrow
    let template_name = template.name.clone();
    let template_params = template.template_params.clone();
    let template_source = template.source.clone();
    let template_type_kind = template.type_kind.clone();
    let template_base = template.base_class;
    let template_methods = template.methods.clone();
    let template_properties = template.properties.clone();
    let template_behaviors = template.behaviors.clone();

    // 5. Validate via callback (if registered)
    if callbacks.has_template_callback(template_hash) {
        let info = TemplateInstanceInfo::new(template_name.clone(), type_args.to_vec());
        let validation = callbacks.validate_template_instance(template_hash, &info);
        if !validation.is_valid {
            return Err(CompilationError::TemplateValidationFailed {
                template: template_name.clone(),
                message: validation.error.unwrap_or_else(|| "validation failed".to_string()),
                span,
            });
        }
    }

    // 6. Build substitution map
    let subst_map = build_substitution_map(&template_params, type_args, span)?;

    // 7. Create instance entry
    let instance_name = format_template_instance_name(&template_name, type_args);

    let mut instance = ClassEntry::new(
        &instance_name,
        vec![], // namespace inherited from template
        &instance_name,
        instance_hash,
        template_type_kind,
        template_source,
    )
    .with_template_instance(template_hash, type_args.to_vec());

    // 8. Substitute base class
    if let Some(base) = template_base {
        let base_type = DataType::simple(base);
        let substituted_base = substitute_type(base_type, &subst_map);
        instance.base_class = Some(substituted_base.type_hash);
    }

    // 9. Copy behaviors (method hashes will be updated when methods are instantiated)
    instance.behaviors = template_behaviors;

    // 10. Copy properties with substituted types
    for prop in &template_properties {
        let mut inst_prop = prop.clone();
        inst_prop.data_type = substitute_type(prop.data_type, &subst_map);
        instance.properties.push(inst_prop);
    }

    // 11. Register instance before instantiating methods (to handle recursive templates)
    registry
        .register_type(instance.into())
        .map_err(|e| match e {
            RegistrationError::DuplicateType(name) => CompilationError::DuplicateDefinition {
                name,
                span,
            },
            _ => CompilationError::Other {
                message: e.to_string(),
                span,
            },
        })?;

    // 12. Cache the instance
    cache.cache_type_instance(template_hash, arg_hashes.clone(), instance_hash);

    // 13. Instantiate methods
    for method_hash in &template_methods {
        let method = registry
            .get_function(*method_hash)
            .or_else(|| global_registry.get_function(*method_hash));

        if let Some(method) = method {
            // Clone method data before mutable borrow
            let method_def = method.def.clone();
            let method_impl = method.implementation.clone();
            let method_source = method.source;

            let inst_method_hash = instantiate_method_for_type(
                &method_def,
                &method_impl,
                &method_source,
                &subst_map,
                instance_hash,
                span,
                registry,
            )?;

            // Add method to instance
            if let Some(instance_entry) = registry.get_mut(instance_hash)
                && let Some(class) = instance_entry.as_class_mut() {
                    class.methods.push(inst_method_hash);
                }
        }
    }

    Ok(instance_hash)
}

/// Instantiate a template function with concrete type arguments.
pub fn instantiate_template_function(
    func_hash: TypeHash,
    type_args: &[DataType],
    span: Span,
    cache: &mut TemplateInstanceCache,
    registry: &mut SymbolRegistry,
    global_registry: &SymbolRegistry,
) -> Result<TypeHash, CompilationError> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(func_hash, &arg_hashes);

    // 2. Check cache
    if let Some(cached) = cache.get_function_instance(func_hash, &arg_hashes) {
        return Ok(cached);
    }

    // 3. Check if already exists
    if registry.contains_function(instance_hash) || global_registry.contains_function(instance_hash)
    {
        cache.cache_function_instance(func_hash, arg_hashes, instance_hash);
        return Ok(instance_hash);
    }

    // 4. Get template definition
    let template = registry
        .get_function(func_hash)
        .or_else(|| global_registry.get_function(func_hash))
        .ok_or_else(|| CompilationError::FunctionNotFound {
            name: format!("{:?}", func_hash),
            span,
        })?;

    if template.def.template_params.is_empty() {
        return Err(CompilationError::NotATemplate {
            name: template.def.name.clone(),
            span,
        });
    }

    // Clone data we need
    let template_def = template.def.clone();
    let template_impl = template.implementation.clone();
    let template_source = template.source;

    // 5. Build substitution map
    let subst_map = build_substitution_map(&template_def.template_params, type_args, span)?;

    // 6. Substitute param types and return type
    let inst_params = substitute_params(&template_def.params, &subst_map);
    let inst_return = substitute_type(template_def.return_type, &subst_map);

    // 7. Create instance
    let inst_name = format!(
        "{}<{}>",
        template_def.name,
        format_type_args(type_args)
    );

    let inst_def = FunctionDef::new(
        instance_hash,
        inst_name,
        template_def.namespace.clone(),
        inst_params,
        inst_return,
        template_def.object_type,
        template_def.traits,
        template_def.is_native,
        template_def.visibility,
    );

    // 8. Handle implementation
    let inst_impl = match &template_impl {
        FunctionImpl::Native(native) => {
            // FFI template - implementation handles any type via generic calling convention
            FunctionImpl::Native(native.clone())
        }
        FunctionImpl::Script { unit_id } => {
            // Script template - bytecode generated during compilation pass
            FunctionImpl::Script { unit_id: *unit_id }
        }
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, template_source);
    registry
        .register_function(inst_entry)
        .map_err(|e| CompilationError::Other {
            message: e.to_string(),
            span,
        })?;

    // 9. Cache the instance
    cache.cache_function_instance(func_hash, arg_hashes, instance_hash);

    Ok(instance_hash)
}

/// Instantiate a child funcdef (e.g., `array<int>::Callback`).
pub fn instantiate_child_funcdef(
    funcdef_hash: TypeHash,
    parent_type_args: &[DataType],
    span: Span,
    cache: &mut TemplateInstanceCache,
    registry: &mut SymbolRegistry,
    global_registry: &SymbolRegistry,
) -> Result<TypeHash, CompilationError> {
    // 1. Compute instance hash and check cache first
    let arg_hashes: Vec<TypeHash> = parent_type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(funcdef_hash, &arg_hashes);

    // 2. Check cache - child funcdefs use type instance cache
    if let Some(cached) = cache.get_type_instance(funcdef_hash, &arg_hashes) {
        return Ok(cached);
    }

    // 3. Check if already in registry
    if registry.contains_type(instance_hash) || global_registry.contains_type(instance_hash) {
        cache.cache_type_instance(funcdef_hash, arg_hashes.clone(), instance_hash);
        return Ok(instance_hash);
    }

    // 4. Get the funcdef
    let funcdef = registry
        .get(funcdef_hash)
        .or_else(|| global_registry.get(funcdef_hash))
        .and_then(|t| t.as_funcdef())
        .ok_or_else(|| CompilationError::UnknownType {
            name: format!("{:?}", funcdef_hash),
            span,
        })?;

    let parent_hash = funcdef.parent_type.ok_or_else(|| CompilationError::Internal {
        message: "Child funcdef must have parent".to_string(),
    })?;

    // Clone data we need
    let funcdef_name = funcdef.name.clone();
    let funcdef_source = funcdef.source.clone();
    let funcdef_params = funcdef.params.clone();
    let funcdef_return = funcdef.return_type;

    // 5. Get parent template's param hashes
    let parent_template = registry
        .get(parent_hash)
        .or_else(|| global_registry.get(parent_hash))
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompilationError::UnknownType {
            name: format!("{:?}", parent_hash),
            span,
        })?;

    if !parent_template.is_template() {
        return Err(CompilationError::NotATemplate {
            name: parent_template.name.clone(),
            span,
        });
    }

    let parent_template_params = parent_template.template_params.clone();
    let parent_template_name = parent_template.name.clone();

    // 6. Build substitution map from parent's template params
    let subst_map = build_substitution_map(&parent_template_params, parent_type_args, span)?;

    // Substitute params and return type
    let inst_params: Vec<DataType> = funcdef_params
        .iter()
        .map(|p| substitute_type(*p, &subst_map))
        .collect();
    let inst_return = substitute_type(funcdef_return, &subst_map);

    // Create instance name (e.g., "array<int>::Callback")
    let parent_instance_name = format_template_instance_name(&parent_template_name, parent_type_args);
    let inst_qualified_name = format!("{}::{}", parent_instance_name, funcdef_name);

    // Get or create instantiated parent (we need the parent instance to exist)
    // Note: We assume the parent was already instantiated, otherwise this is an error
    let parent_instance_hash = TypeHash::from_template_instance(parent_hash, &arg_hashes);
    if !registry.contains_type(parent_instance_hash)
        && !global_registry.contains_type(parent_instance_hash)
    {
        return Err(CompilationError::UnknownType {
            name: format!("Parent instance {} not found", parent_instance_name),
            span,
        });
    }

    // Create funcdef instance
    let inst_entry = FuncdefEntry::new_child(
        funcdef_name,
        vec![], // namespace - inherited from parent
        inst_qualified_name,
        instance_hash,
        funcdef_source,
        inst_params,
        inst_return,
        parent_instance_hash,
    );

    registry
        .register_type(inst_entry.into())
        .map_err(|e| CompilationError::Other {
            message: e.to_string(),
            span,
        })?;

    // 8. Cache the instance
    cache.cache_type_instance(funcdef_hash, arg_hashes, instance_hash);

    Ok(instance_hash)
}

/// Instantiate a method for a specific template type instance.
fn instantiate_method_for_type(
    method_def: &FunctionDef,
    method_impl: &FunctionImpl,
    method_source: &angelscript_core::entries::FunctionSource,
    subst_map: &super::substitution::SubstitutionMap,
    parent_instance_hash: TypeHash,
    span: Span,
    registry: &mut SymbolRegistry,
) -> Result<TypeHash, CompilationError> {
    // Substitute params and return type
    let inst_params = substitute_params(&method_def.params, subst_map);
    let inst_return = substitute_type(method_def.return_type, subst_map);

    // Compute instance method hash
    let param_hashes: Vec<TypeHash> = inst_params.iter().map(|p| p.data_type.type_hash).collect();

    let instance_method_hash = TypeHash::from_method(
        parent_instance_hash,
        &method_def.name,
        &param_hashes,
        method_def.traits.is_const,
        inst_return.is_const,
    );

    // Check if already instantiated
    if registry.contains_function(instance_method_hash) {
        return Ok(instance_method_hash);
    }

    // Create instantiated method
    let inst_def = FunctionDef::new(
        instance_method_hash,
        method_def.name.clone(),
        method_def.namespace.clone(),
        inst_params,
        inst_return,
        Some(parent_instance_hash),
        method_def.traits,
        method_def.is_native,
        method_def.visibility,
    );

    let inst_impl = match method_impl {
        FunctionImpl::Native(native) => FunctionImpl::Native(native.clone()),
        FunctionImpl::Script { unit_id } => FunctionImpl::Script { unit_id: *unit_id },
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, *method_source);
    registry
        .register_function(inst_entry)
        .map_err(|e| CompilationError::Other {
            message: e.to_string(),
            span,
        })?;

    Ok(instance_method_hash)
}

/// Format template instance name: "array<int>" or "dict<string, int>".
pub fn format_template_instance_name(base_name: &str, type_args: &[DataType]) -> String {
    let args_str = format_type_args(type_args);
    format!("{}<{}>", base_name, args_str)
}

/// Format type arguments as comma-separated string.
pub fn format_type_args(type_args: &[DataType]) -> String {
    type_args
        .iter()
        .map(|t| format!("{:?}", t.type_hash)) // TODO: Use proper type names
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        entries::TypeSource, primitives, FunctionTraits, Param, TemplateParamEntry,
        TemplateValidation, TypeKind, Visibility,
    };

    struct NoOpCallbacks;

    impl TemplateCallback for NoOpCallbacks {
        fn has_template_callback(&self, _: TypeHash) -> bool {
            false
        }
        fn validate_template_instance(
            &self,
            _: TypeHash,
            _: &TemplateInstanceInfo,
        ) -> TemplateValidation {
            TemplateValidation::valid()
        }
    }

    fn create_array_template(registry: &mut SymbolRegistry) -> TypeHash {
        let array_hash = TypeHash::from_name("array");

        // Create template param T
        let t_param = TemplateParamEntry::for_template("T", 0, array_hash, "array");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create array template
        let array_entry = ClassEntry::new(
            "array",
            vec![],
            "array",
            array_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_params(vec![t_hash]);

        registry.register_type(array_entry.into()).unwrap();

        // Create push method: void push(const T&in)
        let push_hash = TypeHash::from_method(array_hash, "push", &[t_hash], false, false);
        let push_def = FunctionDef::new(
            push_hash,
            "push".to_string(),
            vec![],
            vec![Param::new("value", DataType::with_ref_in(t_hash))],
            DataType::void(),
            Some(array_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(push_def))
            .unwrap();

        // Add method to class
        if let Some(entry) = registry.get_mut(array_hash) {
            if let Some(class) = entry.as_class_mut() {
                class.methods.push(push_hash);
            }
        }

        array_hash
    }

    #[test]
    fn instantiate_simple_template() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify instance exists
        let instance = registry.get(instance_hash).unwrap();
        assert!(instance.as_class().unwrap().is_template_instance());
    }

    #[test]
    fn cache_prevents_duplicate_instantiation() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);

        let hash1 = instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        )
        .unwrap();

        let hash2 = instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        )
        .unwrap();

        // Same hash returned
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn method_instantiation() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);
        let array_int = instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        )
        .unwrap();

        // Check push method exists with correct signature
        let class = registry.get(array_int).unwrap().as_class().unwrap();
        assert!(!class.methods.is_empty());

        let push_method = class
            .methods
            .iter()
            .find(|m| {
                registry
                    .get_function(**m)
                    .map(|f| f.def.name == "push")
                    .unwrap_or(false)
            })
            .unwrap();

        let push = registry.get_function(*push_method).unwrap();
        assert_eq!(push.def.params[0].data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn not_a_template_error() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Register a non-template class
        let player_hash = TypeHash::from_name("Player");
        let player = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            player_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(player.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_type(
            player_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::NotATemplate { name, .. } => {
                assert_eq!(name, "Player");
            }
            e => panic!("Expected NotATemplate, got {:?}", e),
        }
    }

    #[test]
    fn format_template_instance_name_single_arg() {
        let name = format_template_instance_name(
            "array",
            &[DataType::simple(primitives::INT32)],
        );
        assert!(name.starts_with("array<"));
        assert!(name.ends_with(">"));
    }

    #[test]
    fn format_template_instance_name_multiple_args() {
        let name = format_template_instance_name(
            "dict",
            &[
                DataType::simple(primitives::STRING),
                DataType::simple(primitives::INT32),
            ],
        );
        assert!(name.starts_with("dict<"));
        assert!(name.contains(", "));
        assert!(name.ends_with(">"));
    }
}
