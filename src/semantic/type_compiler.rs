//! Pass 2a: Type Compilation - Fill in all type details and resolve type expressions.
//!
//! This module implements the second pass of semantic analysis which takes the Registry
//! from Pass 1 (with registered names as empty shells) and fills in all type details.
//!
//! # What Pass 2a Does
//!
//! - Resolve all TypeExpr AST nodes → DataType
//! - Fill in class details (fields, methods, inheritance)
//! - Instantiate template types with caching
//! - Register complete function signatures
//! - Build type hierarchy (inheritance, interfaces)
//!
//! # What Pass 2a Does NOT Do
//!
//! - Does NOT track local variables (that's Pass 2b)
//! - Does NOT type check function bodies (that's Pass 2b)
//! - Does NOT generate bytecode (that's Pass 2b)
//!
//! # Example
//!
//! ```ignore
//! use angelscript::{parse_lenient, Registrar, TypeCompiler};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = r#"
//!     class Player {
//!         int health;
//!         array<string> items;
//!     }
//! "#;
//! let (script, _) = parse_lenient(source, &arena);
//!
//! // Pass 1: Registration
//! let registration = Registrar::register(&script);
//!
//! // Pass 2a: Type compilation
//! let type_compilation = TypeCompiler::compile(&script, registration.registry);
//! assert!(type_compilation.errors.is_empty());
//! ```

use super::data_type::DataType;
use super::error::{SemanticError, SemanticErrorKind};
use super::registry::Registry;
#[cfg_attr(not(test), allow(unused_imports))]
use super::type_def::{FieldDef, MethodSignature, TypeDef, TypeId, Visibility};
use crate::ast::decl::{
    ClassDecl, ClassMember, EnumDecl, FuncdefDecl, FunctionDecl, GlobalVarDecl,
    InterfaceDecl, InterfaceMethod, Item, NamespaceDecl, TypedefDecl,
};
use crate::ast::types::{PrimitiveType, TypeBase, TypeExpr, TypeSuffix};
use crate::ast::Script;
use crate::lexer::Span;
use rustc_hash::FxHashMap;

/// Output from Pass 2a: Type Compilation
#[derive(Debug)]
pub struct TypeCompilationData {
    /// Registry with complete type information
    pub registry: Registry,

    /// Maps AST TypeExpr spans to resolved DataType
    pub type_map: FxHashMap<Span, DataType>,

    /// Inheritance relationships (Derived → Base)
    pub inheritance: FxHashMap<TypeId, TypeId>,

    /// Interface implementations (Class → [Interfaces])
    pub implements: FxHashMap<TypeId, Vec<TypeId>>,

    /// Errors found during type compilation
    pub errors: Vec<SemanticError>,
}

/// Pass 2a: Type Compilation visitor
///
/// Walks the AST and fills in all type details in the Registry.
pub struct TypeCompiler<'src, 'ast> {
    /// The registry we're filling in
    registry: Registry,

    /// Maps AST spans to resolved types
    type_map: FxHashMap<Span, DataType>,

    /// Current namespace path (e.g., ["Game", "World"])
    namespace_path: Vec<String>,

    /// Inheritance tracking (Derived → Base)
    inheritance: FxHashMap<TypeId, TypeId>,

    /// Interface implementations (Class → [Interfaces])
    implements: FxHashMap<TypeId, Vec<TypeId>>,

    /// Errors found during compilation
    errors: Vec<SemanticError>,

    /// Phantom markers
    _phantom: std::marker::PhantomData<(&'src str, &'ast ())>,
}

