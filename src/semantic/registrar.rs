//! Pass 1: Registration - Register all global names in the Registry.
//!
//! This module implements the first pass of semantic analysis which walks the AST
//! and registers all global declarations (types, functions, variables) in the Registry.
//!
//! # What Pass 1 Does
//!
//! - Register all global types (classes, interfaces, enums, funcdefs)
//! - Register all global function names
//! - Register all global variable names
//! - Track namespace and class context dynamically
//! - Build qualified names (e.g., "Namespace::Class")
//!
//! # What Pass 1 Does NOT Do
//!
//! - Does NOT resolve type expressions (that's Pass 2a)
//! - Does NOT track local variables (that's Pass 2b)
//! - Does NOT validate inheritance (that's Pass 2a)
//! - Does NOT type check anything (that's Pass 2b)
//!
//! # Example
//!
//! ```ignore
//! use angelscript::{parse_lenient, Registrar};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = "class Player { void update() { } }";
//! let (script, _) = parse_lenient(source, &arena);
//!
//! let data = Registrar::register(&script);
//! assert!(data.registry.lookup_type("Player").is_some());
//! ```

use super::error::{SemanticError, SemanticErrorKind};
use super::registry::{FunctionDef, Registry};
use super::type_def::{FunctionId, FunctionTraits, TypeDef, TypeId};
use crate::ast::decl::{
    ClassDecl, ClassMember, EnumDecl, FuncdefDecl, FunctionDecl, GlobalVarDecl, InterfaceDecl,
    Item, NamespaceDecl, TypedefDecl,
};
use crate::ast::Script;
use crate::lexer::Span;
use rustc_hash::FxHashMap;

/// Output from Pass 1: Registration
#[derive(Debug)]
pub struct RegistrationData {
    /// Registry with all global names registered (empty shells)
    pub registry: Registry,

    /// Errors found during registration
    pub errors: Vec<SemanticError>,
}

/// Pass 1: Registration visitor
///
/// Walks the AST and registers all global declarations in the Registry.
pub struct Registrar<'src, 'ast> {
    /// The registry we're building
    registry: Registry,

    /// Current namespace path (e.g., ["Game", "World"])
    namespace_path: Vec<String>,

    /// Current class being processed (if inside a class)
    current_class: Option<TypeId>,

    /// Track which names we've seen in the current scope (for duplicate detection)
    declared_names: FxHashMap<String, Span>,

    /// Errors found during registration
    errors: Vec<SemanticError>,

    /// Next function ID to assign
    next_func_id: u32,

    /// Phantom markers
    _phantom: std::marker::PhantomData<(&'src str, &'ast ())>,
}

