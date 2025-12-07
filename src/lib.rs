//! AngelScript implementation in Rust.
//!
//! This crate provides a Rust-native implementation of the AngelScript
//! scripting language, designed for embedding in game engines and other
//! applications.
//!
//! # Architecture
//!
//! The implementation follows a traditional compiler pipeline:
//!
//! 1. **Lexer** - Tokenizes source text ([`lexer`] module)
//! 2. **Parser** - Builds an AST ([`Parser::parse`] function)
//! 3. **Semantic Analysis** - 3-pass compilation ([`semantic::Compiler`])
//!    - Pass 1: Registration (register all global names)
//!    - Pass 2a: Type Compilation (resolve types)
//!    - Pass 2b: Function Compilation (compile to bytecode)
//! 4. **VM** - Executes bytecode (planned)
//!
//! # Example: Basic Usage (Recommended)
//!
//! ```ignore
//! use angelscript::{Context, Unit};
//! use std::sync::Arc;
//!
//! // Create context with default modules and seal it
//! let mut ctx = Context::with_default_modules().unwrap();
//! ctx.seal().unwrap();
//! let ctx = Arc::new(ctx);
//!
//! // Create a compilation unit
//! let mut unit = ctx.create_unit().unwrap();
//!
//! // Add source file
//! unit.add_source("main.as", r#"
//!     class Player {
//!         int health;
//!         Player(int h) { health = h; }
//!     }
//!
//!     void main() {
//!         Player p = Player(100);
//!     }
//! "#).unwrap();
//!
//! // Build (parse + compile)
//! unit.build().unwrap();
//!
//! // Unit is now ready for execution
//! println!("Compiled {} functions", unit.function_count());
//! println!("Registered {} types", unit.type_count());
//! ```
//!
//! # Example: Parse Only
//!
//! ```ignore
//! use angelscript_parser::Parser;
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = "int add(int a, int b) { return a + b; }";
//!
//! let (script, errors) = Parser::parse_lenient(source, &arena);
//! if errors.is_empty() {
//!     println!("Successfully parsed {} items", script.items().len());
//! }
//! ```

mod context;
mod unit;

// Re-export compilation unit API (recommended for most users)
pub use unit::{Unit, BuildError, UnitError};

// Re-export FFI module API directly from crates
pub use angelscript_module::Module;
pub use context::{Context, ContextError};

// Re-export RegistrationError from core for convenience
pub use angelscript_core::RegistrationError;

// Re-export built-in module types directly from crate
pub use angelscript_modules::{ScriptArray, ScriptDict, ScriptString};

// Re-export built-in module constructors directly from crate
pub use angelscript_modules::{
    array_module, default_modules, dictionary_module, math_module, std_module, string_module,
};

// Re-export common types
pub use angelscript_core::{ReferenceKind, TypeKind};
