# Task 35: Template Instantiation

## Overview

Implement the `TemplateInstantiator` struct that handles template instantiation for types, functions, and child funcdefs. This is a separate struct for isolation and testability. All template instances go into the **global registry**.

## Goals

1. Create `TemplateInstantiator` struct for isolated, testable instantiation logic
2. Instantiate template types (e.g., `array<int>` from `array<T>`)
3. Instantiate template functions (e.g., `identity<int>` from `identity<T>`)
4. Instantiate child funcdefs (e.g., `array<int>::Callback`)
5. Template instance cache with FFI specialization priority
6. Nested template instantiation

## Architecture

Template instances are **shared across all units** and go into the global registry:

```
┌────────────────────────────────────┐
│ Global TypeRegistry                │  ← Template instances stored here
│ - FFI types                        │
│ - FFI specializations (pre-cached) │
│ - Script template instances        │
└────────────────────────────────────┘
             ↑
             │
    TemplateInstantiator
             │
             ▼
┌────────────────────────────────────┐
│ Per-Unit TypeRegistry              │  ← Non-shared script types
└────────────────────────────────────┘
```

## Dependencies

- Task 33: Compilation Context (layered registry)
- Task 34: Type Resolution (resolving type arguments)

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── template/
│   ├── mod.rs                    # Template module
│   ├── instantiation.rs          # Core instantiation logic
│   ├── substitution.rs           # Type substitution
│   ├── cache.rs                  # Template instance cache
│   └── validation.rs             # Template validation callbacks
└── context.rs                    # Add template cache integration
```

## Detailed Implementation

### Template Module (template/mod.rs)

```rust
//! Template instantiation system.
//!
//! Provides the `TemplateInstantiator` struct for isolated, testable template instantiation.
//! All template instances go into the global registry.
//! Integrates with FFI specializations via the template instance cache.

mod instantiator;
mod substitution;
mod cache;
mod validation;

pub use instantiator::TemplateInstantiator;
pub use substitution::*;
pub use cache::*;
pub use validation::*;
```

### TemplateInstantiator (template/instantiator.rs)

```rust
use angelscript_core::{DataType, TypeHash, TypeEntry};
use angelscript_registry::TypeRegistry;

use crate::error::{CompileError, Result};
use super::substitution::{build_substitution_map, substitute_type};
use super::cache::TemplateInstanceCache;

/// Handles template instantiation logic - isolated for testability.
/// All template instances go into the global registry.
pub struct TemplateInstantiator<'a> {
    /// Global registry where instances are stored
    global_registry: &'a TypeRegistry,
    /// Cache for template instances (includes FFI specializations)
    cache: &'a TemplateInstanceCache,
}

impl<'a> TemplateInstantiator<'a> {
    pub fn new(global_registry: &'a TypeRegistry, cache: &'a TemplateInstanceCache) -> Self {
        Self { global_registry, cache }
    }

    /// Instantiate a template type with the given type arguments.
    /// Returns the instance hash (cached if already instantiated).
    pub fn instantiate_type(
        &self,
        template_hash: TypeHash,
        type_args: &[TypeHash],
    ) -> Result<TypeHash> {
        let instance_hash = TypeHash::from_template_instance(template_hash, type_args);

        // 1. Check cache - includes FFI specializations
        if let Some(cached) = self.cache.get_type_instance(template_hash, type_args) {
            return Ok(cached);
        }

        // 2. Check if already in registry (e.g., FFI specialization not in cache)
        if self.global_registry.contains_type(instance_hash) {
            return Ok(instance_hash);
        }

        // 3. Create and register instance
        let instance = self.create_type_instance(template_hash, type_args)?;
        self.global_registry.register_type(instance)?;

        Ok(instance_hash)
    }

