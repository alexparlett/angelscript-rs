//! Pass 1: Registration
//!
//! Registers all types and functions with complete signatures in a single AST walk.
//!
//! # Architecture
//!
//! The registration pass runs in three phases:
//!
//! 1. **Phase 1a: Register Type Names** - Walk AST and register all type names
//!    (classes, interfaces, enums, funcdefs) so they can be referenced.
//!
//! 2. **Phase 1b: Register Functions** - Walk AST again and register all functions
//!    with complete signatures (parameters fully resolved).
//!
//! 3. **Phase 1c: Validate Relationships** - Validate inheritance hierarchies
//!    and interface implementations.
//!
//! # Key Difference from Old Architecture
//!
//! In the old 3-pass architecture, functions were registered with empty signatures
//! in Pass 1, then filled in Pass 2a. This new 2-pass architecture registers
//! functions with **complete signatures immediately**, eliminating the need for
//! a separate signature-filling pass.
//!
//! # Example
//!
//! ```ignore
//! use angelscript_compiler::{CompilationContext, passes::RegistrationPass};
//! use angelscript_parser::ast::Script;
//!
//! let mut context = CompilationContext::default();
//! let output = RegistrationPass::run(&mut context, script.items());
//!
//! if output.errors.is_empty() {
//!     // All types and functions registered successfully
//! }
//! ```

use rustc_hash::FxHashMap;

use angelscript_parser::ast::decl::{
    ClassDecl, ClassMember, EnumDecl, FieldDecl, FuncdefDecl, FunctionDecl, FunctionParam,
    GlobalVarDecl, InterfaceDecl, InterfaceMember, InterfaceMethod, Item, MixinDecl,
    NamespaceDecl, TypedefDecl, UsingNamespaceDecl, VirtualPropertyDecl,
};
use angelscript_parser::ast::types::{ParamType, PrimitiveType, ReturnType, TypeBase, TypeExpr, TypeSuffix};
use angelscript_parser::ast::{PropertyAccessorKind, RefKind, Visibility as AstVisibility};
use angelscript_parser::lexer::Span;

use crate::context::CompilationContext;
use crate::types::{
    DataType, FieldDef, FunctionDef, FunctionTraits, MethodSignature, OperatorBehavior, Param,
    PropertyAccessors, RefModifier, TypeBehaviors, TypeDef, TypeHash, TypeKind, Visibility,
    primitives,
};

/// Output from Pass 1: Registration.
#[derive(Debug)]
pub struct RegistrationOutput {
    /// Errors found during registration.
    pub errors: Vec<RegistrationError>,
}

impl RegistrationOutput {
    /// Check if registration completed without errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Error during registration.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RegistrationError {
    /// Type not found during resolution.
    #[error("at {span}: unknown type '{name}'")]
    UnknownType { name: String, span: Span },

    /// Duplicate type declaration.
    #[error("at {span}: duplicate type '{name}'")]
    DuplicateType { name: String, span: Span },

    /// Duplicate function declaration (same signature).
    #[error("at {span}: duplicate function '{name}'")]
    DuplicateFunction { name: String, span: Span },

    /// Invalid base class.
    #[error("at {span}: invalid base class '{name}'")]
    InvalidBaseClass { name: String, span: Span },

    /// Invalid interface.
    #[error("at {span}: invalid interface '{name}'")]
    InvalidInterface { name: String, span: Span },

    /// Circular inheritance detected.
    #[error("at {span}: circular inheritance detected for '{name}'")]
    CircularInheritance { name: String, span: Span },

    /// Enum value is not a constant expression.
    #[error("at {span}: enum value must be a constant expression")]
    NonConstantEnumValue { span: Span },
}

impl RegistrationError {
    fn unknown_type(name: impl Into<String>, span: Span) -> Self {
        Self::UnknownType {
            name: name.into(),
            span,
        }
    }

    fn duplicate_type(name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateType {
            name: name.into(),
            span,
        }
    }

    fn invalid_base_class(name: impl Into<String>, span: Span) -> Self {
        Self::InvalidBaseClass {
            name: name.into(),
            span,
        }
    }

    #[allow(dead_code)]
    fn invalid_interface(name: impl Into<String>, span: Span) -> Self {
        Self::InvalidInterface {
            name: name.into(),
            span,
        }
    }
}

/// Pass 1: Registration
///
/// Walks the AST and registers all types and functions with complete signatures.
pub struct RegistrationPass<'a> {
    /// The compilation context we're building.
    context: &'a mut CompilationContext,

    /// Errors accumulated during registration.
    errors: Vec<RegistrationError>,

    /// Current class being processed (for method registration).
    current_class: Option<TypeHash>,

    /// Track declared type names for duplicate detection.
    declared_types: FxHashMap<String, Span>,
}

