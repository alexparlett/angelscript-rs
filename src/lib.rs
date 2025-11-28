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
//! 2. **Parser** - Builds an AST ([`parse_lenient`] function)
//! 3. **Semantic Analysis** - 3-pass compilation ([`semantic::Compiler`])
//!    - Pass 1: Registration (register all global names)
//!    - Pass 2a: Type Compilation (resolve types)
//!    - Pass 2b: Function Compilation (compile to bytecode)
//! 4. **VM** - Executes bytecode (planned)
//!
//! # Example: Basic Usage (Recommended)
//!
//! ```
//! use angelscript::ScriptModule;
//!
//! let mut module = ScriptModule::new();
//!
//! // Add source files
//! module.add_source("player.as", r#"
//!     class Player {
//!         int health;
//!         Player(int h) { }
//!     }
//! "#).unwrap();
//!
//! module.add_source("main.as", r#"
//!     void main() {
//!         Player p = Player(100);
//!     }
//! "#).unwrap();
//!
//! // Build (parse + compile)
//! module.build().unwrap();
//!
//! // Module is now ready for execution
//! println!("Compiled {} functions", module.function_count());
//! println!("Registered {} types", module.type_count());
//! ```
//!
//! # Example: Parse Only
//!
//! ```
//! use angelscript::parse_lenient;
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = "int add(int a, int b) { return a + b; }";
//!
//! let (script, errors) = parse_lenient(source, &arena);
//! if errors.is_empty() {
//!     println!("Successfully parsed {} items", script.items().len());
//! }
//! ```

mod ast;
mod lexer;
mod module;
pub mod semantic;
pub mod codegen;

pub use ast::{parse, parse_expression, parse_lenient, parse_statement, parse_type_expr};

// Re-export visitor for AST traversal
pub use ast::visitor;

// Re-export all AST types for public use
pub use ast::{
    // Top-level script and items
    Script, Item,

    // Declarations
    FunctionDecl, FunctionParam,
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

// Re-export high-level module API (recommended for most users)
pub use module::{ScriptModule, BuildError, ModuleError};

// Re-export semantic compiler (for advanced use cases)
pub use semantic::Compiler;

// Re-export semantic types
pub use semantic::{
    CompilationResult, CompiledModule, SemanticError, SemanticErrorKind, SemanticErrors,
};

// Re-export codegen types
pub use codegen::{BytecodeEmitter, CompiledBytecode, Instruction};