    /// Instantiate a template function with the given type arguments.
    pub fn instantiate_function(
        &self,
        func_hash: TypeHash,
        type_args: &[TypeHash],
    ) -> Result<TypeHash> {
        let instance_hash = TypeHash::from_function_instance(func_hash, type_args);

        // Check cache
        if let Some(cached) = self.cache.get_function_instance(func_hash, type_args) {
            return Ok(cached);
        }

        // Check if already exists
        if self.global_registry.contains_function(instance_hash) {
            return Ok(instance_hash);
        }

        // Create and register
        let instance = self.create_function_instance(func_hash, type_args)?;
        self.global_registry.register_function(instance)?;

        Ok(instance_hash)
    }

    fn create_type_instance(
        &self,
        template_hash: TypeHash,
        type_args: &[TypeHash],
    ) -> Result<TypeEntry> {
        // Get template definition
        let template = self.global_registry.get(template_hash)
            .and_then(|t| t.as_class())
            .ok_or_else(|| CompileError::TypeNotFound {
                name: format!("{:?}", template_hash),
                span: Default::default(),
            })?;

        if !template.is_template() {
            return Err(CompileError::NotATemplate {
                name: template.name.clone(),
                span: Default::default(),
            });
        }

        // Build substitution map and create instance
        // ... (See detailed algorithm in original implementation)
        todo!("Create template instance entry")
    }

    fn create_function_instance(
        &self,
        func_hash: TypeHash,
        type_args: &[TypeHash],
    ) -> Result<FunctionEntry> {
        todo!("Create function instance entry")
    }
}
```

### Template Instance Cache (template/cache.rs)

```rust
use angelscript_core::TypeHash;
use rustc_hash::FxHashMap;

/// Cache for template instances.
///
/// Maps (template_hash, type_args) → instance_hash.
/// Pre-populated with FFI specializations during Context setup.
#[derive(Debug, Default)]
pub struct TemplateInstanceCache {
    /// Type template instances: (template, args) → instance
    type_instances: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
    /// Function template instances: (template, args) → instance
    function_instances: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
}

impl TemplateInstanceCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a type template instance.
    /// Called during FFI registration for specializations and during compilation.
    pub fn cache_type_instance(
        &mut self,
        template: TypeHash,
        args: Vec<TypeHash>,
        instance: TypeHash,
    ) {
        self.type_instances.insert((template, args), instance);
    }

    /// Look up a cached type instance.
    pub fn get_type_instance(
        &self,
        template: TypeHash,
        args: &[TypeHash],
    ) -> Option<TypeHash> {
        self.type_instances.get(&(template, args.to_vec())).copied()
    }

    /// Register a function template instance.
    pub fn cache_function_instance(
        &mut self,
        template: TypeHash,
        args: Vec<TypeHash>,
        instance: TypeHash,
    ) {
        self.function_instances.insert((template, args), instance);
    }

    /// Look up a cached function instance.
    pub fn get_function_instance(
        &self,
        template: TypeHash,
        args: &[TypeHash],
    ) -> Option<TypeHash> {
        self.function_instances.get(&(template, args.to_vec())).copied()
    }

    /// Check if a type instance exists (FFI specialization or previous instantiation).
    pub fn has_type_instance(&self, template: TypeHash, args: &[TypeHash]) -> bool {
        self.type_instances.contains_key(&(template, args.to_vec()))
    }
}
```

### Type Substitution (template/substitution.rs)

```rust
use angelscript_core::{DataType, TypeHash};
use rustc_hash::FxHashMap;

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};

/// Map from template parameter hash to concrete type.
pub type SubstitutionMap = FxHashMap<TypeHash, DataType>;

/// Build a substitution map from template parameters and type arguments.
pub fn build_substitution_map(
    template_params: &[TypeHash],
    type_args: &[DataType],
) -> Result<SubstitutionMap> {
    if template_params.len() != type_args.len() {
        return Err(CompileError::TemplateArgCountMismatch {
            expected: template_params.len(),
            got: type_args.len(),
        });
    }

    let mut map = FxHashMap::default();
    for (param_hash, arg) in template_params.iter().zip(type_args.iter()) {
        map.insert(*param_hash, *arg);
    }
    Ok(map)
}

