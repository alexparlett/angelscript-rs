# Task 33: Type Resolution

## Overview

Implement the `TypeResolver` that converts AST `TypeExpr` nodes into semantic `DataType` values. This includes handling primitives, user types, templates, handles, arrays, and const modifiers.

## Goals

1. Resolve simple type names to TypeHash
2. Handle type modifiers (const, handle @, reference &)
3. Instantiate templates with type arguments
4. Support array types
5. Integrate with template instance cache

## Dependencies

- Task 31: Compiler Foundation
- Task 32: Compilation Context

## Files to Create/Modify

```
crates/angelscript-compiler/src/
├── type_resolver.rs       # TypeResolver implementation
├── template/              # Template instantiation
│   ├── mod.rs
│   ├── cache.rs           # Template instance cache
│   ├── substitution.rs    # Type substitution
│   └── instantiation.rs   # Template instantiation
└── lib.rs                 # Add modules
```

## Detailed Implementation

### 1. Type Resolver (type_resolver.rs)

```rust
use angelscript_core::{DataType, RefModifier, Span, TypeHash};
use angelscript_parser::ast::{TypeExpr, TypeSuffix};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};

/// Resolves AST type expressions to semantic DataTypes.
pub struct TypeResolver<'a, 'reg> {
    ctx: &'a mut CompilationContext<'reg>,
}

impl<'a, 'reg> TypeResolver<'a, 'reg> {
    pub fn new(ctx: &'a mut CompilationContext<'reg>) -> Self {
        Self { ctx }
    }

    /// Resolve a TypeExpr to DataType.
    pub fn resolve(&mut self, type_expr: &TypeExpr, span: Span) -> Result<DataType> {
        match type_expr {
            TypeExpr::Simple { name, type_args } => {
                self.resolve_simple(name, type_args.as_deref(), span)
            }

            TypeExpr::Qualified { path, type_args } => {
                self.resolve_qualified(path, type_args.as_deref(), span)
            }

            TypeExpr::Auto => {
                // Auto is resolved later during type inference
                Ok(DataType::auto())
            }

            TypeExpr::Void => Ok(DataType::void()),

            TypeExpr::WithSuffix { base, suffix } => {
                let base_type = self.resolve(base, span)?;
                self.apply_suffix(base_type, suffix, span)
            }

            TypeExpr::Const { inner } => {
                let mut inner_type = self.resolve(inner, span)?;
                inner_type.is_const = true;
                Ok(inner_type)
            }

            TypeExpr::Reference { inner, ref_kind } => {
                let mut inner_type = self.resolve(inner, span)?;
                inner_type.ref_modifier = match ref_kind {
                    ast::RefKind::In => RefModifier::In,
                    ast::RefKind::Out => RefModifier::Out,
                    ast::RefKind::InOut => RefModifier::InOut,
                };
                Ok(inner_type)
            }
        }
    }

    /// Resolve a simple type name.
    fn resolve_simple(
        &mut self,
        name: &str,
        type_args: Option<&[TypeExpr]>,
        span: Span,
    ) -> Result<DataType> {
        // Resolve base type
        let base_hash = self.ctx.resolve_type(name).ok_or_else(|| {
            CompileError::TypeNotFound {
                name: name.to_string(),
                span,
            }
        })?;

        // Handle template arguments
        if let Some(args) = type_args {
            self.instantiate_template(base_hash, args, span)
        } else {
            // Check if this is a template that requires arguments
            if let Some(type_ref) = self.ctx.get_type(base_hash) {
                if type_ref.is_template() && !type_ref.template_params().is_empty() {
                    return Err(CompileError::WrongTemplateArgCount {
                        expected: type_ref.template_params().len(),
                        got: 0,
                        span,
                    });
                }
            }
            Ok(DataType::simple(base_hash))
        }
    }

    /// Resolve a qualified type path.
    fn resolve_qualified(
        &mut self,
        path: &[String],
        type_args: Option<&[TypeExpr]>,
        span: Span,
    ) -> Result<DataType> {
        let path_refs: Vec<&str> = path.iter().map(|s| s.as_str()).collect();
        let base_hash = self.ctx.resolve_qualified_type(&path_refs).ok_or_else(|| {
            CompileError::TypeNotFound {
                name: path.join("::"),
                span,
            }
        })?;

        if let Some(args) = type_args {
            self.instantiate_template(base_hash, args, span)
        } else {
            Ok(DataType::simple(base_hash))
        }
    }

    /// Apply a type suffix (handle, array, const handle).
    fn apply_suffix(
        &mut self,
        mut data_type: DataType,
        suffix: &TypeSuffix,
        span: Span,
    ) -> Result<DataType> {
        match suffix {
            TypeSuffix::Handle => {
                data_type.is_handle = true;
                Ok(data_type)
            }

            TypeSuffix::ConstHandle => {
                data_type.is_handle = true;
                data_type.is_handle_to_const = true;
                Ok(data_type)
            }

            TypeSuffix::Array { size } => {
                // Instantiate array<T> template
                let array_template = self.ctx.resolve_type("array").ok_or_else(|| {
                    CompileError::TypeNotFound {
                        name: "array".to_string(),
                        span,
                    }
                })?;

                let instance = self.find_or_instantiate_template(
                    array_template,
                    vec![data_type],
                    span,
                )?;

                Ok(DataType::simple(instance))
            }

            TypeSuffix::FixedArray { size } => {
                // Fixed-size arrays are value types with known size
                // For now, treat as regular arrays
                self.apply_suffix(data_type, &TypeSuffix::Array { size: None }, span)
            }
        }
    }

    /// Instantiate a template with type arguments.
    fn instantiate_template(
        &mut self,
        template_hash: TypeHash,
        args: &[TypeExpr],
        span: Span,
    ) -> Result<DataType> {
        // Resolve all type arguments
        let resolved_args: Vec<DataType> = args
            .iter()
            .map(|arg| self.resolve(arg, span))
            .collect::<Result<_>>()?;

        let instance_hash = self.find_or_instantiate_template(template_hash, resolved_args, span)?;
        Ok(DataType::simple(instance_hash))
    }

    /// Find or create a template instance.
    fn find_or_instantiate_template(
        &mut self,
        template_hash: TypeHash,
        type_args: Vec<DataType>,
        span: Span,
    ) -> Result<TypeHash> {
        let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();

        // Check cache first
        if let Some(instance) = self.ctx.get_cached_template(template_hash, &arg_hashes) {
            return Ok(instance);
        }

        // Cache miss - need to instantiate
        let instance_hash = crate::template::instantiate_template_type(
            template_hash,
            type_args,
            self.ctx,
            span,
        )?;

        // Cache the result
        self.ctx.cache_template(template_hash, arg_hashes, instance_hash);

        Ok(instance_hash)
    }
}
```