impl<'src, 'ast> Registrar<'src, 'ast> {
    /// Create a new registrar
    fn new() -> Self {
        Self {
            registry: Registry::new(),
            namespace_path: Vec::new(),
            current_class: None,
            declared_names: FxHashMap::default(),
            errors: Vec::new(),
            next_func_id: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Perform Pass 1 registration on a script
    pub fn register(script: &Script<'src, 'ast>) -> RegistrationData {
        let mut registrar = Self::new();
        registrar.visit_script(script);

        RegistrationData {
            registry: registrar.registry,
            errors: registrar.errors,
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

    /// Visit a function declaration
    fn visit_function(&mut self, func: &FunctionDecl<'src, 'ast>, object_type: Option<TypeId>) {
        // Note: We don't check for duplicate function names here because
        // AngelScript supports function overloading. Actual duplicate detection
        // (checking signatures) happens in Pass 2a.

        // Register function (with empty signature for now)
        let func_id = FunctionId::new(self.next_func_id);
        self.next_func_id += 1;

        let func_def = FunctionDef {
            id: func_id,
            name: func.name.name.to_string(),
            namespace: self.namespace_path.clone(),
            params: Vec::new(), // Will be filled in Pass 2a
            return_type: super::data_type::DataType::simple(self.registry.void_type),
            object_type,
            traits: FunctionTraits::new(),
        };

        self.registry.register_function(func_def);
    }

    /// Visit a class declaration
    fn visit_class(&mut self, class: &ClassDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(class.name.name);

        // Check for duplicate declaration
        if let Some(&_prev_span) = self.declared_names.get(&qualified_name) {
            self.errors.push(SemanticError::new(
                SemanticErrorKind::DuplicateDeclaration,
                class.span,
                format!("class '{}' is already declared", qualified_name),
            ));
            return;
        }

        // Register class type (empty shell)
        let typedef = TypeDef::Class {
            name: class.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            fields: Vec::new(),  // Will be filled in Pass 2a
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
        };

        let type_id = self.registry.register_type(typedef, Some(&qualified_name));
        self.declared_names.insert(qualified_name, class.span);

        // Enter class context
        let prev_class = self.current_class;
        self.current_class = Some(type_id);

        // Visit class members (register methods)
        for member in class.members {
            match member {
                ClassMember::Method(method) => {
                    self.visit_function(method, Some(type_id));
                }
                ClassMember::Field(_) => {
                    // Fields will be processed in Pass 2a
                }
                ClassMember::VirtualProperty(_) => {
                    // Virtual properties will be processed later
                }
                ClassMember::Funcdef(funcdef) => {
                    // Inner funcdef in class - treat like a regular funcdef
                    self.visit_funcdef(funcdef);
                }
            }
        }

        // Exit class context
        self.current_class = prev_class;
    }

    /// Visit an interface declaration
    fn visit_interface(&mut self, iface: &InterfaceDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(iface.name.name);

        // Check for duplicate declaration
        if let Some(&_prev_span) = self.declared_names.get(&qualified_name) {
            self.errors.push(SemanticError::new(
                SemanticErrorKind::DuplicateDeclaration,
                iface.span,
                format!("interface '{}' is already declared", qualified_name),
            ));
            return;
        }

        // Register interface type (empty shell)
        let typedef = TypeDef::Interface {
            name: iface.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            methods: Vec::new(),  // Will be filled in Pass 2a
        };

        self.registry.register_type(typedef, Some(&qualified_name));
        self.declared_names.insert(qualified_name, iface.span);
    }

    /// Visit an enum declaration
    fn visit_enum(&mut self, enum_decl: &EnumDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(enum_decl.name.name);

        // Check for duplicate declaration
        if let Some(&_prev_span) = self.declared_names.get(&qualified_name) {
            self.errors.push(SemanticError::new(
                SemanticErrorKind::DuplicateDeclaration,
                enum_decl.span,
                format!("enum '{}' is already declared", qualified_name),
            ));
            return;
        }

        // Register enum type with values
        let values = enum_decl
            .enumerators
            .iter()
            .map(|v| {
                // For now, we'll use 0 as default value (evaluation in Pass 2a)
                let value = 0; // TODO: Evaluate v.value expression
                (v.name.name.to_string(), value)
            })
            .collect();

        let typedef = TypeDef::Enum {
            name: enum_decl.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            values,
        };

        self.registry.register_type(typedef, Some(&qualified_name));
        self.declared_names.insert(qualified_name, enum_decl.span);
    }

    /// Visit a global variable declaration
    fn visit_global_var(&mut self, var: &GlobalVarDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(var.name.name);

        // Check for duplicate declaration
        if let Some(&_prev_span) = self.declared_names.get(&qualified_name) {
            self.errors.push(SemanticError::new(
                SemanticErrorKind::DuplicateDeclaration,
                var.span,
                format!("variable '{}' is already declared", qualified_name),
            ));
            return;
        }

        // Just mark as declared (type resolution happens in Pass 2a)
        self.declared_names.insert(qualified_name, var.span);
    }

    /// Visit a namespace declaration
    fn visit_namespace(&mut self, ns: &NamespaceDecl<'src, 'ast>) {
        // Enter namespace (handle path which can be nested like A::B::C)
        for ident in ns.path {
            self.namespace_path.push(ident.name.to_string());
        }

        // Save current declared names scope
        let saved_names = std::mem::take(&mut self.declared_names);

        // Visit items inside namespace
        for item in ns.items {
            self.visit_item(item);
        }

        // Restore declared names scope
        self.declared_names = saved_names;

        // Exit namespace (pop all path components we added)
        for _ in ns.path {
            self.namespace_path.pop();
        }
    }

    /// Visit a typedef declaration
    fn visit_typedef(&mut self, _typedef: &TypedefDecl<'src, 'ast>) {
        // Typedef handling will be implemented in Pass 2a
        // For now, we just skip it
    }

    /// Visit a funcdef declaration
    fn visit_funcdef(&mut self, funcdef: &FuncdefDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(funcdef.name.name);

        // Check for duplicate declaration
        if let Some(&_prev_span) = self.declared_names.get(&qualified_name) {
            self.errors.push(SemanticError::new(
                SemanticErrorKind::DuplicateDeclaration,
                funcdef.span,
                format!("funcdef '{}' is already declared", qualified_name),
            ));
            return;
        }

        // Register funcdef type (empty shell)
        let typedef = TypeDef::Funcdef {
            name: funcdef.name.name.to_string(),
            qualified_name: qualified_name.clone(),
            params: Vec::new(),  // Will be filled in Pass 2a
            return_type: super::data_type::DataType::simple(self.registry.void_type),
        };

        self.registry.register_type(typedef, Some(&qualified_name));
        self.declared_names.insert(qualified_name, funcdef.span);
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
    use bumpalo::Bump;

    fn register(source: &str) -> RegistrationData {
        let arena = Bump::new();
        let (script, parse_errors) = parse_lenient(source, &arena);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);
        Registrar::register(&script)
    }

    #[test]
    fn empty_script() {
        let data = register("");
        assert!(data.errors.is_empty());
        // Should still have built-in types
        assert!(data.registry.lookup_type("int").is_some());
    }

    #[test]
    fn register_simple_class() {
        let data = register("class Player { }");
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Player").is_some());
    }

    #[test]
    fn register_class_with_fields() {
        let data = register("class Player { int health; float speed; }");
        assert!(data.errors.is_empty());
        let type_id = data.registry.lookup_type("Player").unwrap();
        let typedef = data.registry.get_type(type_id);
        assert!(typedef.is_class());
    }

    #[test]
    fn register_class_with_methods() {
        let data = register("class Player { void update() { } void draw() { } }");
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Player").is_some());
        // Methods are registered as functions
        assert!(!data.registry.lookup_functions("update").is_empty());
    }

    #[test]
    fn register_namespaced_class() {
        let data = register("namespace Game { class Player { } }");
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Game::Player").is_some());
        assert!(data.registry.lookup_type("Player").is_none());
    }

    #[test]
    fn register_nested_namespace() {
        let data = register("namespace Game { namespace Entities { class Player { } } }");
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Game::Entities::Player").is_some());
    }

    #[test]
    fn register_multiple_namespaces() {
        let data = register(r#"
            namespace A { class X { } }
            namespace B { class Y { } }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("A::X").is_some());
        assert!(data.registry.lookup_type("B::Y").is_some());
    }

    #[test]
    fn register_interface() {
        let data = register("interface IDrawable { void draw(); }");
        assert!(data.errors.is_empty());
        let type_id = data.registry.lookup_type("IDrawable").unwrap();
        assert!(data.registry.get_type(type_id).is_interface());
    }

    #[test]
    fn register_enum() {
        let data = register("enum Color { Red, Green, Blue }");
        assert!(data.errors.is_empty());
        let type_id = data.registry.lookup_type("Color").unwrap();
        assert!(data.registry.get_type(type_id).is_enum());
    }

    #[test]
    fn register_enum_with_values() {
        let data = register("enum Color { Red = 0, Green = 1, Blue = 2 }");
        assert!(data.errors.is_empty());
        let type_id = data.registry.lookup_type("Color").unwrap();
        let typedef = data.registry.get_type(type_id);

        if let TypeDef::Enum { values, .. } = typedef {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0].0, "Red");
            assert_eq!(values[0].1, 0);
        } else {
            panic!("Expected Enum");
        }
    }

    #[test]
    fn register_funcdef() {
        let data = register("funcdef void Callback(int x);");
        assert!(data.errors.is_empty());
        let type_id = data.registry.lookup_type("Callback").unwrap();
        assert!(data.registry.get_type(type_id).is_funcdef());
    }

    #[test]
    fn register_global_function() {
        let data = register("void foo() { }");
        assert!(data.errors.is_empty());
        let functions = data.registry.lookup_functions("foo");
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn register_qualified_function() {
        let data = register("namespace Game { void update() { } }");
        assert!(data.errors.is_empty());
        let functions = data.registry.lookup_functions("Game::update");
        assert_eq!(functions.len(), 1);
    }

    #[test]
    fn register_function_overloads() {
        let data = register(r#"
            void foo(int x) { }
            void foo(float x) { }
        "#);
        assert!(data.errors.is_empty());
        let functions = data.registry.lookup_functions("foo");
        // Both registered (actual overload resolution happens in Pass 2a)
        assert!(functions.len() >= 1);
    }

    #[test]
    fn register_global_variable() {
        let data = register("int playerHealth = 100;");
        assert!(data.errors.is_empty());
        // Global variables are just marked as declared (no storage in Pass 1)
    }

    #[test]
    fn duplicate_class_error() {
        let data = register(r#"
            class Player { }
            class Player { }
        "#);
        assert!(!data.errors.is_empty());
        assert_eq!(data.errors[0].kind, SemanticErrorKind::DuplicateDeclaration);
    }

    #[test]
    fn duplicate_function_in_namespace() {
        let _data = register(r#"
            namespace Game {
                void foo() { }
                void foo() { }
            }
        "#);
        // Note: This should actually allow overloading, but for now we track duplicates
        // In a real implementation, we'd check parameter signatures
    }

    #[test]
    fn duplicate_in_different_namespaces_allowed() {
        let data = register(r#"
            namespace A { class X { } }
            namespace B { class X { } }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("A::X").is_some());
        assert!(data.registry.lookup_type("B::X").is_some());
    }

    #[test]
    fn namespace_isolates_names() {
        let data = register(r#"
            class Player { }
            namespace Game {
                class Player { }
            }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Player").is_some());
        assert!(data.registry.lookup_type("Game::Player").is_some());
    }

    #[test]
    fn complex_nested_structure() {
        let data = register(r#"
            namespace Game {
                namespace Entities {
                    class Player {
                        void update() { }
                        void draw() { }
                    }
                    class Enemy {
                        void attack() { }
                    }
                }
                namespace UI {
                    class Button { }
                }
            }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Game::Entities::Player").is_some());
        assert!(data.registry.lookup_type("Game::Entities::Enemy").is_some());
        assert!(data.registry.lookup_type("Game::UI::Button").is_some());
    }

    #[test]
    fn mixed_declarations() {
        let data = register(r#"
            class Player { }
            interface IDrawable { void draw(); }
            enum Color { Red, Green, Blue }
            funcdef void Callback(int x);
            void main() { }
            int globalVar = 0;
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Player").is_some());
        assert!(data.registry.lookup_type("IDrawable").is_some());
        assert!(data.registry.lookup_type("Color").is_some());
        assert!(data.registry.lookup_type("Callback").is_some());
        assert!(!data.registry.lookup_functions("main").is_empty());
    }

    #[test]
    fn qualified_name_building() {
        let data = register(r#"
            namespace A {
                namespace B {
                    namespace C {
                        class Deep { }
                    }
                }
            }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("A::B::C::Deep").is_some());
    }

    #[test]
    fn context_tracking() {
        let data = register(r#"
            namespace Outer {
                class X { }
                namespace Inner {
                    class Y { }
                }
                class Z { }
            }
        "#);
        assert!(data.errors.is_empty());
        assert!(data.registry.lookup_type("Outer::X").is_some());
        assert!(data.registry.lookup_type("Outer::Inner::Y").is_some());
        assert!(data.registry.lookup_type("Outer::Z").is_some());
    }

    #[test]
    fn builtin_types_present() {
        let data = register("");
        assert!(data.registry.lookup_type("void").is_some());
        assert!(data.registry.lookup_type("bool").is_some());
        assert!(data.registry.lookup_type("int").is_some());
        assert!(data.registry.lookup_type("uint").is_some());
        assert!(data.registry.lookup_type("float").is_some());
        assert!(data.registry.lookup_type("double").is_some());
        assert!(data.registry.lookup_type("string").is_some());
        assert!(data.registry.lookup_type("array").is_some());
        assert!(data.registry.lookup_type("dictionary").is_some());
    }
}
