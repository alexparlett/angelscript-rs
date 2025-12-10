//! Registration Pass (Pass 1) - Register all types and function signatures.
//!
//! This pass walks the AST and registers all type and function declarations into
//! the per-unit registry. Shared types go into the global registry. This pass
//! collects signatures without compiling function bodies.
//!
//! ## Responsibilities
//!
//! - Register classes, interfaces, enums, funcdefs, typedefs
//! - Register function signatures (global and methods)
//! - Register global variables with slot allocation
//! - Handle namespace declarations and using directives
//! - Resolve base classes and implemented interfaces
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────┐
//! │ Global TypeRegistry                │  ← Shared types, FFI types
//! │ ctx.register_shared_type()         │
//! └────────────────────────────────────┘
//!              ▲
//!              │ (layered lookup via ctx.get_type())
//!              │
//! ┌────────────────────────────────────┐
//! │ Per-Unit TypeRegistry              │  ← Non-shared script types
//! │ ctx.register_type()                │
//! │ ctx.register_function()            │
//! └────────────────────────────────────┘
//! ```

use angelscript_core::{
    ClassEntry, CompilationError, DataType, EnumEntry, FuncdefEntry, FunctionDef, FunctionEntry,
    FunctionTraits, GlobalPropertyEntry, GlobalPropertyImpl, InterfaceEntry, Param, Span, TypeHash,
    TypeKind, TypeSource, UnitId, Visibility,
};
use angelscript_parser::ast::{
    ClassDecl, ClassMember, EnumDecl, Enumerator, FieldDecl, FuncdefDecl, FunctionDecl,
    FunctionParam, GlobalVarDecl, InterfaceDecl, InterfaceMember, InterfaceMethod, Item,
    NamespaceDecl, Script, TypedefDecl, UsingNamespaceDecl, VirtualPropertyDecl,
};

use crate::context::CompilationContext;
use crate::type_resolver::TypeResolver;

/// Output of the registration pass.
#[derive(Debug, Default)]
pub struct RegistrationOutput {
    /// Number of types registered.
    pub types_registered: usize,
    /// Number of functions registered.
    pub functions_registered: usize,
    /// Number of global variables registered.
    pub globals_registered: usize,
    /// Collected errors (compilation can continue with some errors).
    pub errors: Vec<CompilationError>,
}

/// Pass 1: Register all types and function signatures.
///
/// This pass walks the AST and registers type declarations and function signatures
/// into the compilation context's unit registry. It does not compile function bodies.
pub struct RegistrationPass<'a, 'reg> {
    ctx: &'a mut CompilationContext<'reg>,
    unit_id: UnitId,
    types_registered: usize,
    functions_registered: usize,
    globals_registered: usize,
    /// Next slot index for script global variables.
    next_global_slot: u32,
    /// Track which classes have user-defined constructors.
    classes_with_constructor: Vec<TypeHash>,
    /// Track which classes have user-defined destructors.
    classes_with_destructor: Vec<TypeHash>,
}

impl<'a, 'reg> RegistrationPass<'a, 'reg> {
    /// Create a new registration pass.
    pub fn new(ctx: &'a mut CompilationContext<'reg>, unit_id: UnitId) -> Self {
        Self {
            ctx,
            unit_id,
            types_registered: 0,
            functions_registered: 0,
            globals_registered: 0,
            next_global_slot: 0,
            classes_with_constructor: Vec::new(),
            classes_with_destructor: Vec::new(),
        }
    }

    /// Run the registration pass on a script.
    pub fn run(mut self, script: &Script<'_>) -> RegistrationOutput {
        // Process all top-level items
        for item in script.items() {
            self.visit_item(item);
        }

        // Auto-generate missing default members
        // (deferred - will be implemented when needed)

        RegistrationOutput {
            types_registered: self.types_registered,
            functions_registered: self.functions_registered,
            globals_registered: self.globals_registered,
            errors: self.ctx.take_errors(),
        }
    }