/// Substitute template parameters in a type.
pub fn substitute_type(
    data_type: DataType,
    subst_map: &SubstitutionMap,
    ctx: &CompilationContext,
) -> Result<DataType> {
    // Check if this is a template parameter
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        // Apply modifiers from original to replacement
        return Ok(DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const || replacement.is_handle_to_const,
            ref_modifier: data_type.ref_modifier,
        });
    }

    // Check if this is a template instance that needs recursive substitution
    if let Some(type_entry) = ctx.get_type(data_type.type_hash) {
        if let Some(class) = type_entry.as_class() {
            if class.is_template_instance() {
                // Recursively substitute nested template args
                let new_args: Vec<DataType> = class.type_args.iter()
                    .map(|arg| substitute_type(*arg, subst_map, ctx))
                    .collect::<Result<_>>()?;

                // Re-instantiate with substituted args
                let template_hash = class.template.unwrap();
                let new_instance = ctx.instantiate_template_type(template_hash, new_args)?;
                return Ok(data_type.with_type_hash(new_instance));
            }
        }
    }

    // Not a template param, return unchanged
    Ok(data_type)
}

/// Substitute template parameters in function parameters.
pub fn substitute_params(
    params: &[Param],
    subst_map: &SubstitutionMap,
    ctx: &CompilationContext,
) -> Result<Vec<Param>> {
    params.iter()
        .map(|p| Ok(Param {
            name: p.name.clone(),
            data_type: substitute_type(p.data_type, subst_map, ctx)?,
            default_value: p.default_value.clone(),
        }))
        .collect()
}

