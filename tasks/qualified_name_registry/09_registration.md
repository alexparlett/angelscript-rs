# Phase 9: Registration Pass Rewrite

## Overview

Rewrite the Registration pass to:
1. Build namespace nodes within the unit's subtree (`$unit_N/`)
2. Add `Mirrors` edges when creating namespaces that exist in `$ffi`/`$shared`
3. Collect all unresolved declarations into `RegistrationResult`
4. Collect `using namespace` directives for `Uses` edge creation

No type resolution, no TypeHash computation.

**Depends on:** Phase 7 (Unified Tree), Phase 8 (SymbolRegistry Integration)

**Files:**
- `crates/angelscript-compiler/src/passes/registration.rs` (rewrite)

---

## Design Principles

1. **Build namespace tree** - Create namespace nodes as we walk the AST
2. **No type resolution** - Store type names as strings, resolve in completion
3. **No type registration** - Don't register types, just collect them
4. **Capture context** - Store namespace context for later resolution
5. **Collect everything** - All declarations go into the result
6. **Collect using directives** - Store `using namespace` directives for edge creation in completion

---

## New RegistrationPass Structure

```rust
// crates/angelscript-compiler/src/passes/registration.rs

use angelscript_core::{
    CompilationError, QualifiedName, Span, UnitId, Visibility,
    UnresolvedClass, UnresolvedEnum, UnresolvedEnumValue, UnresolvedField,
    UnresolvedFuncdef, UnresolvedFunction, UnresolvedGlobal, UnresolvedInheritance,
    UnresolvedInterface, UnresolvedMethod, UnresolvedMixin,
    UnresolvedParam, UnresolvedSignature, UnresolvedType, UnresolvedUsingDirective,
    UnresolvedVirtualProperty, MethodKind,
};
use angelscript_parser::ast::*;
use angelscript_registry::NamespaceTree;

use crate::passes::RegistrationResult;

/// Pass 1: Build namespace tree and collect all declarations from the AST.
///
/// This pass:
/// 1. Builds the namespace tree structure (nodes only, no types yet)
/// 2. Collects all unresolved declarations into RegistrationResult
/// 3. Collects using directives for later edge creation
///
/// No type resolution happens here - types are resolved in the Completion pass.
pub struct RegistrationPass<'tree> {
    /// Unit ID for source tracking.
    unit_id: UnitId,
    /// Current namespace stack.
    namespace_stack: Vec<String>,
    /// The namespace tree to build.
    tree: &'tree mut NamespaceTree,
    /// Result being built.
    result: RegistrationResult,
}

impl<'tree> RegistrationPass<'tree> {
    /// Create a new registration pass for a unit.
    pub fn new(unit_id: UnitId, tree: &'tree mut NamespaceTree) -> Self {
        Self {
            unit_id,
            namespace_stack: Vec::new(),
            tree,
            result: RegistrationResult::new(unit_id),
        }
    }

    /// Run the registration pass on a script.
    ///
    /// Returns `RegistrationResult` containing all unresolved declarations.
    /// The namespace tree is built as a side effect.
    pub fn run(mut self, script: &Script<'_>) -> RegistrationResult {
        for item in script.items() {
            self.visit_item(item);
        }
        self.result
    }

    // === Namespace Management ===

    fn current_namespace(&self) -> Vec<String> {
        self.namespace_stack.clone()
    }

    fn current_namespace_string(&self) -> String {
        self.namespace_stack.join("::")
    }

    fn qualified_name(&self, simple_name: &str) -> QualifiedName {
        QualifiedName::new(simple_name, self.current_namespace())
    }

    // NOTE: No current_imports() - using directives collected in result

    fn enter_namespace(&mut self, ns: &str) {
        for part in ns.split("::") {
            self.namespace_stack.push(part.to_string());
        }
    }

    fn exit_namespace(&mut self, ns: &str) {
        let depth = ns.split("::").count();
        for _ in 0..depth {
            self.namespace_stack.pop();
        }
    }
}
```

---

## Item Visitors