    /// Visit a top-level item.
    fn visit_item(&mut self, item: &Item<'_>) {
        match item {
            Item::Namespace(ns) => self.visit_namespace(ns),
            Item::Class(class) => self.visit_class(class),
            Item::Interface(iface) => self.visit_interface(iface),
            Item::Enum(e) => self.visit_enum(e),
            Item::Function(func) => {
                self.visit_function(func, None);
            }
            Item::GlobalVar(var) => self.visit_global_var(var),
            Item::Typedef(td) => self.visit_typedef(td),
            Item::Funcdef(fd) => self.visit_funcdef(fd),
            Item::UsingNamespace(u) => self.visit_using(u),
            Item::Import(_) => { /* Import handling - deferred */ }
            Item::Mixin(_) => { /* Mixin handling - deferred */ }
        }
    }

    // ==========================================================================
    // Namespace
    // ==========================================================================

    fn visit_namespace(&mut self, ns: &NamespaceDecl<'_>) {
        // Build the namespace path from identifiers
        let ns_path: Vec<&str> = ns.path.iter().map(|id| id.name).collect();
        let ns_string = ns_path.join("::");

        self.ctx.enter_namespace(&ns_string);

        for item in ns.items {
            self.visit_item(item);
        }

        self.ctx.exit_namespace();
    }

    fn visit_using(&mut self, u: &UsingNamespaceDecl<'_>) {
        // Join path parts into qualified namespace string
        let ns: Vec<&str> = u.path.iter().map(|id| id.name).collect();
        let ns_string = ns.join("::");
        self.ctx.add_import(&ns_string);
    }

    // ==========================================================================
    // Class
    // ==========================================================================

    fn visit_class(&mut self, class: &ClassDecl<'_>) {
        let name = class.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);
        let namespace = self.current_namespace_vec();

        // Resolve base class and interfaces from inheritance list
        let (base_class, interfaces) = self.resolve_inheritance(class);

