//! AngelScript Parser crate.
//!
//! This crate provides the lexer and parser for AngelScript source code.
//! It includes:
//! - Lexical analysis (tokenization)
//! - Abstract Syntax Tree (AST) definitions
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

// Lexer module
pub mod lexer;

// AST module
pub mod ast;

// Re-export commonly used types at crate root
pub use ast::Parser;
pub use lexer::{Lexer, Span, Token, TokenKind};