/// Substitute with if_handle_then_const flag support.
pub fn substitute_type_with_flags(
    data_type: DataType,
    subst_map: &SubstitutionMap,
    if_handle_then_const: bool,
    ctx: &CompilationContext,
) -> Result<DataType> {
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        let result = DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const
                || replacement.is_handle_to_const
                || (if_handle_then_const && replacement.is_handle && data_type.is_const),
            ref_modifier: data_type.ref_modifier,
        };
        return Ok(result);
    }

    substitute_type(data_type, subst_map, ctx)
}
```

### Template Instantiation (template/instantiation.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_core::entries::{ClassEntry, FunctionEntry, FuncdefEntry, PropertyEntry};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use super::substitution::{build_substitution_map, substitute_type, substitute_params, SubstitutionMap};
use super::validation::validate_template_instance;

/// Instantiate a template type with concrete type arguments.
///
/// Returns the hash of the instantiated type.
/// Uses cache to avoid duplicate instantiation and respect FFI specializations.
pub fn instantiate_template_type(
    ctx: &mut CompilationContext,
    template_hash: TypeHash,
    type_args: Vec<DataType>,
    span: Span,
) -> Result<TypeHash> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);

    // 2. Check cache - includes FFI specializations and previous instantiations
    if let Some(cached) = ctx.template_cache().get_type_instance(template_hash, &arg_hashes) {
        return Ok(cached);
    }

    // 3. Get template definition
    let template = ctx.get_type(template_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeNotFound {
            name: format!("{:?}", template_hash),
            span,
        })?;

    if !template.is_template() {
        return Err(CompileError::NotATemplate {
            name: template.name.clone(),
            span,
        });
    }

    // 4. Validate via callback (if registered)
    validate_template_instance(ctx, template_hash, &type_args, span)?;

    // 5. Build substitution map
    let subst_map = build_substitution_map(&template.template_params, &type_args)?;

    // 6. Create instance entry
    let instance_name = format_template_instance_name(&template.name, &type_args);

    let mut instance = ClassEntry::new(
        instance_name.clone(),
        instance_name.clone(),
        instance_hash,
        template.type_kind,
        template.source.clone(),
    )
    .with_template_instance(template_hash, type_args.clone());

    // 7. Substitute base class
    if let Some(base) = template.base_class {
        let base_type = DataType::simple(base);
        let substituted_base = substitute_type(base_type, &subst_map, ctx)?;
        instance = instance.with_base(Some(substituted_base.type_hash));
    }

    // 8. Instantiate and substitute behaviors
    instance.behaviors = substitute_behaviors(&template.behaviors, &subst_map, ctx)?;

    // 9. Instantiate methods
    for method_hash in &template.methods {
        let instantiated = instantiate_method_for_type(
            ctx,
            *method_hash,
            &subst_map,
            instance_hash,
            span,
        )?;
        instance.methods.push(instantiated);
    }

    // 10. Instantiate properties
    for prop in &template.properties {
        let inst_prop = PropertyEntry {
            name: prop.name.clone(),
            data_type: substitute_type(prop.data_type, &subst_map, ctx)?,
            visibility: prop.visibility,
            getter: substitute_optional_func(prop.getter, &subst_map, instance_hash, ctx)?,
            setter: substitute_optional_func(prop.setter, &subst_map, instance_hash, ctx)?,
        };
        instance.properties.push(inst_prop);
    }

    // 11. Register instance
    ctx.register_type(instance.into())?;

    // 12. Cache the instance
    ctx.template_cache_mut().cache_type_instance(template_hash, arg_hashes, instance_hash);

    Ok(instance_hash)
}

/// Instantiate a template function with concrete type arguments.
pub fn instantiate_template_function(
    ctx: &mut CompilationContext,
    func_hash: TypeHash,
    type_args: Vec<DataType>,
    span: Span,
) -> Result<TypeHash> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_function_instance(func_hash, &arg_hashes);

    // 2. Check cache
    if let Some(cached) = ctx.template_cache().get_function_instance(func_hash, &arg_hashes) {
        return Ok(cached);
    }

    // 3. Get template definition
    let template = ctx.get_function(func_hash)
        .ok_or_else(|| CompileError::FunctionNotFound {
            name: format!("{:?}", func_hash),
            span,
        })?;

    if template.def.template_params.is_empty() {
        return Err(CompileError::NotATemplate {
            name: template.def.name.clone(),
            span,
        });
    }

    // 4. Build substitution map
    let subst_map = build_substitution_map(&template.def.template_params, &type_args)?;

    // 5. Substitute param types and return type
    let inst_params = substitute_params(&template.def.params, &subst_map, ctx)?;
    let inst_return = substitute_type(template.def.return_type, &subst_map, ctx)?;

    // 6. Create instance
    let inst_name = format!("{}<{}>", template.def.name, format_type_args(&type_args));
    let inst_def = FunctionDef::new(
        instance_hash,
        inst_name,
        template.def.namespace.clone(),
        inst_params,
        inst_return,
        template.def.object_type,
        template.def.traits.clone(),
        true,  // is_instantiated
        template.def.visibility,
    );

    // 7. Handle implementation
    let inst_impl = match &template.implementation {
        FunctionImpl::Native(native) => {
            // FFI template - implementation handles any type via generic calling convention
            FunctionImpl::Native(native.clone())
        }
        FunctionImpl::Script { unit_id, .. } => {
            // Script template - bytecode generated during compilation pass
            FunctionImpl::Script { unit_id: *unit_id, bytecode: None }
        }
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, template.source.clone());
    ctx.register_function(inst_entry)?;

    // 8. Cache the instance
    ctx.template_cache_mut().cache_function_instance(func_hash, arg_hashes, instance_hash);

    Ok(instance_hash)
}

/// Instantiate a child funcdef (e.g., `array<int>::Callback`).
pub fn instantiate_child_funcdef(
    ctx: &mut CompilationContext,
    funcdef_hash: TypeHash,
    parent_type_args: Vec<DataType>,
    span: Span,
) -> Result<TypeHash> {
    let funcdef = ctx.get_type(funcdef_hash)
        .and_then(|t| t.as_funcdef())
        .ok_or_else(|| CompileError::TypeNotFound {
            name: format!("{:?}", funcdef_hash),
            span,
        })?;

    let parent_hash = funcdef.parent_type
        .ok_or_else(|| CompileError::Internal {
            message: "Child funcdef must have parent".to_string(),
        })?;

    // Get parent template's param hashes
    let parent_template = ctx.get_type(parent_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeNotFound {
            name: format!("{:?}", parent_hash),
            span,
        })?;

    if !parent_template.is_template() {
        return Err(CompileError::NotATemplate {
            name: parent_template.name.clone(),
            span,
        });
    }

    // Build substitution map from parent's template params
    let subst_map = build_substitution_map(&parent_template.template_params, &parent_type_args)?;

    // Compute instance hash
    let arg_hashes: Vec<TypeHash> = parent_type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(funcdef_hash, &arg_hashes);

    // Check cache
    if ctx.get_type(instance_hash).is_some() {
        return Ok(instance_hash);
    }

    // Substitute params and return type
    let inst_params: Vec<DataType> = funcdef.params.iter()
        .map(|p| substitute_type(*p, &subst_map, ctx))
        .collect::<Result<_>>()?;
    let inst_return = substitute_type(funcdef.return_type, &subst_map, ctx)?;

    // Create instance name (e.g., "array<int>::Callback")
    let parent_instance_name = format_template_instance_name(&parent_template.name, &parent_type_args);
    let inst_name = format!("{}::{}", parent_instance_name, funcdef.name);

    // Get or create instantiated parent
    let parent_instance_hash = instantiate_template_type(ctx, parent_hash, parent_type_args.clone(), span)?;

    // Create funcdef instance
    let inst_entry = FuncdefEntry::new_child(
        funcdef.name.clone(),
        inst_name,
        instance_hash,
        funcdef.source.clone(),
        inst_params,
        inst_return,
        parent_instance_hash,
    );

    ctx.register_type(inst_entry.into())?;

    Ok(instance_hash)
}

/// Instantiate a method for a specific template type instance.
fn instantiate_method_for_type(
    ctx: &mut CompilationContext,
    method_hash: TypeHash,
    subst_map: &SubstitutionMap,
    parent_instance_hash: TypeHash,
    span: Span,
) -> Result<TypeHash> {
    let method = ctx.get_function(method_hash)
        .ok_or_else(|| CompileError::FunctionNotFound {
            name: format!("{:?}", method_hash),
            span,
        })?;

    // Compute instance method hash
    let inst_params = substitute_params(&method.def.params, subst_map, ctx)?;
    let param_hashes: Vec<TypeHash> = inst_params.iter().map(|p| p.data_type.type_hash).collect();

    let instance_method_hash = TypeHash::from_method(
        parent_instance_hash,
        &method.def.name,
        &param_hashes,
        method.def.traits.is_const,
    );

    // Check if already instantiated
    if ctx.get_function(instance_method_hash).is_some() {
        return Ok(instance_method_hash);
    }

    // Create instantiated method
    let inst_return = substitute_type(method.def.return_type, subst_map, ctx)?;

    let inst_def = FunctionDef::new(
        instance_method_hash,
        method.def.name.clone(),
        method.def.namespace.clone(),
        inst_params,
        inst_return,
        Some(parent_instance_hash),
        method.def.traits.clone(),
        true,
        method.def.visibility,
    );

    let inst_impl = match &method.implementation {
        FunctionImpl::Native(native) => FunctionImpl::Native(native.clone()),
        FunctionImpl::Script { unit_id, .. } => {
            FunctionImpl::Script { unit_id: *unit_id, bytecode: None }
        }
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, method.source.clone());
    ctx.register_function(inst_entry)?;

    Ok(instance_method_hash)
}

/// Substitute behaviors, instantiating method references.
fn substitute_behaviors(
    behaviors: &TypeBehaviors,
    subst_map: &SubstitutionMap,
    ctx: &CompilationContext,
) -> Result<TypeBehaviors> {
    // For now, just copy - method hashes will be updated when methods are instantiated
    Ok(behaviors.clone())
}

fn substitute_optional_func(
    func: Option<TypeHash>,
    subst_map: &SubstitutionMap,
    parent_hash: TypeHash,
    ctx: &CompilationContext,
) -> Result<Option<TypeHash>> {
    // Property getters/setters are updated when the parent type is fully instantiated
    Ok(func)
}

/// Format template instance name: "array<int>" or "dict<string, int>".
fn format_template_instance_name(base_name: &str, type_args: &[DataType]) -> String {
    let args_str = format_type_args(type_args);
    format!("{}<{}>", base_name, args_str)
}

/// Format type arguments as comma-separated string.
fn format_type_args(type_args: &[DataType]) -> String {
    type_args.iter()
        .map(|t| format!("{:?}", t.type_hash))  // TODO: Use proper type names
        .collect::<Vec<_>>()
        .join(", ")
}
```

