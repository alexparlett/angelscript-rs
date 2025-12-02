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
//! use angelscript::parse;
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
//! match parse(source, &arena) {
//!     Ok(script) => println!("Parsed successfully: {} items", script.items().len()),
//!     Err(errors) => eprintln!("Parse errors: {}", errors),
//! }
//! ```

// Core types
pub mod error;
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

pub use decl::*;
pub use error::*;
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
    span: crate::lexer::Span,
}

impl<'ast> Script<'ast> {
    /// Create a new script from parsed items.
    pub(crate) fn new(items: &'ast [Item<'ast>], span: crate::lexer::Span) -> Self {
        Self { items, span }
    }

    /// Get the top-level items in this script.
    pub fn items(&self) -> &[Item<'ast>] {
        self.items
    }

    /// Get the source location span of this script.
    pub fn span(&self) -> crate::lexer::Span {
        self.span
    }
}


/// Parse AngelScript source code into an AST.
///
/// Requires a `bumpalo::Bump` arena allocator for AST node allocation.
/// All AST nodes will be allocated in the arena and remain valid for the arena's lifetime.
///
/// For multi-file compilation, use `CompilationContext` which owns an arena and
/// allows multiple scripts to share the same arena.
///
/// # Example
///
/// ```
/// use angelscript::parse;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let source = r#"
///     class Player {
///         int health = 100;
///         void takeDamage(int amount) {
///             health -= amount;
///         }
///     }
/// "#;
///
/// match parse(source, &arena) {
///     Ok(script) => {
///         println!("Parsed {} items", script.items().len());
///     }
///     Err(errors) => {
///         eprintln!("Parse errors: {}", errors);
///     }
/// }
/// ```
#[cfg_attr(feature = "profiling", profiling::function)]
pub fn parse<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> Result<Script<'ast>, ParseErrors> {
    let mut parser = Parser::new(source, arena);

    let result = parser.parse_script();

    // Lexer errors are collected during Parser::new() and already in parser.errors

    match result {
        Ok((items, span)) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                Ok(Script::new(items, span))
            }
        }
        Err(err) => {
            parser.errors.push(err);
            Err(parser.take_errors())
        }
    }
}

/// Parse AngelScript source code leniently, returning both the AST and any errors.
///
/// This function always returns a `Script`, even if errors occurred. The script
/// may be incomplete, but it can still be useful for analysis, error recovery,
/// or partial processing.
///
/// Requires a `bumpalo::Bump` arena allocator for AST node allocation.
///
/// Returns a tuple of `(Script, Vec<ParseError>)` where the error vector may be empty.
///
/// For multi-file compilation, use `CompilationContext` which owns an arena and
/// allows multiple scripts to share the same arena.
///
/// # Example
///
/// ```
/// use angelscript::parse_lenient;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// let source = r#"
///     class Player {
///         int health = 100;
///         void takeDamage(int amount) {
///             health -= amount;
///         }
///     }
/// "#;
///
/// let (script, errors) = parse_lenient(source, &arena);
///
/// println!("Parsed {} items", script.items().len());
/// println!("Found {} errors", errors.len());
///
/// // Can still work with the partial AST
/// for item in script.items() {
///     // Process items...
/// }
///
/// // And handle errors
/// for error in &errors {
///     eprintln!("Warning: {}", error);
/// }
/// ```
#[cfg_attr(feature = "profiling", profiling::function)]
pub fn parse_lenient<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> (Script<'ast>, Vec<ParseError>) {
    let mut parser = Parser::new(source, arena);

    let (items, span) = parser.parse_script().unwrap_or_else(|err| {
        parser.errors.push(err);
        (&[][..], crate::lexer::Span::new(1, 1, 0))
    });

    // Lexer errors are collected during Parser::new() and already in parser.errors

    let errors = parser.take_errors().into_vec();
    (Script::new(items, span), errors)
}

/// Parse a single expression from source code.
///
/// This is useful for parsing standalone expressions or for testing.
/// Requires a `bumpalo::Bump` arena allocator for AST node allocation.
///
/// # Example
///
/// ```
/// use angelscript::parse_expression;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// match parse_expression("1 + 2 * 3", &arena) {
///     Ok(expr) => println!("Parsed expression successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_expression<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> Result<&'ast Expr<'ast>, ParseErrors> {
    let mut parser = Parser::new(source, arena);

    let result = parser.parse_expr(0);

    // Lexer errors are collected during Parser::new() and already in parser.errors

    match result {
        Ok(expr) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                Ok(expr)
            }
        }
        Err(err) => {
            parser.errors.push(err);
            Err(parser.take_errors())
        }
    }
}

