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
    FunctionSource, FunctionTraits, GlobalPropertyEntry, GlobalPropertyImpl, InterfaceEntry,
    OperatorBehavior, Param, PropertyEntry, RefModifier, Span, TypeHash, TypeKind, TypeSource,
    UnitId, Visibility,
};
use angelscript_parser::ast::{
    ClassDecl, ClassMember, EnumDecl, Enumerator, FieldDecl, FuncdefDecl, FunctionDecl,
    FunctionParam, GlobalVarDecl, InterfaceDecl, InterfaceMember, InterfaceMethod, Item, MixinDecl,
    NamespaceDecl, Script, TypedefDecl, UsingNamespaceDecl, VirtualPropertyDecl,
};
use rustc_hash::FxHashMap;

use crate::context::CompilationContext;
use crate::passes::{PendingInheritance, PendingResolutions};
use crate::type_resolver::TypeResolver;

/// Tracks which special members a class has defined or deleted.
///
/// This consolidates the 6 separate Vec fields into a single hashmap,
/// providing O(1) lookup and reducing struct size.
#[derive(Debug, Default, Clone, Copy)]
struct ClassTraits {
    /// Class has any user-defined constructor (prevents default ctor generation).
    has_constructor: bool,
    /// Class has a user-defined copy constructor.
    has_copy_constructor: bool,
    /// Class has a user-defined opAssign.
    has_op_assign: bool,
    /// Default constructor is explicitly deleted.
    deleted_default_ctor: bool,
    /// Copy constructor is explicitly deleted.
    deleted_copy_ctor: bool,
    /// opAssign is explicitly deleted.
    deleted_op_assign: bool,
}

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
    /// Pending inheritance resolutions for Pass 1b.
    pub pending_resolutions: PendingResolutions,
}

