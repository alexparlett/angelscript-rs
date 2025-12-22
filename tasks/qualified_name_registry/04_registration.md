# Phase 4: Registration Pass Updates

## Overview

Update the Registration pass to store `UnresolvedType` instead of immediately resolving types. No `TypeResolver` usage during registration - all type resolution deferred to Completion pass.

**Files:**
- `crates/angelscript-compiler/src/passes/registration.rs` (update)
- `crates/angelscript-compiler/src/type_resolver.rs` (move usage to completion)

---

## Key Changes

### Remove TypeResolver Usage

Currently, `resolve_params()` and return type resolution use `TypeResolver` during registration:

```rust
// BEFORE: Resolves types immediately (fails on forward refs)
fn resolve_params(&mut self, params: &[FunctionParam<'_>]) -> Vec<Param> {
    let mut resolver = TypeResolver::new(self.ctx);
    params.iter().map(|p| {
        let data_type = resolver.resolve(&p.param_type.ty).unwrap_or(DataType::void());
        Param::new(p.name.name, data_type)
    }).collect()
}
```

New approach: Store `UnresolvedParam` with raw type info:

```rust
// AFTER: Stores unresolved types (resolved in completion)
fn collect_params(&mut self, params: &[FunctionParam<'_>]) -> Vec<UnresolvedParam> {
    params.iter().map(|p| {
        let param_type = self.collect_type(&p.param_type);
        UnresolvedParam::new(p.name.name, param_type)
            .with_has_default(p.default_value.is_some())
    }).collect()
}

fn collect_type(&self, ty: &TypeExpr<'_>) -> UnresolvedType {
    UnresolvedType::with_context(
        self.type_expr_to_string(&ty.ty),
        self.current_namespace_vec(),
        self.ctx.imports().to_vec(),
    )
    .with_const(ty.is_const)
    .with_handle(ty.is_handle)
    .with_handle_to_const(ty.is_handle_to_const)
    .with_ref_modifier(ty.ref_modifier)
}
```

---

## Updated RegistrationPass Structure

```rust
// crates/angelscript-compiler/src/passes/registration.rs

use angelscript_core::{
    QualifiedName, UnresolvedType, UnresolvedParam, UnresolvedSignature,
    ClassEntry, InterfaceEntry, FuncdefEntry, FunctionDef, FunctionEntry,
    CompilationError, Span, TypeKind, TypeSource, UnitId, Visibility,
};

/// Pass 1: Register all types and function signatures.
///
/// This pass walks the AST and registers type declarations with UNRESOLVED
/// type references. Type resolution is deferred to the Completion pass.
pub struct RegistrationPass<'a, 'reg> {
    ctx: &'a mut CompilationContext<'reg>,
    unit_id: UnitId,
    types_registered: usize,
    functions_registered: usize,
    globals_registered: usize,
    next_global_slot: u32,
    /// All script classes registered (for auto-generation).
    registered_classes: Vec<QualifiedName>,
    /// Tracks special member traits per class.
    class_traits: FxHashMap<QualifiedName, ClassTraits>,
}
```

---

## Class Registration

```rust
impl<'a, 'reg> RegistrationPass<'a, 'reg> {
    fn visit_class(&mut self, class: &ClassDecl<'_>) {
        let name = class.name.name.to_string();
        let namespace = self.current_namespace_vec();
        let qualified_name = QualifiedName::new(&name, namespace.clone());

        // Collect UNRESOLVED inheritance (not resolved yet)
        let mut class_entry = ClassEntry::new(
            qualified_name.clone(),
            TypeKind::ScriptObject,
            TypeSource::script(self.unit_id, class.span),
        );

        // Store inheritance as UnresolvedType
        for inherit in &class.inheritance {
            let unresolved = UnresolvedType::with_context(
                self.ident_expr_to_string(inherit),
                namespace.clone(),
                self.ctx.imports().to_vec(),
            );

            // We don't know if this is a base class, mixin, or interface yet
            // That's determined during completion when we can look up the type
            class_entry = class_entry.with_unresolved_inheritance(unresolved);
        }

        if class.modifiers.final_ {
            class_entry = class_entry.as_final();
        }
        if class.modifiers.abstract_ {
            class_entry = class_entry.as_abstract();
        }

        // Register the class
        if let Err(e) = self.ctx.register_type(class_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register class {}: {}", qualified_name, e),
                span: class.span,
            });
            return;
        }
        self.types_registered += 1;
        self.registered_classes.push(qualified_name.clone());

        // Register class members with unresolved types
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    if method.is_constructor() {
                        self.visit_constructor(method, &qualified_name);
                    } else if method.is_destructor {
                        self.visit_destructor(method, &qualified_name);
                    } else {
                        self.visit_function(method, Some(qualified_name.clone()));
                    }
                }
                ClassMember::Field(field) => {
                    self.visit_field(field, &qualified_name);
                }
                ClassMember::VirtualProperty(prop) => {
                    self.visit_virtual_property(prop, &qualified_name);
                }
                ClassMember::Funcdef(fd) => {
                    self.visit_funcdef(fd);
                }
            }
        }
    }
}
```

