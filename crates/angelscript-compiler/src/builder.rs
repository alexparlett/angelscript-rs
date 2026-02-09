//! Module builder — multi-pass pipeline that populates a `SymbolRegistry`
//! from a parsed AST.
//!
//! Pipeline:
//! 1. **Registration** — walk the AST once, register all type names,
//!    function signatures, and global variables. Type references are
//!    stored as `QualifiedName` and not yet resolved.
//! 2. **Completion** — resolve all type references, validate inheritance
//!    chains, build vtables, compute object sizes, and build the TypeId
//!    reverse index.

use angelscript_core::{
    AccessModifier, DataType, Diagnostic, FunctionKind, FunctionSignature, FunctionTraits,
    ParamInfo, PrimitiveType, QualifiedName, RefMode, Severity,
};
use angelscript_parser::ast;

use crate::registry::*;

// ── Builder ───────────────────────────────────────────────────────────────

/// Builds a `SymbolRegistry` from one or more parsed scripts.
pub struct Builder {
    pub registry: SymbolRegistry,
    pub diagnostics: Vec<Diagnostic>,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            registry: SymbolRegistry::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Run the full build pipeline: registration → completion.
    pub fn build(mut self, scripts: &[&ast::Script]) -> BuildResult {
        // Pass 1: Registration.
        for script in scripts {
            self.register_script(script);
        }

        // Pass 2: Completion.
        self.complete();

        BuildResult {
            registry: self.registry,
            diagnostics: self.diagnostics,
        }
    }

