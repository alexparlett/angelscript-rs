use crate::core::span::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Script {
    pub items: Vec<ScriptNode>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ScriptNode {
    Import(Import),
    Enum(Enum),
    Typedef(Typedef),
    Class(Class),
    Mixin(Mixin),
    Interface(Interface),
    FuncDef(FuncDef),
    VirtProp(VirtProp),
    Var(Var),
    Func(Func),
    Namespace(Namespace),

    Include(Include),
    Pragma(Pragma),
    ConditionalBlock(ConditionalBlock),
    CustomDirective(CustomDirective),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Include {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Pragma {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CustomDirective {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ConditionalBlock {
    pub if_branch: ConditionalBranch,
    pub elif_branches: Vec<ConditionalBranch>,
    pub else_branch: Option<Vec<ScriptNode>>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ConditionalBranch {
    pub condition: String,
    pub items: Vec<ScriptNode>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Import {
    pub type_name: Type,
    pub is_ref: bool,
    pub identifier: String,
    pub params: Vec<Param>,
    pub from: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Using {
    pub namespace: Vec<String>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Namespace {
    pub name: Vec<String>,
    pub items: Vec<ScriptNode>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Enum {
    pub modifiers: Vec<String>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FuncDef {
    pub modifiers: Vec<String>,
    pub return_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Func {
    pub modifiers: Vec<String>,
    pub visibility: Option<Visibility>,
    pub return_type: Option<Type>,
    pub is_ref: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
    pub attributes: Vec<String>,
    pub body: Option<StatBlock>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct VirtProp {
    pub visibility: Option<Visibility>,
    pub prop_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub accessors: Vec<PropertyAccessor>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PropertyAccessor {
    pub kind: AccessorKind,
    pub is_const: bool,
    pub attributes: Vec<String>,
    pub body: Option<StatBlock>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum AccessorKind {
    Get,
    Set,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Interface {
    pub modifiers: Vec<String>,
    pub name: String,
    pub extends: Vec<String>,
    pub members: Vec<InterfaceMember>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum InterfaceMember {
    VirtProp(VirtProp),
    Method(IntfMthd),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct IntfMthd {
    pub return_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Mixin {
    pub class: Class,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Class {
    pub modifiers: Vec<String>,
    pub name: String,
    pub extends: Vec<String>,
    pub members: Vec<ClassMember>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ClassMember {
    VirtProp(VirtProp),
    Func(Func),
    Var(Var),
    FuncDef(FuncDef),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Var {
    pub visibility: Option<Visibility>,
    pub var_type: Type,
    pub declarations: Vec<VarDecl>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct VarDecl {
    pub name: String,
    pub initializer: Option<VarInit>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum VarInit {
    Expr(Expr),
    InitList(InitList),
    ArgList(Vec<Arg>),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Typedef {
    pub prim_type: String,
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Visibility {
    Private,
    Protected,
    Public,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Type {
    pub is_const: bool,
    pub scope: Scope,
    pub datatype: DataType,
    pub template_types: Vec<Type>,
    pub modifiers: Vec<TypeModifier>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum TypeModifier {
    Array,
    Handle,
    ConstHandle,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Scope {
    pub is_global: bool,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DataType {
    Identifier(String),
    PrimType(String),
    Question,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Param {
    pub param_type: Type,
    pub type_mod: Option<TypeMod>,
    pub name: Option<String>,
    pub default_value: Option<Expr>,
    pub is_variadic: bool,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum TypeMod {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct InitList {
    pub items: Vec<InitListItem>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum InitListItem {
    Expr(Expr),
    InitList(InitList),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct StatBlock {
    pub statements: Vec<Statement>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Statement {
    If(IfStmt),
    For(ForStmt),
    ForEach(ForEachStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Return(ReturnStmt),
    Break(Option<Span>),
    Continue(Option<Span>),
    Switch(SwitchStmt),
    Block(StatBlock),
    Expr(Option<Expr>),
    Var(Var),
    Using(Using),
    Try(TryStmt),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ForStmt {
    pub init: ForInit,
    pub condition: Option<Expr>,
    pub increment: Vec<Expr>,
    pub body: Box<Statement>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ForInit {
    Var(Var),
    Expr(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ForEachStmt {
    pub variables: Vec<(Type, String)>,
    pub iterable: Expr,
    pub body: Box<Statement>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Statement>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct DoWhileStmt {
    pub body: Box<Statement>,
    pub condition: Expr,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct SwitchStmt {
    pub value: Expr,
    pub cases: Vec<Case>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Case {
    pub pattern: CasePattern,
    pub statements: Vec<Statement>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CasePattern {
    Value(Expr),
    Default,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct TryStmt {
    pub try_block: StatBlock,
    pub catch_block: StatBlock,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Expr {
    Binary(Box<Expr>, BinaryOp, Box<Expr>, Option<Span>),
    Unary(UnaryOp, Box<Expr>, Option<Span>),
    Postfix(Box<Expr>, PostfixOp, Option<Span>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>, Option<Span>),
    Literal(Literal, Option<Span>),
    VarAccess(Scope, String, Option<Span>),
    FuncCall(FuncCall, Option<Span>),
    ConstructCall(Type, Vec<Arg>, Option<Span>),
    Cast(Type, Box<Expr>, Option<Span>),
    Lambda(Lambda, Option<Span>),
    InitList(InitList),
    Void(Option<Span>),
}

impl Expr {
    pub fn span(&self) -> Option<&Span> {
        match self {
            Expr::Binary(_, _, _, span) => span.as_ref(),
            Expr::Unary(_, _, span) => span.as_ref(),
            Expr::Postfix(_, _, span) => span.as_ref(),
            Expr::Ternary(_, _, _, span) => span.as_ref(),
            Expr::Literal(_, span) => span.as_ref(),
            Expr::VarAccess(_, _, span) => span.as_ref(),
            Expr::FuncCall(_, span) => span.as_ref(),
            Expr::ConstructCall(_, _, span) => span.as_ref(),
            Expr::Cast(_, _, span) => span.as_ref(),
            Expr::Lambda(_, span) => span.as_ref(),
            Expr::InitList(init_list) => init_list.span.as_ref(),
            Expr::Void(span) => span.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct FuncCall {
    pub scope: Scope,
    pub name: String,
    pub template_types: Vec<Type>,
    pub args: Vec<Arg>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Is,
    IsNot,
    And,
    Or,
    Xor,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
    PreInc,
    PreDec,
    BitNot,
    Handle,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum PostfixOp {
    PostInc,
    PostDec,
    MemberAccess(String),
    MemberCall(FuncCall),
    Index(Vec<IndexArg>),
    Call(Vec<Arg>),
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct IndexArg {
    pub name: Option<String>,
    pub value: Expr,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Lambda {
    pub params: Vec<LambdaParam>,
    pub body: StatBlock,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct LambdaParam {
    pub param_type: Option<Type>,
    pub type_mod: Option<TypeMod>,
    pub name: Option<String>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Literal {
    Number(String),
    String(String),
    Bits(String),
    Bool(bool),
    Null,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Number(n) => write!(f, "{}", n),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Bits(b) => write!(f, "{}", b),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::Null => write!(f, "null"),
        }
    }
}