---

## Function Registration

```rust
impl<'a, 'reg> RegistrationPass<'a, 'reg> {
    fn visit_function(
        &mut self,
        func: &FunctionDecl<'_>,
        object_type: Option<QualifiedName>,
    ) {
        let name = func.name.name.to_string();
        let namespace = self.current_namespace_vec();
        let qualified_name = if object_type.is_some() {
            QualifiedName::new(&name, namespace.clone())
        } else {
            QualifiedName::new(&name, namespace.clone())
        };

        // Collect UNRESOLVED parameters (not resolved yet)
        let unresolved_params = self.collect_params(func.params);

        // Collect UNRESOLVED return type
        let unresolved_return_type = if let Some(ref ret) = func.return_type {
            self.collect_type(&ret)
        } else {
            UnresolvedType::simple("void")
        };

        // Build function traits
        let traits = FunctionTraits {
            is_virtual: func.modifiers.virtual_,
            is_final: func.modifiers.final_,
            is_abstract: func.modifiers.abstract_,
            is_const: func.is_const,
            ..FunctionTraits::default()
        };

        // Create function def with UNRESOLVED signature
        let func_def = FunctionDef::new_unresolved(
            qualified_name.clone(),
            object_type,
            unresolved_params,
            unresolved_return_type,
            traits,
            false, // not native
            self.parse_visibility(func.visibility),
        );

        let source = FunctionSource::script(func.span);
        let entry = if object_type.is_some() {
            FunctionEntry::script_method(func_def, source)
        } else {
            FunctionEntry::script_function(func_def, source)
        };

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register function: {}", e),
                span: func.span,
            });
        } else {
            self.functions_registered += 1;
        }
    }

    /// Collect parameters without resolving types.
    fn collect_params(&self, params: &[FunctionParam<'_>]) -> Vec<UnresolvedParam> {
        params
            .iter()
            .map(|p| {
                let param_type = self.collect_type(&p.param_type);
                let mut param = UnresolvedParam::new(p.name.name, param_type);
                if p.default_value.is_some() {
                    param = param.with_default();
                }
                param
            })
            .collect()
    }

    /// Collect type without resolving.
    fn collect_type(&self, ty: &TypeExpr<'_>) -> UnresolvedType {
        UnresolvedType::with_context(
            self.type_expr_to_string(&ty.ty),
            self.current_namespace_vec(),
            self.ctx.imports().to_vec(),
        )
        .with_const(ty.is_const)
        .with_handle(ty.is_handle)
        .with_handle_to_const(ty.is_handle_to_const)
        .with_ref_modifier(self.convert_ref_modifier(ty.ref_modifier))
    }

    /// Convert AST type to string representation.
    fn type_expr_to_string(&self, ty: &Type<'_>) -> String {
        match ty {
            Type::Named(path) => {
                // e.g., "Game::Player" or just "Player"
                path.segments.iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join("::")
            }
            Type::Template { name, args } => {
                // e.g., "array<int>"
                let base = self.type_expr_to_string(name);
                let args_str = args.iter()
                    .map(|a| self.type_expr_to_string(&a.ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, args_str)
            }
            Type::Auto => "auto".to_string(),
            Type::Void => "void".to_string(),
        }
    }
}
```

---

## Interface Registration