impl<'src, 'ast> TypeCompiler<'src, 'ast> {
    /// Create a new type compiler
    fn new(registry: Registry) -> Self {
        Self {
            registry,
            type_map: FxHashMap::default(),
            namespace_path: Vec::new(),
            inheritance: FxHashMap::default(),
            implements: FxHashMap::default(),
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Perform Pass 2a type compilation on a script
    pub fn compile(
        script: &Script<'src, 'ast>,
        registry: Registry,
    ) -> TypeCompilationData {
        let mut compiler = Self::new(registry);
        compiler.visit_script(script);

        TypeCompilationData {
            registry: compiler.registry,
            type_map: compiler.type_map,
            inheritance: compiler.inheritance,
            implements: compiler.implements,
            errors: compiler.errors,
        }
    }

    /// Visit the entire script
    fn visit_script(&mut self, script: &Script<'src, 'ast>) {
        for item in script.items() {
            self.visit_item(item);
        }
    }

    /// Visit a top-level item
    fn visit_item(&mut self, item: &Item<'src, 'ast>) {
        match item {
            Item::Function(func) => self.visit_function(func, None),
            Item::Class(class) => self.visit_class(class),
            Item::Interface(iface) => self.visit_interface(iface),
            Item::Enum(enum_decl) => self.visit_enum(enum_decl),
            Item::GlobalVar(var) => self.visit_global_var(var),
            Item::Namespace(ns) => self.visit_namespace(ns),
            Item::Typedef(typedef) => self.visit_typedef(typedef),
            Item::Funcdef(funcdef) => self.visit_funcdef(funcdef),
            Item::Mixin(_) | Item::Import(_) => {
                // Skip mixins and imports for now
            }
        }
    }

    /// Visit a function declaration and fill in its signature
    fn visit_function(&mut self, func: &FunctionDecl<'src, 'ast>, object_type: Option<TypeId>) {
        let qualified_name = self.build_qualified_name(func.name.name);

        // Resolve parameter types
        let params: Vec<DataType> = func
            .params
            .iter()
            .filter_map(|p| self.resolve_type_expr(&p.ty.ty))
            .collect();

        // Check if we got all params (if any failed to resolve, we already logged errors)
        if params.len() != func.params.len() {
            // Some params failed to resolve, skip this function
            return;
        }

        // Resolve return type (default to void if None, e.g., for constructors)
        let return_type = if let Some(ret_ty) = func.return_type {
            match self.resolve_type_expr(&ret_ty.ty) {
                Some(dt) => dt,
                None => return, // Error already logged
            }
        } else {
            // Constructor or destructor - use void type
            DataType::simple(self.registry.void_type)
        };

        // Update the function signature in the registry
        self.registry.update_function_signature(&qualified_name, params, return_type, object_type);
    }

    /// Visit a class declaration and fill in its details
    fn visit_class(&mut self, class: &ClassDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(class.name.name);

        // Look up the class type (registered in Pass 1)
        let type_id = match self.registry.lookup_type(&qualified_name) {
            Some(id) => id,
            None => {
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    class.span,
                    format!("class '{}' was not registered in Pass 1", qualified_name),
                ));
                return;
            }
        };

        // Resolve base class and interfaces from inheritance list
        // In AngelScript, the first item in inheritance is the base class (if it's a class),
        // remaining items are interfaces
        let mut base_class = None;
        let mut interfaces = Vec::new();

        for (i, inherited_ident) in class.inheritance.iter().enumerate() {
            // Look up the type
            let inherited_name = self.build_qualified_name(inherited_ident.name);
            if let Some(inherited_id) = self.registry.lookup_type(&inherited_name) {
                let inherited_typedef = self.registry.get_type(inherited_id);

                // First item can be a base class (if it's a class type)
                if i == 0 && inherited_typedef.is_class() {
                    base_class = Some(inherited_id);
                    self.inheritance.insert(type_id, inherited_id);
                } else if inherited_typedef.is_interface() {
                    interfaces.push(inherited_id);
                } else if i == 0 {
                    // First item is not a class or interface
                    self.errors.push(SemanticError::new(
                        SemanticErrorKind::UndefinedType,
                        inherited_ident.span,
                        format!("'{}' is not a class or interface", inherited_name),
                    ));
                }
            } else {
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    inherited_ident.span,
                    format!("undefined type '{}'", inherited_name),
                ));
            }
        }

        // Track interface implementations
        if !interfaces.is_empty() {
            self.implements.insert(type_id, interfaces.clone());
        }

        // Resolve field types
        let mut fields = Vec::new();
        for member in class.members {
            if let ClassMember::Field(field) = member
                && let Some(field_type) = self.resolve_type_expr(&field.ty) {
                    fields.push(FieldDef {
                        name: field.name.name.to_string(),
                        data_type: field_type,
                        visibility: Visibility::Private, // TODO: Extract from field modifiers
                    });
                }
        }

        // Collect method IDs
        let mut method_ids = Vec::new();

        // Enter namespace context for method processing
        self.namespace_path.push(class.name.name.to_string());

        for member in class.members {
            if let ClassMember::Method(method) = member {
                // Register method signature
                self.visit_function(method, Some(type_id));

                // Find the function ID for this method
                let method_qualified_name = self.build_qualified_name(method.name.name);
                let func_ids = self.registry.lookup_functions(&method_qualified_name);
                method_ids.extend(func_ids.iter().copied());
            }
        }

        self.namespace_path.pop();

        // Update the class definition in the registry
        self.registry.update_class_details(type_id, fields, method_ids, base_class, interfaces);
    }

    /// Visit an interface declaration and fill in its details
    fn visit_interface(&mut self, iface: &InterfaceDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(iface.name.name);

        // Look up the interface type (registered in Pass 1)
        let type_id = match self.registry.lookup_type(&qualified_name) {
            Some(id) => id,
            None => {
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    iface.span,
                    format!("interface '{}' was not registered in Pass 1", qualified_name),
                ));
                return;
            }
        };

        // Resolve method signatures
        let mut methods = Vec::new();
        for member in iface.members {
            if let crate::ast::decl::InterfaceMember::Method(method) = member
                && let Some(method_sig) = self.resolve_interface_method(method) {
                    methods.push(method_sig);
                }
        }

        // Update the interface definition in the registry
        self.registry.update_interface_details(type_id, methods);
    }

    /// Resolve an interface method to a MethodSignature
    fn resolve_interface_method(&mut self, method: &InterfaceMethod<'src, 'ast>) -> Option<MethodSignature> {
        // Resolve parameter types
        let params: Vec<DataType> = method
            .params
            .iter()
            .filter_map(|p| self.resolve_type_expr(&p.ty.ty))
            .collect();

        if params.len() != method.params.len() {
            return None; // Some params failed to resolve
        }

        // Resolve return type
        let return_type = self.resolve_type_expr(&method.return_type.ty)?;

        Some(MethodSignature {
            name: method.name.name.to_string(),
            params,
            return_type,
        })
    }

    /// Visit an enum declaration (already complete from Pass 1)
    fn visit_enum(&mut self, _enum_decl: &EnumDecl<'src, 'ast>) {
        // Enums are fully registered in Pass 1, nothing to do here
    }

    /// Visit a global variable declaration and resolve its type
    fn visit_global_var(&mut self, var: &GlobalVarDecl<'src, 'ast>) {
        // Resolve the variable's type
        if let Some(var_type) = self.resolve_type_expr(&var.ty) {
            // Register the global variable with its resolved type
            self.registry.register_global_var(
                var.name.name.to_string(),
                self.namespace_path.clone(),
                var_type,
            );
        }
    }

    /// Visit a namespace declaration
    fn visit_namespace(&mut self, ns: &NamespaceDecl<'src, 'ast>) {
        // Enter namespace (handle path which can be nested like A::B::C)
        for ident in ns.path {
            self.namespace_path.push(ident.name.to_string());
        }

        // Visit items inside namespace
        for item in ns.items {
            self.visit_item(item);
        }

        // Exit namespace (pop all path components we added)
        for _ in ns.path {
            self.namespace_path.pop();
        }
    }

    /// Visit a typedef declaration
    fn visit_typedef(&mut self, _typedef: &TypedefDecl<'src, 'ast>) {
        // TODO: Implement typedef handling
    }

    /// Visit a funcdef declaration and fill in its signature
    fn visit_funcdef(&mut self, funcdef: &FuncdefDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(funcdef.name.name);

        // Look up the funcdef type (registered in Pass 1)
        let type_id = match self.registry.lookup_type(&qualified_name) {
            Some(id) => id,
            None => {
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    funcdef.span,
                    format!("funcdef '{}' was not registered in Pass 1", qualified_name),
                ));
                return;
            }
        };

        // Resolve parameter types
        let params: Vec<DataType> = funcdef
            .params
            .iter()
            .filter_map(|p| self.resolve_type_expr(&p.ty.ty))
            .collect();

        if params.len() != funcdef.params.len() {
            return; // Some params failed to resolve
        }

        // Resolve return type
        let return_type = match self.resolve_type_expr(&funcdef.return_type.ty) {
            Some(dt) => dt,
            None => return, // Error already logged
        };

        // Update the funcdef in the registry
        self.registry.update_funcdef_signature(type_id, params, return_type);
    }

    /// Resolve a TypeExpr AST node to a complete DataType
    ///
    /// This is the core type resolution method that:
    /// - Looks up the base type in the registry
    /// - Handles scoped names (Namespace::Type)
    /// - Instantiates templates recursively
    /// - Applies type modifiers (const, @)
    /// - Stores the result in type_map for later reference
    fn resolve_type_expr(&mut self, expr: &TypeExpr<'src, 'ast>) -> Option<DataType> {
        // Step 1: Resolve the base type name
        let base_type_id = self.resolve_base_type(&expr.base, expr.scope.as_ref(), expr.span)?;

        // Step 2: Handle template arguments
        let type_id = if !expr.template_args.is_empty() {
            // Resolve all template argument types recursively
            let arg_types: Vec<DataType> = expr
                .template_args
                .iter()
                .filter_map(|arg| self.resolve_type_expr(arg))
                .collect();

            // Check that all template args resolved
            if arg_types.len() != expr.template_args.len() {
                return None; // Some template args failed to resolve
            }

            // Instantiate the template
            match self.registry.instantiate_template(base_type_id, arg_types) {
                Ok(instance_id) => instance_id,
                Err(err) => {
                    self.errors.push(err);
                    return None;
                }
            }
        } else {
            base_type_id
        };

        // Step 3: Build DataType with modifiers from suffixes
        let mut data_type = DataType::simple(type_id);

        // Apply const modifier from leading const keyword
        if expr.is_const {
            data_type.is_const = true;
        }

        // Apply modifiers from suffixes
        // Note: In AngelScript, only the first @ suffix matters for handle semantics
        for suffix in expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const } => {
                    data_type.is_handle = true;
                    data_type.is_handle_to_const = *is_const;
                    // In AngelScript, leading const + @ means handle to const
                    if expr.is_const && data_type.is_handle {
                        data_type.is_handle_to_const = true;
                        data_type.is_const = false; // Move const to handle_to_const
                    }
                    break; // Only first @ matters
                }
                TypeSuffix::Array => {
                    // TODO: Handle array types - might need to create array<T> template instance
                    // For now, we'll skip array suffix handling
                }
            }
        }

        // Step 4: Store in type_map for later reference
        self.type_map.insert(expr.span, data_type.clone());

        Some(data_type)
    }

    /// Resolve the base type (without template args or modifiers)
    fn resolve_base_type(
        &mut self,
        base: &TypeBase<'src>,
        scope: Option<&crate::ast::Scope<'src, 'ast>>,
        span: Span,
    ) -> Option<TypeId> {
        match base {
            TypeBase::Primitive(prim) => Some(self.primitive_to_type_id(*prim)),

            TypeBase::Named(ident) => {
                // Build the qualified name
                let type_name = if let Some(scope) = scope {
                    // Scoped type: Namespace::Type
                    self.build_scoped_name(scope, ident.name)
                } else {
                    // Try current namespace first, then global
                    let qualified = self.build_qualified_name(ident.name);

                    // Look up in registry
                    if let Some(type_id) = self.registry.lookup_type(&qualified) {
                        return Some(type_id);
                    }

                    // If not found in current namespace, try global scope
                    if !self.namespace_path.is_empty()
                        && let Some(type_id) = self.registry.lookup_type(ident.name) {
                            return Some(type_id);
                        }

                    // Not found anywhere
                    self.errors.push(SemanticError::new(
                        SemanticErrorKind::UndefinedType,
                        span,
                        format!("undefined type '{}'", ident.name),
                    ));
                    return None;
                };

                // Look up the type
                match self.registry.lookup_type(&type_name) {
                    Some(type_id) => Some(type_id),
                    None => {
                        self.errors.push(SemanticError::new(
                            SemanticErrorKind::UndefinedType,
                            span,
                            format!("undefined type '{}'", type_name),
                        ));
                        None
                    }
                }
            }

            TypeBase::Auto => {
                // Auto type inference - not supported in type resolution yet
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    span,
                    "auto type inference not yet supported".to_string(),
                ));
                None
            }

            TypeBase::Unknown => {
                self.errors.push(SemanticError::new(
                    SemanticErrorKind::UndefinedType,
                    span,
                    "unknown type '?'".to_string(),
                ));
                None
            }
        }
    }

    /// Map a primitive type to its TypeId
    #[inline]
    fn primitive_to_type_id(&self, prim: PrimitiveType) -> TypeId {
        match prim {
            PrimitiveType::Void => self.registry.void_type,
            PrimitiveType::Bool => self.registry.bool_type,
            PrimitiveType::Int => self.registry.int32_type,
            PrimitiveType::Int8 => self.registry.int8_type,
            PrimitiveType::Int16 => self.registry.int16_type,
            PrimitiveType::Int64 => self.registry.int64_type,
            PrimitiveType::UInt => self.registry.uint32_type,
            PrimitiveType::UInt8 => self.registry.uint8_type,
            PrimitiveType::UInt16 => self.registry.uint16_type,
            PrimitiveType::UInt64 => self.registry.uint64_type,
            PrimitiveType::Float => self.registry.float_type,
            PrimitiveType::Double => self.registry.double_type,
        }
    }

    /// Build a scoped name from a Scope and identifier
    fn build_scoped_name(&self, scope: &crate::ast::Scope<'src, 'ast>, name: &str) -> String {
        let scope_parts: Vec<&str> = scope.segments.iter().map(|ident| ident.name).collect();
        format!("{}::{}", scope_parts.join("::"), name)
    }

    /// Build a qualified name from the current namespace path
    fn build_qualified_name(&self, name: &str) -> String {
        if self.namespace_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace_path.join("::"), name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_lenient;
    use crate::semantic::Registrar;
    use bumpalo::Bump;

    fn compile(source: &str) -> TypeCompilationData {
        let arena = Bump::new();
        let (script, parse_errors) = parse_lenient(source, &arena);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

        // Pass 1: Registration
        let registration = Registrar::register(&script);
        assert!(registration.errors.is_empty(), "Registration errors: {:?}", registration.errors);

        // Pass 2a: Type compilation
        TypeCompiler::compile(&script, registration.registry)
    }

    #[test]
    fn empty_script() {
        let data = compile("");
        assert!(data.errors.is_empty());
    }

    #[test]
    fn resolve_primitive_type() {
        let data = compile("class Player { int health; }");
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "health");
            assert_eq!(fields[0].data_type.type_id, data.registry.int32_type);
        } else {
            panic!("Expected Class typedef");
        }
    }

    #[test]
    fn resolve_multiple_field_types() {
        let data = compile(r#"
            class Player {
                int health;
                float speed;
                bool isAlive;
                double score;
            }
        "#);
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 4);
            assert_eq!(fields[0].data_type.type_id, data.registry.int32_type);
            assert_eq!(fields[1].data_type.type_id, data.registry.float_type);
            assert_eq!(fields[2].data_type.type_id, data.registry.bool_type);
            assert_eq!(fields[3].data_type.type_id, data.registry.double_type);
        } else {
            panic!("Expected Class typedef");
        }
    }

    #[test]
    fn resolve_user_defined_type() {
        let data = compile(r#"
            class Position { int x; int y; }
            class Player { Position pos; }
        "#);
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let position_id = data.registry.lookup_type("Position").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].data_type.type_id, position_id);
        } else {
            panic!("Expected Class typedef");
        }
    }

    #[test]
    fn resolve_const_modifier() {
        let data = compile("class Player { const int maxHealth; }");
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert!(fields[0].data_type.is_const);
        } else {
            panic!("Expected Class typedef");
        }
    }

    #[test]
    fn resolve_handle_modifier() {
        let data = compile(r#"
            class Item { }
            class Player { Item@ currentItem; }
        "#);
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert!(fields[0].data_type.is_handle);
            assert!(!fields[0].data_type.is_handle_to_const);
        } else {
            panic!("Expected Class typedef");
        }
    }

    #[test]
    fn resolve_const_handle() {
        let data = compile(r#"
            class Item { }
            class Player { const Item@ currentItem; }
        "#);
        assert!(data.errors.is_empty(), "Errors: {:?}", data.errors);

        let player_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(player_id);

        if let TypeDef::Class { fields, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert!(fields[0].data_type.is_handle);
            assert!(fields[0].data_type.is_handle_to_const);
        } else {
            panic!("Expected Class typedef");
        }
    }

    // Template tests will be added once Registry::instantiate_template is available
}