/// Pass 1: Register all types and function signatures.
///
/// This pass walks the AST and registers type declarations and function signatures
/// into the compilation context's unit registry. It does not compile function bodies.
/// Inheritance references are collected but not resolved (enabling forward references).
pub struct RegistrationPass<'a, 'reg> {
    ctx: &'a mut CompilationContext<'reg>,
    unit_id: UnitId,
    types_registered: usize,
    functions_registered: usize,
    globals_registered: usize,
    /// Next slot index for script global variables.
    next_global_slot: u32,
    /// All script classes registered in this pass (for auto-generation).
    registered_classes: Vec<TypeHash>,
    /// Tracks special member traits (constructors, copy ctors, opAssign) per class.
    class_traits: FxHashMap<TypeHash, ClassTraits>,
    /// Pending inheritance resolutions (resolved in Pass 1b).
    pending_resolutions: PendingResolutions,
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
            registered_classes: Vec::new(),
            class_traits: FxHashMap::default(),
            pending_resolutions: PendingResolutions::default(),
        }
    }

    /// Run the registration pass on a script.
    pub fn run(mut self, script: &Script<'_>) -> RegistrationOutput {
        // Process all top-level items
        for item in script.items() {
            self.visit_item(item);
        }

        // Auto-generate missing default members
        self.generate_defaults();

        RegistrationOutput {
            types_registered: self.types_registered,
            functions_registered: self.functions_registered,
            globals_registered: self.globals_registered,
            errors: self.ctx.take_errors(),
            pending_resolutions: self.pending_resolutions,
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
            Item::Mixin(mixin) => self.visit_mixin(mixin),
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

        // Collect pending inheritance (don't resolve yet - enables forward references)
        if !class.inheritance.is_empty() {
            let pending: Vec<PendingInheritance> = class
                .inheritance
                .iter()
                .map(|expr| PendingInheritance {
                    name: self.ident_expr_to_string(expr),
                    span: expr.span,
                    namespace_context: self.current_namespace_vec(),
                    imports: self.ctx.imports().to_vec(),
                })
                .collect();
            self.pending_resolutions
                .class_inheritance
                .insert(type_hash, pending);
        }

        // Create the class entry WITHOUT inheritance (will be set in Pass 1b)
        let source = TypeSource::script(self.unit_id, class.span);
        let mut class_entry = ClassEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            TypeKind::ScriptObject,
            source,
        );

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
        self.registered_classes.push(type_hash);

        // Register class members
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    // Check if this is a constructor or destructor
                    if method.is_constructor() {
                        self.visit_constructor(method, type_hash);
                    } else if method.is_destructor {
                        self.visit_destructor(method, type_hash);
                    } else {
                        // Check if this is opAssign
                        if method.name.name == "opAssign" {
                            let traits = self.class_traits.entry(type_hash).or_default();
                            if method.attrs.delete {
                                // Deleted opAssign - track it but don't register
                                traits.deleted_op_assign = true;
                                continue;
                            }
                            traits.has_op_assign = true;
                        }
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
    }

    // ==========================================================================
    // Mixin
    // ==========================================================================

    fn visit_mixin(&mut self, mixin: &MixinDecl<'_>) {
        let class = &mixin.class;
        let name = class.name.name.to_string();
        let qualified_name = self.qualified_name(&name);
        let type_hash = TypeHash::from_name(&qualified_name);
        let namespace = self.current_namespace_vec();

        // Collect pending inheritance (don't resolve yet - enables forward references)
        // Note: Mixin inheritance validation (no base classes) happens in Pass 1b
        if !class.inheritance.is_empty() {
            let pending: Vec<PendingInheritance> = class
                .inheritance
                .iter()
                .map(|expr| PendingInheritance {
                    name: self.ident_expr_to_string(expr),
                    span: expr.span,
                    namespace_context: self.current_namespace_vec(),
                    imports: self.ctx.imports().to_vec(),
                })
                .collect();
            self.pending_resolutions
                .class_inheritance
                .insert(type_hash, pending);
        }

        // Create the mixin class entry WITHOUT inheritance (will be set in Pass 1b)
        let source = TypeSource::script(self.unit_id, class.span);
        let class_entry =
            ClassEntry::script_mixin(name.clone(), namespace, qualified_name.clone(), source);

        // Register the mixin
        if let Err(e) = self.ctx.register_type(class_entry.into()) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register mixin class {}: {}", qualified_name, e),
                span: class.span,
            });
            return;
        }
        self.types_registered += 1;

        // Note: We don't track mixins in registered_classes since they don't need
        // auto-generated constructors (mixins cannot be instantiated)

        // Register mixin members (methods, properties)
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    // Mixins don't have constructors/destructors that make sense
                    if method.is_constructor() || method.is_destructor {
                        self.ctx.add_error(CompilationError::InvalidOperation {
                            message: format!(
                                "mixin class '{}' cannot have constructors or destructors",
                                name
                            ),
                            span: method.span,
                        });
                        continue;
                    }
                    self.visit_function(method, Some(type_hash));
                }
                ClassMember::Field(field) => {
                    self.visit_field(field, type_hash);
                }
                ClassMember::VirtualProperty(prop) => {
                    self.visit_virtual_property(prop, type_hash);
                }
                ClassMember::Funcdef(fd) => {
                    self.visit_funcdef(fd);
                }
            }
        }
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

    fn visit_field(&mut self, field: &FieldDecl<'_>, class_hash: TypeHash) {
        // Resolve the field type
        let mut resolver = TypeResolver::new(self.ctx);
        let data_type = match resolver.resolve(&field.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                return;
            }
        };

        // Validate type is instantiable (not a mixin, interfaces must be handles)
        if let Err(e) =
            self.ctx
                .validate_instantiable_type(&data_type, field.ty.span, "as class field type")
        {
            self.ctx.add_error(e);
            return;
        }

        // Create PropertyEntry for this field (direct field, no getter/setter)
        let property = PropertyEntry::new(
            field.name.name.to_string(),
            data_type,
            convert_visibility(field.visibility),
            None, // No getter - direct field access
            None, // No setter - direct field access
        );

        // Add field to class properties
        if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
            class.properties.push(property);
        }
    }

    fn visit_virtual_property(&mut self, prop: &VirtualPropertyDecl<'_>, class_hash: TypeHash) {
        // Resolve property type first (shared across accessors)
        let mut resolver = TypeResolver::new(self.ctx);
        let prop_type = match resolver.resolve(&prop.ty.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.ctx.add_error(e);
                return;
            }
        };

        // Validate property type is instantiable (not a mixin, interfaces must be handles)
        if let Err(e) = self.ctx.validate_instantiable_type(
            &prop_type,
            prop.ty.span,
            "as virtual property type",
        ) {
            self.ctx.add_error(e);
            return;
        }

        // Track getter/setter hashes for the PropertyEntry
        let mut getter_hash: Option<TypeHash> = None;
        let mut setter_hash: Option<TypeHash> = None;

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

            // Create getter or setter signature
            let (params, return_type) = match accessor.kind {
                angelscript_parser::ast::PropertyAccessorKind::Get => (vec![], prop_type),
                angelscript_parser::ast::PropertyAccessorKind::Set => {
                    (vec![Param::new("value", prop_type)], DataType::void())
                }
            };

            let param_hashes: Vec<TypeHash> =
                params.iter().map(|p| p.data_type.type_hash).collect();
            let func_hash = TypeHash::from_method(class_hash, &method_name, &param_hashes);

            // Track the hash for the PropertyEntry
            match accessor.kind {
                angelscript_parser::ast::PropertyAccessorKind::Get => getter_hash = Some(func_hash),
                angelscript_parser::ast::PropertyAccessorKind::Set => setter_hash = Some(func_hash),
            }

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

        // Create PropertyEntry with getter/setter hashes
        let property = PropertyEntry::new(
            prop.name.name.to_string(),
            prop_type,
            convert_visibility(prop.visibility),
            getter_hash,
            setter_hash,
        );

        // Add property to class
        if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
            class.properties.push(property);
        }
    }

    fn visit_constructor(&mut self, ctor: &FunctionDecl<'_>, class_hash: TypeHash) {
        // Resolve parameters
        let params = self.resolve_params(ctor.params);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Check if this is deleted
        let is_deleted = ctor.attrs.delete;

        // Check if this is a copy constructor (single param of const ClassName &in)
        let is_copy_constructor = params.len() == 1 && {
            let param = &params[0];
            param.data_type.type_hash == class_hash
                && param.data_type.is_const
                && param.data_type.ref_modifier == RefModifier::In
        };

        // Check if this is a default constructor (no params)
        let is_default_constructor = params.is_empty();

        // Track user-defined or deleted constructors to prevent auto-generation
        let traits = self.class_traits.entry(class_hash).or_default();
        if is_deleted {
            // Track deleted constructors - these prevent auto-generation
            if is_default_constructor {
                traits.deleted_default_ctor = true;
            }
            if is_copy_constructor {
                traits.deleted_copy_ctor = true;
            }
            // Any constructor (deleted or not) counts as "having a constructor"
            // to prevent default constructor auto-generation
            traits.has_constructor = true;
            // Deleted constructors are not registered - they don't exist
            return;
        }

        // Track that this class has a user-defined constructor
        traits.has_constructor = true;

        if is_copy_constructor {
            traits.has_copy_constructor = true;
        }

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

        let source = FunctionSource::script(ctor.span);
        let entry = FunctionEntry::script(func_def, self.unit_id, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!("failed to register constructor: {}", e),
                span: ctor.span,
            });
        } else {
            self.functions_registered += 1;

            // Add constructor to class's behaviors.constructors
            if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
                class.behaviors.constructors.push(func_hash);
            }
        }
    }

    fn visit_destructor(&mut self, dtor: &FunctionDecl<'_>, class_hash: TypeHash) {
        // Destructor has no parameters and no return value
        let func_hash = TypeHash::from_method(class_hash, "~", &[]);

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

            // Add destructor to class's behaviors.destructor
            if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
                class.behaviors.destructor = Some(func_hash);
            }
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

        // Collect pending base interfaces (don't resolve yet - enables forward references)
        if !iface.bases.is_empty() {
            let pending: Vec<PendingInheritance> = iface
                .bases
                .iter()
                .map(|base| PendingInheritance {
                    name: base.name.to_string(),
                    span: iface.span,
                    namespace_context: self.current_namespace_vec(),
                    imports: self.ctx.imports().to_vec(),
                })
                .collect();
            self.pending_resolutions
                .interface_bases
                .insert(type_hash, pending);
        }

        let source = TypeSource::script(self.unit_id, iface.span);
        // Create interface entry WITHOUT base interfaces (will be set in Pass 1b)
        let interface_entry = InterfaceEntry::new(
            name.clone(),
            namespace,
            qualified_name.clone(),
            type_hash,
            source,
        );

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
        let params = self.resolve_params(method.params);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Resolve return type
        let mut resolver = TypeResolver::new(self.ctx);
        let return_type = match resolver.resolve(&method.return_type.ty) {
            Ok(dt) => {
                // Validate return type is instantiable (not a mixin, interfaces must be handles)
                if let Err(e) = self.ctx.validate_instantiable_type(
                    &dt,
                    method.return_type.span,
                    "as interface method return type",
                ) {
                    self.ctx.add_error(e);
                }
                dt
            }
            Err(e) => {
                self.ctx.add_error(e);
                DataType::void()
            }
        };

        let func_hash = TypeHash::from_method(iface_hash, &name, &param_hashes);

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
        let params = self.resolve_params(func.params);
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Resolve return type
        let return_type = if let Some(ref ret) = func.return_type {
            let mut resolver = TypeResolver::new(self.ctx);
            match resolver.resolve(&ret.ty) {
                Ok(dt) => {
                    // Validate return type is instantiable (not a mixin, interfaces must be handles)
                    if let Err(e) = self.ctx.validate_instantiable_type(
                        &dt,
                        ret.span,
                        "as function return type",
                    ) {
                        self.ctx.add_error(e);
                    }
                    dt
                }
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
            TypeHash::from_method(obj, &name, &param_hashes)
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

            // Add method to class's method map (for interface compliance checking etc.)
            if let Some(obj_hash) = object_type
                && let Some(class) = self.ctx.unit_registry_mut().get_class_mut(obj_hash)
            {
                class.add_method(func.name.name, func_hash);

                // Also add operator methods to behaviors.operators for O(1) lookup
                // For conversion operators, use return type as target type
                let target_type = if return_type.is_handle || !return_type.is_void() {
                    Some(return_type.type_hash)
                } else {
                    None
                };

                if let Some(op_behavior) =
                    OperatorBehavior::from_method_name(func.name.name, target_type)
                {
                    class
                        .behaviors
                        .operators
                        .entry(op_behavior)
                        .or_default()
                        .push(func_hash);
                }
            }
        }
    }

    fn resolve_params(&mut self, params: &[FunctionParam<'_>]) -> Vec<Param> {
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

            // Validate parameter type is instantiable (not a mixin, interfaces must be handles)
            // Use parameter type span for precise error location
            if let Err(e) = self.ctx.validate_instantiable_type(
                &data_type,
                param.ty.span,
                "as function parameter type",
            ) {
                self.ctx.add_error(e);
            }

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

        // Validate type is instantiable (not a mixin)
        if let Err(e) =
            self.ctx
                .validate_instantiable_type(&data_type, var.ty.span, "as global variable type")
        {
            self.ctx.add_error(e);
            return;
        }

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

        // Resolve parameter types with validation
        let params: Vec<DataType> = fd
            .params
            .iter()
            .filter_map(|p| {
                let mut resolver = TypeResolver::new(self.ctx);
                match resolver.resolve_param(&p.ty) {
                    Ok(dt) => {
                        // Validate parameter type is instantiable
                        if let Err(e) = self.ctx.validate_instantiable_type(
                            &dt,
                            p.ty.span,
                            "as funcdef parameter type",
                        ) {
                            self.ctx.add_error(e);
                            return None;
                        }
                        Some(dt)
                    }
                    Err(e) => {
                        self.ctx.add_error(e);
                        None
                    }
                }
            })
            .collect();

        // Resolve return type with validation
        let mut resolver = TypeResolver::new(self.ctx);
        let return_type = match resolver.resolve(&fd.return_type.ty) {
            Ok(dt) => {
                // Validate return type is instantiable
                if let Err(e) = self.ctx.validate_instantiable_type(
                    &dt,
                    fd.return_type.span,
                    "as funcdef return type",
                ) {
                    self.ctx.add_error(e);
                }
                dt
            }
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

    // ==========================================================================
    // Auto-Generation
    // ==========================================================================

    /// Generate default constructors, copy constructors, and opAssign for classes
    /// that don't have user-defined versions and haven't been explicitly deleted.
    fn generate_defaults(&mut self) {
        // Clone the list to avoid borrow issues
        let classes: Vec<TypeHash> = self.registered_classes.clone();

        for class_hash in classes {
            // Get class info
            let (class_name, namespace) = match self.ctx.get_type(class_hash) {
                Some(entry) => {
                    if let Some(class) = entry.as_class() {
                        (class.name.clone(), class.namespace.clone())
                    } else {
                        continue;
                    }
                }
                None => continue,
            };

            // Get traits for this class (default if none recorded)
            let traits = self
                .class_traits
                .get(&class_hash)
                .copied()
                .unwrap_or_default();

            // Generate default constructor if no user-defined constructor exists
            // and it hasn't been explicitly deleted
            if !traits.has_constructor && !traits.deleted_default_ctor {
                self.generate_default_constructor(class_hash, &class_name, &namespace);
            }

            // Generate copy constructor if no user-defined copy constructor exists
            // and it hasn't been explicitly deleted
            if !traits.has_copy_constructor && !traits.deleted_copy_ctor {
                self.generate_copy_constructor(class_hash, &class_name, &namespace);
            }

            // Generate opAssign if no user-defined opAssign exists
            // and it hasn't been explicitly deleted
            if !traits.has_op_assign && !traits.deleted_op_assign {
                self.generate_op_assign(class_hash, &class_name, &namespace);
            }
        }
    }

    /// Generate a default constructor (no parameters) for a class.
    fn generate_default_constructor(
        &mut self,
        class_hash: TypeHash,
        class_name: &str,
        namespace: &[String],
    ) {
        let func_hash = TypeHash::from_constructor(class_hash, &[]);

        let func_def = FunctionDef::new(
            func_hash,
            class_name.to_string(),
            namespace.to_vec(),
            vec![], // No parameters
            DataType::void(),
            Some(class_hash),
            FunctionTraits::constructor(),
            false,
            Visibility::Public,
        );

        let source = FunctionSource::auto_generated();
        let entry = FunctionEntry::auto_default_constructor(func_def, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!(
                    "failed to register auto-generated default constructor for {}: {}",
                    class_name, e
                ),
                span: Span::default(),
            });
        } else {
            self.functions_registered += 1;

            // Add auto-generated constructor to class's behaviors.constructors
            if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
                class.behaviors.constructors.push(func_hash);
            }
        }
    }

    /// Generate a copy constructor (takes `const ClassName &in`) for a class.
    fn generate_copy_constructor(
        &mut self,
        class_hash: TypeHash,
        class_name: &str,
        namespace: &[String],
    ) {
        // Parameter type: const ClassName &in
        let param_type = DataType {
            type_hash: class_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::In,
            is_mixin: false,
            is_interface: false,
        };

        let params = vec![Param::new("other", param_type)];
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let func_hash = TypeHash::from_constructor(class_hash, &param_hashes);

        let func_def = FunctionDef::new(
            func_hash,
            class_name.to_string(),
            namespace.to_vec(),
            params,
            DataType::void(),
            Some(class_hash),
            FunctionTraits::constructor(),
            false,
            Visibility::Public,
        );

        let source = FunctionSource::auto_generated();
        let entry = FunctionEntry::auto_copy_constructor(func_def, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!(
                    "failed to register auto-generated copy constructor for {}: {}",
                    class_name, e
                ),
                span: Span::default(),
            });
        } else {
            self.functions_registered += 1;

            // Add auto-generated copy constructor to class's behaviors.constructors
            if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
                class.behaviors.constructors.push(func_hash);
            }
        }
    }

    /// Generate opAssign (takes `const ClassName &in`, returns `ClassName&`) for a class.
    fn generate_op_assign(&mut self, class_hash: TypeHash, class_name: &str, namespace: &[String]) {
        // Parameter type: const ClassName &in
        let param_type = DataType {
            type_hash: class_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::In,
            is_mixin: false,
            is_interface: false,
        };

        let params = vec![Param::new("other", param_type)];
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();

        // Return type: ClassName& (reference to self)
        let return_type = DataType {
            type_hash: class_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::InOut,
            is_mixin: false,
            is_interface: false,
        };

        let func_hash = TypeHash::from_method(class_hash, "opAssign", &param_hashes);

        let func_def = FunctionDef::new(
            func_hash,
            "opAssign".to_string(),
            namespace.to_vec(),
            params,
            return_type,
            Some(class_hash),
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );

        let source = FunctionSource::auto_generated();
        let entry = FunctionEntry::auto_op_assign(func_def, source);

        if let Err(e) = self.ctx.register_function(entry) {
            self.ctx.add_error(CompilationError::Other {
                message: format!(
                    "failed to register auto-generated opAssign for {}: {}",
                    class_name, e
                ),
                span: Span::default(),
            });
        } else {
            self.functions_registered += 1;

            // Add auto-generated opAssign to class's behaviors.operators
            if let Some(class) = self.ctx.unit_registry_mut().get_class_mut(class_hash) {
                class
                    .behaviors
                    .operators
                    .entry(OperatorBehavior::OpAssign)
                    .or_default()
                    .push(func_hash);
            }
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
    use crate::passes::TypeCompletionPass;
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
        // update, getHealth + auto-generated (default ctor, copy ctor, opAssign)
        assert_eq!(output.functions_registered, 5);
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
        // Two user-defined constructors + auto-generated copy constructor + auto-generated opAssign
        assert_eq!(output.functions_registered, 4);
        assert!(output.errors.is_empty());

        // Verify constructors are added to behaviors
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        // Should have 3 constructors in behaviors: 2 user-defined + 1 auto-generated copy
        assert_eq!(
            player_entry.behaviors.constructors.len(),
            3,
            "Expected 3 constructors in behaviors (2 user + 1 copy)"
        );

        // Verify by hash
        let default_ctor = TypeHash::from_constructor(player_hash, &[]);
        let int_hash = ctx.resolve_type("int").unwrap();
        let param_ctor = TypeHash::from_constructor(player_hash, &[int_hash]);
        let copy_ctor = TypeHash::from_constructor(player_hash, &[player_hash]);
        assert!(player_entry.behaviors.constructors.contains(&default_ctor));
        assert!(player_entry.behaviors.constructors.contains(&param_ctor));
        assert!(player_entry.behaviors.constructors.contains(&copy_ctor));

        // Verify auto-generated opAssign is in behaviors.operators
        let op_assign_ops = player_entry
            .behaviors
            .operators
            .get(&OperatorBehavior::OpAssign);
        assert!(
            op_assign_ops.is_some(),
            "behaviors.operators should contain OpAssign"
        );
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
        // Destructor + auto-generated (default ctor, copy ctor, opAssign)
        assert_eq!(output.functions_registered, 4);
        assert!(output.errors.is_empty());

        // Verify destructor is added to behaviors
        let resource_hash = ctx.resolve_type("Resource").unwrap();
        let resource_entry = ctx.get_type(resource_hash).unwrap().as_class().unwrap();

        assert!(
            resource_entry.behaviors.destructor.is_some(),
            "behaviors.destructor should be set"
        );

        // Verify by hash
        let dtor_hash = TypeHash::from_method(resource_hash, "~", &[]);
        assert_eq!(resource_entry.behaviors.destructor, Some(dtor_hash));

        // Verify auto-generated constructors are in behaviors
        assert_eq!(
            resource_entry.behaviors.constructors.len(),
            2,
            "Expected 2 auto-generated constructors (default + copy)"
        );
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

    // ==========================================================================
    // Auto-generation tests
    // ==========================================================================

    #[test]
    fn auto_generate_default_constructor() {
        let (registry, arena) = setup_context();
        // Class with no constructor - should auto-generate default constructor
        let source = "class Player {}";
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // Should have auto-generated: default constructor, copy constructor, opAssign
        assert_eq!(output.functions_registered, 3);
        assert!(output.errors.is_empty());

        // Verify auto-generated constructors are in behaviors
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(
            player_entry.behaviors.constructors.len(),
            2,
            "Expected 2 auto-generated constructors (default + copy)"
        );

        // Verify by hash
        let default_ctor = TypeHash::from_constructor(player_hash, &[]);
        let copy_ctor = TypeHash::from_constructor(player_hash, &[player_hash]);
        assert!(
            player_entry.behaviors.constructors.contains(&default_ctor),
            "behaviors.constructors should contain default constructor"
        );
        assert!(
            player_entry.behaviors.constructors.contains(&copy_ctor),
            "behaviors.constructors should contain copy constructor"
        );

        // Verify auto-generated opAssign is in behaviors.operators
        let op_assign_ops = player_entry
            .behaviors
            .operators
            .get(&OperatorBehavior::OpAssign);
        assert!(
            op_assign_ops.is_some(),
            "behaviors.operators should contain auto-generated OpAssign"
        );
        let op_assign_hash = TypeHash::from_method(player_hash, "opAssign", &[player_hash]);
        assert!(
            op_assign_ops.unwrap().contains(&op_assign_hash),
            "behaviors.operators[OpAssign] should contain correct hash"
        );
    }

    #[test]
    fn no_auto_generate_when_constructor_exists() {
        let (registry, arena) = setup_context();
        // Class with explicit constructor - should NOT auto-generate default constructor
        // but should still auto-generate copy constructor and opAssign
        let source = r#"
            class Player {
                Player(int health) {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // user-defined constructor + auto-generated copy constructor + auto-generated opAssign
        assert_eq!(output.functions_registered, 3);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn no_auto_generate_when_copy_constructor_exists() {
        let (registry, arena) = setup_context();
        // Class with explicit copy constructor
        let source = r#"
            class Player {
                Player(const Player &in other) {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // user-defined copy constructor (also counts as constructor) + auto-generated opAssign
        // No default constructor or copy constructor auto-generated
        assert_eq!(output.functions_registered, 2);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn no_auto_generate_when_op_assign_exists() {
        let (registry, arena) = setup_context();
        // Class with explicit opAssign
        let source = r#"
            class Player {
                Player& opAssign(const Player &in other) { return this; }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // user-defined opAssign + auto-generated default constructor + auto-generated copy constructor
        assert_eq!(output.functions_registered, 3);
        assert!(output.errors.is_empty());

        // Verify user-defined opAssign is in behaviors.operators
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        let op_assign_ops = player_entry
            .behaviors
            .operators
            .get(&OperatorBehavior::OpAssign);
        assert!(
            op_assign_ops.is_some(),
            "behaviors.operators should contain user-defined OpAssign"
        );
        assert_eq!(op_assign_ops.unwrap().len(), 1);
    }

    #[test]
    fn deleted_default_constructor_prevents_auto_generation() {
        let (registry, arena) = setup_context();
        // Class with deleted default constructor
        let source = r#"
            class NonDefaultConstructible {
                NonDefaultConstructible() delete;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // deleted default constructor is not registered
        // auto-generated copy constructor + auto-generated opAssign
        assert_eq!(output.functions_registered, 2);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn deleted_copy_constructor_prevents_auto_generation() {
        let (registry, arena) = setup_context();
        // Class with deleted copy constructor
        let source = r#"
            class NonCopyable {
                NonCopyable(const NonCopyable &in) delete;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // deleted copy constructor is not registered (also counts as constructor so no default auto-gen)
        // auto-generated opAssign only
        assert_eq!(output.functions_registered, 1);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn deleted_op_assign_prevents_auto_generation() {
        let (registry, arena) = setup_context();
        // Class with deleted opAssign
        let source = r#"
            class NonAssignable {
                NonAssignable& opAssign(const NonAssignable &in) delete;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // deleted opAssign is not registered
        // auto-generated default constructor + auto-generated copy constructor
        assert_eq!(output.functions_registered, 2);
        assert!(output.errors.is_empty());
    }

    #[test]
    fn all_deleted_prevents_all_auto_generation() {
        let (registry, arena) = setup_context();
        // Class with all special members deleted
        let source = r#"
            class Static {
                Static() delete;
                Static(const Static &in) delete;
                Static& opAssign(const Static &in) delete;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert_eq!(output.types_registered, 1);
        // All deleted - no functions registered
        assert_eq!(output.functions_registered, 0);
        assert!(output.errors.is_empty());
    }

    // ==========================================================================
    // Inheritance validation tests (Task 41c)
    // ==========================================================================

    #[test]
    fn register_class_cannot_extend_ffi_class() {
        let (mut registry, arena) = setup_context();

        // Register an FFI class
        let ffi_class = ClassEntry::ffi("GameObject", TypeKind::reference());
        registry.register_type(ffi_class.into()).unwrap();

        // Try to extend it from script
        let source = r#"
            class Player : GameObject {
                void update() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed (inheritance is deferred)
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );

        // Run completion pass - this is where inheritance validation happens
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        // Should have an error from completion
        assert_eq!(comp_output.errors.len(), 1);
        match &comp_output.errors[0] {
            CompilationError::InvalidOperation { message, .. } => {
                assert!(
                    message.contains("cannot extend FFI class"),
                    "Expected 'cannot extend FFI class' in message, got: {}",
                    message
                );
            }
            other => panic!("Expected InvalidOperation error, got: {:?}", other),
        }
    }

    #[test]
    fn register_class_cannot_extend_final_class() {
        let (registry, arena) = setup_context();
        let source = r#"
            final class Entity {
                void update() {}
            }

            class Player : Entity {
                void render() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed (inheritance is deferred)
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );

        // Run completion pass - this is where inheritance validation happens
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        // Should have an error from completion
        assert_eq!(comp_output.errors.len(), 1);
        match &comp_output.errors[0] {
            CompilationError::InvalidOperation { message, .. } => {
                assert!(
                    message.contains("cannot extend final class"),
                    "Expected 'cannot extend final class' in message, got: {}",
                    message
                );
            }
            other => panic!("Expected InvalidOperation error, got: {:?}", other),
        }
    }

    #[test]
    fn register_class_can_extend_script_class() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Entity {
                void update() {}
            }

            class Player : Entity {
                void render() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 2);

        // Run completion pass to resolve inheritance
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify Player has Entity as base
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap();
        let player_class = player_entry.as_class().unwrap();
        assert!(player_class.base_class.is_some());
    }

    #[test]
    fn register_class_can_implement_ffi_interface() {
        use angelscript_core::MethodSignature;

        let (mut registry, arena) = setup_context();

        // Register an FFI interface
        let iface_hash = TypeHash::from_name("IDrawable");
        let draw_method = MethodSignature::new("draw", vec![], DataType::void());
        let interface = InterfaceEntry::ffi("IDrawable").with_method(draw_method);
        registry.register_type(interface.into()).unwrap();

        let source = r#"
            class Sprite : IDrawable {
                void draw() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 1);

        // Run completion pass to resolve interface implementation
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify Sprite implements IDrawable
        let sprite_hash = ctx.resolve_type("Sprite").unwrap();
        let sprite_entry = ctx.get_type(sprite_hash).unwrap();
        let sprite_class = sprite_entry.as_class().unwrap();
        assert!(
            sprite_class.interfaces.contains(&iface_hash),
            "Sprite should have IDrawable in interfaces"
        );

        // Verify the draw method was added to the class's method map
        let draw_methods = sprite_class.find_methods("draw");
        assert_eq!(
            draw_methods.len(),
            1,
            "Expected exactly 1 draw method in class.methods"
        );

        // Verify the function is registered correctly
        let draw_hash = draw_methods[0];
        let draw_func = ctx.unit_registry().get_function(draw_hash);
        assert!(draw_func.is_some(), "draw function should be in registry");

        let draw_func = draw_func.unwrap();
        assert_eq!(draw_func.def.name, "draw");
        assert_eq!(draw_func.def.object_type, Some(sprite_hash));
        assert_eq!(draw_func.def.params.len(), 0);
        assert_eq!(draw_func.def.return_type, DataType::void());
    }

    // ==========================================================================
    // Mixin tests (Task 41e)
    // ==========================================================================

    #[test]
    fn register_mixin_class() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void helpMethod() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );
        assert_eq!(output.types_registered, 1);
        // 1 method registered (no auto-generated members for mixins)
        assert_eq!(output.functions_registered, 1);

        // Verify it's registered as a mixin
        let helper_hash = ctx.resolve_type("Helper").unwrap();
        let helper = ctx.get_type(helper_hash).unwrap().as_class().unwrap();
        assert!(helper.is_mixin);
    }

    #[test]
    fn register_mixin_cannot_inherit_from_class() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Base {}
            mixin class Helper : Base {}
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed (inheritance is deferred)
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );

        // Run completion pass - this is where mixin inheritance validation happens
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        // Should have error about mixin inheriting from class
        assert!(!comp_output.errors.is_empty());
        assert!(comp_output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("cannot inherit"))
        }));
    }

    #[test]
    fn register_mixin_with_interface() {
        use angelscript_core::MethodSignature;

        let (mut registry, arena) = setup_context();

        // Register an interface
        let iface = InterfaceEntry::ffi("IDrawable").with_method(MethodSignature::new(
            "draw",
            vec![],
            DataType::void(),
        ));
        registry.register_type(iface.into()).unwrap();

        let source = r#"
            mixin class RenderMixin : IDrawable {
                void draw() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        // Registration should succeed
        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 1);

        // Run completion pass to resolve interface implementation
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify mixin has the interface
        let mixin_hash = ctx.resolve_type("RenderMixin").unwrap();
        let mixin_entry = ctx.get_type(mixin_hash).unwrap().as_class().unwrap();
        assert!(mixin_entry.is_mixin);
        assert_eq!(mixin_entry.interfaces.len(), 1);
    }

    #[test]
    fn register_class_with_mixin() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void help() {}
            }

            class Player : Helper {
                void update() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 2); // Helper (mixin) + Player

        // Run completion pass to resolve mixin
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify Player has the mixin in its mixins list
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();
        assert_eq!(player_entry.mixins.len(), 1);

        let helper_hash = ctx.resolve_type("Helper").unwrap();
        assert!(player_entry.mixins.contains(&helper_hash));
    }

    #[test]
    fn register_class_with_base_and_mixin() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Entity {
                void update() {}
            }

            mixin class Helper {
                void help() {}
            }

            class Player : Entity, Helper {
                void render() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 3); // Entity + Helper + Player

        // Run completion pass to resolve base class and mixin
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify Player has base class and mixin
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        let entity_hash = ctx.resolve_type("Entity").unwrap();
        assert_eq!(player_entry.base_class, Some(entity_hash));

        let helper_hash = ctx.resolve_type("Helper").unwrap();
        assert!(player_entry.mixins.contains(&helper_hash));
    }

    #[test]
    fn register_mixin_cannot_have_constructor() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                Helper() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Should have error about mixin constructor
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("constructor"))
        }));
    }

    #[test]
    fn register_mixin_cannot_have_destructor() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                ~Helper() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Should have error about mixin destructor
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("destructor"))
        }));
    }

    #[test]
    fn register_class_with_multiple_mixins() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Movable {
                void move() {}
            }

            mixin class Renderable {
                void render() {}
            }

            class Player : Movable, Renderable {
                void update() {}
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let reg_output = pass.run(&script);

        assert!(
            reg_output.errors.is_empty(),
            "Expected no registration errors, got: {:?}",
            reg_output.errors
        );
        assert_eq!(reg_output.types_registered, 3); // Movable + Renderable + Player

        // Run completion pass to resolve mixins
        let comp_pass = TypeCompletionPass::new(
            ctx.unit_registry_mut(),
            &registry,
            reg_output.pending_resolutions,
        );
        let comp_output = comp_pass.run();

        assert!(
            comp_output.errors.is_empty(),
            "Expected no completion errors, got: {:?}",
            comp_output.errors
        );

        // Verify Player has both mixins
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();
        assert_eq!(player_entry.mixins.len(), 2);

        let movable_hash = ctx.resolve_type("Movable").unwrap();
        let renderable_hash = ctx.resolve_type("Renderable").unwrap();
        assert!(player_entry.mixins.contains(&movable_hash));
        assert!(player_entry.mixins.contains(&renderable_hash));
    }

    #[test]
    fn mixin_global_variable_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void help() {}
            }

            Helper h;
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Should have error about mixin instantiation
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("Helper") && message.contains("instantiable"))
        }));
    }

    #[test]
    fn mixin_global_handle_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void help() {}
            }

            Helper@ h;
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Handle to mixin should also be rejected
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("Helper") && message.contains("instantiable"))
        }));
    }

    #[test]
    fn interface_global_value_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            IDrawable d;
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface value type should be rejected
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("interface") && message.contains("IDrawable") && message.contains("handle"))
        }));
    }

    #[test]
    fn interface_global_handle_allowed() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            IDrawable@ d;
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface handle type should be allowed
        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );
        assert_eq!(output.globals_registered, 1);
    }

    // ==========================================================================
    // Field registration tests (Task 42 - visit_field behavior)
    // ==========================================================================

    #[test]
    fn field_added_to_class_properties() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health;
                float speed;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );

        // Verify fields are added to class.properties
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 2);
        assert_eq!(player_entry.properties[0].name, "health");
        assert_eq!(player_entry.properties[1].name, "speed");
    }

    #[test]
    fn field_is_direct_field_not_virtual_property() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        // Verify field has no getter/setter (is_direct_field)
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 1);
        let health_prop = &player_entry.properties[0];
        assert!(
            health_prop.getter.is_none(),
            "Direct field should have no getter"
        );
        assert!(
            health_prop.setter.is_none(),
            "Direct field should have no setter"
        );
    }

    #[test]
    fn field_type_mixin_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void help() {}
            }

            class Player {
                Helper helper;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Should have error about mixin as field type
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("Helper"))
        }));
    }

    #[test]
    fn field_type_interface_value_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Player {
                IDrawable drawable;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface value type as field should be rejected
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("interface") && message.contains("IDrawable") && message.contains("handle"))
        }));
    }

    #[test]
    fn field_type_interface_handle_allowed() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Player {
                IDrawable@ drawable;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface handle type as field should be allowed
        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );

        // Verify field is registered
        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();
        assert_eq!(player_entry.properties.len(), 1);
        assert_eq!(player_entry.properties[0].name, "drawable");
    }

    #[test]
    fn field_preserves_correct_data_type() {
        use angelscript_core::primitives;

        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                const int health;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 1);
        let health_prop = &player_entry.properties[0];
        assert_eq!(health_prop.data_type.type_hash, primitives::INT32);
        assert!(
            health_prop.data_type.is_const,
            "Field should preserve const modifier"
        );
    }

    #[test]
    fn field_preserves_visibility() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                private int secret;
                protected int shared;
                int public_field;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 3);
        assert_eq!(player_entry.properties[0].visibility, Visibility::Private);
        assert_eq!(player_entry.properties[1].visibility, Visibility::Protected);
        assert_eq!(player_entry.properties[2].visibility, Visibility::Public);
    }

    // ==========================================================================
    // Virtual property registration tests (Task 42 - visit_virtual_property behavior)
    // ==========================================================================

    #[test]
    fn virtual_property_has_getter_hash() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health { get { return 0; } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 1);
        let health_prop = &player_entry.properties[0];
        assert_eq!(health_prop.name, "health");
        assert!(
            health_prop.getter.is_some(),
            "Virtual property should have getter hash"
        );
        assert!(
            health_prop.setter.is_none(),
            "Read-only property should have no setter"
        );
    }

    #[test]
    fn virtual_property_has_setter_hash() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health { set { } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 1);
        let health_prop = &player_entry.properties[0];
        assert!(
            health_prop.getter.is_none(),
            "Write-only property should have no getter"
        );
        assert!(
            health_prop.setter.is_some(),
            "Virtual property should have setter hash"
        );
    }

    #[test]
    fn virtual_property_has_both_getter_and_setter() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health { get { return 0; } set { } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        assert_eq!(player_entry.properties.len(), 1);
        let health_prop = &player_entry.properties[0];
        assert!(health_prop.getter.is_some(), "Property should have getter");
        assert!(health_prop.setter.is_some(), "Property should have setter");
    }

    #[test]
    fn virtual_property_type_mixin_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            mixin class Helper {
                void help() {}
            }

            class Player {
                Helper helper { get { return Helper(); } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Mixin as property type should be rejected
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("mixin") && message.contains("Helper"))
        }));
    }

    #[test]
    fn virtual_property_type_interface_value_rejected() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Player {
                IDrawable drawable { get { return null; } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface value type as property should be rejected
        assert!(!output.errors.is_empty());
        assert!(output.errors.iter().any(|e| {
            matches!(e, CompilationError::InvalidOperation { message, .. }
                if message.contains("interface") && message.contains("IDrawable"))
        }));
    }

    #[test]
    fn virtual_property_type_interface_handle_allowed() {
        let (registry, arena) = setup_context();
        let source = r#"
            interface IDrawable {
                void draw();
            }

            class Player {
                IDrawable@ drawable { get { return null; } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        // Interface handle type as property should be allowed
        assert!(
            output.errors.is_empty(),
            "Expected no errors, got: {:?}",
            output.errors
        );

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();
        assert_eq!(player_entry.properties.len(), 1);
    }

    #[test]
    fn virtual_property_accessor_methods_registered() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int health { get { return 0; } set { } }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        // Verify accessor methods are registered (get_health, set_health)
        // Plus auto-generated: default ctor, copy ctor, opAssign = 5 total
        assert_eq!(output.functions_registered, 5);
    }

    #[test]
    fn mixed_fields_and_virtual_properties() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Player {
                int directField;
                int virtualProp { get { return 0; } set { } }
                float anotherField;
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        let player_hash = ctx.resolve_type("Player").unwrap();
        let player_entry = ctx.get_type(player_hash).unwrap().as_class().unwrap();

        // All three should be in properties
        assert_eq!(player_entry.properties.len(), 3);

        // directField - no getter/setter
        assert_eq!(player_entry.properties[0].name, "directField");
        assert!(player_entry.properties[0].getter.is_none());
        assert!(player_entry.properties[0].setter.is_none());

        // virtualProp - has getter and setter
        assert_eq!(player_entry.properties[1].name, "virtualProp");
        assert!(player_entry.properties[1].getter.is_some());
        assert!(player_entry.properties[1].setter.is_some());

        // anotherField - no getter/setter
        assert_eq!(player_entry.properties[2].name, "anotherField");
        assert!(player_entry.properties[2].getter.is_none());
        assert!(player_entry.properties[2].setter.is_none());
    }

    // ==========================================================================
    // Conversion operator behavior test - demonstrates O(1) lookup with target type
    // ==========================================================================

    #[test]
    fn conversion_operator_added_to_behaviors_with_target_type() {
        let (registry, arena) = setup_context();
        let source = r#"
            class Wrapper {
                int value;
                int opConv() const { return value; }
                int opImplConv() const { return value; }
            }
        "#;
        let script = Parser::parse(source, &arena).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        let pass = RegistrationPass::new(&mut ctx, UnitId::new(0));
        let output = pass.run(&script);

        assert!(output.errors.is_empty());

        let wrapper_hash = ctx.resolve_type("Wrapper").unwrap();
        let wrapper_entry = ctx.get_type(wrapper_hash).unwrap().as_class().unwrap();

        // Should have opConv(int) in behaviors.operators with int as target
        let int_hash = ctx.resolve_type("int").unwrap();
        let op_conv = wrapper_entry
            .behaviors
            .operators
            .get(&OperatorBehavior::OpConv(int_hash));
        assert!(
            op_conv.is_some(),
            "behaviors.operators should contain OpConv(int)"
        );

        // Should have opImplConv(int) in behaviors.operators with int as target
        let op_impl_conv = wrapper_entry
            .behaviors
            .operators
            .get(&OperatorBehavior::OpImplConv(int_hash));
        assert!(
            op_impl_conv.is_some(),
            "behaviors.operators should contain OpImplConv(int)"
        );
    }
}
