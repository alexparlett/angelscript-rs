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
//! 2. **Parser** - Builds an AST (planned)
//! 3. **Type Checker** - Validates types (planned)
//! 4. **Compiler** - Generates bytecode (planned)
//! 5. **VM** - Executes bytecode (planned)
//!
//! # Example
//!
//! ```
//! use angelscript::{parse_lenient, Script};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let source = r#"
//!     int add(int a, int b) {
//!         return a + b;
//!     }
//! "#;
//!
//! let (script, errors) = parse_lenient(source, &arena);
//! if errors.is_empty() {
//!     println!("Successfully parsed {} items", script.items().len());
//! }
//! ```

mod ast;
mod compiler;
mod lexer;
pub mod semantic;

pub use ast::{parse, parse_expression, parse_lenient, parse_statement, parse_type_expr};

// Re-export compilation context for multi-file compilation
pub use compiler::CompilationContext;

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

// Re-export semantic types
pub use semantic::{SemanticError, SemanticErrorKind, SemanticErrors};