```rust
impl RegistrationPass {
    fn visit_item(&mut self, item: &Item<'_>) {
        match item {
            Item::Namespace(ns) => self.visit_namespace(ns),
            Item::Class(class) => self.visit_class(class),
            Item::Interface(iface) => self.visit_interface(iface),
            Item::Enum(e) => self.visit_enum(e),
            Item::Function(func) => self.visit_function(func),
            Item::GlobalVar(var) => self.visit_global_var(var),
            Item::Funcdef(fd) => self.visit_funcdef(fd),
            Item::Mixin(mixin) => self.visit_mixin(mixin),
            Item::UsingNamespace(u) => self.visit_using(u),
            Item::Typedef(_) => { /* Handled separately or in completion */ }
            Item::Import(_) => { /* Handled separately */ }
        }
    }

    fn visit_namespace(&mut self, ns: &NamespaceDecl<'_>) {
        // Namespace declaration is a single identifier (not a path)
        let ns_name = ns.name.name;

        self.enter_namespace(ns_name);

        // Build the namespace node in the tree
        self.tree.get_or_create_path(&self.current_namespace());

        for item in ns.items {
            self.visit_item(item);
        }

        self.exit_namespace(ns_name);
    }

    fn visit_using(&mut self, u: &UsingNamespaceDecl<'_>) {
        // Collect using directive for later resolution in Completion pass
        let target: Vec<String> = u.path.iter()
            .map(|id| id.name.to_string())
            .collect();

        let directive = UnresolvedUsingDirective::new(
            self.current_namespace(),  // Source namespace where directive appears
            target,                     // Target namespace path
            u.span,
        );

        self.result.add_using_directive(directive);
    }
}
```

---

## Class Registration

```rust
impl RegistrationPass {
    fn visit_class(&mut self, class: &ClassDecl<'_>) {
        let name = self.qualified_name(class.name.name);

        let mut unresolved = UnresolvedClass::new(name, class.span, self.unit_id);

        // Modifiers
        if class.modifiers.final_ {
            unresolved = unresolved.with_final();
        }
        if class.modifiers.abstract_ {
            unresolved = unresolved.with_abstract();
        }
        if class.modifiers.shared {
            unresolved = unresolved.with_shared();
        }

        // Inheritance (don't resolve - just store names)
        for inherit in &class.inheritance {
            let type_ref = self.collect_ident_expr_type(inherit);
            unresolved = unresolved.with_inheritance(
                UnresolvedInheritance::new(type_ref, inherit.span)
            );
        }

        // Members
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    if let Some(m) = self.collect_method(method) {
                        unresolved = unresolved.with_method(m);
                    }
                }
                ClassMember::Field(field) => {
                    unresolved = unresolved.with_field(self.collect_field(field));
                }
                ClassMember::VirtualProperty(prop) => {
                    unresolved.virtual_properties.push(self.collect_virtual_property(prop));
                }
                ClassMember::Funcdef(fd) => {
                    unresolved.funcdefs.push(self.collect_funcdef(fd));
                }
            }
        }

        self.result.add_class(unresolved);
    }

    fn visit_mixin(&mut self, mixin: &MixinDecl<'_>) {
        // Mixins use the same structure as classes
        let class = &mixin.class;
        let name = self.qualified_name(class.name.name);

        let mut unresolved = UnresolvedClass::new(name, class.span, self.unit_id);

        // Inheritance (mixins can include other mixins)
        for inherit in &class.inheritance {
            let type_ref = self.collect_ident_expr_type(inherit);
            unresolved = unresolved.with_inheritance(
                UnresolvedInheritance::new(type_ref, inherit.span)
            );
        }

        // Members (same as class)
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    // Mixins can't have constructors/destructors
                    if method.is_constructor() || method.is_destructor {
                        self.result.add_error(CompilationError::InvalidOperation {
                            message: "mixins cannot have constructors or destructors".into(),
                            span: method.span,
                        });
                        continue;
                    }
                    if let Some(m) = self.collect_method(method) {
                        unresolved = unresolved.with_method(m);
                    }
                }
                ClassMember::Field(field) => {
                    unresolved = unresolved.with_field(self.collect_field(field));
                }
                ClassMember::VirtualProperty(prop) => {
                    unresolved.virtual_properties.push(self.collect_virtual_property(prop));
                }
                ClassMember::Funcdef(fd) => {
                    unresolved.funcdefs.push(self.collect_funcdef(fd));
                }
            }
        }

        self.result.add_mixin(UnresolvedMixin::new(unresolved));
    }
}
```

---

## Method Collection