    /// Check if any error diagnostics were emitted.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    fn error(&mut self, msg: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(msg));
    }

    #[allow(dead_code)]
    fn warning(&mut self, msg: impl Into<String>) {
        self.diagnostics.push(Diagnostic::warning(msg));
    }

    // ── Pass 1: Registration ──────────────────────────────────────────

    fn register_script(&mut self, script: &ast::Script) {
        let namespace = Vec::new();
        for decl in &script.declarations {
            self.register_declaration(decl, &namespace);
        }
    }

    fn register_declaration(&mut self, decl: &ast::Declaration, namespace: &[String]) {
        match decl {
            ast::Declaration::Class(c) => self.register_class(c, namespace),
            ast::Declaration::Interface(i) => self.register_interface(i, namespace),
            ast::Declaration::Enum(e) => self.register_enum(e, namespace),
            ast::Declaration::Function(f) => {
                self.register_function(f, namespace, None);
            }
            ast::Declaration::Namespace(n) => self.register_namespace(n, namespace),
            ast::Declaration::Import(_) => {
                // Imports are handled in a separate linking phase.
            }
            ast::Declaration::Typedef(t) => self.register_typedef(t, namespace),
            ast::Declaration::Funcdef(fd) => self.register_funcdef(fd, namespace),
            ast::Declaration::Mixin(c) => {
                // Mixins are registered like classes but handled specially.
                self.register_class(c, namespace);
            }
            ast::Declaration::GlobalVar(v) => self.register_global_var(v, namespace),
            ast::Declaration::VirtualProperty(vp) => {
                self.register_virtual_property(vp, namespace, None);
            }
            ast::Declaration::UsingNamespace(_) => {
                // Using-namespace is a scope directive, not a registration.
            }
        }
    }

    fn register_class(&mut self, class: &ast::ClassDecl, namespace: &[String]) {
        let name = QualifiedName::new(namespace.to_vec(), &class.name);
        let mut entry = ClassEntry::new(name.clone());

        // Parse flags.
        for flag in &class.flags {
            match flag {
                ast::ClassFlag::Shared => entry.flags |= TypeFlags::SHARED,
                ast::ClassFlag::Abstract => entry.flags |= TypeFlags::ABSTRACT,
                ast::ClassFlag::Final => entry.flags |= TypeFlags::FINAL,
                ast::ClassFlag::External => entry.flags |= TypeFlags::EXTERNAL,
            }
        }

        // Store base class references (unresolved).
        for (i, base) in class.base_classes.iter().enumerate() {
            let base_name = scoped_to_qualified(base, namespace);
            if i == 0 {
                // First base might be a class or interface.
                // We'll resolve in completion; store as base_class for now.
                entry.base_class = Some(base_name);
            } else {
                entry.interfaces.push(base_name);
            }
        }

        if let Err(e) = self.registry.register_type(TypeEntry::Class(entry)) {
            self.error(format!("{}", e));
        }

        // Register class members.
        for member in &class.members {
            match member {
                ast::ClassMember::Function(f) => {
                    self.register_function(f, namespace, Some(&name));
                }
                ast::ClassMember::VirtualProperty(vp) => {
                    self.register_virtual_property(vp, namespace, Some(&name));
                }
                ast::ClassMember::Variable(v) => {
                    self.register_class_variable(v, &name);
                }
                ast::ClassMember::Funcdef(fd) => {
                    // Nested funcdef — register in class namespace.
                    let mut nested_ns = namespace.to_vec();
                    nested_ns.push(class.name.clone());
                    self.register_funcdef(fd, &nested_ns);
                }
            }
        }
    }

    fn register_interface(&mut self, iface: &ast::InterfaceDecl, namespace: &[String]) {
        let name = QualifiedName::new(namespace.to_vec(), &iface.name);
        let mut entry = InterfaceEntry::new(name.clone());

        for flag in &iface.flags {
            match flag {
                ast::InterfaceFlag::Shared => entry.flags |= TypeFlags::SHARED,
                ast::InterfaceFlag::External => entry.flags |= TypeFlags::EXTERNAL,
            }
        }

        for base in &iface.bases {
            entry.bases.push(scoped_to_qualified(base, namespace));
        }

        if let Err(e) = self.registry.register_type(TypeEntry::Interface(entry)) {
            self.error(format!("{}", e));
        }

        // Register interface methods.
        for method in &iface.methods {
            self.register_interface_method(method, namespace, &name);
        }
    }

    fn register_enum(&mut self, en: &ast::EnumDecl, namespace: &[String]) {
        let name = QualifiedName::new(namespace.to_vec(), &en.name);
        let mut entry = EnumEntry::new(name.clone());

        for flag in &en.flags {
            match flag {
                ast::EnumFlag::Shared => entry.flags |= TypeFlags::SHARED,
                ast::EnumFlag::External => entry.flags |= TypeFlags::EXTERNAL,
            }
        }

        // Auto-assign enum values.
        let mut next_value: i64 = 0;
        for val in &en.values {
            // TODO: evaluate constant expressions for explicit values.
            let value = next_value;
            entry.values.push(EnumValueEntry {
                name: val.name.clone(),
                value,
            });
            next_value = value + 1;
        }

        if let Err(e) = self.registry.register_type(TypeEntry::Enum(entry)) {
            self.error(format!("{}", e));
        }
    }

    fn register_function(
        &mut self,
        func: &ast::FunctionDecl,
        namespace: &[String],
        owner: Option<&QualifiedName>,
    ) {
        let func_name = if func.is_destructor {
            format!("~{}", func.name)
        } else {
            func.name.clone()
        };

        let name = if let Some(owner_name) = owner {
            // Method: qualified as OwnerType::methodName.
            let mut ns = owner_name.namespace.clone();
            ns.push(owner_name.name.clone());
            QualifiedName::new(ns, &func_name)
        } else {
            QualifiedName::new(namespace.to_vec(), &func_name)
        };

        let return_type = func
            .return_type
            .as_ref()
            .map(|t| resolve_type_expr_to_data_type(t))
            .unwrap_or_else(DataType::void);

        let params: Vec<ParamInfo> = func
            .params
            .iter()
            .map(|p| ast_param_to_param_info(p))
            .collect();

        let mut traits = FunctionTraits::new();
        if func.is_const {
            traits = traits.set(FunctionTraits::CONST);
        }
        if func.is_destructor {
            traits = traits.set(FunctionTraits::DESTRUCTOR);
        }
        // Check if this is a constructor (name matches owner type).
        if let Some(owner_name) = owner {
            if func.name == owner_name.name && !func.is_destructor {
                traits = traits.set(FunctionTraits::CONSTRUCTOR);
            }
        }
        for attr in &func.attributes {
            match attr {
                ast::FuncAttr::Override => traits = traits.set(FunctionTraits::OVERRIDE),
                ast::FuncAttr::Final => traits = traits.set(FunctionTraits::FINAL),
                ast::FuncAttr::Explicit => traits = traits.set(FunctionTraits::EXPLICIT),
                ast::FuncAttr::Property => traits = traits.set(FunctionTraits::PROPERTY),
                ast::FuncAttr::Delete => traits = traits.set(FunctionTraits::DELETED),
            }
        }

        let access = match func.access {
            Some(ast::AccessModifier::Private) => {
                traits = traits.set(FunctionTraits::PRIVATE);
                AccessModifier::Private
            }
            Some(ast::AccessModifier::Protected) => {
                traits = traits.set(FunctionTraits::PROTECTED);
                AccessModifier::Protected
            }
            None => AccessModifier::Public,
        };

        let kind = if owner.is_some() {
            FunctionKind::Script
        } else {
            FunctionKind::Script
        };

        let sig = FunctionSignature::new(name.clone(), return_type, params).with_traits(traits);

        let id = self.registry.next_function_id();
        let entry = FunctionEntry {
            id,
            name: name.clone(),
            signature: sig,
            kind,
            traits,
            owner: owner.cloned(),
            access,
        };

        let fid = self.registry.register_function(entry);

        // If this is a method, add it to the class's method list.
        if let Some(owner_name) = owner {
            if let Some(TypeEntry::Class(class)) = self.registry.get_type_mut(owner_name) {
                class.methods.push(fid);
            }
        }
    }

    fn register_interface_method(
        &mut self,
        method: &ast::InterfaceMethod,
        _namespace: &[String],
        iface_name: &QualifiedName,
    ) {
        let mut ns = iface_name.namespace.clone();
        ns.push(iface_name.name.clone());
        let name = QualifiedName::new(ns, &method.name);

        let return_type = resolve_type_expr_to_data_type(&method.return_type);
        let params: Vec<ParamInfo> = method
            .params
            .iter()
            .map(|p| ast_param_to_param_info(p))
            .collect();

        let mut traits = FunctionTraits::new();
        if method.is_const {
            traits = traits.set(FunctionTraits::CONST);
        }

        let sig = FunctionSignature::new(name.clone(), return_type, params).with_traits(traits);

        let id = self.registry.next_function_id();
        let entry = FunctionEntry {
            id,
            name,
            signature: sig,
            kind: FunctionKind::Interface,
            traits,
            owner: Some(iface_name.clone()),
            access: AccessModifier::Public,
        };

        let fid = self.registry.register_function(entry);

        if let Some(TypeEntry::Interface(iface)) = self.registry.get_type_mut(iface_name) {
            iface.methods.push(fid);
        }
    }

    fn register_namespace(&mut self, ns_decl: &ast::NamespaceDecl, parent_ns: &[String]) {
        let mut new_ns = parent_ns.to_vec();
        new_ns.extend(ns_decl.name.iter().cloned());
        self.registry.register_namespace(new_ns.clone());

        for decl in &ns_decl.declarations {
            self.register_declaration(decl, &new_ns);
        }
    }

    fn register_typedef(&mut self, td: &ast::TypedefDecl, namespace: &[String]) {
        let name = QualifiedName::new(namespace.to_vec(), &td.name);
        let aliased = resolve_type_expr_to_data_type(&td.primitive);
        let entry = TypedefEntry::new(name, aliased);
        if let Err(e) = self.registry.register_type(TypeEntry::Typedef(entry)) {
            self.error(format!("{}", e));
        }
    }

    fn register_funcdef(&mut self, fd: &ast::FuncdefDecl, namespace: &[String]) {
        let name = QualifiedName::new(namespace.to_vec(), &fd.name);
        let mut entry = FuncdefEntry::new(name.clone());

        for flag in &fd.flags {
            match flag {
                ast::FuncdefFlag::Shared => entry.flags |= TypeFlags::SHARED,
                ast::FuncdefFlag::External => entry.flags |= TypeFlags::EXTERNAL,
            }
        }

        let return_type = resolve_type_expr_to_data_type(&fd.return_type);
        let params: Vec<ParamInfo> = fd
            .params
            .iter()
            .map(|p| ast_param_to_param_info(p))
            .collect();
        entry.signature = Some(FunctionSignature::new(name, return_type, params));

        if let Err(e) = self.registry.register_type(TypeEntry::Funcdef(entry)) {
            self.error(format!("{}", e));
        }
    }

    fn register_global_var(&mut self, var: &ast::VarDecl, namespace: &[String]) {
        let data_type = resolve_type_expr_to_data_type(&var.type_expr);
        let access = match var.access {
            Some(ast::AccessModifier::Private) => AccessModifier::Private,
            Some(ast::AccessModifier::Protected) => AccessModifier::Protected,
            None => AccessModifier::Public,
        };

        for declarator in &var.declarators {
            let name = QualifiedName::new(namespace.to_vec(), &declarator.name);
            let entry = GlobalEntry {
                name,
                data_type: data_type.clone(),
                access,
            };
            if let Err(e) = self.registry.register_global(entry) {
                self.error(format!("{}", e));
            }
        }
    }

    fn register_class_variable(&mut self, var: &ast::VarDecl, class_name: &QualifiedName) {
        let data_type = resolve_type_expr_to_data_type(&var.type_expr);
        let access = match var.access {
            Some(ast::AccessModifier::Private) => AccessModifier::Private,
            Some(ast::AccessModifier::Protected) => AccessModifier::Protected,
            None => AccessModifier::Public,
        };

        if let Some(TypeEntry::Class(class)) = self.registry.get_type_mut(class_name) {
            for declarator in &var.declarators {
                let prop = angelscript_core::PropertyInfo::new(
                    &declarator.name,
                    data_type.clone(),
                    0, // offset computed during completion
                )
                .with_access(access);
                class.properties.push(prop);
            }
        }
    }

    fn register_virtual_property(
        &mut self,
        vp: &ast::VirtualPropDecl,
        namespace: &[String],
        owner: Option<&QualifiedName>,
    ) {
        let prop_type = resolve_type_expr_to_data_type(&vp.type_expr);

        for accessor in &vp.accessors {
            let func_name = match accessor.kind {
                ast::PropAccessorKind::Get => format!("get_{}", vp.name),
                ast::PropAccessorKind::Set => format!("set_{}", vp.name),
            };

            let (return_type, params) = match accessor.kind {
                ast::PropAccessorKind::Get => (prop_type.clone(), vec![]),
                ast::PropAccessorKind::Set => (
                    DataType::void(),
                    vec![ParamInfo::new("value", prop_type.clone())],
                ),
            };

            let name = if let Some(owner_name) = owner {
                let mut ns = owner_name.namespace.clone();
                ns.push(owner_name.name.clone());
                QualifiedName::new(ns, &func_name)
            } else {
                QualifiedName::new(namespace.to_vec(), &func_name)
            };

            let mut traits = FunctionTraits::new().set(FunctionTraits::PROPERTY);
            if accessor.is_const {
                traits = traits.set(FunctionTraits::CONST);
            }
            for attr in &accessor.attributes {
                match attr {
                    ast::FuncAttr::Override => traits = traits.set(FunctionTraits::OVERRIDE),
                    ast::FuncAttr::Final => traits = traits.set(FunctionTraits::FINAL),
                    ast::FuncAttr::Explicit => traits = traits.set(FunctionTraits::EXPLICIT),
                    ast::FuncAttr::Property => {}
                    ast::FuncAttr::Delete => traits = traits.set(FunctionTraits::DELETED),
                }
            }

            let access = match vp.access {
                Some(ast::AccessModifier::Private) => {
                    traits = traits.set(FunctionTraits::PRIVATE);
                    AccessModifier::Private
                }
                Some(ast::AccessModifier::Protected) => {
                    traits = traits.set(FunctionTraits::PROTECTED);
                    AccessModifier::Protected
                }
                None => AccessModifier::Public,
            };

            let sig = FunctionSignature::new(name.clone(), return_type, params).with_traits(traits);

            let id = self.registry.next_function_id();
            let entry = FunctionEntry {
                id,
                name,
                signature: sig,
                kind: FunctionKind::Script,
                traits,
                owner: owner.cloned(),
                access,
            };

            let fid = self.registry.register_function(entry);

            if let Some(owner_name) = owner {
                if let Some(TypeEntry::Class(class)) = self.registry.get_type_mut(owner_name) {
                    class.methods.push(fid);
                }
            }
        }
    }

    // ── Pass 2: Completion ────────────────────────────────────────────

    fn complete(&mut self) {
        self.validate_inheritance();
        self.compute_class_sizes();
        self.registry.build_type_id_index();
    }

    /// Validate that base classes and interfaces exist and are of the correct kind.
    fn validate_inheritance(&mut self) {
        // Collect class info first to avoid borrow conflicts.
        let class_info: Vec<(QualifiedName, Option<QualifiedName>, Vec<QualifiedName>)> = self
            .registry
            .types()
            .filter_map(|(name, entry)| match entry {
                TypeEntry::Class(c) => {
                    Some((name.clone(), c.base_class.clone(), c.interfaces.clone()))
                }
                _ => None,
            })
            .collect();

        for (class_name, base_class, interfaces) in class_info {
            // Validate base class.
            if let Some(ref base_name) = base_class {
                match self.registry.get_type(base_name) {
                    Some(TypeEntry::Class(base)) => {
                        if base.flags.contains(TypeFlags::FINAL) {
                            self.error(format!(
                                "class '{}' cannot inherit from final class '{}'",
                                class_name, base_name
                            ));
                        }
                    }
                    Some(TypeEntry::Interface(_)) => {
                        // First base was actually an interface — move it.
                        // We reclassify: clear base_class, add to interfaces.
                        if let Some(TypeEntry::Class(class)) =
                            self.registry.get_type_mut(&class_name)
                        {
                            let iface_name = class.base_class.take().unwrap();
                            class.interfaces.insert(0, iface_name);
                        }
                    }
                    Some(_) => {
                        self.error(format!(
                            "class '{}' cannot inherit from non-class type '{}'",
                            class_name, base_name
                        ));
                    }
                    None => {
                        self.error(format!(
                            "class '{}' inherits from unknown type '{}'",
                            class_name, base_name
                        ));
                    }
                }
            }

            // Validate interfaces.
            for iface_name in &interfaces {
                match self.registry.get_type(iface_name) {
                    Some(TypeEntry::Interface(_)) => { /* ok */ }
                    Some(_) => {
                        self.error(format!(
                            "class '{}' implements non-interface type '{}'",
                            class_name, iface_name
                        ));
                    }
                    None => {
                        self.error(format!(
                            "class '{}' implements unknown interface '{}'",
                            class_name, iface_name
                        ));
                    }
                }
            }
        }

        // Validate interface inheritance.
        let iface_info: Vec<(QualifiedName, Vec<QualifiedName>)> = self
            .registry
            .types()
            .filter_map(|(name, entry)| match entry {
                TypeEntry::Interface(i) => Some((name.clone(), i.bases.clone())),
                _ => None,
            })
            .collect();

        for (iface_name, bases) in iface_info {
            for base_name in &bases {
                match self.registry.get_type(base_name) {
                    Some(TypeEntry::Interface(_)) => { /* ok */ }
                    Some(_) => {
                        self.error(format!(
                            "interface '{}' extends non-interface type '{}'",
                            iface_name, base_name
                        ));
                    }
                    None => {
                        self.error(format!(
                            "interface '{}' extends unknown type '{}'",
                            iface_name, base_name
                        ));
                    }
                }
            }
        }
    }

    /// Compute object sizes for classes based on their properties.
    fn compute_class_sizes(&mut self) {
        let class_names: Vec<QualifiedName> = self
            .registry
            .types()
            .filter_map(|(name, entry)| match entry {
                TypeEntry::Class(_) => Some(name.clone()),
                _ => None,
            })
            .collect();

        for name in class_names {
            let mut offset: usize = 0;

            // Get property count to iterate.
            let prop_count = match self.registry.get_type(&name) {
                Some(TypeEntry::Class(c)) => c.properties.len(),
                _ => continue,
            };

            for i in 0..prop_count {
                let size = {
                    let class = match self.registry.get_type(&name) {
                        Some(TypeEntry::Class(c)) => c,
                        _ => break,
                    };
                    let prop = &class.properties[i];
                    let size = prop.data_type.stack_size_dwords() as usize * 4;
                    size.max(1) // minimum 1 byte
                };

                // Align to natural alignment.
                let align = size.min(8);
                if align > 0 {
                    offset = (offset + align - 1) & !(align - 1);
                }

                if let Some(TypeEntry::Class(class)) = self.registry.get_type_mut(&name) {
                    class.properties[i].byte_offset = offset;
                }
                offset += size;
            }

            if let Some(TypeEntry::Class(class)) = self.registry.get_type_mut(&name) {
                class.size = offset;
            }
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of the build pipeline.
pub struct BuildResult {
    pub registry: SymbolRegistry,
    pub diagnostics: Vec<Diagnostic>,
}

impl BuildResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

// ── Helper functions ──────────────────────────────────────────────────────

/// Convert a `ScopedIdentifier` from the AST to a `QualifiedName`.
fn scoped_to_qualified(id: &ast::ScopedIdentifier, current_ns: &[String]) -> QualifiedName {
    if id.is_global || !id.scopes.is_empty() {
        // Fully or partially qualified — use scopes as namespace.
        QualifiedName::new(id.scopes.clone(), &id.name)
    } else {
        // Unqualified — in current namespace.
        QualifiedName::new(current_ns.to_vec(), &id.name)
    }
}

/// Convert an AST `TypeExpr` to a core `DataType`.
/// During registration, named types are resolved to `TypeKind::Object` with a
/// `TypeId` computed from the qualified name. Full resolution happens in completion.
fn resolve_type_expr_to_data_type(te: &ast::TypeExpr) -> DataType {
    let mut dt = match &te.kind {
        ast::TypeExprKind::Primitive(p) => {
            let prim = ast_prim_to_core_prim(p);
            DataType::primitive(prim)
        }
        ast::TypeExprKind::Void => DataType::void(),
        ast::TypeExprKind::Auto => DataType::auto(),
        ast::TypeExprKind::Named(id) => {
            let qn = QualifiedName::new(id.scopes.clone(), &id.name);
            DataType::object(qn.type_id())
        }
        ast::TypeExprKind::Template { base, args: _ } => {
            // Template types are stored as object types for now.
            let qn = QualifiedName::new(base.scopes.clone(), &base.name);
            DataType::object(qn.type_id())
        }
        ast::TypeExprKind::Infer => DataType::auto(),
    };

    if te.is_const {
        dt = dt.with_const();
    }
    if te.is_handle {
        if te.is_handle_to_const {
            dt = dt.with_handle_to_const();
        } else {
            dt = dt.with_handle();
        }
    }
    if te.is_ref {
        let mode = match te.ref_modifier {
            Some(ast::RefModifier::In) => RefMode::In,
            Some(ast::RefModifier::Out) => RefMode::Out,
            Some(ast::RefModifier::InOut) => RefMode::InOut,
            None => RefMode::InOut,
        };
        dt = dt.with_ref(mode);
    }

    dt
}

/// Convert AST primitive type to core primitive type.
fn ast_prim_to_core_prim(p: &ast::PrimType) -> PrimitiveType {
    match p {
        ast::PrimType::Bool => PrimitiveType::Bool,
        ast::PrimType::Int8 => PrimitiveType::Int8,
        ast::PrimType::Int16 => PrimitiveType::Int16,
        ast::PrimType::Int => PrimitiveType::Int32,
        ast::PrimType::Int64 => PrimitiveType::Int64,
        ast::PrimType::UInt8 => PrimitiveType::UInt8,
        ast::PrimType::UInt16 => PrimitiveType::UInt16,
        ast::PrimType::UInt => PrimitiveType::UInt32,
        ast::PrimType::UInt64 => PrimitiveType::UInt64,
        ast::PrimType::Float => PrimitiveType::Float,
        ast::PrimType::Double => PrimitiveType::Double,
    }
}

/// Convert an AST parameter to a core `ParamInfo`.
fn ast_param_to_param_info(p: &ast::Param) -> ParamInfo {
    let dt = resolve_type_expr_to_data_type(&p.type_expr);
    let name = p.name.clone().unwrap_or_default();
    let mut info = ParamInfo::new(name, dt);
    if p.default.is_some() {
        info = info.with_default();
    }
    if let Some(ref modifier) = p.type_expr.ref_modifier {
        let dir = match modifier {
            ast::RefModifier::In => angelscript_core::ParamDirection::In,
            ast::RefModifier::Out => angelscript_core::ParamDirection::Out,
            ast::RefModifier::InOut => angelscript_core::ParamDirection::InOut,
        };
        info = info.with_direction(dir);
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::parser::Parser;

    fn parse(source: &str) -> ast::Script {
        let mut parser = Parser::new(source);
        let script = parser.parse_script();
        assert!(
            !parser.has_errors(),
            "parse errors: {:?}",
            parser.diagnostics()
        );
        script
    }

    #[test]
    fn register_simple_function() {
        let script = parse("void main() {}");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("main");
        let fids = result.registry.get_functions_by_name(&name);
        assert_eq!(fids.len(), 1);
        let f = result.registry.get_function(fids[0]).unwrap();
        assert_eq!(f.signature.return_type, DataType::void());
    }

    #[test]
    fn register_function_with_params() {
        let script = parse("int add(int a, int b) { return a + b; }");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("add");
        let fids = result.registry.get_functions_by_name(&name);
        assert_eq!(fids.len(), 1);
        let f = result.registry.get_function(fids[0]).unwrap();
        assert_eq!(f.signature.params.len(), 2);
        assert!(f.signature.return_type.is_primitive());
    }

    #[test]
    fn register_class() {
        let script = parse(
            r#"
            class Player {
                int health;
                float speed;
                void takeDamage(int amount) {}
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Player");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert_eq!(c.properties.len(), 2);
                assert_eq!(c.methods.len(), 1);
                assert!(c.size > 0);
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn register_class_with_constructor() {
        let script = parse(
            r#"
            class Foo {
                Foo() {}
                Foo(int x) {}
                ~Foo() {}
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Foo");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert_eq!(c.methods.len(), 3);
                // First two should be constructors.
                let f0 = result.registry.get_function(c.methods[0]).unwrap();
                assert!(f0.traits.is_constructor());
                let f1 = result.registry.get_function(c.methods[1]).unwrap();
                assert!(f1.traits.is_constructor());
                // Third is destructor.
                let f2 = result.registry.get_function(c.methods[2]).unwrap();
                assert!(f2.traits.is_destructor());
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn register_class_with_inheritance() {
        let script = parse(
            r#"
            class Base {}
            class Derived : Base {}
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Derived");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert_eq!(c.base_class, Some(QualifiedName::global("Base")));
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn register_interface() {
        let script = parse(
            r#"
            interface IDrawable {
                void draw();
                float getWidth() const;
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("IDrawable");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Interface(i) => {
                assert_eq!(i.methods.len(), 2);
            }
            _ => panic!("expected interface"),
        }
    }

    #[test]
    fn register_enum() {
        let script = parse(
            r#"
            enum Color { Red, Green, Blue }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Color");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Enum(e) => {
                assert_eq!(e.values.len(), 3);
                assert_eq!(e.values[0].name, "Red");
                assert_eq!(e.values[0].value, 0);
                assert_eq!(e.values[1].value, 1);
                assert_eq!(e.values[2].value, 2);
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn register_namespace() {
        let script = parse(
            r#"
            namespace Game {
                class Player {}
                void init() {}
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let player = QualifiedName::new(vec!["Game".into()], "Player");
        assert!(result.registry.has_type(&player));

        let init = QualifiedName::new(vec!["Game".into()], "init");
        let fids = result.registry.get_functions_by_name(&init);
        assert_eq!(fids.len(), 1);
    }

    #[test]
    fn register_nested_namespace() {
        let script = parse(
            r#"
            namespace Game::Entities {
                class Enemy {}
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let enemy = QualifiedName::new(vec!["Game".into(), "Entities".into()], "Enemy");
        assert!(result.registry.has_type(&enemy));
    }

    #[test]
    fn register_typedef() {
        let script = parse("typedef float real;");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("real");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Typedef(t) => {
                assert!(t.aliased_type.is_primitive());
            }
            _ => panic!("expected typedef"),
        }
    }

    #[test]
    fn register_funcdef() {
        let script = parse("funcdef void Callback(int value);");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Callback");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Funcdef(fd) => {
                let sig = fd.signature.as_ref().unwrap();
                assert!(sig.return_type.is_void());
                assert_eq!(sig.params.len(), 1);
            }
            _ => panic!("expected funcdef"),
        }
    }

    #[test]
    fn register_global_var() {
        let script = parse("int g_score = 0;");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("g_score");
        let g = result.registry.get_global(&name).unwrap();
        assert!(g.data_type.is_primitive());
    }

    #[test]
    fn validate_inheritance_final_class() {
        let script = parse(
            r#"
            final class Base {}
            class Derived : Base {}
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(result.has_errors());
        assert!(result.diagnostics[0].message.contains("final"));
    }

    #[test]
    fn class_interface_reclassification() {
        let script = parse(
            r#"
            interface IFoo {}
            class Bar : IFoo {}
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        // Bar's base_class should be None, IFoo should be in interfaces.
        let name = QualifiedName::global("Bar");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert!(c.base_class.is_none());
                assert_eq!(c.interfaces.len(), 1);
                assert_eq!(c.interfaces[0], QualifiedName::global("IFoo"));
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn class_size_computation() {
        let script = parse(
            r#"
            class Sizes {
                int a;
                float b;
                double c;
                bool d;
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Sizes");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert!(c.size > 0);
                // int(4) + float(4) + double(8) + bool(1) = at least 17 bytes.
                assert!(c.size >= 17, "size was {}", c.size);
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn shared_class_flag() {
        let script = parse("shared class Shared {}");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Shared");
        let te = result.registry.get_type(&name).unwrap();
        assert!(te.flags().contains(TypeFlags::SHARED));
    }

    #[test]
    fn multiple_scripts() {
        let s1 = parse("class A {}");
        let s2 = parse("class B : A {}");
        let result = Builder::new().build(&[&s1, &s2]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        assert!(result.registry.has_type(&QualifiedName::global("A")));
        assert!(result.registry.has_type(&QualifiedName::global("B")));
    }

    #[test]
    fn type_id_index_built() {
        let script = parse("class Foo {}");
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Foo");
        let tid = name.type_id();
        let te = result.registry.get_type_by_id(tid).unwrap();
        assert_eq!(te.name(), &name);
    }

    #[test]
    fn access_modifiers_on_methods() {
        let script = parse(
            r#"
            class Foo {
                private void secret() {}
                protected void internal() {}
                void public_method() {}
            }
            "#,
        );
        let result = Builder::new().build(&[&script]);
        assert!(!result.has_errors(), "{:?}", result.diagnostics);

        let name = QualifiedName::global("Foo");
        let te = result.registry.get_type(&name).unwrap();
        match te {
            TypeEntry::Class(c) => {
                assert_eq!(c.methods.len(), 3);
                let f0 = result.registry.get_function(c.methods[0]).unwrap();
                assert_eq!(f0.access, AccessModifier::Private);
                let f1 = result.registry.get_function(c.methods[1]).unwrap();
                assert_eq!(f1.access, AccessModifier::Protected);
                let f2 = result.registry.get_function(c.methods[2]).unwrap();
                assert_eq!(f2.access, AccessModifier::Public);
            }
            _ => panic!("expected class"),
        }
    }
}
