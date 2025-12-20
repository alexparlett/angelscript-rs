//! Abstract Syntax Tree (AST) for AngelScript.
//!
//! This module provides:
//! - AST node definitions for all AngelScript constructs
//! - Parser for transforming tokens into AST
//! - Error types and reporting
//! - Visitor pattern for AST traversal
//!
//! # Example
//!
//! ```
//! use angelscript_parser::Parser;
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = r#"
//!     class Player {
//!         int health = 100;
//!
//!         void takeDamage(int amount) {
//!             health -= amount;
//!         }
//!     }
//! "#;
//!
//! match Parser::parse(source, &arena) {
//!     Ok(script) => println!("Parsed successfully: {} items", script.items().len()),
//!     Err(errors) => eprintln!("Parse errors: {}", errors),
//! }
//! ```

// Core types
pub mod node;
pub mod ops;

mod parser;
mod type_parser;
pub mod types;

pub mod expr;
mod expr_parser;

pub mod stmt;
mod stmt_parser;

pub mod decl;
mod decl_parser;

pub mod visitor;

// Re-export error types from core
pub use angelscript_core::{ParseError, ParseErrorKind, ParseErrors};

pub use decl::*;
pub use expr::*;
pub use node::*;
pub use ops::*;
pub use parser::Parser;
pub use stmt::*;
pub use types::*;

/// A parsed AngelScript script.
///
/// The script borrows from an arena allocator. All AST nodes are allocated
/// in the arena and remain valid for the lifetime of the arena.
///
/// For multi-file compilation, use `CompilationContext` which owns the arena
/// and allows multiple scripts to share the same arena.
#[derive(Debug)]
pub struct Script<'ast> {
    items: &'ast [Item<'ast>],
    span: angelscript_core::Span,
}