```rust
impl<'a, 'reg> RegistrationPass<'a, 'reg> {
    fn visit_interface(&mut self, iface: &InterfaceDecl<'_>) {
        let name = iface.name.name.to_string();
        let namespace = self.current_namespace_vec();
        let qualified_name = QualifiedName::new(&name, namespace.clone());

        let source = TypeSource::script(self.unit_id, iface.span);
        let mut iface_entry = InterfaceEntry::new(qualified_name.clone(), source);

        // Store UNRESOLVED base interfaces
        for base in &iface.bases {
            let unresolved = UnresolvedType::with_context(
                self.ident_expr_to_string(base),
                namespace.clone(),
                self.ctx.imports().to_vec(),
            );
            iface_entry = iface_entry.with_unresolved_base(unresolved);
        }

        // Register interface
        if let Err(e) = self.ctx.register_type(iface_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register interface: {}", e),
                span: iface.span,
            });
            return;
        }
        self.types_registered += 1;

        // Register interface methods with UNRESOLVED signatures
        for member in iface.members {
            if let InterfaceMember::Method(method) = member {
                self.visit_interface_method(method, &qualified_name);
            }
        }
    }

    fn visit_interface_method(
        &mut self,
        method: &InterfaceMethod<'_>,
        iface_name: &QualifiedName,
    ) {
        let name = method.name.name.to_string();
        let method_name = QualifiedName::new(&name, vec![]);

        // Collect UNRESOLVED parameters
        let unresolved_params = self.collect_params(method.params);

        // Collect UNRESOLVED return type
        let unresolved_return_type = self.collect_type(&method.return_type);

        // Create unresolved signature
        let sig = UnresolvedSignature::new(name, unresolved_params, unresolved_return_type)
            .with_const(method.is_const);

        // Add to interface's unresolved methods
        if let Some(entry) = self.ctx.get_type_mut(iface_name) {
            if let Some(iface) = entry.as_interface_mut() {
                iface.unresolved_methods.push(sig);
            }
        }
    }
}
```

---

## Funcdef Registration

```rust
impl<'a, 'reg> RegistrationPass<'a, 'reg> {
    fn visit_funcdef(&mut self, fd: &FuncdefDecl<'_>) {
        let name = fd.name.name.to_string();
        let namespace = self.current_namespace_vec();
        let qualified_name = QualifiedName::new(&name, namespace.clone());

        // Collect UNRESOLVED parameters
        let unresolved_params = self.collect_params(fd.params);

        // Collect UNRESOLVED return type
        let unresolved_return_type = if let Some(ref ret) = fd.return_type {
            self.collect_type(ret)
        } else {
            UnresolvedType::simple("void")
        };

        let source = TypeSource::script(self.unit_id, fd.span);
        let entry = FuncdefEntry::new(
            qualified_name.clone(),
            source,
            unresolved_params,
            unresolved_return_type,
        );

        if let Err(e) = self.ctx.register_type(entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register funcdef: {}", e),
                span: fd.span,
            });
            return;
        }
        self.types_registered += 1;
    }
}
```

---

## PendingResolutions Removal

With `UnresolvedType` stored directly in entries, `PendingResolutions` becomes unnecessary:

```rust
// BEFORE: Separate pending storage
pub struct PendingResolutions {
    pub class_inheritance: FxHashMap<TypeHash, Vec<PendingInheritance>>,
    pub interface_bases: FxHashMap<TypeHash, Vec<PendingInheritance>>,
}

// AFTER: Stored directly in entries
// ClassEntry.base_class, mixins, interfaces = Vec<InheritanceRef::Unresolved>
// InterfaceEntry.base_interfaces = Vec<InheritanceRef::Unresolved>
// No separate tracking needed
```

---

## RegistrationOutput Changes

```rust
/// Output of the registration pass.
#[derive(Debug, Default)]
pub struct RegistrationOutput {
    /// Number of types registered.
    pub types_registered: usize,
    /// Number of functions registered.
    pub functions_registered: usize,
    /// Number of global variables registered.
    pub globals_registered: usize,
    /// Collected errors.
    pub errors: Vec<CompilationError>,
    // REMOVED: pending_resolutions (now stored in entries)
}
```

---

## What Stays the Same

1. **Namespace tracking** - Still track current namespace and imports
2. **Class traits** - Still track constructors/destructors for auto-generation
3. **Global slot allocation** - Still allocate slots during registration
4. **Enum value evaluation** - Still evaluate enum values (literals only)
5. **Error collection** - Still collect errors for reporting

---

## Dependencies

- Phase 1: `UnresolvedType`, `UnresolvedParam`, `UnresolvedSignature`
- Phase 2: Entry types with unresolved fields
- Phase 3: Registry with `QualifiedName` as key

Phase 5 (Completion) will resolve all the unresolved types.