```rust
impl RegistrationPass {
    fn collect_method(&mut self, method: &FunctionDecl<'_>) -> Option<UnresolvedMethod> {
        let signature = self.collect_signature(method);
        let mut m = UnresolvedMethod::new(method.name.name, signature, method.span);

        // Visibility
        m = m.with_visibility(convert_visibility(method.visibility));

        // Modifiers
        if method.modifiers.virtual_ {
            m = m.with_virtual();
        }
        if method.modifiers.override_ {
            m = m.with_override();
        }
        if method.modifiers.final_ {
            m = m.with_final();
        }
        if method.modifiers.abstract_ {
            m = m.with_abstract();
        }
        if method.is_const {
            m = m.with_const();
        }

        // Special methods
        if method.is_constructor() {
            // Determine if copy constructor
            let kind = if self.is_copy_constructor(method) {
                MethodKind::CopyConstructor
            } else {
                MethodKind::Constructor
            };
            m = m.with_kind(kind);
        } else if method.is_destructor {
            m = m.with_kind(MethodKind::Destructor);
        }

        // Deleted methods
        if method.attrs.delete {
            m = m.with_deleted();
        }

        // Has body?
        m.has_body = method.body.is_some() && !method.attrs.delete && !method.modifiers.abstract_;

        Some(m)
    }

    fn is_copy_constructor(&self, method: &FunctionDecl<'_>) -> bool {
        // Copy constructor: single param of same type with &in
        if method.params.len() != 1 {
            return false;
        }
        let param = &method.params[0];
        // Would need to compare type name to class name
        // For now, check if it's a reference parameter
        param.param_type.is_in_ref()
    }

    fn collect_field(&self, field: &FieldDecl<'_>) -> UnresolvedField {
        let field_type = self.collect_type_expr(&field.ty);
        let mut f = UnresolvedField::new(field.name.name, field_type, field.span);

        f = f.with_visibility(convert_visibility(field.visibility));

        if field.default_value.is_some() {
            f = f.with_initializer();
        }

        f
    }

    fn collect_virtual_property(&self, prop: &VirtualPropertyDecl<'_>) -> UnresolvedVirtualProperty {
        let property_type = self.collect_type_expr(&prop.ty);

        let getter = prop.accessors.iter()
            .find(|a| matches!(a.kind, PropertyAccessorKind::Get))
            .map(|a| UnresolvedAccessor {
                span: a.span,
                is_const: a.is_const,
                has_body: a.body.is_some(),
            });

        let setter = prop.accessors.iter()
            .find(|a| matches!(a.kind, PropertyAccessorKind::Set))
            .map(|a| UnresolvedAccessor {
                span: a.span,
                is_const: a.is_const,
                has_body: a.body.is_some(),
            });

        UnresolvedVirtualProperty {
            name: prop.name.name.to_string(),
            property_type,
            span: prop.span,
            visibility: convert_visibility(prop.visibility),
            getter,
            setter,
        }
    }
}
```

---

## Interface Registration

```rust
impl RegistrationPass {
    fn visit_interface(&mut self, iface: &InterfaceDecl<'_>) {
        let name = self.qualified_name(iface.name.name);

        let mut unresolved = UnresolvedInterface::new(name, iface.span, self.unit_id);

        // Base interfaces
        for base in &iface.bases {
            let type_ref = self.collect_ident_expr_type(base);
            unresolved = unresolved.with_base(
                UnresolvedInheritance::new(type_ref, base.span)
            );
        }

        // Methods
        for member in iface.members {
            if let InterfaceMember::Method(method) = member {
                let sig = self.collect_interface_method(method);
                unresolved = unresolved.with_method(sig);
            }
        }

        if iface.modifiers.shared {
            unresolved = unresolved.with_shared();
        }

        self.result.add_interface(unresolved);
    }

    fn collect_interface_method(&self, method: &InterfaceMethod<'_>) -> UnresolvedSignature {
        let params = self.collect_params(method.params);
        let return_type = self.collect_type_expr(&method.return_type);

        UnresolvedSignature::new(method.name.name, params, return_type)
            .with_const(method.is_const)
    }
}
```

---

## Function and Global Registration