        // Create the class entry
        let source = TypeSource::script(self.unit_id, class.span);
        let mut class_entry = ClassEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            TypeKind::ScriptObject,
            source,
        );

        if let Some(base) = base_class {
            class_entry = class_entry.with_base(base);
        }

        for iface in interfaces {
            class_entry = class_entry.with_interface(iface);
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

        // Register class members
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    // Check if this is a constructor or destructor
                    if method.is_constructor() {
                        self.classes_with_constructor.push(type_hash);
                        self.visit_constructor(method, type_hash);
                    } else if method.is_destructor {
                        self.classes_with_destructor.push(type_hash);
                        self.visit_destructor(method, type_hash);
                    } else {
                        self.visit_function(method, Some(type_hash));
                    }
                }
                ClassMember::Field(field) => {
                    self.visit_field(field, type_hash);
                }
                ClassMember::VirtualProperty(prop) => {
                    self.visit_virtual_property(prop, type_hash);
                }
                ClassMember::Funcdef(fd) => {
                    // Nested funcdef - register with class as parent namespace
                    self.visit_funcdef(fd);
                }
            }
        }

        // Add methods to the class entry
        // (Methods are registered separately, we need to update the class entry)
    }

    /// Resolve base class and interfaces from inheritance list.
    fn resolve_inheritance(&mut self, class: &ClassDecl<'_>) -> (Option<TypeHash>, Vec<TypeHash>) {
        let mut base_class = None;
        let mut interfaces = Vec::new();

        for (i, inherit_expr) in class.inheritance.iter().enumerate() {
            // Build the type name from the IdentExpr
            let type_name = self.ident_expr_to_string(inherit_expr);

            // Try to resolve the type
            if let Some(hash) = self.ctx.resolve_type(&type_name) {
                // Check if it's a class or interface
                if let Some(entry) = self.ctx.get_type(hash) {
                    if entry.is_interface() {
                        interfaces.push(hash);
                    } else if entry.is_class() {
                        // First non-interface is the base class
                        if i == 0 && base_class.is_none() {
                            base_class = Some(hash);
                        } else {
                            // Additional classes are not allowed in single inheritance
                            self.ctx.add_error(CompilationError::Other {
                                message: format!(
                                    "multiple inheritance not supported: {} already has a base class",
                                    class.name.name
                                ),
                                span: class.span,
                            });
                        }
                    }
                }
            } else {
                self.ctx.add_error(CompilationError::UnknownType {
                    name: type_name,
                    span: class.span,
                });
            }
        }

        (base_class, interfaces)
    }

    fn ident_expr_to_string(&self, expr: &angelscript_parser::ast::IdentExpr<'_>) -> String {
        match &expr.scope {
            Some(scope) if !scope.is_empty() => {
                let mut parts: Vec<&str> = scope.segments.iter().map(|id| id.name).collect();
                parts.push(expr.ident.name);
                parts.join("::")
            }
            _ => expr.ident.name.to_string(),
        }
    }

    fn visit_field(&mut self, field: &FieldDecl<'_>, _class_hash: TypeHash) {
        // Fields are stored in the class entry, not as separate registry entries
        // For now, we just validate the field type can be resolved
        let mut resolver = TypeResolver::new(self.ctx);
        if let Err(e) = resolver.resolve(&field.ty) {
            self.ctx.add_error(e);
        }
    }

    fn visit_virtual_property(&mut self, prop: &VirtualPropertyDecl<'_>, class_hash: TypeHash) {
        // Virtual properties are backed by getter/setter methods
        // Register accessor methods
        for accessor in prop.accessors {
            let method_name = match accessor.kind {
                angelscript_parser::ast::PropertyAccessorKind::Get => {
                    format!("get_{}", prop.name.name)
                }
                angelscript_parser::ast::PropertyAccessorKind::Set => {
                    format!("set_{}", prop.name.name)
                }
            };

            // Resolve property type
            let mut resolver = TypeResolver::new(self.ctx);
            let prop_type = match resolver.resolve(&prop.ty.ty) {
                Ok(dt) => dt,
                Err(e) => {
                    self.ctx.add_error(e);
                    continue;
                }
            };

            // Create getter or setter signature
            let (params, return_type) = match accessor.kind {
                angelscript_parser::ast::PropertyAccessorKind::Get => (vec![], prop_type),
                angelscript_parser::ast::PropertyAccessorKind::Set => {
                    (vec![Param::new("value", prop_type)], DataType::void())
                }
            };

            let param_hashes: Vec<TypeHash> =
                params.iter().map(|p| p.data_type.type_hash).collect();
            let func_hash = TypeHash::from_method(
                class_hash,
                &method_name,
                &param_hashes,
                accessor.is_const,
                false,
            );

            let mut traits = FunctionTraits::default();
            if accessor.is_const {
                traits.is_const = true;
            }

            let func_def = FunctionDef::new(
                func_hash,
                method_name,
                vec![],
                params,
                return_type,
                Some(class_hash),
                traits,
                false,
                convert_visibility(prop.visibility),
            );

            let source = angelscript_core::FunctionSource::script(accessor.span);
            let entry = FunctionEntry::script(func_def, self.unit_id, source);

            if let Err(e) = self.ctx.register_function(entry) {
                self.ctx.add_error(CompilationError::Other {
                    message: format!("failed to register property accessor: {}", e),
                    span: accessor.span,
                });
            } else {
                self.functions_registered += 1;
            }
        }
    }

    fn visit_constructor(&mut self, ctor: &FunctionDecl<'_>, class_hash: TypeHash) {
        // Resolve parameters
        let params = self.resolve_params(ctor.params, ctor.span);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        let func_hash = TypeHash::from_constructor(class_hash, &param_hashes);

        let class_name = self
            .ctx
            .get_type(class_hash)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| format!("{:?}", class_hash));

        let func_def = FunctionDef::new(
            func_hash,
            class_name,
            vec![],
            params,
            DataType::void(),
            Some(class_hash),
            FunctionTraits::constructor(),
            false,
            convert_visibility(ctor.visibility),
        );

        let source = angelscript_core::FunctionSource::script(ctor.span);
        let entry = FunctionEntry::script(func_def, self.unit_id, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register constructor: {}", e),
                span: ctor.span,
            });
        } else {
            self.functions_registered += 1;
        }
    }

    fn visit_destructor(&mut self, dtor: &FunctionDecl<'_>, class_hash: TypeHash) {
        // Destructor has no parameters and no return value
        let func_hash = TypeHash::from_method(class_hash, "~", &[], false, false);

        let class_name = self
            .ctx
            .get_type(class_hash)
            .map(|t| t.name().to_string())
            .unwrap_or_else(|| format!("{:?}", class_hash));

        let func_def = FunctionDef::new(
            func_hash,
            format!("~{}", class_name),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::destructor(),
            false,
            Visibility::Public,
        );

        let source = angelscript_core::FunctionSource::script(dtor.span);
        let entry = FunctionEntry::script(func_def, self.unit_id, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register destructor: {}", e),
                span: dtor.span,
            });
        } else {
            self.functions_registered += 1;
        }
    }

    // ==========================================================================
    // Interface
    // ==========================================================================

    fn visit_interface(&mut self, iface: &InterfaceDecl<'_>) {
        let name = iface.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);
        let namespace = self.current_namespace_vec();

        // Resolve base interfaces
        let mut base_interfaces = Vec::new();
        for base in iface.bases {
            if let Some(hash) = self.ctx.resolve_type(base.name) {
                base_interfaces.push(hash);
            } else {
                self.ctx.add_error(CompilationError::UnknownType {
                    name: base.name.to_string(),
                    span: iface.span,
                });
            }
        }

        let source = TypeSource::script(self.unit_id, iface.span);
        let mut interface_entry = InterfaceEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            source,
        );

        for base in base_interfaces {
            interface_entry.base_interfaces.push(base);
        }

        // Register the interface
        if let Err(e) = self.ctx.register_type(interface_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register interface {}: {}", qualified_name, e),
                span: iface.span,
            });
            return;
        }
        self.types_registered += 1;

        // Register interface members
        for member in iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    self.visit_interface_method(method, type_hash);
                }
                InterfaceMember::VirtualProperty(prop) => {
                    self.visit_virtual_property(prop, type_hash);
                }
            }
        }
    }

    fn visit_interface_method(&mut self, method: &InterfaceMethod<'_>, iface_hash: TypeHash) {
        let name = method.name.name.to_string();

        // Resolve parameters
        let params = self.resolve_params(method.params, method.span);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Resolve return type
        let mut resolver = TypeResolver::new(self.ctx);
        let return_type = match resolver.resolve(&method.return_type.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                DataType::void()
            }
        };

        let func_hash =
            TypeHash::from_method(iface_hash, &name, &param_hashes, method.is_const, false);

        let traits = FunctionTraits {
            is_const: method.is_const,
            is_abstract: true, // Interface methods are always abstract
            ..FunctionTraits::default()
        };

        let func_def = FunctionDef::new(
            func_hash,
            name,
            vec![],
            params,
            return_type,
            Some(iface_hash),
            traits,
            false,
            Visibility::Public,
        );

        let source = angelscript_core::FunctionSource::script(method.span);
        let entry = FunctionEntry::abstract_method(func_def, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register interface method: {}", e),
                span: method.span,
            });
        } else {
            self.functions_registered += 1;
        }
    }

    // ==========================================================================
    // Enum
    // ==========================================================================

    fn visit_enum(&mut self, e: &EnumDecl<'_>) {
        let name = e.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);
        let namespace = self.current_namespace_vec();

        let source = TypeSource::script(self.unit_id, e.span);
        let mut enum_entry = EnumEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            source,
        );

        // Process enumerators
        let mut next_value: i64 = 0;
        for enumerator in e.enumerators {
            let value = self.evaluate_enum_value(enumerator, next_value);
            enum_entry = enum_entry.with_value(enumerator.name.name, value);
            next_value = value + 1;
        }

        // Register the enum
        if let Err(err) = self.ctx.register_type(enum_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register enum {}: {}", qualified_name, err),
                span: e.span,
            });
            return;
        }
        self.types_registered += 1;
    }

    fn evaluate_enum_value(&self, enumerator: &Enumerator<'_>, default: i64) -> i64 {
        // For now, only handle literal integer values
        // Full expression evaluation will come with Pass 2
        if let Some(angelscript_parser::ast::Expr::Literal(lit)) = enumerator.value
            && let angelscript_parser::ast::LiteralKind::Int(v) = lit.kind
        {
            return v;
        }
        default
    }

    // ==========================================================================
    // Function
    // ==========================================================================

    fn visit_function(&mut self, func: &FunctionDecl<'_>, object_type: Option<TypeHash>) {
        let name = func.name.name.to_string();
        let namespace = self.current_namespace_vec();

        // Resolve parameters
        let params = self.resolve_params(func.params, func.span);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Resolve return type
        let return_type = if let Some(ref ret) = func.return_type {
            let mut resolver = TypeResolver::new(self.ctx);
            match resolver.resolve(&ret.ty) {
                Ok(dt) => dt,
                Err(e) => {
                    self.ctx.add_error(e);
                    DataType::void()
                }
            }
        } else {
            DataType::void()
        };

        // Compute function hash
        let func_hash = if let Some(obj) = object_type {
            TypeHash::from_method(obj, &name, &param_hashes, func.is_const, false)
        } else {
            let qualified_name = self.qualified_name(&name);
            TypeHash::from_function(&qualified_name, &param_hashes)
        };

        let traits = FunctionTraits {
            is_const: func.is_const,
            is_final: func.attrs.final_,
            is_virtual: func.attrs.override_,
            ..FunctionTraits::default()
        };

        let func_def = FunctionDef::new(
            func_hash,
            name,
            namespace,
            params,
            return_type,
            object_type,
            traits,
            false,
            convert_visibility(func.visibility),
        );

        let source = angelscript_core::FunctionSource::script(func.span);
        let entry = FunctionEntry::script(func_def, self.unit_id, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register function: {}", e),
                span: func.span,
            });
        } else {
            self.functions_registered += 1;
        }
    }

    fn resolve_params(&mut self, params: &[FunctionParam<'_>], _span: Span) -> Vec<Param> {
        let mut result = Vec::with_capacity(params.len());

        for param in params {
            let mut resolver = TypeResolver::new(self.ctx);
            let data_type = match resolver.resolve_param(&param.ty) {
                Ok(dt) => dt,
                Err(e) => {
                    self.ctx.add_error(e);
                    DataType::void()
                }
            };

            let name = param
                .name
                .map(|id| id.name.to_string())
                .unwrap_or_else(|| format!("_param{}", result.len()));

            let p = if param.default.is_some() {
                Param::with_default(name, data_type)
            } else {
                Param::new(name, data_type)
            };

            result.push(p);
        }

        result
    }

    // ==========================================================================
    // Global Variable
    // ==========================================================================

    fn visit_global_var(&mut self, var: &GlobalVarDecl<'_>) {
        let name = var.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let namespace = self.current_namespace_vec();

        // Compute hash using the qualified name
        let type_hash = TypeHash::from_name(&qualified_name);

        // Resolve the variable's type
        let mut resolver = TypeResolver::new(self.ctx);
        let data_type = match resolver.resolve(&var.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                return;
            }
        };

        // Allocate slot in unit's global variable table
        let slot = self.next_global_slot;
        self.next_global_slot += 1;

        // Determine if const from the type expression
        let is_const = var.ty.is_const;

        let entry = GlobalPropertyEntry {
            name,
            namespace,
            qualified_name,
            type_hash,
            data_type,
            is_const,
            source: TypeSource::script(self.unit_id, var.span),
            implementation: GlobalPropertyImpl::Script { slot, data_type },
        };

        if let Err(e) = self.ctx.register_global(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register global: {}", e),
                span: var.span,
            });
        } else {
            self.globals_registered += 1;
        }
    }

    // ==========================================================================
    // Funcdef and Typedef
    // ==========================================================================

    fn visit_funcdef(&mut self, fd: &FuncdefDecl<'_>) {
        let name = fd.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);
        let namespace = self.current_namespace_vec();

        // Resolve parameter types
        let params: Vec<DataType> = fd
            .params
            .iter()
            .filter_map(|p| {
                let mut resolver = TypeResolver::new(self.ctx);
                resolver.resolve_param(&p.ty).ok()
            })
            .collect();

        // Resolve return type
        let mut resolver = TypeResolver::new(self.ctx);
        let return_type = match resolver.resolve(&fd.return_type.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                DataType::void()
            }
        };

        let source = TypeSource::script(self.unit_id, fd.span);
        let funcdef_entry = FuncdefEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            source,
            params,
            return_type,
        );

        if let Err(e) = self.ctx.register_type(funcdef_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register funcdef {}: {}", qualified_name, e),
                span: fd.span,
            });
            return;
        }
        self.types_registered += 1;
    }

    fn visit_typedef(&mut self, _td: &TypedefDecl<'_>) {
        // Typedef creates an alias - handled in type resolution
        // For now, we just skip registration as typedefs don't create new types
        // They're resolved as aliases during type resolution
    }

    // ==========================================================================
    // Helpers
    // ==========================================================================

    fn qualified_name(&self, name: &str) -> String {
        let ns = self.ctx.current_namespace();
        if ns.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", ns, name)
        }
    }

    fn current_namespace_vec(&self) -> Vec<String> {
        let ns = self.ctx.current_namespace();
        if ns.is_empty() {
            Vec::new()
        } else {
            ns.split("::").map(String::from).collect()
        }
    }
}

