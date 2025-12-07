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
//! ```
//! use angelscript::Unit;
//!
//! let mut unit = Unit::new();
//!
//! // Add source files
//! unit.add_source("player.as", r#"
//!     class Player {
//!         int health;
//!         Player(int h) { }
//!     }
//! "#).unwrap();
//!
//! unit.add_source("main.as", r#"
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
//! ```
//! use angelscript::Parser;
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

mod ast;
mod context;
pub mod ffi;
mod lexer;
mod module;
pub mod modules;
pub mod semantic;
pub mod codegen;
pub mod types;
mod unit;

// Re-export Parser as the primary parsing interface
pub use ast::Parser;

// Re-export visitor for AST traversal
pub use ast::visitor;

// Re-export all AST types for public use
pub use ast::{
    // Top-level script and items
    Script, Item,

    // Declarations
    FunctionDecl, FunctionParam, FunctionSignatureDecl, PropertyDecl,
    ClassDecl, ClassMember, FieldDecl, VirtualPropertyDecl, PropertyAccessor,
    InterfaceDecl, InterfaceMember, InterfaceMethod,
    EnumDecl, Enumerator,
    GlobalVarDecl,
    NamespaceDecl,
    TypedefDecl,
    FuncdefDecl,
    MixinDecl,
    ImportDecl,

    // Expressions
    Expr,
    LiteralExpr, LiteralKind,
    IdentExpr,
    BinaryExpr,
    UnaryExpr,
    AssignExpr,
    TernaryExpr,
    CallExpr, Argument,
    IndexExpr, IndexItem,
    MemberExpr, MemberAccess,
    PostfixExpr,
    CastExpr,
    LambdaExpr, LambdaParam,
    InitListExpr, InitElement,
    ParenExpr,

    // Statements
    Stmt,
    ExprStmt,
    VarDeclStmt, VarDeclarator,
    ReturnStmt,
    BreakStmt,
    ContinueStmt,
    Block,
    IfStmt,
    WhileStmt,
    DoWhileStmt,
    ForStmt, ForInit,
    ForeachStmt, ForeachVar,
    SwitchStmt, SwitchCase,
    TryCatchStmt,

    // Types
    TypeExpr, TypeBase, PrimitiveType, TypeSuffix, ReturnType, ParamType,

    // Common nodes
    Ident, Scope, Visibility, DeclModifiers, FuncAttr, RefKind, PropertyAccessorKind,

    // Operators
    BinaryOp, UnaryOp, PostfixOp, AssignOp,

    // Errors
    ParseError, ParseErrorKind, ParseErrors,
};

// Re-export compilation unit API (recommended for most users)
pub use unit::{Unit, BuildError, UnitError};

// Re-export FFI module and context API
pub use module::{Module, FfiModuleError};
pub use types::FfiEnumDef;
pub use context::{Context, ContextError};

// Backwards compatibility alias (deprecated)
#[deprecated(since = "0.2.0", note = "Use `Unit` instead")]
pub type ScriptModule<'app> = Unit<'app>;

// Re-export semantic compiler (for advanced use cases)
pub use semantic::Compiler;

// Re-export semantic types
pub use semantic::{
    CompilationResult, CompiledModule, SemanticError, SemanticErrorKind, SemanticErrors,
};

// Re-export codegen types
pub use codegen::{BytecodeEmitter, CompiledBytecode, Instruction};

// Re-export built-in module types
pub use modules::{ScriptArray, ScriptDict, ScriptString};

// Re-export built-in module constructors
pub use modules::{
    array_module, default_modules, dictionary_module, math_module, std_module, string_module,
};

// Re-export common types
pub use types::{ReferenceKind, TypeKind};