### Template Validation (template/validation.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};

/// Validate template instantiation via registered callback.
pub fn validate_template_instance(
    ctx: &CompilationContext,
    template_hash: TypeHash,
    type_args: &[DataType],
    span: Span,
) -> Result<()> {
    // Check if a validation callback is registered for this template
    if !ctx.has_template_callback(template_hash) {
        return Ok(());
    }

    // Build validation info
    let template = ctx.get_type(template_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::Internal {
            message: "Template not found for validation".to_string(),
        })?;

    let info = TemplateInstanceInfo {
        template_name: template.name.clone(),
        sub_types: type_args.to_vec(),
    };

    // Call validation callback
    let validation = ctx.validate_template_instance(template_hash, &info);

    if !validation.is_valid {
        return Err(CompileError::TemplateValidationFailed {
            template: template.name.clone(),
            args: type_args.to_vec(),
            message: validation.error.unwrap_or_default(),
            span,
        });
    }

    Ok(())
}

/// Information passed to template validation callbacks.
#[derive(Debug, Clone)]
pub struct TemplateInstanceInfo {
    pub template_name: String,
    pub sub_types: Vec<DataType>,
}

/// Result of template validation.
#[derive(Debug, Clone)]
pub struct TemplateValidation {
    pub is_valid: bool,
    pub error: Option<String>,
    pub needs_gc: bool,
}

