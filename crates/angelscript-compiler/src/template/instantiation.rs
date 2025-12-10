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
                message: validation
                    .error
                    .unwrap_or_else(|| "validation failed".to_string()),
                span,
            });
        }
    }

    // 6. Build substitution map
    let subst_map = build_substitution_map(&template_params, type_args, span)?;

    // 7. Create instance entry
    let instance_name =
        format_template_instance_name(&template_name, type_args, registry, global_registry);

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
            RegistrationError::DuplicateType(name) => {
                CompilationError::DuplicateDefinition { name, span }
            }
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
                && let Some(class) = instance_entry.as_class_mut()
            {
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
        format_type_args(type_args, registry, global_registry)
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

    let parent_hash = funcdef
        .parent_type
        .ok_or_else(|| CompilationError::Internal {
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
    let parent_instance_name = format_template_instance_name(
        &parent_template_name,
        parent_type_args,
        registry,
        global_registry,
    );
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
pub fn format_template_instance_name(
    base_name: &str,
    type_args: &[DataType],
    registry: &SymbolRegistry,
    global_registry: &SymbolRegistry,
) -> String {
    let args_str = format_type_args(type_args, registry, global_registry);
    format!("{}<{}>", base_name, args_str)
}

/// Format type arguments as comma-separated string.
///
/// Looks up actual type names from the registry. Falls back to hex hash if not found.
pub fn format_type_args(
    type_args: &[DataType],
    registry: &SymbolRegistry,
    global_registry: &SymbolRegistry,
) -> String {
    type_args
        .iter()
        .map(|t| {
            // Try to look up the type name from registries
            registry
                .get(t.type_hash)
                .or_else(|| global_registry.get(t.type_hash))
                .map(|entry| entry.qualified_name().to_string())
                .unwrap_or_else(|| format!("{:?}", t.type_hash))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        FunctionTraits, Param, TemplateParamEntry, TemplateValidation, TypeKind, Visibility,
        entries::TypeSource, primitives,
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
        if let Some(entry) = registry.get_mut(array_hash)
            && let Some(class) = entry.as_class_mut()
        {
            class.methods.push(push_hash);
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
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let name = format_template_instance_name(
            "array",
            &[DataType::simple(primitives::INT32)],
            &registry,
            &global_registry,
        );
        assert_eq!(name, "array<int>");
    }

    #[test]
    fn format_template_instance_name_multiple_args() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let name = format_template_instance_name(
            "dict",
            &[
                DataType::simple(primitives::FLOAT),
                DataType::simple(primitives::INT32),
            ],
            &registry,
            &global_registry,
        );
        assert_eq!(name, "dict<float, int>");
    }

    // === Tests for instantiate_template_function ===

    fn create_identity_template_function(registry: &mut SymbolRegistry) -> TypeHash {
        // Create template param T
        let func_hash = TypeHash::from_name("identity");
        let t_param = TemplateParamEntry::for_template("T", 0, func_hash, "identity");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create identity<T>(T value) -> T
        let func_def = FunctionDef::new_template(
            func_hash,
            "identity".to_string(),
            vec![],
            vec![Param::new("value", DataType::simple(t_hash))],
            DataType::simple(t_hash),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
            vec![t_hash],
        );

        let func_entry = FunctionEntry::ffi(func_def);
        registry.register_function(func_entry).unwrap();

        func_hash
    }

    #[test]
    fn instantiate_template_function_simple() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let identity_hash = create_identity_template_function(&mut registry);

        let mut cache = TemplateInstanceCache::new();

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_function(
            identity_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify instance exists
        let instance = registry.get_function(instance_hash).unwrap();
        assert_eq!(
            instance.def.params[0].data_type.type_hash,
            primitives::INT32
        );
        assert_eq!(instance.def.return_type.type_hash, primitives::INT32);
    }

    #[test]
    fn instantiate_template_function_cache_hit() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let identity_hash = create_identity_template_function(&mut registry);

        let mut cache = TemplateInstanceCache::new();

        let int_type = DataType::simple(primitives::INT32);

        // First instantiation
        let hash1 = instantiate_template_function(
            identity_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        )
        .unwrap();

        // Second instantiation should hit cache
        let hash2 = instantiate_template_function(
            identity_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        )
        .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn instantiate_template_function_not_a_template() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Create a non-template function
        let func_hash = TypeHash::from_name("regular_func");
        let func_def = FunctionDef::new(
            func_hash,
            "regular_func".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(func_def))
            .unwrap();

        let mut cache = TemplateInstanceCache::new();

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_function(
            func_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::NotATemplate { name, .. } => {
                assert_eq!(name, "regular_func");
            }
            e => panic!("Expected NotATemplate, got {:?}", e),
        }
    }

    #[test]
    fn instantiate_template_function_not_found() {
        let mut registry = SymbolRegistry::new();
        let global_registry = SymbolRegistry::new();

        let mut cache = TemplateInstanceCache::new();

        let nonexistent_hash = TypeHash::from_name("nonexistent");
        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_function(
            nonexistent_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            CompilationError::FunctionNotFound { .. }
        );
    }

    #[test]
    fn instantiate_template_function_already_in_registry() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let identity_hash = create_identity_template_function(&mut registry);

        // Pre-register the instance in the registry (simulating FFI specialization)
        let int_type = DataType::simple(primitives::INT32);
        let instance_hash = TypeHash::from_template_instance(identity_hash, &[primitives::INT32]);

        let inst_def = FunctionDef::new(
            instance_hash,
            "identity<int>".to_string(),
            vec![],
            vec![Param::new("value", int_type)],
            int_type,
            None,
            FunctionTraits::default(),
            true, // native
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(inst_def))
            .unwrap();

        let mut cache = TemplateInstanceCache::new();

        // Should detect already registered and return that hash
        let result = instantiate_template_function(
            identity_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), instance_hash);
        // Cache should now have the instance
        assert!(cache.has_function_instance(identity_hash, &[primitives::INT32]));
    }

    // === Tests for instantiate_child_funcdef ===

    fn create_array_with_callback(registry: &mut SymbolRegistry) -> (TypeHash, TypeHash) {
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

        // Create child funcdef: array::Callback(const T&in) -> bool
        let callback_hash = TypeHash::from_name("array::Callback");
        let callback_entry = FuncdefEntry::new_child(
            "Callback",
            vec![],
            "array::Callback",
            callback_hash,
            TypeSource::ffi_untyped(),
            vec![DataType::with_ref_in(t_hash)],
            DataType::simple(primitives::BOOL),
            array_hash,
        );
        registry.register_type(callback_entry.into()).unwrap();

        (array_hash, callback_hash)
    }

    #[test]
    fn instantiate_child_funcdef_simple() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let (array_hash, callback_hash) = create_array_with_callback(&mut registry);

        // First instantiate the parent type
        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;
        let int_type = DataType::simple(primitives::INT32);

        let _array_int_hash = instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        )
        .unwrap();

        // Now instantiate the child funcdef
        let result = instantiate_child_funcdef(
            callback_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify instance exists
        let instance = registry.get(instance_hash).unwrap();
        let funcdef = instance.as_funcdef().unwrap();
        assert_eq!(funcdef.params[0].type_hash, primitives::INT32);
        assert_eq!(funcdef.return_type.type_hash, primitives::BOOL);
    }

    #[test]
    fn instantiate_child_funcdef_cache_hit() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let (array_hash, callback_hash) = create_array_with_callback(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;
        let int_type = DataType::simple(primitives::INT32);

        // First instantiate the parent type
        instantiate_template_type(
            array_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        )
        .unwrap();

        // First instantiation
        let hash1 = instantiate_child_funcdef(
            callback_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        )
        .unwrap();

        // Second instantiation should hit cache
        let hash2 = instantiate_child_funcdef(
            callback_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        )
        .unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn instantiate_child_funcdef_parent_not_instantiated() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let (_array_hash, callback_hash) = create_array_with_callback(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let int_type = DataType::simple(primitives::INT32);

        // Try to instantiate child funcdef without instantiating parent first
        let result = instantiate_child_funcdef(
            callback_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        matches!(result.unwrap_err(), CompilationError::UnknownType { .. });
    }

    // === Tests for validation callback failure ===

    struct FailingCallbacks;

    impl TemplateCallback for FailingCallbacks {
        fn has_template_callback(&self, _: TypeHash) -> bool {
            true
        }
        fn validate_template_instance(
            &self,
            _: TypeHash,
            _: &TemplateInstanceInfo,
        ) -> TemplateValidation {
            TemplateValidation::invalid("type not allowed")
        }
    }

    #[test]
    fn instantiate_template_type_validation_failure() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        let mut cache = TemplateInstanceCache::new();
        let callbacks = FailingCallbacks;

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

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TemplateValidationFailed {
                template, message, ..
            } => {
                assert_eq!(template, "array");
                assert!(message.contains("type not allowed"));
            }
            e => panic!("Expected TemplateValidationFailed, got {:?}", e),
        }
    }

    // === Tests for properties substitution ===

    #[test]
    fn instantiate_template_with_properties() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let container_hash = TypeHash::from_name("Container");

        // Create template param T
        let t_param = TemplateParamEntry::for_template("T", 0, container_hash, "Container");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create Container template with a property of type T
        use angelscript_core::PropertyEntry;

        let mut container = ClassEntry::new(
            "Container",
            vec![],
            "Container",
            container_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_params(vec![t_hash]);

        container.properties.push(PropertyEntry::new(
            "value",
            DataType::simple(t_hash),
            Visibility::Public,
            None,
            None,
        ));

        registry.register_type(container.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_type(
            container_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify property type was substituted
        let instance = registry.get(instance_hash).unwrap().as_class().unwrap();
        assert_eq!(instance.properties.len(), 1);
        assert_eq!(
            instance.properties[0].data_type.type_hash,
            primitives::INT32
        );
    }

    // === Tests for base class substitution ===

    #[test]
    fn instantiate_template_with_base_class() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Create a base class
        let base_hash = TypeHash::from_name("BaseClass");
        let base = ClassEntry::new(
            "BaseClass",
            vec![],
            "BaseClass",
            base_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(base.into()).unwrap();

        let derived_hash = TypeHash::from_name("Derived");

        // Create template param T
        let t_param = TemplateParamEntry::for_template("T", 0, derived_hash, "Derived");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create Derived template with a base class
        let mut derived = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_params(vec![t_hash]);

        derived.base_class = Some(base_hash);

        registry.register_type(derived.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_type(
            derived_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify base class was preserved
        let instance = registry.get(instance_hash).unwrap().as_class().unwrap();
        assert_eq!(instance.base_class, Some(base_hash));
    }

    // === Tests for FFI specialization in registry ===

    #[test]
    fn instantiate_template_type_already_in_registry() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        // Pre-register the instance (simulating FFI specialization)
        let int_type = DataType::simple(primitives::INT32);
        let instance_hash = TypeHash::from_template_instance(array_hash, &[primitives::INT32]);

        let pre_registered = ClassEntry::new(
            "array<int>",
            vec![],
            "array<int>",
            instance_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_instance(array_hash, vec![int_type]);

        registry.register_type(pre_registered.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        // Should detect already registered and return that hash
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
        assert_eq!(result.unwrap(), instance_hash);
        // Cache should now have the instance
        assert!(cache.has_type_instance(array_hash, &[primitives::INT32]));
    }

    #[test]
    fn instantiate_template_type_already_in_global_registry() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let mut global_registry = SymbolRegistry::new();

        let array_hash = create_array_template(&mut registry);

        // Pre-register the instance in GLOBAL registry (simulating FFI specialization)
        let int_type = DataType::simple(primitives::INT32);
        let instance_hash = TypeHash::from_template_instance(array_hash, &[primitives::INT32]);

        let pre_registered = ClassEntry::new(
            "array<int>",
            vec![],
            "array<int>",
            instance_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_template_instance(array_hash, vec![int_type]);

        global_registry
            .register_type(pre_registered.into())
            .unwrap();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        // Should detect already registered in global registry and return that hash
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
        assert_eq!(result.unwrap(), instance_hash);
        // Cache should now have the instance
        assert!(cache.has_type_instance(array_hash, &[primitives::INT32]));
    }

    #[test]
    fn instantiate_template_function_already_in_global_registry() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let mut global_registry = SymbolRegistry::new();

        let identity_hash = create_identity_template_function(&mut registry);

        // Pre-register the instance in GLOBAL registry (simulating FFI specialization)
        let int_type = DataType::simple(primitives::INT32);
        let instance_hash = TypeHash::from_template_instance(identity_hash, &[primitives::INT32]);

        let inst_def = FunctionDef::new(
            instance_hash,
            "identity<int>".to_string(),
            vec![],
            vec![Param::new("value", int_type)],
            int_type,
            None,
            FunctionTraits::default(),
            true, // native
            Visibility::Public,
        );
        global_registry
            .register_function(FunctionEntry::ffi(inst_def))
            .unwrap();

        let mut cache = TemplateInstanceCache::new();

        // Should detect already registered in global registry and return that hash
        let result = instantiate_template_function(
            identity_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), instance_hash);
        // Cache should now have the instance
        assert!(cache.has_function_instance(identity_hash, &[primitives::INT32]));
    }

    // === Tests for script function implementation ===

    #[test]
    fn instantiate_template_function_with_script_impl() {
        use angelscript_core::{UnitId, entries::FunctionSource};

        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Create template param T
        let func_hash = TypeHash::from_name("script_func");
        let t_param = TemplateParamEntry::for_template("T", 0, func_hash, "script_func");
        let t_hash = t_param.type_hash;
        registry.register_type(t_param.into()).unwrap();

        // Create script template function
        let func_def = FunctionDef::new_template(
            func_hash,
            "script_func".to_string(),
            vec![],
            vec![Param::new("value", DataType::simple(t_hash))],
            DataType::simple(t_hash),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
            vec![t_hash],
        );

        let func_entry = FunctionEntry::new(
            func_def,
            FunctionImpl::Script {
                unit_id: UnitId::new(42),
            },
            FunctionSource::script(Span::default()),
        );
        registry.register_function(func_entry).unwrap();

        let mut cache = TemplateInstanceCache::new();

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_function(
            func_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify implementation is still Script with same unit_id
        let instance = registry.get_function(instance_hash).unwrap();
        assert!(
            matches!(instance.implementation, FunctionImpl::Script { unit_id } if unit_id == UnitId::new(42))
        );
    }

    // === Test for unknown type error ===

    #[test]
    fn instantiate_template_type_unknown() {
        let mut registry = SymbolRegistry::new();
        let global_registry = SymbolRegistry::new();

        let mut cache = TemplateInstanceCache::new();
        let callbacks = NoOpCallbacks;

        let nonexistent_hash = TypeHash::from_name("nonexistent");
        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_template_type(
            nonexistent_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
            &callbacks,
        );

        assert!(result.is_err());
        matches!(result.unwrap_err(), CompilationError::UnknownType { .. });
    }

    // === Test for child funcdef not a funcdef error ===

    #[test]
    fn instantiate_child_funcdef_not_a_funcdef() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Use a regular class hash instead of a funcdef
        let class_hash = TypeHash::from_name("SomeClass");
        let class = ClassEntry::new(
            "SomeClass",
            vec![],
            "SomeClass",
            class_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(class.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let int_type = DataType::simple(primitives::INT32);

        let result = instantiate_child_funcdef(
            class_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        // Should fail because SomeClass is not a funcdef
        matches!(result.unwrap_err(), CompilationError::UnknownType { .. });
    }

    // === Test for global funcdef (no parent) ===

    #[test]
    fn instantiate_child_funcdef_no_parent() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Create a global funcdef (not a child)
        let funcdef_hash = TypeHash::from_name("GlobalCallback");
        let funcdef = FuncdefEntry::ffi(
            "GlobalCallback",
            vec![DataType::simple(primitives::INT32)],
            DataType::simple(primitives::BOOL),
        );
        registry.register_type(funcdef.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let int_type = DataType::simple(primitives::INT32);

        let result = instantiate_child_funcdef(
            funcdef_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        // Should fail because GlobalCallback has no parent
        matches!(result.unwrap_err(), CompilationError::Internal { .. });
    }

    // === Test for child funcdef parent not a template ===

    #[test]
    fn instantiate_child_funcdef_parent_not_template() {
        let mut registry = SymbolRegistry::new();
        registry.register_all_primitives();
        let global_registry = SymbolRegistry::new();

        // Create a non-template parent class
        let parent_hash = TypeHash::from_name("NonTemplate");
        let parent = ClassEntry::new(
            "NonTemplate",
            vec![],
            "NonTemplate",
            parent_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(parent.into()).unwrap();

        // Create child funcdef with non-template parent
        let callback_hash = TypeHash::from_name("NonTemplate::Callback");
        let callback = FuncdefEntry::new_child(
            "Callback",
            vec![],
            "NonTemplate::Callback",
            callback_hash,
            TypeSource::ffi_untyped(),
            vec![DataType::simple(primitives::INT32)],
            DataType::simple(primitives::BOOL),
            parent_hash,
        );
        registry.register_type(callback.into()).unwrap();

        let mut cache = TemplateInstanceCache::new();
        let int_type = DataType::simple(primitives::INT32);

        let result = instantiate_child_funcdef(
            callback_hash,
            &[int_type],
            Span::default(),
            &mut cache,
            &mut registry,
            &global_registry,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::NotATemplate { name, .. } => {
                assert_eq!(name, "NonTemplate");
            }
            e => panic!("Expected NotATemplate, got {:?}", e),
        }
    }
}