```rust
impl RegistrationPass {
    fn visit_function(&mut self, func: &FunctionDecl<'_>) {
        let name = self.qualified_name(func.name.name);
        let signature = self.collect_signature(func);

        let mut unresolved = UnresolvedFunction::new(name, func.span, self.unit_id, signature);

        unresolved = unresolved.with_visibility(convert_visibility(func.visibility));

        if func.body.is_none() {
            unresolved = unresolved.declaration_only();
        }

        if func.modifiers.shared {
            unresolved = unresolved.with_shared();
        }

        self.result.add_function(unresolved);
    }

    fn visit_global_var(&mut self, var: &GlobalVarDecl<'_>) {
        let name = self.qualified_name(var.name.name);
        let var_type = self.collect_type_expr(&var.ty);

        let mut unresolved = UnresolvedGlobal::new(name, var.span, self.unit_id, var_type);

        unresolved = unresolved.with_visibility(convert_visibility(var.visibility));

        if var.initializer.is_some() {
            unresolved = unresolved.with_initializer();
        }

        if var.modifiers.const_ {
            unresolved = unresolved.with_const();
        }

        self.result.add_global(unresolved);
    }

    fn visit_enum(&mut self, e: &EnumDecl<'_>) {
        let name = self.qualified_name(e.name.name);

        let mut unresolved = UnresolvedEnum::new(name, e.span, self.unit_id);

        for enumerator in e.values {
            let value = self.collect_enum_value(enumerator);
            unresolved = unresolved.with_value(value);
        }

        self.result.add_enum(unresolved);
    }

    fn collect_enum_value(&self, e: &Enumerator<'_>) -> UnresolvedEnumValue {
        let mut value = UnresolvedEnumValue::new(e.name.name, e.span);

        if let Some(ref expr) = e.value {
            // Try to evaluate constant expression
            if let Some(v) = self.try_eval_const_int(expr) {
                value = value.with_value(v);
            }
        }

        value
    }

    fn try_eval_const_int(&self, _expr: &Expr<'_>) -> Option<i64> {
        // Simple constant evaluation for enum values
        // Full implementation would handle literals, const refs, simple arithmetic
        None  // Placeholder - completion pass handles complex evaluation
    }

    fn visit_funcdef(&mut self, fd: &FuncdefDecl<'_>) {
        let funcdef = self.collect_funcdef(fd);
        self.result.add_funcdef(funcdef);
    }

    fn collect_funcdef(&self, fd: &FuncdefDecl<'_>) -> UnresolvedFuncdef {
        let name = self.qualified_name(fd.name.name);
        let params = self.collect_params(fd.params);
        let return_type = fd.return_type.as_ref()
            .map(|t| self.collect_type_expr(t))
            .unwrap_or_else(|| UnresolvedType::simple("void"));

        UnresolvedFuncdef::new(name, fd.span, self.unit_id, params, return_type)
    }
}
```

---

## Type Collection Helpers