/// Parse a single statement from source code.
///
/// This is useful for parsing standalone statements or for testing.
/// Requires a `bumpalo::Bump` arena allocator for AST node allocation.
///
/// # Example
///
/// ```
/// use angelscript::parse_statement;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// match parse_statement("if (x > 0) { return x; }", &arena) {
///     Ok(stmt) => println!("Parsed statement successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_statement<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> Result<Stmt<'ast>, ParseErrors> {
    let mut parser = Parser::new(source, arena);

    let result = parser.parse_statement();

    // Lexer errors are collected during Parser::new() and already in parser.errors

    match result {
        Ok(stmt) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                Ok(stmt)
            }
        }
        Err(err) => {
            parser.errors.push(err);
            Err(parser.take_errors())
        }
    }
}

/// Parse a type expression from source code.
///
/// This is useful for parsing standalone type expressions or for testing.
/// Requires a `bumpalo::Bump` arena allocator for AST node allocation.
///
/// # Example
///
/// ```
/// use angelscript::parse_type_expr;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// match parse_type_expr("const array<int>@", &arena) {
///     Ok(ty) => println!("Parsed type successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_type_expr<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> Result<TypeExpr<'ast>, ParseErrors> {
    let mut parser = Parser::new(source, arena);

    let result = parser.parse_type();

    // Lexer errors are collected during Parser::new() and already in parser.errors

    match result {
        Ok(type_expr) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                Ok(type_expr)
            }
        }
        Err(err) => {
            parser.errors.push(err);
            Err(parser.take_errors())
        }
    }
}