impl TemplateValidation {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error: None,
            needs_gc: false,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error: Some(message.into()),
            needs_gc: false,
        }
    }

    pub fn with_gc(mut self) -> Self {
        self.needs_gc = true;
        self
    }
}
```

### Context Integration (update context.rs)

```rust
// Add to CompilationContext:

impl<'ctx> CompilationContext<'ctx> {
    /// Get the template instance cache.
    pub fn template_cache(&self) -> &TemplateInstanceCache {
        &self.template_cache
    }

    /// Get mutable template instance cache.
    pub fn template_cache_mut(&mut self) -> &mut TemplateInstanceCache {
        &mut self.template_cache
    }

    /// Instantiate a template type.
    pub fn instantiate_template_type(
        &mut self,
        template_hash: TypeHash,
        type_args: Vec<DataType>,
    ) -> Result<TypeHash> {
        instantiate_template_type(self, template_hash, type_args, Span::default())
    }

    /// Instantiate a template function.
    pub fn instantiate_template_function(
        &mut self,
        func_hash: TypeHash,
        type_args: Vec<DataType>,
    ) -> Result<TypeHash> {
        instantiate_template_function(self, func_hash, type_args, Span::default())
    }

    /// Check if a template validation callback exists.
    pub fn has_template_callback(&self, template_hash: TypeHash) -> bool {
        self.registry.has_template_callback(template_hash)
    }

    /// Validate a template instance.
    pub fn validate_template_instance(
        &self,
        template_hash: TypeHash,
        info: &TemplateInstanceInfo,
    ) -> TemplateValidation {
        self.registry.validate_template_instance(template_hash, info)
            .unwrap_or_else(TemplateValidation::valid)
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instantiate_simple_template() {
        // array<T> → array<int>
        let mut ctx = test_context_with_array_template();

        let int_type = DataType::simple(primitives::INT32);
        let result = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        );

        assert!(result.is_ok());
        let instance_hash = result.unwrap();

        // Verify instance exists
        let instance = ctx.get_type(instance_hash).unwrap();
        assert!(instance.as_class().unwrap().is_template_instance());
    }