```rust
impl RegistrationPass {
    /// Collect a type expression without resolving it.
    ///
    /// NOTE: Only stores context_namespace, NOT imports. Using directives are
    /// resolved via namespace tree edges in the Completion pass.
    fn collect_type_expr(&self, ty: &TypeExpr<'_>) -> UnresolvedType {
        let name = self.type_to_string(&ty.ty);

        UnresolvedType::with_context(name, self.current_namespace())
            .with_const(ty.is_const)
            .with_handle(ty.is_handle)
            .with_handle_to_const(ty.is_handle_to_const)
            .with_ref_modifier(convert_ref_modifier(ty.ref_modifier))
    }

    /// Convert an IdentExpr (used for inheritance) to UnresolvedType.
    fn collect_ident_expr_type(&self, expr: &IdentExpr<'_>) -> UnresolvedType {
        let name = self.ident_expr_to_string(expr);
        UnresolvedType::with_context(name, self.current_namespace())
    }

    /// Collect function signature.
    fn collect_signature(&self, func: &FunctionDecl<'_>) -> UnresolvedSignature {
        let params = self.collect_params(func.params);
        let return_type = func.return_type.as_ref()
            .map(|t| self.collect_type_expr(t))
            .unwrap_or_else(|| UnresolvedType::simple("void"));

        UnresolvedSignature::new(func.name.name, params, return_type)
            .with_const(func.is_const)
    }

    /// Collect function parameters.
    fn collect_params(&self, params: &[FunctionParam<'_>]) -> Vec<UnresolvedParam> {
        params.iter().map(|p| {
            let param_type = self.collect_type_expr(&p.param_type);
            let mut param = UnresolvedParam::new(p.name.name, param_type);
            if p.default_value.is_some() {
                param = param.with_default();
            }
            param
        }).collect()
    }

    /// Convert AST type to string.
    fn type_to_string(&self, ty: &Type<'_>) -> String {
        match ty {
            Type::Named(path) => {
                path.segments.iter()
                    .map(|s| s.name)
                    .collect::<Vec<_>>()
                    .join("::")
            }
            Type::Template { name, args } => {
                let base = self.type_to_string(name);
                let args_str = args.iter()
                    .map(|a| {
                        let mut s = self.type_to_string(&a.ty);
                        if a.is_handle {
                            s.push('@');
                        }
                        s
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, args_str)
            }
            Type::Auto => "auto".to_string(),
            Type::Void => "void".to_string(),
        }
    }

    /// Convert IdentExpr to string (for inheritance).
    fn ident_expr_to_string(&self, expr: &IdentExpr<'_>) -> String {
        match &expr.scope {
            Some(scope) if !scope.is_empty() => {
                let mut parts: Vec<&str> = scope.segments.iter().map(|id| id.name).collect();
                parts.push(expr.ident.name);
                parts.join("::")
            }
            _ => expr.ident.name.to_string(),
        }
    }
}

// Helper functions
fn convert_visibility(v: Option<angelscript_parser::ast::Visibility>) -> Visibility {
    match v {
        Some(angelscript_parser::ast::Visibility::Private) => Visibility::Private,
        Some(angelscript_parser::ast::Visibility::Protected) => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn convert_ref_modifier(m: angelscript_parser::ast::RefModifier) -> RefModifier {
    match m {
        angelscript_parser::ast::RefModifier::In => RefModifier::In,
        angelscript_parser::ast::RefModifier::Out => RefModifier::Out,
        angelscript_parser::ast::RefModifier::InOut => RefModifier::InOut,
        angelscript_parser::ast::RefModifier::None => RefModifier::None,
    }
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::parse_script;

    fn parse_and_register(source: &str) -> RegistrationResult {
        let ast = parse_script(source).unwrap();
        RegistrationPass::new(UnitId::new(0)).run(&ast)
    }

    #[test]
    fn register_simple_class() {
        let result = parse_and_register("class Player {}");

        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].name, QualifiedName::global("Player"));
    }

    #[test]
    fn register_class_in_namespace() {
        let result = parse_and_register("namespace Game { class Player {} }");

        assert_eq!(result.classes.len(), 1);
        assert_eq!(
            result.classes[0].name,
            QualifiedName::new("Player", vec!["Game".into()])
        );
    }

    #[test]
    fn register_class_with_inheritance() {
        let result = parse_and_register("class Player : Entity, IDrawable {}");

        assert_eq!(result.classes.len(), 1);
        assert_eq!(result.classes[0].inheritance.len(), 2);
        assert_eq!(result.classes[0].inheritance[0].type_ref.name, "Entity");
        assert_eq!(result.classes[0].inheritance[1].type_ref.name, "IDrawable");
    }

    #[test]
    fn register_forward_reference() {
        // This should succeed - no type resolution during registration
        let result = parse_and_register(r#"
            interface IDamageable {
                void attack(Player@ p);
            }
            class Player : IDamageable {
                void attack(Player@ p) {}
            }
        "#);

        assert!(!result.has_errors());
        assert_eq!(result.interfaces.len(), 1);
        assert_eq!(result.classes.len(), 1);

        // Interface method has unresolved "Player" type
        let method = &result.interfaces[0].methods[0];
        assert_eq!(method.params[0].param_type.name, "Player");
    }

    #[test]
    fn register_enum() {
        let result = parse_and_register("enum Color { Red, Green = 5, Blue }");

        assert_eq!(result.enums.len(), 1);
        assert_eq!(result.enums[0].values.len(), 3);
        assert_eq!(result.enums[0].values[0].name, "Red");
        assert_eq!(result.enums[0].values[1].explicit_value, Some(5));
    }

    #[test]
    fn register_function() {
        let result = parse_and_register("void update(float dt) {}");

        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].name, QualifiedName::global("update"));
        assert_eq!(result.functions[0].signature.params.len(), 1);
    }

    #[test]
    fn namespace_context_captured() {
        let result = parse_and_register(r#"
            namespace Game {
                class Entity {
                    Player@ owner;  // Player not declared yet
                }
            }
        "#);

        let field = &result.classes[0].fields[0];
        assert_eq!(field.field_type.name, "Player");
        assert_eq!(field.field_type.context_namespace, vec!["Game"]);
    }
}
```

---

## What Stays the Same

1. **AST walking structure** - Same visitor pattern
2. **Namespace tracking** - Same stack-based approach
3. **Error collection** - Still collect errors

## What's Removed

1. **TypeResolver** - No type resolution
2. **Registry access** - No lookups or registrations
3. **TypeHash computation** - Deferred to completion
4. **PendingResolutions** - Not needed, info in entries
5. **`imports` field** - Using directives collected in `RegistrationResult.using_directives`
6. **`current_imports()` helper** - No longer needed
7. **Per-type imports** - `UnresolvedType` no longer has `imports` field

---

## Dependencies

- Phase 1: Core types (`UnresolvedType`, etc.)
- Phase 2: Unresolved entry types
- Phase 3: `RegistrationResult`

---

## What's Next

Phase 6 will rewrite the Completion pass to transform `RegistrationResult` into resolved entries and populate the registry.