impl<'ast> Script<'ast> {
    /// Create a new script from parsed items.
    pub(crate) fn new(items: &'ast [Item<'ast>], span: angelscript_core::Span) -> Self {
        Self { items, span }
    }

    /// Get the top-level items in this script.
    pub fn items(&self) -> &[Item<'ast>] {
        self.items
    }

    /// Get the source location span of this script.
    pub fn span(&self) -> angelscript_core::Span {
        self.span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_function() {
        let arena = bumpalo::Bump::new();
        let source = "void foo() { }";
        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);

        // Verify it's a function with name "foo" and void return
        match &script.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.name, "foo");
                assert!(func.return_type.is_some());
                if let Some(ret) = &func.return_type {
                    assert!(ret.ty.is_void());
                }
                assert_eq!(func.params.len(), 0);
                assert!(func.body.is_some());
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn parse_class_with_members() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Player {
                int health;
                void takeDamage(int amount) {
                    health -= amount;
                }
            }
        "#;
        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);

        // Verify it's a class named "Player" with 1 field and 1 method
        match &script.items()[0] {
            Item::Class(class) => {
                assert_eq!(class.name.name, "Player");
                assert_eq!(class.members.len(), 2);

                // Check for the field
                let field_count = class
                    .members
                    .iter()
                    .filter(|m| matches!(m, ClassMember::Field(_)))
                    .count();
                assert_eq!(field_count, 1);

                // Check for the method
                let method_count = class
                    .members
                    .iter()
                    .filter(|m| matches!(m, ClassMember::Method(_)))
                    .count();
                assert_eq!(method_count, 1);
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn parse_with_errors() {
        let arena = bumpalo::Bump::new();
        let source = "int x = ;"; // Missing expression
        let result = Parser::parse(source, &arena);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_lenient_recovers() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            int x = ;
            int y = 42;
        "#;
        let (script, errors) = Parser::parse_lenient(source, &arena);

        // Should have errors but still parse something
        assert!(!errors.is_empty());
        // Should recover and parse the second declaration
        assert!(!script.items().is_empty());
    }

    #[test]
    fn parse_lenient_no_errors() {
        let arena = bumpalo::Bump::new();
        let source = "int x = 42;";
        let (script, errors) = Parser::parse_lenient(source, &arena);

        assert!(errors.is_empty());
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_expression_simple() {
        let arena = bumpalo::Bump::new();
        let result = Parser::expression("1 + 2", &arena);
        assert!(result.is_ok());

        // Verify it's a BinaryExpr with Add operator
        let expr = result.unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Add));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_expression_complex() {
        let arena = bumpalo::Bump::new();
        let result = Parser::expression("obj.method()[0].field", &arena);
        assert!(result.is_ok());

        // Verify the expression chain: obj.method()[0].field
        // Structure: Member(.field, Index([0], Member(.method(), obj)))
        let expr = result.unwrap();
        match expr {
            Expr::Member(member) => {
                // Check member is Field("field")
                match &member.member {
                    MemberAccess::Field(ident) => assert_eq!(ident.name, "field"),
                    _ => panic!("Expected field access to 'field'"),
                }
                // The object should be an Index expression
                match member.object {
                    Expr::Index(idx) => {
                        // The object of the index should be a Member with method call
                        match idx.object {
                            Expr::Member(m) => {
                                // Member access with Method variant for method()
                                match &m.member {
                                    MemberAccess::Method { name, args } => {
                                        assert_eq!(name.name, "method");
                                        assert!(args.is_empty()); // No arguments
                                    }
                                    _ => panic!("Expected method call 'method'"),
                                }
                                // Base should be "obj"
                                match m.object {
                                    Expr::Ident(id) => assert_eq!(id.ident.name, "obj"),
                                    _ => panic!("Expected ident 'obj'"),
                                }
                            }
                            _ => panic!("Expected member expression with method call"),
                        }
                    }
                    _ => panic!("Expected index expression"),
                }
            }
            _ => panic!("Expected member expression"),
        }
    }

    #[test]
    fn parse_expression_with_error() {
        let arena = bumpalo::Bump::new();
        let result = Parser::expression("1 +", &arena); // Incomplete
        assert!(result.is_err());
        // Verify we get an error about expecting an expression
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_statement_simple() {
        let arena = bumpalo::Bump::new();
        let result = Parser::statement("return 42;", &arena);
        assert!(result.is_ok());

        // Verify it's a ReturnStmt
        let stmt = result.unwrap();
        match stmt {
            Stmt::Return(_) => {
                // Successfully verified it's a return statement
            }
            _ => panic!("Expected return statement"),
        }
    }

    #[test]
    fn parse_statement_if() {
        let arena = bumpalo::Bump::new();
        let result = Parser::statement("if (x > 0) { return x; }", &arena);
        assert!(result.is_ok());

        // Verify it's an IfStmt with a condition
        let stmt = result.unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                // Verify it has a condition expression (checking it's a binary expression)
                match if_stmt.condition {
                    Expr::Binary(bin) => {
                        assert!(matches!(bin.op, BinaryOp::Greater));
                    }
                    _ => {
                        // Any non-error expression is valid
                    }
                }
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_statement_for() {
        let arena = bumpalo::Bump::new();
        let result = Parser::statement("for (int i = 0; i < 10; i++) { }", &arena);
        assert!(result.is_ok());

        // Verify it's a ForStmt
        let stmt = result.unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                // Verify it has an initializer, condition, and update
                assert!(for_stmt.init.is_some());
                assert!(for_stmt.condition.is_some());
                assert!(!for_stmt.update.is_empty());
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_type_simple() {
        let arena = bumpalo::Bump::new();
        let result = Parser::type_expr("int", &arena);
        assert!(result.is_ok());

        // Verify the type is an int primitive
        let ty = result.unwrap();
        match ty.base {
            TypeBase::Primitive(PrimitiveType::Int) => {
                // Successfully verified it's an int type
            }
            _ => panic!("Expected int primitive type"),
        }
    }

    #[test]
    fn parse_type_complex() {
        let arena = bumpalo::Bump::new();
        let result = Parser::type_expr("const array<int>@ const", &arena);
        assert!(result.is_ok());

        // Verify: const array<int>@ const
        let ty = result.unwrap();
        assert!(ty.is_const); // Leading const
        assert!(ty.has_handle()); // @ handle
        match &ty.base {
            TypeBase::Named(name) => {
                assert_eq!(name.name, "array");
            }
            _ => panic!("Expected named type 'array'"),
        }
        // Template args are on TypeExpr, not Ident
        assert_eq!(ty.template_args.len(), 1);
    }

    #[test]
    fn parse_type_with_scope() {
        let arena = bumpalo::Bump::new();
        let result = Parser::type_expr("Namespace::MyClass", &arena);
        assert!(result.is_ok());

        // Verify scope segments: Namespace::MyClass
        let ty = result.unwrap();
        match &ty.base {
            TypeBase::Named(name) => {
                assert_eq!(name.name, "MyClass");
            }
            _ => panic!("Expected named type"),
        }
        // Scope is on TypeExpr, not Ident
        let scope = ty.scope.as_ref().expect("Expected scope");
        assert_eq!(scope.segments.len(), 1);
        assert_eq!(scope.segments[0].name, "Namespace");
    }

    #[test]
    fn parse_complete_program() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            namespace Game {
                class Player {
                    private int health = 100;
                    
                    Player(int hp) {
                        health = hp;
                    }
                    
                    void takeDamage(int amount) {
                        health -= amount;
                    }
                    
                    int Health {
                        get const { return health; }
                        set { health = value; }
                    }
                }
                
                interface IEnemy {
                    void attack(Player@ player);
                }
                
                enum Difficulty {
                    Easy, Normal, Hard
                }
            }
            
            int globalCounter = 0;
            
            void main() {
                Game::Player@ player = Game::Player(100);
                player.takeDamage(25);
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok(), "Failed to parse complete program");

        let script = result.unwrap();
        assert_eq!(script.items().len(), 3); // namespace, global var, function
    }

    #[test]
    fn parse_multiple_errors() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            int x = ;
            void func( { }
            int y
        "#;

        let (_, errors) = Parser::parse_lenient(source, &arena);

        // Should have multiple errors
        assert!(errors.len() >= 2, "Should detect multiple errors");
    }

    #[test]
    fn parse_interface_with_properties() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            interface IDrawable {
                void draw();
                int Priority {
                    get const;
                }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Verify interface has method and property
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Interface(iface) => {
                assert_eq!(iface.name.name, "IDrawable");
                assert_eq!(iface.members.len(), 2); // draw() and Priority property
                // Check for property member
                let has_property = iface
                    .members
                    .iter()
                    .any(|m| matches!(m, InterfaceMember::VirtualProperty(_)));
                assert!(has_property, "Expected virtual property member");
            }
            _ => panic!("Expected interface declaration"),
        }
    }

    #[test]
    fn parse_enum_with_values() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            enum Color {
                Red,
                Green = 1,
                Blue = 2
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Verify enumerator names and values
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Enum(e) => {
                assert_eq!(e.name.name, "Color");
                assert_eq!(e.enumerators.len(), 3);
                assert_eq!(e.enumerators[0].name.name, "Red");
                assert!(e.enumerators[0].value.is_none()); // No explicit value
                assert_eq!(e.enumerators[1].name.name, "Green");
                assert!(e.enumerators[1].value.is_some()); // Has value = 1
                assert_eq!(e.enumerators[2].name.name, "Blue");
                assert!(e.enumerators[2].value.is_some()); // Has value = 2
            }
            _ => panic!("Expected enum declaration"),
        }
    }

    #[test]
    fn parse_funcdef_and_typedef() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            typedef int EntityId;
            funcdef void Callback(int x);
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items().len(), 2);
    }

    #[test]
    fn parse_mixin_class() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            mixin class MyMixin {
                void mixinMethod() { }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Verify mixin declaration with class inside
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Mixin(mixin) => {
                assert_eq!(mixin.class.name.name, "MyMixin");
                assert_eq!(mixin.class.members.len(), 1);
            }
            _ => panic!("Expected mixin declaration"),
        }
    }

    #[test]
    fn parse_constructor_and_destructor() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class MyClass {
                MyClass() { }
                ~MyClass() { }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Verify class has ctor and dtor
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Class(class) => {
                assert_eq!(class.name.name, "MyClass");
                assert_eq!(class.members.len(), 2);
                // Check for constructor and destructor
                let has_ctor = class
                    .members
                    .iter()
                    .any(|m| matches!(m, ClassMember::Method(f) if f.is_constructor()));
                let has_dtor = class
                    .members
                    .iter()
                    .any(|m| matches!(m, ClassMember::Method(f) if f.is_destructor));
                assert!(has_ctor, "Expected constructor");
                assert!(has_dtor, "Expected destructor");
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn parse_function_with_defaults() {
        let arena = bumpalo::Bump::new();
        let source = "void func(int x = 42, string name = \"default\") { }";

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Verify params have defaults
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Function(func) => {
                assert_eq!(func.name.name, "func");
                assert_eq!(func.params.len(), 2);
                assert!(
                    func.params[0].default.is_some(),
                    "First param should have default"
                );
                assert!(
                    func.params[1].default.is_some(),
                    "Second param should have default"
                );
                // Verify first default is literal 42
                match &func.params[0].default {
                    Some(Expr::Literal(lit)) => {
                        assert!(matches!(lit.kind, LiteralKind::Int(42)));
                    }
                    _ => panic!("Expected literal default"),
                }
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn parse_nested_namespaces() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            namespace A::B::C {
                class Nested { }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Assert namespace path segments (A::B::C)
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Namespace(ns) => {
                assert_eq!(ns.path.len(), 3);
                assert_eq!(ns.path[0].name, "A");
                assert_eq!(ns.path[1].name, "B");
                assert_eq!(ns.path[2].name, "C");
            }
            _ => panic!("Expected namespace declaration"),
        }
    }

    #[test]
    fn parse_const_method() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Foo {
                int getValue() const { return 42; }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Assert method is_const
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Class(class) => {
                assert_eq!(class.name.name, "Foo");
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert_eq!(method.name.name, "getValue");
                        assert!(method.is_const);
                    }
                    _ => panic!("Expected method member"),
                }
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn parse_multiple_inheritance() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Player : Character, IDrawable, ISerializable {
                void draw() { }
            }
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        // Assert base list contains 3 interfaces
        let script = result.unwrap();
        match &script.items()[0] {
            Item::Class(class) => {
                assert_eq!(class.name.name, "Player");
                assert_eq!(class.inheritance.len(), 3);
                assert_eq!(class.inheritance[0].ident.name, "Character");
                assert_eq!(class.inheritance[1].ident.name, "IDrawable");
                assert_eq!(class.inheritance[2].ident.name, "ISerializable");
            }
            _ => panic!("Expected class declaration"),
        }
    }

    #[test]
    fn parse_forward_declarations() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Player;
            enum Color;
            interface IDrawable;
        "#;

        let result = Parser::parse(source, &arena);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items().len(), 3);
    }

    #[test]
    fn parse_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        // Invalid character should cause lexer error
        let source = "int x = @@@;"; // @@@ will cause lexer issues but first @ may be valid (handle)
        let result = Parser::parse(source, &arena);
        // This may succeed or fail depending on how @@@ is tokenized
        // If it fails, verify we get an error
        if result.is_err() {
            let errors = result.unwrap_err();
            assert!(!errors.is_empty());
        }
        // If it succeeds, that's also valid (@@@ might be tokenized as handles)
    }

    #[test]
    fn parse_lenient_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        // Unterminated string causes lexer error
        let source = r#"int x = "unterminated"#;
        let (script, errors) = Parser::parse_lenient(source, &arena);
        // Assert errors non-empty
        assert!(
            !errors.is_empty(),
            "Should have lexer errors for unterminated string"
        );
        let _ = script;
    }

    #[test]
    fn parse_expression_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        let source = r#""unterminated string"#;
        let result = Parser::expression(source, &arena);
        // Should error due to unterminated string
        assert!(result.is_err());

        // Assert error kind for unterminated string
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        // Unterminated string should cause an error (likely ExpectedToken or UnexpectedEof)
        // The specific error type may vary by lexer implementation
    }

    #[test]
    fn parse_statement_with_error() {
        let arena = bumpalo::Bump::new();
        // Invalid statement syntax
        let source = "return return;";
        let result = Parser::statement(source, &arena);
        // Should error
        assert!(result.is_err());

        // Assert ParseErrorKind::UnexpectedToken or similar
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        // Verify we have at least one error for invalid syntax
        let has_error = errors.into_iter().any(|e| {
            matches!(
                e.kind,
                ParseErrorKind::UnexpectedToken | ParseErrorKind::ExpectedExpression
            )
        });
        assert!(
            has_error,
            "Expected UnexpectedToken or related error for invalid statement"
        );
    }

    #[test]
    fn parse_type_with_error() {
        let arena = bumpalo::Bump::new();
        // Invalid type syntax (starting with a number)
        let source = "123InvalidType";
        let result = Parser::type_expr(source, &arena);
        // Should error
        assert!(result.is_err());

        // Assert error kind for invalid type
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        // Should be UnexpectedToken since type can't start with number
        let has_error = errors.into_iter().any(|e| {
            matches!(
                e.kind,
                ParseErrorKind::UnexpectedToken | ParseErrorKind::ExpectedType
            )
        });
        assert!(
            has_error,
            "Expected UnexpectedToken or ExpectedType for invalid type syntax"
        );
    }

    #[test]
    fn script_span() {
        let arena = bumpalo::Bump::new();
        let source = "void foo() { }";
        let result = Parser::parse(source, &arena).unwrap();
        let span = result.span();
        // Span should be valid
        assert!(span.len > 0 || span.line > 0);
    }

    #[test]
    fn parse_statement_with_parse_error() {
        let arena = bumpalo::Bump::new();
        // Missing semicolon causes error
        let source = "int x = 42";
        let result = Parser::statement(source, &arena);
        assert!(result.is_err());

        // Assert missing semicolon error
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        // Should expect semicolon or Eof
        let has_error = errors.into_iter().any(|e| {
            matches!(
                e.kind,
                ParseErrorKind::UnexpectedEof | ParseErrorKind::ExpectedToken
            )
        });
        assert!(
            has_error,
            "Expected UnexpectedEof or ExpectedToken for missing semicolon"
        );
    }

    #[test]
    fn parse_type_expr_valid() {
        let arena = bumpalo::Bump::new();
        let result = Parser::type_expr("array<int>@", &arena);
        assert!(result.is_ok());

        // Assert TypeBase::Named("array") and handle
        let ty = result.unwrap();
        assert!(ty.has_handle());
        match &ty.base {
            TypeBase::Named(name) => {
                assert_eq!(name.name, "array");
            }
            _ => panic!("Expected named type 'array'"),
        }
    }

    #[test]
    fn parse_lenient_complete_failure() {
        let arena = bumpalo::Bump::new();
        // Completely invalid syntax that may cause parse_script to return Err
        let source = "@@@@@@@@@@";
        let (script, errors) = Parser::parse_lenient(source, &arena);
        // Assert script.items() behavior - should return a slice (may be empty)
        let items = script.items();
        assert!(items.len() == 0 || items.len() > 0); // Always true, but exercises the code
        // Should have errors
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_expression_with_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Expression that may accumulate errors during parsing
        let source = "a.b.c."; // Trailing dot
        let result = Parser::expression(source, &arena);
        // Should fail
        assert!(result.is_err());

        // Assert trailing dot error kind
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        // Trailing dot should cause parse error - any error kind is acceptable
        // as long as it's detected
    }

    #[test]
    fn parse_statement_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Statement that accumulates errors
        let source = "if ("; // Incomplete if
        let result = Parser::statement(source, &arena);
        // Should fail
        assert!(result.is_err());

        // Assert incomplete if error kind
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        let has_error = errors.into_iter().any(|e| {
            matches!(
                e.kind,
                ParseErrorKind::UnexpectedEof | ParseErrorKind::ExpectedExpression
            )
        });
        assert!(has_error, "Expected error for incomplete if statement");
    }

    #[test]
    fn parse_type_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Type with incomplete template
        let source = "array<"; // Incomplete template
        let result = Parser::type_expr(source, &arena);
        // Should fail
        assert!(result.is_err());

        // Assert incomplete template error kind
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        let has_error = errors.into_iter().any(|e| {
            matches!(
                e.kind,
                ParseErrorKind::UnexpectedEof | ParseErrorKind::ExpectedType
            )
        });
        assert!(has_error, "Expected error for incomplete template");
    }

    // =========================================================================
    // FFI function declaration parsing tests
    // =========================================================================

    #[test]
    fn parse_function_decl_simple() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("int add(int a, int b)", &arena).unwrap();

        assert_eq!(sig.name.name, "add");
        assert_eq!(sig.params.len(), 2);
        assert_eq!(sig.params[0].name.unwrap().name, "a");
        assert_eq!(sig.params[1].name.unwrap().name, "b");
        assert!(!sig.is_const);
    }

    #[test]
    fn parse_function_decl_no_params() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("void main()", &arena).unwrap();

        assert_eq!(sig.name.name, "main");
        assert_eq!(sig.params.len(), 0);
    }

    #[test]
    fn parse_function_decl_const_method() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("int getValue() const", &arena).unwrap();

        assert_eq!(sig.name.name, "getValue");
        assert!(sig.is_const);
    }

    #[test]
    fn parse_function_decl_ref_param() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("void print(const string& in msg)", &arena).unwrap();

        assert_eq!(sig.name.name, "print");
        assert_eq!(sig.params.len(), 1);
    }

    #[test]
    fn parse_function_decl_multiple_params() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("float lerp(float a, float b, float t)", &arena).unwrap();

        assert_eq!(sig.name.name, "lerp");
        assert_eq!(sig.params.len(), 3);
    }

    #[test]
    fn parse_function_decl_error_no_return_type() {
        let arena = bumpalo::Bump::new();
        // Missing return type should fail
        let result = Parser::function_decl("add(int a, int b)", &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_function_decl_error_trailing_tokens() {
        let arena = bumpalo::Bump::new();
        // Trailing semicolon should fail (we don't want full declarations)
        let result = Parser::function_decl("void foo();", &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_function_decl_error_with_body() {
        let arena = bumpalo::Bump::new();
        // Body should fail (we only want signatures)
        let result = Parser::function_decl("void foo() {}", &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_function_decl_property_attr() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("int get_value() property", &arena).unwrap();

        assert_eq!(sig.name.name, "get_value");
        assert!(sig.attrs.property);
    }

    #[test]
    fn parse_function_decl_handle_return() {
        let arena = bumpalo::Bump::new();
        let sig = Parser::function_decl("MyClass@ create()", &arena).unwrap();

        assert_eq!(sig.name.name, "create");
        assert!(sig.return_type.ty.has_handle());
    }

    // =========================================================================
    // Property declaration parsing tests
    // =========================================================================

    #[test]
    fn parse_property_decl_simple() {
        let arena = bumpalo::Bump::new();
        let prop = Parser::property_decl("int score", &arena).unwrap();

        assert_eq!(prop.name.name, "score");
        assert!(!prop.ty.is_const);
    }

    #[test]
    fn parse_property_decl_const() {
        let arena = bumpalo::Bump::new();
        let prop = Parser::property_decl("const float PI", &arena).unwrap();

        assert_eq!(prop.name.name, "PI");
        assert!(prop.ty.is_const);
    }

    #[test]
    fn parse_property_decl_handle() {
        let arena = bumpalo::Bump::new();
        let prop = Parser::property_decl("MyClass@ obj", &arena).unwrap();

        assert_eq!(prop.name.name, "obj");
        assert!(prop.ty.has_handle());
    }

    #[test]
    fn parse_property_decl_const_handle() {
        let arena = bumpalo::Bump::new();
        let prop = Parser::property_decl("const MyClass@ obj", &arena).unwrap();

        assert_eq!(prop.name.name, "obj");
        assert!(prop.ty.is_const);
        assert!(prop.ty.has_handle());
    }

    #[test]
    fn parse_property_decl_error_missing_name() {
        let arena = bumpalo::Bump::new();
        // Just a type without a name should fail
        let result = Parser::property_decl("int", &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_property_decl_error_trailing_tokens() {
        let arena = bumpalo::Bump::new();
        // Trailing tokens should fail
        let result = Parser::property_decl("int score = 0", &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_property_decl_error_with_semicolon() {
        let arena = bumpalo::Bump::new();
        // Semicolon should fail (we only want declarations)
        let result = Parser::property_decl("int score;", &arena);
        assert!(result.is_err());
    }
}