// ==========================================================================
// Visibility conversion
// ==========================================================================

/// Convert parser visibility to core visibility.
fn convert_visibility(v: angelscript_parser::ast::Visibility) -> Visibility {
    match v {
        angelscript_parser::ast::Visibility::Public => Visibility::Public,
        angelscript_parser::ast::Visibility::Private => Visibility::Private,
        angelscript_parser::ast::Visibility::Protected => Visibility::Protected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_parser::Parser;
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn setup_context() -> (SymbolRegistry, Bump) {
        let registry = SymbolRegistry::with_primitives();
        let arena = Bump::new();
        (registry, arena)
    }

    #[test]
    fn register_simple_class() {
        let (registry, arena) = setup_context();
        let source = "class Player {}";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert!(output.errors.is_empty());
        assert!(ctx.resolve_type("Player").is_some());
    }

    #[test]
    fn register_class_with_methods() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                void update() {}
                int getHealth() const { return 0; }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1); // Player class
        assert_eq!(output.functions_registered, 2); // update, getHealth
        assert!(output.errors.is_empty());
    }

    #[test]
    fn register_namespace() {
        let (registry, arena) = setup_context();
        let source = r#"
            namespace Game {
                class Entity {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert!(output.errors.is_empty());

        // Should be resolvable by qualified name
        assert!(ctx.resolve_type("Game::Entity").is_some());
    }

    #[test]
    fn register_global_variable() {
        let (registry, arena) = setup_context();
        let source = "int g_score = 0;";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.globals_registered, 1);
        assert!(output.errors.is_empty());
        assert!(ctx.resolve_global("g_score").is_some());
    }

    #[test]
    fn register_const_global() {
        let (registry, arena) = setup_context();
        let source = "const int MAX_PLAYERS = 8;";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.globals_registered, 1);
        assert!(output.errors.is_empty());

        let hash = ctx.resolve_global("MAX_PLAYERS").unwrap();
        let global = ctx.get_global_entry(hash).unwrap();
        assert!(global.is_const);
    }

    #[test]
    fn register_enum() {
        let (registry, arena) = setup_context();
        let source = r#"
            enum Color {
                Red,
                Green = 5,
                Blue
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert!(output.errors.is_empty());

        let hash = ctx.resolve_type("Color").unwrap();
        let entry = ctx.get_type(hash).unwrap();
        let enum_entry = entry.as_enum().unwrap();

        assert_eq!(enum_entry.values.len(), 3);
        assert_eq!(enum_entry.values[0].name, "Red");
        assert_eq!(enum_entry.values[0].value, 0);
        assert_eq!(enum_entry.values[1].name, "Green");
        assert_eq!(enum_entry.values[1].value, 5);
        assert_eq!(enum_entry.values[2].name, "Blue");
        assert_eq!(enum_entry.values[2].value, 6);
    }

    #[test]
    fn register_interface() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
                int getPriority() const;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert_eq!(output.functions_registered, 2);
        assert!(output.errors.is_empty());
        assert!(ctx.resolve_type("IDrawable").is_some());
    }

    #[test]
    fn register_funcdef() {
        let (registry, arena) = setup_context();
        let source = "funcdef void Callback(int value);";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert!(output.errors.is_empty());

        let hash = ctx.resolve_type("Callback").unwrap();
        let entry = ctx.get_type(hash).unwrap();
        assert!(entry.is_funcdef());
    }

    #[test]
    fn register_global_function() {
        let (registry, arena) = setup_context();
        let source = "void main() {}";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.functions_registered, 1);
        assert!(output.errors.is_empty());

        let funcs = ctx.resolve_function("main");
        assert!(funcs.is_some());
    }

    #[test]
    fn register_namespaced_global() {
        let (registry, arena) = setup_context();
        let source = r#"
            namespace Game {
                int g_level = 1;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.globals_registered, 1);
        assert!(output.errors.is_empty());

        // Should be resolvable by qualified name
        let hash = TypeHash::from_name("Game::g_level");
        assert!(ctx.get_global_entry(hash).is_some());
    }

    #[test]
    fn register_constructor() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                Player() {}
                Player(int health) {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert_eq!(output.functions_registered, 2); // Two constructors
        assert!(output.errors.is_empty());
    }

    #[test]
    fn register_destructor() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Resource {
                ~Resource() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        assert_eq!(output.functions_registered, 1); // Destructor
        assert!(output.errors.is_empty());
    }

    #[test]
    fn global_slot_allocation_is_sequential() {
        let (registry, arena) = setup_context();
        let source = r#"
            int a = 0;
            int b = 1;
            int c = 2;
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.globals_registered, 3);

        // Check slot allocation
        let a = ctx.get_global_entry(TypeHash::from_name("a")).unwrap();
        let b = ctx.get_global_entry(TypeHash::from_name("b")).unwrap();
        let c = ctx.get_global_entry(TypeHash::from_name("c")).unwrap();

        if let GlobalPropertyImpl::Script { slot, .. } = a.implementation {
            assert_eq!(slot, 0);
        } else {
            panic!("Expected Script implementation");
        }
        if let GlobalPropertyImpl::Script { slot, .. } = b.implementation {
            assert_eq!(slot, 1);
        } else {
            panic!("Expected Script implementation");
        }
        if let GlobalPropertyImpl::Script { slot, .. } = c.implementation {
            assert_eq!(slot, 2);
        } else {
            panic!("Expected Script implementation");
        }
    }
}