### 2. Template Instantiation (template/instantiation.rs)

```rust
use angelscript_core::{
    ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionImpl,
    Param, PropertyEntry, Span, TypeHash, TypeSource,
};

use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use super::substitution::{build_substitution_map, substitute_type};

/// Instantiate a template type with concrete type arguments.
pub fn instantiate_template_type(
    template_hash: TypeHash,
    type_args: Vec<DataType>,
    ctx: &mut CompilationContext<'_>,
    span: Span,
) -> Result<TypeHash> {
    // 1. Compute instance hash
    let arg_hashes: Vec<TypeHash> = type_args.iter().map(|a| a.type_hash).collect();
    let instance_hash = TypeHash::from_template_instance(template_hash, &arg_hashes);

    // 2. Get template definition
    let template = ctx.get_type(template_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeNotFound {
            name: format!("{:?}", template_hash),
            span,
        })?
        .clone();

    // 3. Validate via callback if registered
    if let Some(validation) = ctx.registry_mut().validate_template(&template_hash, &type_args) {
        if !validation.is_valid {
            return Err(CompileError::TemplateValidationFailed {
                template: template.name.clone(),
                args: format_type_args(&type_args),
                message: validation.error.unwrap_or_default(),
                span,
            });
        }
    }

    // 4. Build substitution map
    let subst_map = build_substitution_map(&template.template_params, &type_args);

    // 5. Create instance entry
    let instance_name = format_template_instance_name(&template.name, &type_args);
    let mut instance = ClassEntry::new(
        instance_name.clone(),
        instance_name,
        instance_hash,
        template.type_kind,
        TypeSource::script(ctx.unit_id(), span),
    )
    .with_template_instance(template_hash, type_args.clone());

    // 6. Substitute base class
    if let Some(base) = template.base_class {
        let subst_base = substitute_type(DataType::simple(base), &subst_map)?;
        instance = instance.with_base(subst_base.type_hash);
    }

    // 7. Instantiate methods
    for method_hash in &template.methods {
        let inst_method = instantiate_method(*method_hash, &subst_map, instance_hash, ctx, span)?;
        instance.methods.push(inst_method);
    }

    // 8. Instantiate properties
    for prop in &template.properties {
        let inst_prop = PropertyEntry {
            name: prop.name.clone(),
            data_type: substitute_type(prop.data_type, &subst_map)?,
            visibility: prop.visibility,
            getter: prop.getter,
            setter: prop.setter,
        };
        instance.properties.push(inst_prop);
    }

    // 9. Register instance
    ctx.registry_mut().register_type(instance.into())?;

    Ok(instance_hash)
}

/// Instantiate a method for a template instance.
fn instantiate_method(
    method_hash: TypeHash,
    subst_map: &rustc_hash::FxHashMap<TypeHash, DataType>,
    parent_instance: TypeHash,
    ctx: &mut CompilationContext<'_>,
    span: Span,
) -> Result<TypeHash> {
    let method = ctx.registry_mut().get_function(method_hash)
        .ok_or_else(|| CompileError::Internal {
            message: format!("Method {:?} not found", method_hash),
        })?
        .clone();

    // Substitute param types
    let inst_params: Vec<Param> = method.def.params
        .iter()
        .map(|p| Ok(Param {
            name: p.name.clone(),
            data_type: substitute_type(p.data_type, subst_map)?,
            default_value: p.default_value.clone(),
        }))
        .collect::<Result<_>>()?;

    let inst_return = substitute_type(method.def.return_type, subst_map)?;

    // Compute instance method hash
    let param_hashes: Vec<TypeHash> = inst_params.iter().map(|p| p.data_type.type_hash).collect();
    let instance_hash = TypeHash::from_method(
        parent_instance,
        &method.def.name,
        &param_hashes,
        method.def.traits.is_const,
    );

    // Check if already exists
    if ctx.registry_mut().get_function(instance_hash).is_some() {
        return Ok(instance_hash);
    }

    // Create instance definition
    let inst_def = FunctionDef::new(
        instance_hash,
        method.def.name.clone(),
        method.def.namespace.clone(),
        inst_params,
        inst_return,
        Some(parent_instance),
        method.def.traits.clone(),
        true,
        method.def.visibility,
    );

    let inst_impl = match &method.implementation {
        FunctionImpl::Native(native) => FunctionImpl::Native(native.clone()),
        FunctionImpl::Script { unit_id, .. } => FunctionImpl::Script {
            unit_id: *unit_id,
            bytecode: None,
        },
        other => other.clone(),
    };

    let inst_entry = FunctionEntry::new(inst_def, inst_impl, method.source.clone());
    ctx.registry_mut().register_function(inst_entry)?;

    Ok(instance_hash)
}

fn format_template_instance_name(template: &str, args: &[DataType]) -> String {
    let args_str = args.iter()
        .map(|a| format!("{:?}", a.type_hash))  // TODO: proper type name lookup
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{}>", template, args_str)
}

fn format_type_args(args: &[DataType]) -> String {
    args.iter()
        .map(|a| format!("{:?}", a.type_hash))
        .collect::<Vec<_>>()
        .join(", ")
}
```