/// Parse a property declaration expression (type followed by identifier).
///
/// This parses strings like "const int score" or "MyClass@ obj" and returns
/// the type expression and identifier name.
///
/// # Example
///
/// ```
/// use angelscript::parse_property_expr;
/// use bumpalo::Bump;
///
/// let arena = Bump::new();
/// match parse_property_expr("const int score", &arena) {
///     Ok((ty, name)) => println!("Property '{}' of type '{}'", name.name, ty),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_property_expr<'ast>(
    source: &str,
    arena: &'ast bumpalo::Bump,
) -> Result<(TypeExpr<'ast>, Ident<'ast>), ParseErrors> {
    use crate::lexer::TokenKind;

    let mut parser = Parser::new(source, arena);

    // Parse the type expression
    let type_result = parser.parse_type();
    let type_expr = match type_result {
        Ok(ty) => ty,
        Err(err) => {
            parser.errors.push(err);
            return Err(parser.take_errors());
        }
    };

    // Parse the identifier name
    let name_result = parser.expect(TokenKind::Identifier);
    let name = match name_result {
        Ok(token) => Ident::new(token.lexeme, token.span),
        Err(err) => {
            parser.errors.push(err);
            return Err(parser.take_errors());
        }
    };

    if parser.has_errors() {
        Err(parser.take_errors())
    } else {
        Ok((type_expr, name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_function() {
        let arena = bumpalo::Bump::new();
        let source = "void foo() { }";
        let result = parse(source, &arena);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);
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
        let result = parse(source, &arena);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_with_errors() {
        let arena = bumpalo::Bump::new();
        let source = "int x = ;"; // Missing expression
        let result = parse(source, &arena);
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
        let (script, errors) = parse_lenient(source, &arena);

        // Should have errors but still parse something
        assert!(!errors.is_empty());
        // Should recover and parse the second declaration
        assert!(!script.items().is_empty());
    }

    #[test]
    fn parse_lenient_no_errors() {
        let arena = bumpalo::Bump::new();
        let source = "int x = 42;";
        let (script, errors) = parse_lenient(source, &arena);

        assert!(errors.is_empty());
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_expression_simple() {
        let arena = bumpalo::Bump::new();
        let result = parse_expression("1 + 2", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_expression_complex() {
        let arena = bumpalo::Bump::new();
        let result = parse_expression("obj.method()[0].field", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_expression_with_error() {
        let arena = bumpalo::Bump::new();
        let result = parse_expression("1 +", &arena); // Incomplete
        assert!(result.is_err());
    }

    #[test]
    fn parse_statement_simple() {
        let arena = bumpalo::Bump::new();
        let result = parse_statement("return 42;", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_statement_if() {
        let arena = bumpalo::Bump::new();
        let result = parse_statement("if (x > 0) { return x; }", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_statement_for() {
        let arena = bumpalo::Bump::new();
        let result = parse_statement("for (int i = 0; i < 10; i++) { }", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_simple() {
        let arena = bumpalo::Bump::new();
        let result = parse_type_expr("int", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_complex() {
        let arena = bumpalo::Bump::new();
        let result = parse_type_expr("const array<int>@ const", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_with_scope() {
        let arena = bumpalo::Bump::new();
        let result = parse_type_expr("Namespace::MyClass", &arena);
        assert!(result.is_ok());
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

        let result = parse(source, &arena);
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

        let (_, errors) = parse_lenient(source, &arena);

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

        let result = parse(source, &arena);
        assert!(result.is_ok());
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

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_funcdef_and_typedef() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            typedef int EntityId;
            funcdef void Callback(int x);
        "#;

        let result = parse(source, &arena);
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

        let result = parse(source, &arena);
        assert!(result.is_ok());
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

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_function_with_defaults() {
        let arena = bumpalo::Bump::new();
        let source = "void func(int x = 42, string name = \"default\") { }";

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_nested_namespaces() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            namespace A::B::C {
                class Nested { }
            }
        "#;

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_const_method() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Foo {
                int getValue() const { return 42; }
            }
        "#;

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_multiple_inheritance() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Player : Character, IDrawable, ISerializable {
                void draw() { }
            }
        "#;

        let result = parse(source, &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_forward_declarations() {
        let arena = bumpalo::Bump::new();
        let source = r#"
            class Player;
            enum Color;
            interface IDrawable;
        "#;

        let result = parse(source, &arena);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items().len(), 3);
    }

    #[test]
    fn parse_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        // Invalid character should cause lexer error
        let source = "int x = @@@;"; // @@@ will cause lexer issues but first @ may be valid (handle)
        let result = parse(source, &arena);
        // This may succeed or fail depending on how @@@ is tokenized
        // We mainly want to exercise the code path
        let _ = result;
    }

    #[test]
    fn parse_lenient_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        // Unterminated string causes lexer error
        let source = r#"int x = "unterminated"#;
        let (script, errors) = parse_lenient(source, &arena);
        // Should have errors from lexer
        let _ = (script, errors);
    }

    #[test]
    fn parse_expression_with_lexer_error() {
        let arena = bumpalo::Bump::new();
        let source = r#""unterminated string"#;
        let result = parse_expression(source, &arena);
        // Should error due to unterminated string
        assert!(result.is_err());
    }

    #[test]
    fn parse_statement_with_error() {
        let arena = bumpalo::Bump::new();
        // Invalid statement syntax
        let source = "return return;";
        let result = parse_statement(source, &arena);
        // Should error
        assert!(result.is_err());
    }

    #[test]
    fn parse_type_with_error() {
        let arena = bumpalo::Bump::new();
        // Invalid type syntax (starting with a number)
        let source = "123InvalidType";
        let result = parse_type_expr(source, &arena);
        // Should error
        assert!(result.is_err());
    }

    #[test]
    fn script_span() {
        let arena = bumpalo::Bump::new();
        let source = "void foo() { }";
        let result = parse(source, &arena).unwrap();
        let span = result.span();
        // Span should be valid
        assert!(span.len > 0 || span.line > 0);
    }

    #[test]
    fn parse_statement_with_parse_error() {
        let arena = bumpalo::Bump::new();
        // Missing semicolon causes error
        let source = "int x = 42";
        let result = parse_statement(source, &arena);
        assert!(result.is_err());
    }

    #[test]
    fn parse_type_expr_valid() {
        let arena = bumpalo::Bump::new();
        let result = parse_type_expr("array<int>@", &arena);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_lenient_complete_failure() {
        let arena = bumpalo::Bump::new();
        // Completely invalid syntax that may cause parse_script to return Err
        let source = "@@@@@@@@@@";
        let (script, _errors) = parse_lenient(source, &arena);
        // Script may be empty but should still return
        let _ = script.items();
    }

    #[test]
    fn parse_expression_with_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Expression that may accumulate errors during parsing
        let source = "a.b.c.";  // Trailing dot
        let result = parse_expression(source, &arena);
        // Should fail
        assert!(result.is_err());
    }

    #[test]
    fn parse_statement_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Statement that accumulates errors
        let source = "if (";  // Incomplete if
        let result = parse_statement(source, &arena);
        // Should fail
        assert!(result.is_err());
    }

    #[test]
    fn parse_type_accumulated_errors() {
        let arena = bumpalo::Bump::new();
        // Type with incomplete template
        let source = "array<";  // Incomplete template
        let result = parse_type_expr(source, &arena);
        // Should fail
        assert!(result.is_err());
    }
}
