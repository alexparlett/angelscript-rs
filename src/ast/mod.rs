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
//!
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
//! match parse(source) {
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

/// A parsed AngelScript script with owned arena.
///
/// This struct owns the parsed AST and its arena allocator.
/// All AST nodes are allocated in the arena and remain valid
/// for the lifetime of this struct.
#[derive(Debug)]
pub struct Script {
    arena: bumpalo::Bump,
    items: &'static [Item<'static, 'static>],
    span: crate::lexer::Span,
}

impl Script {
    /// Get the top-level items in this script.
    pub fn items(&self) -> &[Item<'static, 'static>] {
        self.items
    }

    /// Get the source location span of this script.
    pub fn span(&self) -> crate::lexer::Span {
        self.span
    }
}

/// Parse AngelScript source code into an AST.
///
/// Returns `Ok(Script)` if parsing succeeds with no errors, or `Err(ParseErrors)`
/// if any errors occurred during parsing.
///
/// # Example
///
/// ```
/// use angelscript::parse;
///
/// let source = r#"
///     class Player {
///         int health = 100;
///         void takeDamage(int amount) {
///             health -= amount;
///         }
///     }
/// "#;
///
/// match parse(source) {
///     Ok(script) => {
///         println!("Parsed {} items", script.items().len());
///     }
///     Err(errors) => {
///         eprintln!("Parse errors: {}", errors);
///     }
/// }
/// ```
pub fn parse(source: &str) -> Result<Script, ParseErrors> {
    let arena = bumpalo::Bump::new();
    let mut parser = Parser::new(source, &arena);

    let result = parser.parse_script();

    // Check for any remaining lexer errors that weren't caught during parsing
    // (e.g., errors in tokens that were never actually consumed)
    if parser.lexer.has_errors() {
        for lexer_error in parser.lexer.take_errors() {
            parser.errors.push(ParseError::new(
                ParseErrorKind::InvalidSyntax,
                lexer_error.span,
                format!("lexer error: {}", lexer_error.message),
            ));
        }
    }

    match result {
        Ok((items, span)) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                // SAFETY: The items slice borrows from the arena, and we're moving both into Script.
                // The arena will live as long as the Script, so the references remain valid.
                // We transmute the lifetime to 'static since the arena is owned by Script.
                let items = unsafe { std::mem::transmute::<&[Item<'_, '_>], &'static [Item<'static, 'static>]>(items) };
                Ok(Script { arena, items, span })
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
/// Returns a tuple of `(Script, Vec<ParseError>)` where the error vector may be empty.
///
/// # Example
///
/// ```
/// use angelscript::parse_lenient;
///
/// let source = r#"
///     class Player {
///         int health = 100;
///         void takeDamage(int amount) {
///             health -= amount;
///         }
///     }
/// "#;
///
/// let (script, errors) = parse_lenient(source);
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
pub fn parse_lenient(source: &str) -> (Script, Vec<ParseError>) {
    let arena = bumpalo::Bump::new();
    let mut parser = Parser::new(source, &arena);

    let (items, span) = parser.parse_script().unwrap_or_else(|err| {
        parser.errors.push(err);
        (&[][..], crate::lexer::Span::new(1, 1, 0))
    });

    // Check for any remaining lexer errors
    if parser.lexer.has_errors() {
        for lexer_error in parser.lexer.take_errors() {
            parser.errors.push(ParseError::new(
                ParseErrorKind::InvalidSyntax,
                lexer_error.span,
                format!("lexer error: {}", lexer_error.message),
            ));
        }
    }

    let errors = parser.take_errors().into_vec();

    // SAFETY: The items slice borrows from the arena, and we're moving both into Script.
    // The arena will live as long as the Script, so the references remain valid.
    let items = unsafe { std::mem::transmute::<&[Item<'_, '_>], &'static [Item<'static, 'static>]>(items) };

    (Script { arena, items, span }, errors)
}

/// Container for a parsed expression that owns its arena.
pub struct ParsedExpr {
    arena: bumpalo::Bump,
    pub expr: &'static Expr<'static, 'static>,
}

impl ParsedExpr {
    /// Get a reference to the expression.
    pub fn expr(&self) -> &Expr<'static, 'static> {
        self.expr
    }
}

/// Container for a parsed statement that owns its arena.
pub struct ParsedStmt {
    arena: bumpalo::Bump,
    pub stmt: Stmt<'static, 'static>,
}

impl ParsedStmt {
    /// Get a reference to the statement.
    pub fn stmt(&self) -> &Stmt<'static, 'static> {
        &self.stmt
    }
}

/// Container for a parsed type expression that owns its arena.
pub struct ParsedTypeExpr {
    arena: bumpalo::Bump,
    pub type_expr: TypeExpr<'static, 'static>,
}

impl ParsedTypeExpr {
    /// Get a reference to the type expression.
    pub fn type_expr(&self) -> &TypeExpr<'static, 'static> {
        &self.type_expr
    }
}

/// Parse a single expression from source code.
///
/// This is useful for parsing standalone expressions or for testing.
///
/// # Example
///
/// ```
/// use angelscript::parse_expression;
///
/// match parse_expression("1 + 2 * 3") {
///     Ok(parsed) => println!("Parsed expression successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_expression(source: &str) -> Result<ParsedExpr, ParseErrors> {
    let arena = bumpalo::Bump::new();
    let mut parser = Parser::new(source, &arena);

    let result = parser.parse_expr(0);

    // Check for any remaining lexer errors
    if parser.lexer.has_errors() {
        for lexer_error in parser.lexer.take_errors() {
            parser.errors.push(ParseError::new(
                ParseErrorKind::InvalidSyntax,
                lexer_error.span,
                format!("lexer error: {}", lexer_error.message),
            ));
        }
    }

    match result {
        Ok(expr) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                // SAFETY: The expr borrows from the arena, and we're moving both into ParsedExpr.
                // The arena will live as long as the ParsedExpr, so the references remain valid.
                let expr = unsafe { std::mem::transmute::<&Expr<'_, '_>, &'static Expr<'static, 'static>>(expr) };
                Ok(ParsedExpr { arena, expr })
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
///
/// # Example
///
/// ```
/// use angelscript::parse_statement;
///
/// match parse_statement("if (x > 0) { return x; }") {
///     Ok(stmt) => println!("Parsed statement successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_statement(source: &str) -> Result<ParsedStmt, ParseErrors> {
    let arena = bumpalo::Bump::new();
    let mut parser = Parser::new(source, &arena);

    let result = parser.parse_statement();

    // Check for any remaining lexer errors
    if parser.lexer.has_errors() {
        for lexer_error in parser.lexer.take_errors() {
            parser.errors.push(ParseError::new(
                ParseErrorKind::InvalidSyntax,
                lexer_error.span,
                format!("lexer error: {}", lexer_error.message),
            ));
        }
    }

    match result {
        Ok(stmt) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                // SAFETY: The stmt borrows from the arena, and we're moving both into ParsedStmt.
                // The arena will live as long as the ParsedStmt, so the references remain valid.
                let stmt = unsafe { std::mem::transmute::<Stmt<'_, '_>, Stmt<'static, 'static>>(stmt) };
                Ok(ParsedStmt { arena, stmt })
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
///
/// # Example
///
/// ```
/// use angelscript::parse_type_expr;
///
/// match parse_type_expr("const array<int>@") {
///     Ok(ty) => println!("Parsed type successfully"),
///     Err(errors) => eprintln!("Errors: {}", errors),
/// }
/// ```
pub fn parse_type_expr(source: &str) -> Result<ParsedTypeExpr, ParseErrors> {
    let arena = bumpalo::Bump::new();
    let mut parser = Parser::new(source, &arena);

    let result = parser.parse_type();

    // Check for any remaining lexer errors
    if parser.lexer.has_errors() {
        for lexer_error in parser.lexer.take_errors() {
            parser.errors.push(ParseError::new(
                ParseErrorKind::InvalidSyntax,
                lexer_error.span,
                format!("lexer error: {}", lexer_error.message),
            ));
        }
    }

    match result {
        Ok(type_expr) => {
            if parser.has_errors() {
                Err(parser.take_errors())
            } else {
                // SAFETY: The type_expr borrows from the arena, and we're moving both into ParsedTypeExpr.
                // The arena will live as long as the ParsedTypeExpr, so the references remain valid.
                let type_expr = unsafe { std::mem::transmute::<TypeExpr<'_, '_>, TypeExpr<'static, 'static>>(type_expr) };
                Ok(ParsedTypeExpr { arena, type_expr })
            }
        }
        Err(err) => {
            parser.errors.push(err);
            Err(parser.take_errors())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_function() {
        let source = "void foo() { }";
        let result = parse(source);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_class_with_members() {
        let source = r#"
            class Player {
                int health;
                void takeDamage(int amount) {
                    health -= amount;
                }
            }
        "#;
        let result = parse(source);
        assert!(result.is_ok());
        let script = result.unwrap();
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_with_errors() {
        let source = "int x = ;"; // Missing expression
        let result = parse(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn parse_lenient_recovers() {
        let source = r#"
            int x = ;
            int y = 42;
        "#;
        let (script, errors) = parse_lenient(source);

        // Should have errors but still parse something
        assert!(!errors.is_empty());
        // Should recover and parse the second declaration
        assert!(!script.items().is_empty());
    }

    #[test]
    fn parse_lenient_no_errors() {
        let source = "int x = 42;";
        let (script, errors) = parse_lenient(source);

        assert!(errors.is_empty());
        assert_eq!(script.items().len(), 1);
    }

    #[test]
    fn parse_expression_simple() {
        let result = parse_expression("1 + 2");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_expression_complex() {
        let result = parse_expression("obj.method()[0].field");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_expression_with_error() {
        let result = parse_expression("1 +"); // Incomplete
        assert!(result.is_err());
    }

    #[test]
    fn parse_statement_simple() {
        let result = parse_statement("return 42;");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_statement_if() {
        let result = parse_statement("if (x > 0) { return x; }");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_statement_for() {
        let result = parse_statement("for (int i = 0; i < 10; i++) { }");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_simple() {
        let result = parse_type_expr("int");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_complex() {
        let result = parse_type_expr("const array<int>@ const");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_type_with_scope() {
        let result = parse_type_expr("Namespace::MyClass");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_complete_program() {
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

        let result = parse(source);
        assert!(result.is_ok(), "Failed to parse complete program");

        let script = result.unwrap();
        assert_eq!(script.items().len(), 3); // namespace, global var, function
    }

    #[test]
    fn parse_multiple_errors() {
        let source = r#"
            int x = ;
            void func( { }
            int y
        "#;

        let (script, errors) = parse_lenient(source);

        // Should have multiple errors
        assert!(errors.len() >= 2, "Should detect multiple errors");

        // Should still produce some AST
        // (may be empty or partial depending on recovery)
    }

    #[test]
    fn parse_interface_with_properties() {
        let source = r#"
            interface IDrawable {
                void draw();
                int Priority {
                    get const;
                }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_enum_with_values() {
        let source = r#"
            enum Color {
                Red,
                Green = 1,
                Blue = 2
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_funcdef_and_typedef() {
        let source = r#"
            typedef int EntityId;
            funcdef void Callback(int x);
        "#;

        let result = parse(source);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items().len(), 2);
    }

    #[test]
    fn parse_mixin_class() {
        let source = r#"
            mixin class MyMixin {
                void mixinMethod() { }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_constructor_and_destructor() {
        let source = r#"
            class MyClass {
                MyClass() { }
                ~MyClass() { }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_function_with_defaults() {
        let source = "void func(int x = 42, string name = \"default\") { }";

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_nested_namespaces() {
        let source = r#"
            namespace A::B::C {
                class Nested { }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_const_method() {
        let source = r#"
            class Foo {
                int getValue() const { return 42; }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_multiple_inheritance() {
        let source = r#"
            class Player : Character, IDrawable, ISerializable {
                void draw() { }
            }
        "#;

        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_forward_declarations() {
        let source = r#"
            class Player;
            enum Color;
            interface IDrawable;
        "#;

        let result = parse(source);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert_eq!(script.items().len(), 3);
    }
}