    #[test]
    fn cache_prevents_duplicate_instantiation() {
        let mut ctx = test_context_with_array_template();

        let int_type = DataType::simple(primitives::INT32);

        let hash1 = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        ).unwrap();

        let hash2 = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        ).unwrap();

        // Same hash returned
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn ffi_specialization_takes_priority() {
        let mut ctx = test_context_with_int_array_specialization();

        let int_type = DataType::simple(primitives::INT32);
        let result = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        ).unwrap();

        // Should return FFI specialization, not create new instance
        let instance = ctx.get_type(result).unwrap().as_class().unwrap();
        assert!(instance.source.is_ffi());
    }

    #[test]
    fn nested_template_instantiation() {
        // array<array<int>>
        let mut ctx = test_context_with_array_template();

        let int_type = DataType::simple(primitives::INT32);
        let inner = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        ).unwrap();

        let outer = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![DataType::simple(inner)],
        ).unwrap();

        assert!(ctx.get_type(outer).is_some());
    }

    #[test]
    fn instantiate_template_function() {
        // identity<T>(T) → identity<int>(int)
        let mut ctx = test_context_with_identity_template();

        let int_type = DataType::simple(primitives::INT32);
        let result = ctx.instantiate_template_function(
            TypeHash::from_name("identity"),
            vec![int_type],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn instantiate_child_funcdef() {
        // array<T>::Callback → array<int>::Callback
        let mut ctx = test_context_with_array_callback();

        let int_type = DataType::simple(primitives::INT32);
        let result = instantiate_child_funcdef(
            &mut ctx,
            TypeHash::from_name("array::Callback"),
            vec![int_type],
            Span::default(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn substitution_preserves_modifiers() {
        // const T& with T=int@ → const int@ &
        let mut ctx = test_context();
        let subst_map = build_substitution_map(
            &[TypeHash::from_name("T")],
            &[DataType::handle(primitives::INT32)],
        ).unwrap();

        let input = DataType {
            type_hash: TypeHash::from_name("T"),
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::In,
        };

        let result = substitute_type(input, &subst_map, &ctx).unwrap();

        assert!(result.is_const);
        assert!(result.is_handle);
        assert_eq!(result.ref_modifier, RefModifier::In);
    }

    #[test]
    fn template_validation_callback() {
        let mut ctx = test_context_with_validated_template();

        // Valid instantiation
        let result = ctx.instantiate_template_type(
            TypeHash::from_name("ValidatedTemplate"),
            vec![DataType::simple(primitives::INT32)],
        );
        assert!(result.is_ok());

        // Invalid instantiation (callback rejects void)
        let result = ctx.instantiate_template_type(
            TypeHash::from_name("ValidatedTemplate"),
            vec![DataType::void()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn method_instantiation() {
        // array<T>::push(T) → array<int>::push(int)
        let mut ctx = test_context_with_array_template();

        let int_type = DataType::simple(primitives::INT32);
        let array_int = ctx.instantiate_template_type(
            TypeHash::from_name("array"),
            vec![int_type],
        ).unwrap();

        // Check push method exists with correct signature
        let class = ctx.get_type(array_int).unwrap().as_class().unwrap();
        assert!(!class.methods.is_empty());

        let push_method = class.methods.iter()
            .find(|m| ctx.get_function(*m).unwrap().def.name == "push")
            .unwrap();

        let push = ctx.get_function(*push_method).unwrap();
        assert_eq!(push.def.params[0].data_type.type_hash, primitives::INT32);
    }
}
```

## Acceptance Criteria

- [ ] Template types instantiate correctly
- [ ] Template functions instantiate correctly
- [ ] Child funcdefs instantiate correctly
- [ ] Cache prevents duplicate instantiation
- [ ] FFI specializations take priority over generic instantiation
- [ ] Nested templates work (e.g., `array<array<int>>`)
- [ ] Type substitution preserves modifiers (const, handle, ref)
- [ ] Template validation callbacks invoked
- [ ] Methods of template types get correct substituted signatures
- [ ] All tests pass

## Next Phase

Task 36: Conversion System (type conversions with costs)