impl<'a> RegistrationPass<'a> {
    /// Run the registration pass on the given items.
    ///
    /// This is the main entry point for Pass 1.
    pub fn run(context: &'a mut CompilationContext, items: &[Item<'_>]) -> RegistrationOutput {
        let mut pass = Self {
            context,
            errors: Vec::new(),
            current_class: None,
            declared_types: FxHashMap::default(),
        };

        // Phase 1a: Register all type names first
        for item in items {
            pass.register_type_names(item);
        }

        // Phase 1b: Register all functions with complete signatures
        for item in items {
            pass.register_declarations(item);
        }

        // Phase 1c: Validate inheritance and interfaces
        pass.validate_type_relationships();

        RegistrationOutput {
            errors: pass.errors,
        }
    }

    // =========================================================================
    // Phase 1a: Register Type Names
    // =========================================================================

    /// Register type names from an item (Phase 1a).
    fn register_type_names(&mut self, item: &Item<'_>) {
        match item {
            Item::Class(class) => self.register_class_name(class),
            Item::Interface(iface) => self.register_interface_name(iface),
            Item::Enum(enum_decl) => self.register_enum_name(enum_decl),
            Item::Funcdef(funcdef) => self.register_funcdef_name(funcdef),
            Item::Namespace(ns) => self.register_namespace_type_names(ns),
            Item::Mixin(mixin) => self.register_mixin_name(mixin),
            Item::Typedef(typedef) => self.register_typedef(typedef),
            // These don't introduce type names
            Item::Function(_) | Item::GlobalVar(_) | Item::Import(_) | Item::UsingNamespace(_) => {}
        }
    }

    fn register_class_name(&mut self, class: &ClassDecl<'_>) {
        let qualified_name = self.context.qualified_name(class.name.name);

        // Check for duplicates
        if self.declared_types.contains_key(&qualified_name) {
            self.errors.push(RegistrationError::duplicate_type(
                &qualified_name,
                class.span,
            ));
            return;
        }

        self.declared_types.insert(qualified_name.clone(), class.span);

        let type_hash = TypeHash::from_name(&qualified_name);

        // All script classes are reference types
        // TODO: handle value types when `final` + no handles
        let _shared = class.modifiers.shared;
        let type_kind = TypeKind::script_object();

        // Create placeholder TypeDef - will be filled in Phase 1b
        let typedef = TypeDef::Class {
            name: class.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            type_hash,
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: class.modifiers.final_,
            is_abstract: class.modifiers.abstract_,
            template_params: class.template_params.iter().map(|p| TypeHash::from_name(p.name)).collect(),
            template: None,
            type_args: Vec::new(),
            type_kind,
        };

        self.context.register_type(typedef);
    }

    fn register_interface_name(&mut self, iface: &InterfaceDecl<'_>) {
        let qualified_name = self.context.qualified_name(iface.name.name);

        if self.declared_types.contains_key(&qualified_name) {
            self.errors.push(RegistrationError::duplicate_type(
                &qualified_name,
                iface.span,
            ));
            return;
        }

        self.declared_types.insert(qualified_name.clone(), iface.span);

        let type_hash = TypeHash::from_name(&qualified_name);

        let typedef = TypeDef::Interface {
            name: iface.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            type_hash,
            methods: Vec::new(), // Filled in Phase 1b
        };

        self.context.register_type(typedef);
    }

    fn register_enum_name(&mut self, enum_decl: &EnumDecl<'_>) {
        let qualified_name = self.context.qualified_name(enum_decl.name.name);

        if self.declared_types.contains_key(&qualified_name) {
            self.errors.push(RegistrationError::duplicate_type(
                &qualified_name,
                enum_decl.span,
            ));
            return;
        }

        self.declared_types.insert(qualified_name.clone(), enum_decl.span);

        let type_hash = TypeHash::from_name(&qualified_name);

        // Evaluate enum values
        let mut values = Vec::new();
        let mut next_value: i64 = 0;

        for enumerator in enum_decl.enumerators {
            let value = if let Some(expr) = enumerator.value {
                // Try to evaluate constant expression
                match self.eval_const_int(expr) {
                    Some(v) => {
                        next_value = v + 1;
                        v
                    }
                    None => {
                        self.errors.push(RegistrationError::NonConstantEnumValue {
                            span: enumerator.span,
                        });
                        let v = next_value;
                        next_value += 1;
                        v
                    }
                }
            } else {
                let v = next_value;
                next_value += 1;
                v
            };

            values.push((enumerator.name.name.to_string(), value));
        }

        let typedef = TypeDef::Enum {
            name: enum_decl.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            type_hash,
            values,
        };

        self.context.register_type(typedef);
    }

    fn register_funcdef_name(&mut self, funcdef: &FuncdefDecl<'_>) {
        let qualified_name = self.context.qualified_name(funcdef.name.name);

        if self.declared_types.contains_key(&qualified_name) {
            self.errors.push(RegistrationError::duplicate_type(
                &qualified_name,
                funcdef.span,
            ));
            return;
        }

        self.declared_types.insert(qualified_name.clone(), funcdef.span);

        // Funcdef params/return will be resolved in Phase 1b
        // For now, register with empty signature
        let type_hash = TypeHash::from_name(&qualified_name);

        let typedef = TypeDef::Funcdef {
            name: funcdef.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            type_hash,
            params: Vec::new(),
            return_type: DataType::void(),
        };

        self.context.register_type(typedef);
    }

    fn register_mixin_name(&mut self, mixin: &MixinDecl<'_>) {
        // Mixins are registered as classes but tracked separately
        // For now, just register the class name
        self.register_class_name(&mixin.class);
    }

    fn register_typedef(&mut self, typedef: &TypedefDecl<'_>) {
        // Typedef creates an alias to an existing type
        let alias_name = self.context.qualified_name(typedef.name.name);

        // Resolve the target type
        match self.resolve_type_expr(&typedef.base_type) {
            Ok(target_type) => {
                self.context.register_type_alias(&alias_name, target_type.type_hash);
            }
            Err(e) => {
                self.errors.push(e);
            }
        }
    }

    fn register_namespace_type_names(&mut self, ns: &NamespaceDecl<'_>) {
        // Enter namespace
        for segment in ns.path {
            self.context.enter_namespace(segment.name);
        }

        // Register type names in namespace
        for item in ns.items {
            self.register_type_names(item);
        }

        // Exit namespace
        for _ in ns.path {
            self.context.exit_namespace();
        }
    }

    // =========================================================================
    // Phase 1b: Register Declarations
    // =========================================================================

    /// Register declarations from an item (Phase 1b).
    fn register_declarations(&mut self, item: &Item<'_>) {
        match item {
            Item::Class(class) => self.register_class(class),
            Item::Interface(iface) => self.register_interface(iface),
            Item::Funcdef(funcdef) => self.register_funcdef(funcdef),
            Item::Function(func) => {
                self.register_function(func, None);
            }
            Item::GlobalVar(var) => self.register_global_var(var),
            Item::Namespace(ns) => self.register_namespace_declarations(ns),
            Item::UsingNamespace(using) => self.register_using_namespace(using),
            // Already handled in Phase 1a or no declarations
            Item::Enum(_) | Item::Typedef(_) | Item::Mixin(_) | Item::Import(_) => {}
        }
    }

    fn register_class(&mut self, class: &ClassDecl<'_>) {
        let qualified_name = self.context.qualified_name(class.name.name);
        let type_hash = TypeHash::from_name(&qualified_name);

        // Resolve base class and interfaces
        let mut base_class = None;
        let mut interfaces = Vec::new();

        for (i, inherit) in class.inheritance.iter().enumerate() {
            let inherit_name = if let Some(scope) = &inherit.scope {
                let mut parts: Vec<&str> = scope.segments.iter().map(|s| s.name).collect();
                parts.push(inherit.ident.name);
                parts.join("::")
            } else {
                inherit.ident.name.to_string()
            };

            match self.context.resolve_type(&inherit_name) {
                Ok(hash) => {
                    if let Some(typedef) = self.context.get_type(hash) {
                        if typedef.is_class() {
                            if i == 0 && base_class.is_none() {
                                base_class = Some(hash);
                            } else {
                                self.errors.push(RegistrationError::invalid_base_class(
                                    &inherit_name,
                                    class.span,
                                ));
                            }
                        } else if typedef.is_interface() {
                            interfaces.push(hash);
                        } else {
                            self.errors.push(RegistrationError::invalid_base_class(
                                &inherit_name,
                                class.span,
                            ));
                        }
                    }
                }
                Err(_) => {
                    self.errors.push(RegistrationError::unknown_type(
                        &inherit_name,
                        class.span,
                    ));
                }
            }
        }

        // Set current class context
        self.current_class = Some(type_hash);

        // Collect fields, methods, and properties
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut operator_methods: FxHashMap<OperatorBehavior, Vec<TypeHash>> = FxHashMap::default();
        let mut properties: FxHashMap<String, PropertyAccessors> = FxHashMap::default();
        let mut behaviors = TypeBehaviors::default();

        for member in class.members {
            match member {
                ClassMember::Method(func) => {
                    if let Some((func_hash, return_type_hash)) = self.register_function(func, Some(type_hash)) {
                        // Check if this is a constructor, destructor, or operator
                        if func.is_constructor() {
                            behaviors.constructors.push(func_hash);
                        } else if func.is_destructor {
                            behaviors.destructor = Some(func_hash);
                        } else if let Some(op) = self.get_operator_behavior(func.name.name, return_type_hash) {
                            operator_methods.entry(op).or_default().push(func_hash);
                        } else {
                            methods.push(func_hash);
                        }
                    }
                }
                ClassMember::Field(field) => {
                    if let Some(field_def) = self.register_field(field) {
                        fields.push(field_def);
                    }
                }
                ClassMember::VirtualProperty(prop) => {
                    if let Some((name, accessors)) = self.register_virtual_property(prop, type_hash) {
                        properties.insert(name, accessors);
                    }
                }
                ClassMember::Funcdef(_funcdef) => {
                    // Nested funcdef - register in class namespace
                    // TODO: handle nested funcdefs
                }
            }
        }

        // Clear current class context
        self.current_class = None;

        // Update the TypeDef with resolved information
        if let Some(typedef) = self.context.script_mut().get_type_mut(type_hash) {
            if let TypeDef::Class {
                base_class: bc,
                interfaces: ifaces,
                fields: f,
                methods: m,
                operator_methods: om,
                properties: p,
                ..
            } = typedef
            {
                *bc = base_class;
                *ifaces = interfaces;
                *f = fields;
                *m = methods;
                *om = operator_methods;
                *p = properties;
            }
        }

        // Register behaviors
        if !behaviors.is_empty() {
            self.context.script_mut().register_behaviors(type_hash, behaviors);
        }
    }

    fn register_interface(&mut self, iface: &InterfaceDecl<'_>) {
        let qualified_name = self.context.qualified_name(iface.name.name);
        let type_hash = TypeHash::from_name(&qualified_name);

        let mut methods = Vec::new();

        for member in iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    if let Some(sig) = self.build_method_signature(method) {
                        methods.push(sig);
                    }
                }
                InterfaceMember::VirtualProperty(_prop) => {
                    // Interface virtual properties - skip for now
                }
            }
        }

        // Update the TypeDef with resolved methods
        if let Some(typedef) = self.context.script_mut().get_type_mut(type_hash) {
            if let TypeDef::Interface {
                methods: m, ..
            } = typedef
            {
                *m = methods;
            }
        }
    }

    fn register_funcdef(&mut self, funcdef: &FuncdefDecl<'_>) {
        let qualified_name = self.context.qualified_name(funcdef.name.name);
        let type_hash = TypeHash::from_name(&qualified_name);

        // Resolve parameters
        let params: Vec<DataType> = funcdef
            .params
            .iter()
            .filter_map(|p| self.resolve_param_type(&p.ty).ok())
            .collect();

        // Resolve return type
        let return_type = self
            .resolve_return_type(&funcdef.return_type)
            .unwrap_or_else(|_| DataType::void());

        // Update the TypeDef with resolved signature
        if let Some(typedef) = self.context.script_mut().get_type_mut(type_hash) {
            if let TypeDef::Funcdef {
                params: p,
                return_type: rt,
                ..
            } = typedef
            {
                *p = params;
                *rt = return_type;
            }
        }
    }

    /// Register a function and return (func_hash, return_type_hash).
    ///
    /// Returns the function hash and the return type hash. The return type hash
    /// is needed for conversion operators (opCast, opConv, etc.) which are keyed
    /// by their target type.
    fn register_function(
        &mut self,
        func: &FunctionDecl<'_>,
        object_type: Option<TypeHash>,
    ) -> Option<(TypeHash, TypeHash)> {
        // Resolve parameters
        let params: Vec<Param> = func
            .params
            .iter()
            .filter_map(|p| self.build_param(p))
            .collect();

        // Resolve return type
        let return_type = if let Some(rt) = &func.return_type {
            self.resolve_return_type(rt).unwrap_or_else(|e| {
                self.errors.push(e);
                DataType::void()
            })
        } else {
            DataType::void()
        };

        let return_type_hash = return_type.type_hash;

        // Compute function hash
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        let is_const = func.is_const;
        let return_is_const = return_type.is_const;

        let func_hash = if let Some(owner) = object_type {
            if func.is_constructor() {
                TypeHash::from_constructor(owner, &param_hashes)
            } else if self.is_operator_name(func.name.name) {
                TypeHash::from_operator(owner, func.name.name, &param_hashes, is_const, return_is_const)
            } else {
                TypeHash::from_method(owner, func.name.name, &param_hashes, is_const, return_is_const)
            }
        } else {
            let qualified_name = self.context.qualified_name(func.name.name);
            TypeHash::from_function(&qualified_name, &param_hashes)
        };

        // Build function traits
        let traits = FunctionTraits {
            is_constructor: func.is_constructor(),
            is_destructor: func.is_destructor,
            is_final: func.attrs.final_,
            is_virtual: func.attrs.override_ || !func.attrs.final_,
            is_abstract: func.body.is_none() && object_type.is_some(),
            is_const: func.is_const,
            is_explicit: func.attrs.explicit,
            auto_generated: None,
        };

        // Build FunctionDef
        let func_def = FunctionDef {
            func_hash,
            name: func.name.name.to_string(),
            namespace: self.context.namespace_path().to_vec(),
            params,
            return_type,
            object_type,
            traits,
            is_native: false,
            visibility: convert_visibility(func.visibility),
        };

        self.context.register_function(func_def);

        Some((func_hash, return_type_hash))
    }

    fn register_field(&mut self, field: &FieldDecl<'_>) -> Option<FieldDef> {
        let data_type = match self.resolve_type_expr(&field.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.errors.push(e);
                return None;
            }
        };

        Some(FieldDef {
            name: field.name.name.to_string(),
            data_type,
            visibility: convert_visibility(field.visibility),
        })
    }

    fn register_virtual_property(
        &mut self,
        prop: &VirtualPropertyDecl<'_>,
        owner_type: TypeHash,
    ) -> Option<(String, PropertyAccessors)> {
        let prop_type = match self.resolve_return_type(&prop.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.errors.push(e);
                return None;
            }
        };

        let mut getter = None;
        let mut setter = None;

        for accessor in prop.accessors {
            match accessor.kind {
                PropertyAccessorKind::Get => {
                    // Register getter method
                    let func_hash = TypeHash::from_method(
                        owner_type,
                        &format!("get_{}", prop.name.name),
                        &[],
                        accessor.is_const,
                        prop_type.is_const,
                    );
                    getter = Some(func_hash);

                    // Register the getter function
                    let func_def = FunctionDef {
                        func_hash,
                        name: format!("get_{}", prop.name.name),
                        namespace: self.context.namespace_path().to_vec(),
                        params: Vec::new(),
                        return_type: prop_type,
                        object_type: Some(owner_type),
                        traits: FunctionTraits {
                            is_const: accessor.is_const,
                            is_final: accessor.attrs.final_,
                            is_virtual: !accessor.attrs.final_,
                            ..FunctionTraits::default()
                        },
                        is_native: false,
                        visibility: convert_visibility(prop.visibility),
                    };
                    self.context.register_function(func_def);
                }
                PropertyAccessorKind::Set => {
                    // Register setter method
                    let func_hash = TypeHash::from_method(
                        owner_type,
                        &format!("set_{}", prop.name.name),
                        &[prop_type.type_hash],
                        false,
                        false,
                    );
                    setter = Some(func_hash);

                    // Register the setter function
                    let func_def = FunctionDef {
                        func_hash,
                        name: format!("set_{}", prop.name.name),
                        namespace: self.context.namespace_path().to_vec(),
                        params: vec![Param::new("value", prop_type)],
                        return_type: DataType::void(),
                        object_type: Some(owner_type),
                        traits: FunctionTraits {
                            is_final: accessor.attrs.final_,
                            is_virtual: !accessor.attrs.final_,
                            ..FunctionTraits::default()
                        },
                        is_native: false,
                        visibility: convert_visibility(prop.visibility),
                    };
                    self.context.register_function(func_def);
                }
            }
        }

        Some((
            prop.name.name.to_string(),
            PropertyAccessors {
                getter,
                setter,
                visibility: convert_visibility(prop.visibility),
            },
        ))
    }

    fn register_global_var(&mut self, _var: &GlobalVarDecl<'_>) {
        // Global variables are tracked separately
        // For now, skip registration
    }

    fn register_namespace_declarations(&mut self, ns: &NamespaceDecl<'_>) {
        // Enter namespace
        for segment in ns.path {
            self.context.enter_namespace(segment.name);
        }

        // Register declarations in namespace
        for item in ns.items {
            self.register_declarations(item);
        }

        // Exit namespace
        for _ in ns.path {
            self.context.exit_namespace();
        }
    }

    fn register_using_namespace(&mut self, using: &UsingNamespaceDecl<'_>) {
        let path: Vec<&str> = using.path.iter().map(|p| p.name).collect();
        let ns = path.join("::");
        self.context.add_import(&ns);
    }

    // =========================================================================
    // Phase 1c: Validate Relationships
    // =========================================================================

    fn validate_type_relationships(&mut self) {
        // Validate inheritance hierarchies for circular dependencies
        // This is a simplified check - a full implementation would use
        // graph algorithms to detect cycles

        // For now, we rely on the context's is_subclass_of which handles
        // basic chain following. More complex validation could be added later.
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn resolve_type_expr(&self, type_expr: &TypeExpr<'_>) -> Result<DataType, RegistrationError> {
        let type_hash = self.resolve_type_base(type_expr)?;

        // Determine modifiers
        let is_const = type_expr.is_const;
        let mut is_handle = false;
        let mut is_handle_to_const = false;

        for suffix in type_expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const: _handle_const } => {
                    is_handle = true;
                    is_handle_to_const = is_const; // Leading const applies to the object
                    // Note: _handle_const refers to the handle itself being const
                }
            }
        }

        Ok(DataType {
            type_hash,
            is_const: is_const && !is_handle, // For handles, const applies to object
            is_handle,
            is_handle_to_const,
            ref_modifier: RefModifier::None,
        })
    }

    fn resolve_type_base(&self, type_expr: &TypeExpr<'_>) -> Result<TypeHash, RegistrationError> {
        match &type_expr.base {
            TypeBase::Primitive(prim) => Ok(primitive_to_hash(*prim)),
            TypeBase::Named(ident) => {
                let name = if let Some(scope) = &type_expr.scope {
                    let mut parts: Vec<&str> = scope.segments.iter().map(|s| s.name).collect();
                    parts.push(ident.name);
                    parts.join("::")
                } else {
                    ident.name.to_string()
                };

                self.context
                    .resolve_type(&name)
                    .map_err(|_| RegistrationError::unknown_type(&name, type_expr.span))
            }
            TypeBase::Auto => Ok(primitives::VOID), // Auto resolved later
            TypeBase::Unknown => Ok(primitives::VOID),
            TypeBase::TemplateParam(ident) => {
                // Template parameter - use a placeholder hash
                Ok(TypeHash::from_name(&format!("__template_{}", ident.name)))
            }
        }
    }

    fn resolve_param_type(&self, param_type: &ParamType<'_>) -> Result<DataType, RegistrationError> {
        let mut data_type = self.resolve_type_expr(&param_type.ty)?;

        // Apply reference modifier
        data_type.ref_modifier = convert_ref_kind(param_type.ref_kind);

        Ok(data_type)
    }

    fn resolve_return_type(&self, return_type: &ReturnType<'_>) -> Result<DataType, RegistrationError> {
        let mut data_type = self.resolve_type_expr(&return_type.ty)?;

        if return_type.is_ref {
            data_type.ref_modifier = RefModifier::InOut; // Return by reference
        }

        Ok(data_type)
    }

    fn build_param(&mut self, param: &FunctionParam<'_>) -> Option<Param> {
        let data_type = match self.resolve_param_type(&param.ty) {
            Ok(dt) => dt,
            Err(e) => {
                self.errors.push(e);
                return None;
            }
        };

        let name = param
            .name
            .map(|n| n.name.to_string())
            .unwrap_or_else(|| "_".to_string());

        let has_default = param.default.is_some();

        Some(if has_default {
            Param::with_default(name, data_type)
        } else {
            Param::new(name, data_type)
        })
    }

    fn build_method_signature(&mut self, method: &InterfaceMethod<'_>) -> Option<MethodSignature> {
        let return_type = match self.resolve_return_type(&method.return_type) {
            Ok(rt) => rt,
            Err(e) => {
                self.errors.push(e);
                return None;
            }
        };

        let params: Vec<DataType> = method
            .params
            .iter()
            .filter_map(|p| self.resolve_param_type(&p.ty).ok())
            .collect();

        Some(MethodSignature {
            name: method.name.name.to_string(),
            params,
            return_type,
            is_const: method.is_const,
        })
    }

    fn is_operator_name(&self, name: &str) -> bool {
        name.starts_with("op")
            && matches!(
                name,
                "opAdd" | "opSub" | "opMul" | "opDiv" | "opMod"
                    | "opAddAssign" | "opSubAssign" | "opMulAssign" | "opDivAssign" | "opModAssign"
                    | "opEquals" | "opCmp"
                    | "opIndex" | "opCall"
                    | "opCast" | "opImplCast" | "opConv" | "opImplConv"
                    | "opNeg" | "opCom" | "opPreInc" | "opPreDec" | "opPostInc" | "opPostDec"
                    | "opAnd" | "opOr" | "opXor" | "opShl" | "opShr" | "opUShr"
                    | "opAndAssign" | "opOrAssign" | "opXorAssign" | "opShlAssign" | "opShrAssign" | "opUShrAssign"
                    | "opAssign" | "opPow" | "opPowAssign"
            )
    }

    /// Build an OperatorBehavior from a method name and return type.
    ///
    /// The return type is needed for conversion operators (opCast, opConv, etc.)
    /// which are keyed by their target type.
    fn get_operator_behavior(&self, name: &str, return_type: TypeHash) -> Option<OperatorBehavior> {
        match name {
            // Conversion operators - keyed by target type
            "opCast" => Some(OperatorBehavior::OpCast(return_type)),
            "opImplCast" => Some(OperatorBehavior::OpImplCast(return_type)),
            "opConv" => Some(OperatorBehavior::OpConv(return_type)),
            "opImplConv" => Some(OperatorBehavior::OpImplConv(return_type)),
            // All other operators - delegate to simple name lookup
            _ => Self::operator_from_name(name),
        }
    }

    /// Map operator method name to OperatorBehavior (for non-conversion operators).
    fn operator_from_name(name: &str) -> Option<OperatorBehavior> {
        match name {
            "opAdd" => Some(OperatorBehavior::OpAdd),
            "opSub" => Some(OperatorBehavior::OpSub),
            "opMul" => Some(OperatorBehavior::OpMul),
            "opDiv" => Some(OperatorBehavior::OpDiv),
            "opMod" => Some(OperatorBehavior::OpMod),
            "opAddAssign" => Some(OperatorBehavior::OpAddAssign),
            "opSubAssign" => Some(OperatorBehavior::OpSubAssign),
            "opMulAssign" => Some(OperatorBehavior::OpMulAssign),
            "opDivAssign" => Some(OperatorBehavior::OpDivAssign),
            "opModAssign" => Some(OperatorBehavior::OpModAssign),
            "opEquals" => Some(OperatorBehavior::OpEquals),
            "opCmp" => Some(OperatorBehavior::OpCmp),
            "opIndex" => Some(OperatorBehavior::OpIndex),
            "opCall" => Some(OperatorBehavior::OpCall),
            "opNeg" => Some(OperatorBehavior::OpNeg),
            "opCom" => Some(OperatorBehavior::OpCom),
            "opPreInc" => Some(OperatorBehavior::OpPreInc),
            "opPreDec" => Some(OperatorBehavior::OpPreDec),
            "opPostInc" => Some(OperatorBehavior::OpPostInc),
            "opPostDec" => Some(OperatorBehavior::OpPostDec),
            "opAnd" => Some(OperatorBehavior::OpAnd),
            "opOr" => Some(OperatorBehavior::OpOr),
            "opXor" => Some(OperatorBehavior::OpXor),
            "opShl" => Some(OperatorBehavior::OpShl),
            "opShr" => Some(OperatorBehavior::OpShr),
            "opUShr" => Some(OperatorBehavior::OpUShr),
            "opAndAssign" => Some(OperatorBehavior::OpAndAssign),
            "opOrAssign" => Some(OperatorBehavior::OpOrAssign),
            "opXorAssign" => Some(OperatorBehavior::OpXorAssign),
            "opShlAssign" => Some(OperatorBehavior::OpShlAssign),
            "opShrAssign" => Some(OperatorBehavior::OpShrAssign),
            "opUShrAssign" => Some(OperatorBehavior::OpUShrAssign),
            "opAssign" => Some(OperatorBehavior::OpAssign),
            "opPow" => Some(OperatorBehavior::OpPow),
            "opPowAssign" => Some(OperatorBehavior::OpPowAssign),
            _ => None,
        }
    }

    /// Evaluate a constant integer expression.
    ///
    /// This is a simplified implementation - a full implementation would
    /// handle more expression types (e.g., constant lookups from context).
    #[allow(clippy::only_used_in_recursion)]
    fn eval_const_int(&self, expr: &angelscript_parser::ast::Expr<'_>) -> Option<i64> {
        use angelscript_parser::ast::Expr;
        use angelscript_parser::ast::expr::LiteralKind;

        match expr {
            Expr::Literal(lit) => match &lit.kind {
                LiteralKind::Int(v) => Some(*v),
                _ => None,
            },
            Expr::Unary(unary) => {
                use angelscript_parser::ast::UnaryOp;
                let inner = self.eval_const_int(unary.operand)?;
                match unary.op {
                    UnaryOp::Neg => Some(-inner),
                    UnaryOp::Plus => Some(inner),
                    UnaryOp::BitwiseNot => Some(!inner),
                    _ => None,
                }
            }
            Expr::Binary(binary) => {
                use angelscript_parser::ast::BinaryOp;
                let left = self.eval_const_int(binary.left)?;
                let right = self.eval_const_int(binary.right)?;
                match binary.op {
                    BinaryOp::Add => Some(left + right),
                    BinaryOp::Sub => Some(left - right),
                    BinaryOp::Mul => Some(left * right),
                    BinaryOp::Div => right.checked_div(left).or(Some(0)),
                    BinaryOp::Mod => right.checked_rem(left).or(Some(0)),
                    BinaryOp::BitwiseAnd => Some(left & right),
                    BinaryOp::BitwiseOr => Some(left | right),
                    BinaryOp::BitwiseXor => Some(left ^ right),
                    BinaryOp::ShiftLeft => Some(left << (right as u32)),
                    BinaryOp::ShiftRight => Some(left >> (right as u32)),
                    _ => None,
                }
            }
            Expr::Paren(paren) => self.eval_const_int(paren.expr),
            _ => None,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn convert_visibility(vis: AstVisibility) -> Visibility {
    match vis {
        AstVisibility::Public => Visibility::Public,
        AstVisibility::Protected => Visibility::Protected,
        AstVisibility::Private => Visibility::Private,
    }
}

fn convert_ref_kind(ref_kind: RefKind) -> RefModifier {
    match ref_kind {
        RefKind::None => RefModifier::None,
        RefKind::Ref => RefModifier::InOut,
        RefKind::RefIn => RefModifier::In,
        RefKind::RefOut => RefModifier::Out,
        RefKind::RefInOut => RefModifier::InOut,
    }
}

fn primitive_to_hash(prim: PrimitiveType) -> TypeHash {
    match prim {
        PrimitiveType::Void => primitives::VOID,
        PrimitiveType::Bool => primitives::BOOL,
        PrimitiveType::Int => primitives::INT32,
        PrimitiveType::Int8 => primitives::INT8,
        PrimitiveType::Int16 => primitives::INT16,
        PrimitiveType::Int64 => primitives::INT64,
        PrimitiveType::UInt => primitives::UINT32,
        PrimitiveType::UInt8 => primitives::UINT8,
        PrimitiveType::UInt16 => primitives::UINT16,
        PrimitiveType::UInt64 => primitives::UINT64,
        PrimitiveType::Float => primitives::FLOAT,
        PrimitiveType::Double => primitives::DOUBLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumpalo::Bump;
    use angelscript_parser::Parser;

    fn parse_and_register(source: &str) -> (CompilationContext, RegistrationOutput) {
        let arena = Bump::new();
        let (script, _errors) = Parser::parse_lenient(source, &arena);

        let mut context = CompilationContext::default();
        let output = RegistrationPass::run(&mut context, script.items());

        (context, output)
    }

    #[test]
    fn register_empty_script() {
        let (context, output) = parse_and_register("");
        assert!(output.is_ok());
        // Only primitives from FFI
        assert!(context.lookup_type("int").is_some());
    }

    #[test]
    fn register_simple_class() {
        let (context, output) = parse_and_register("class Player { }");
        assert!(output.is_ok());
        assert!(context.lookup_type("Player").is_some());
    }

    #[test]
    fn register_class_with_methods() {
        let (context, output) = parse_and_register(
            "class Player {
                void update(float dt) { }
                int getHealth() const { return 0; }
            }",
        );
        assert!(output.is_ok());

        let player_hash = context.lookup_type("Player").unwrap();
        let methods = context.get_methods(player_hash);
        assert_eq!(methods.len(), 2);
    }

    #[test]
    fn register_class_with_constructor() {
        let (context, output) = parse_and_register(
            "class Player {
                Player() { }
                Player(int health) { }
            }",
        );
        assert!(output.is_ok());

        let player_hash = context.lookup_type("Player").unwrap();
        let constructors = context.find_constructors(player_hash);
        assert_eq!(constructors.len(), 2);
    }

    #[test]
    fn register_class_with_fields() {
        let (context, output) = parse_and_register(
            "class Player {
                int health;
                float speed;
            }",
        );
        assert!(output.is_ok());

        let player_hash = context.lookup_type("Player").unwrap();
        if let Some(TypeDef::Class { fields, .. }) = context.get_type(player_hash) {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "health");
            assert_eq!(fields[1].name, "speed");
        } else {
            panic!("Expected class typedef");
        }
    }

    #[test]
    fn register_interface() {
        let (context, output) = parse_and_register(
            "interface IDrawable {
                void draw();
            }",
        );
        assert!(output.is_ok());
        assert!(context.lookup_type("IDrawable").is_some());
    }

    #[test]
    fn register_enum() {
        let (context, output) = parse_and_register(
            "enum Color {
                Red,
                Green = 5,
                Blue
            }",
        );
        assert!(output.is_ok());

        let color_hash = context.lookup_type("Color").unwrap();
        assert_eq!(context.lookup_enum_value(color_hash, "Red"), Some(0));
        assert_eq!(context.lookup_enum_value(color_hash, "Green"), Some(5));
        assert_eq!(context.lookup_enum_value(color_hash, "Blue"), Some(6));
    }

    #[test]
    fn register_global_function() {
        let (context, output) = parse_and_register("void main() { }");
        assert!(output.is_ok());

        let funcs = context.lookup_functions("main");
        assert_eq!(funcs.len(), 1);
    }

    #[test]
    fn register_function_with_params() {
        let (context, output) = parse_and_register("int add(int a, int b) { return a + b; }");
        assert!(output.is_ok());

        let funcs = context.lookup_functions("add");
        assert_eq!(funcs.len(), 1);

        let func = context.get_function(funcs[0]).unwrap();
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "a");
        assert_eq!(func.params[1].name, "b");
    }

    #[test]
    fn register_function_with_defaults() {
        let (context, output) = parse_and_register("void repeat(int value, int times = 1) { }");
        assert!(output.is_ok());

        let funcs = context.lookup_functions("repeat");
        assert_eq!(funcs.len(), 1);

        let func = context.get_function(funcs[0]).unwrap();
        assert_eq!(func.params.len(), 2);
        assert!(!func.params[0].has_default);
        assert!(func.params[1].has_default);
        assert_eq!(func.required_param_count(), 1);
    }

    #[test]
    fn register_namespace() {
        let (context, output) = parse_and_register(
            "namespace Game {
                class Player { }
            }",
        );
        assert!(output.is_ok());
        assert!(context.lookup_type("Game::Player").is_some());
    }

    #[test]
    fn register_nested_namespace() {
        let (context, output) = parse_and_register(
            "namespace Game::World {
                class Entity { }
            }",
        );
        assert!(output.is_ok());
        assert!(context.lookup_type("Game::World::Entity").is_some());
    }

    #[test]
    fn register_class_inheritance() {
        let (context, output) = parse_and_register(
            "class Base { }
            class Derived : Base { }",
        );
        assert!(output.is_ok());

        let base_hash = context.lookup_type("Base").unwrap();
        let derived_hash = context.lookup_type("Derived").unwrap();

        assert!(context.is_subclass_of(derived_hash, base_hash));
    }

    #[test]
    fn register_class_with_interface() {
        let (context, output) = parse_and_register(
            "interface IDrawable { void draw(); }
            class Player : IDrawable {
                void draw() { }
            }",
        );
        assert!(output.is_ok());

        let player_hash = context.lookup_type("Player").unwrap();
        let interfaces = context.get_interfaces(player_hash);
        assert_eq!(interfaces.len(), 1);
    }

    #[test]
    fn register_funcdef() {
        let (context, output) = parse_and_register("funcdef void Callback(int);");
        assert!(output.is_ok());
        assert!(context.lookup_type("Callback").is_some());
    }

    #[test]
    fn register_typedef() {
        let (context, output) = parse_and_register("typedef int EntityId;");
        assert!(output.is_ok());

        let entity_id_hash = context.lookup_type("EntityId");
        let int_hash = context.lookup_type("int");
        assert_eq!(entity_id_hash, int_hash);
    }

    #[test]
    fn register_operator_method() {
        let (context, output) = parse_and_register(
            "class Vec2 {
                Vec2 opAdd(const Vec2 &in other) { return Vec2(); }
            }",
        );
        assert!(output.is_ok());

        let vec2_hash = context.lookup_type("Vec2").unwrap();
        let op_methods = context.find_operator_methods(vec2_hash, OperatorBehavior::OpAdd);
        assert_eq!(op_methods.len(), 1);
    }

    #[test]
    fn duplicate_type_error() {
        let (_context, output) = parse_and_register(
            "class Player { }
            class Player { }",
        );
        assert!(!output.is_ok());
        assert!(matches!(
            output.errors[0],
            RegistrationError::DuplicateType { .. }
        ));
    }

    #[test]
    fn unknown_type_error() {
        let (_context, output) = parse_and_register("class Player : UnknownBase { }");
        assert!(!output.is_ok());
        assert!(matches!(
            output.errors[0],
            RegistrationError::UnknownType { .. }
        ));
    }
}
