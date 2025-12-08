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
//! 3. **Compiler** - Compiles to bytecode ([`angelscript_compiler`])
//! 4. **VM** - Executes bytecode (planned)
//!
//! # Example: Basic Usage (Recommended)
//!
//! ```ignore
//! use angelscript::{Context, Module, Unit};
//! use std::sync::Arc;
//!
//! // Create context and install modules
//! let mut ctx = Context::new();
//! ctx.install(Module::new().class::<MyClass>())?;
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

// Re-export context API
pub use context::{Context, ContextError};

// Re-export error types from core for unified error handling
pub use angelscript_core::{
    AngelScriptError,
    LexError,
    ParseError,
    ParseErrorKind,
    ParseErrors,
    RegistrationError,
    CompilationError,
    RuntimeError,
    Span,
};

// Re-export common types
pub use angelscript_core::{ReferenceKind, TypeKind};

// Re-export types needed for proc macros
pub use angelscript_core::{
    Any, TypeHash, Operator, RefModifier,
    ClassMeta, FunctionMeta, PropertyMeta, ParamMeta, Behavior,
    InterfaceMeta, InterfaceMethodMeta, FuncdefMeta,
    // Enhanced function metadata types
    ReturnMode, ReturnMeta, GenericParamMeta, ListPatternMeta,
    // Native function types for generic calling convention
    CallContext,
};

// Re-export proc macros
pub use angelscript_macros::{Any, function, interface, funcdef};

// Re-export Module and registry types
pub use angelscript_registry::{Module, HasClassMeta, HasFunctionMeta, SymbolRegistry};
