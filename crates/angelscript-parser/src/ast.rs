//! Typed AST for AngelScript.
//!
//! All nodes carry a `Span` for error reporting. The AST is a typed enum tree
//! rather than the C++ generic linked-list-of-node approach.

use angelscript_lexer::token::Span;

// ── Top-level ──────────────────────────────────────────────────────────────

/// A complete parsed script (one source file / code string).
#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub declarations: Vec<Declaration>,
    pub span: Span,
}

/// A top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
    Function(FunctionDecl),
    Namespace(NamespaceDecl),
    Import(ImportDecl),
    Typedef(TypedefDecl),
    Funcdef(FuncdefDecl),
    Mixin(ClassDecl),
    GlobalVar(VarDecl),
    VirtualProperty(VirtualPropDecl),
    UsingNamespace(UsingNamespaceDecl),
}

// ── Declarations ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: String,
    pub flags: Vec<ClassFlag>,
    pub base_classes: Vec<ScopedIdentifier>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassFlag {
    Shared,
    Abstract,
    Final,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Function(FunctionDecl),
    VirtualProperty(VirtualPropDecl),
    Variable(VarDecl),
    Funcdef(FuncdefDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: String,
    pub flags: Vec<InterfaceFlag>,
    pub bases: Vec<ScopedIdentifier>,
    pub methods: Vec<InterfaceMethod>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceFlag {
    Shared,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub return_type: TypeExpr,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: String,
    pub flags: Vec<EnumFlag>,
    pub values: Vec<EnumValue>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumFlag {
    Shared,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub return_type: Option<TypeExpr>,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
    pub attributes: Vec<FuncAttr>,
    pub access: Option<AccessModifier>,
    pub body: Option<Box<Stmt>>,
    pub is_destructor: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncAttr {
    Override,
    Final,
    Explicit,
    Property,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessModifier {
    Private,
    Protected,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub name: Vec<String>,
    pub declarations: Vec<Declaration>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub return_type: TypeExpr,
    pub name: String,
    pub params: Vec<Param>,
    pub attributes: Vec<FuncAttr>,
    pub module: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedefDecl {
    pub primitive: TypeExpr,
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncdefDecl {
    pub flags: Vec<FuncdefFlag>,
    pub return_type: TypeExpr,
    pub name: String,
    pub params: Vec<Param>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncdefFlag {
    Shared,
    External,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualPropDecl {
    pub type_expr: TypeExpr,
    pub name: String,
    pub access: Option<AccessModifier>,
    pub accessors: Vec<PropAccessor>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropAccessor {
    pub kind: PropAccessorKind,
    pub is_const: bool,
    pub attributes: Vec<FuncAttr>,
    pub body: Option<Box<Stmt>>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropAccessorKind {
    Get,
    Set,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsingNamespaceDecl {
    pub namespace: Vec<String>,
    pub span: Span,
}

// ── Parameters ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub type_expr: TypeExpr,
    pub name: Option<String>,
    pub default: Option<Expr>,
    pub span: Span,
}

// ── Types ──────────────────────────────────────────────────────────────────

/// A type expression as it appears in source code (not yet resolved).
#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_handle_to_const: bool,
    pub is_ref: bool,
    pub ref_modifier: Option<RefModifier>,
    pub array_dimensions: u32,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExprKind {
    /// Primitive type: `int`, `float`, `bool`, etc.
    Primitive(PrimType),
    /// Named type with optional scope: `Foo`, `NS::Bar`.
    Named(ScopedIdentifier),
    /// Template type: `array<int>`, `dictionary<string, int>`.
    Template {
        base: ScopedIdentifier,
        args: Vec<TypeExpr>,
    },
    /// `auto` type.
    Auto,
    /// `?` for deduced type in list patterns.
    Infer,
    /// `void`.
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimType {
    Bool,
    Int8,
    Int16,
    Int,
    Int64,
    UInt8,
    UInt16,
    UInt,
    UInt64,
    Float,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefModifier {
    In,
    Out,
    InOut,
}

/// A possibly-scoped identifier: `::Foo`, `NS::Bar`, `Foo`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopedIdentifier {
    pub is_global: bool,
    pub scopes: Vec<String>,
    pub name: String,
    pub span: Span,
}

impl ScopedIdentifier {
    pub fn simple(name: String, span: Span) -> Self {
        ScopedIdentifier {
            is_global: false,
            scopes: Vec::new(),
            name,
            span,
        }
    }
}

// ── Statements ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// `{ ... }`
    Block(Vec<Stmt>),

    /// Variable declaration: `int x = 5;`
    VarDecl(VarDecl),

    /// `if (cond) body [else body]`
    If {
        condition: Expr,
        then_body: Box<Stmt>,
        else_body: Option<Box<Stmt>>,
    },

    /// `for (init; cond; step) body`
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Vec<Expr>,
        body: Box<Stmt>,
    },

    /// `while (cond) body`
    While { condition: Expr, body: Box<Stmt> },

    /// `do body while (cond);`
    DoWhile { body: Box<Stmt>, condition: Expr },

    /// `switch (expr) { case ... }`
    Switch { expr: Expr, cases: Vec<CaseClause> },

    /// `return [expr];`
    Return(Option<Expr>),

    /// `break;`
    Break,

    /// `continue;`
    Continue,

    /// `try { ... } catch { ... }`
    TryCatch {
        try_body: Box<Stmt>,
        catch_body: Box<Stmt>,
    },

    /// Expression used as statement: `foo();`
    ExprStatement(Expr),

    /// Empty statement `;`
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub access: Option<AccessModifier>,
    pub type_expr: TypeExpr,
    pub declarators: Vec<VarDeclarator>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclarator {
    pub name: String,
    pub init: Option<VarInit>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarInit {
    /// `= expr`
    Expr(Expr),
    /// `= { ... }`
    InitList(InitList),
    /// `(args)`
    ConstructorArgs(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseClause {
    pub label: CaseLabel,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseLabel {
    Expr(Expr),
    Default,
}

// ── Expressions ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Integer literal: `42`, `0xFF`.
    IntLiteral(i64),
    /// Float literal: `3.14f`.
    FloatLiteral(f64),
    /// Double literal: `3.14`.
    DoubleLiteral(f64),
    /// String literal: `"hello"`.
    StringLiteral(String),
    /// `true` / `false`.
    BoolLiteral(bool),
    /// `null`.
    Null,

    /// Simple identifier: `foo`.
    Identifier(ScopedIdentifier),

    /// Binary operation: `a + b`, `a == b`.
    BinaryOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },

    /// Unary prefix operation: `-x`, `!x`, `++x`.
    UnaryOp { op: UnaryOp, operand: Box<Expr> },

    /// Postfix operation: `x++`, `x--`.
    PostfixOp { op: PostfixOp, operand: Box<Expr> },

    /// Assignment: `x = 5`, `x += 1`.
    Assign {
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
    },

    /// Member access: `obj.member`.
    MemberAccess { object: Box<Expr>, member: String },

    /// Indexing: `arr[i]`.
    Index { object: Box<Expr>, index: Box<Expr> },

    /// Function call: `foo(a, b)`.
    FunctionCall {
        callee: Box<Expr>,
        args: Vec<FuncArg>,
    },

    /// Constructor call: `Type(args)`.
    ConstructCall {
        type_expr: TypeExpr,
        args: Vec<FuncArg>,
    },

    /// Type cast: `cast<Type>(expr)`.
    Cast {
        type_expr: TypeExpr,
        expr: Box<Expr>,
    },

    /// Ternary conditional: `cond ? a : b`.
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Handle-of operator: `@obj`.
    HandleOf(Box<Expr>),

    /// Lambda: `function(params) { body }`.
    Lambda { params: Vec<Param>, body: Box<Stmt> },

    /// Initialization list: `{1, 2, 3}`.
    InitList(InitList),

    /// `void` expression.
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncArg {
    pub name: Option<String>,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitList {
    pub items: Vec<InitListItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitListItem {
    Expr(Expr),
    Nested(InitList),
}

// ── Operators ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Comparison
    Eq,
    NotEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    Is,
    NotIs,

    // Logical
    LogicAnd,
    LogicOr,
    LogicXor,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    ShiftRightArith,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
    LogicNot,
    BitNot,
    PreInc,
    PreDec,
    HandleOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostfixOp {
    Inc,
    Dec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    OrAssign,
    AndAssign,
    XorAssign,
    ShiftLeftAssign,
    ShiftRightLAssign,
    ShiftRightAAssign,
}

// ── Operator precedence (for Pratt parsing) ────────────────────────────────

impl BinOp {
    /// Binding power for Pratt parsing.
    /// Higher = binds tighter. Returns (left_bp, right_bp).
    pub fn binding_power(self) -> (u8, u8) {
        match self {
            BinOp::LogicOr => (2, 3),
            BinOp::LogicXor => (4, 5),
            BinOp::LogicAnd => (6, 7),
            BinOp::BitOr => (8, 9),
            BinOp::BitXor => (10, 11),
            BinOp::BitAnd => (12, 13),
            BinOp::Eq | BinOp::NotEq | BinOp::Is | BinOp::NotIs => (14, 15),
            BinOp::Less | BinOp::LessEq | BinOp::Greater | BinOp::GreaterEq => (16, 17),
            BinOp::ShiftLeft | BinOp::ShiftRight | BinOp::ShiftRightArith => (18, 19),
            BinOp::Add | BinOp::Sub => (20, 21),
            BinOp::Mul | BinOp::Div | BinOp::Mod => (22, 23),
            BinOp::Pow => (25, 24), // Right-associative
        }
    }
}