### 3. Type Substitution (template/substitution.rs)

```rust
use angelscript_core::{DataType, TypeHash};
use rustc_hash::FxHashMap;

use crate::error::Result;

/// Map from template parameter hash to concrete type.
pub type SubstitutionMap = FxHashMap<TypeHash, DataType>;

/// Build a substitution map from template params to concrete types.
pub fn build_substitution_map(
    template_params: &[TypeHash],
    type_args: &[DataType],
) -> SubstitutionMap {
    template_params
        .iter()
        .zip(type_args.iter())
        .map(|(param, arg)| (*param, *arg))
        .collect()
}

/// Substitute a type through the map.
pub fn substitute_type(data_type: DataType, subst_map: &SubstitutionMap) -> Result<DataType> {
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        Ok(DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const || replacement.is_handle_to_const,
            ref_modifier: data_type.ref_modifier,
        })
    } else {
        Ok(data_type)
    }
}

/// Substitute with if_handle_then_const flag.
pub fn substitute_type_with_flags(
    data_type: DataType,
    subst_map: &SubstitutionMap,
    if_handle_then_const: bool,
) -> Result<DataType> {
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        Ok(DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const
                || replacement.is_handle_to_const
                || (if_handle_then_const && replacement.is_handle && data_type.is_const),
            ref_modifier: data_type.ref_modifier,
        })
    } else {
        Ok(data_type)
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_primitive() {
        let mut registry = TypeRegistry::new();
        registry.register_primitives();
        let mut ctx = CompilationContext::new(&mut registry, UnitId::new(0));

        let mut resolver = TypeResolver::new(&mut ctx);
        let type_expr = TypeExpr::Simple { name: "int".to_string(), type_args: None };
        let result = resolver.resolve(&type_expr, Span::default());

        assert!(result.is_ok());
        assert_eq!(result.unwrap().type_hash, primitives::INT32);
    }

    #[test]
    fn resolve_with_handle() {
        // Test Player@ resolves to handle type
    }

    #[test]
    fn resolve_const_handle() {
        // Test const Player@ resolves correctly
    }

    #[test]
    fn resolve_template() {
        // Test array<int> resolves and instantiates
    }
}
```

## Acceptance Criteria

- [ ] Simple type names resolve correctly
- [ ] Qualified paths (foo::bar::Type) resolve
- [ ] Handle suffix (@) sets is_handle flag
- [ ] Const modifier sets is_const flag
- [ ] Reference modifiers (&in, &out, &inout) set ref_modifier
- [ ] Templates instantiate with type arguments
- [ ] Template instances are cached
- [ ] Array types create array<T> instances
- [ ] All tests pass

## Next Phase

Task 35: Conversion System - type conversion rules and costs
