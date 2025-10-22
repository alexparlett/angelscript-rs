use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Script {
    pub items: Vec<ScriptNode>,
}

#[derive(Debug, Clone, PartialEq)]
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

    // Simplified preprocessor directives
    Include(Include),
    Pragma(Pragma),
    ConditionalBlock(ConditionalBlock),
    CustomDirective(CustomDirective),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Include {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pragma {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomDirective {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBlock {
    pub if_branch: ConditionalBranch,
    pub elif_branches: Vec<ConditionalBranch>,
    pub else_branch: Option<Vec<ScriptNode>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalBranch {
    pub condition: String,
    pub items: Vec<ScriptNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub type_name: Type,
    pub is_ref: bool,
    pub identifier: String,
    pub params: Vec<Param>,
    pub from: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Using {
    pub namespace: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Namespace {
    pub name: Vec<String>,
    pub items: Vec<ScriptNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub modifiers: Vec<String>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDef {
    pub modifiers: Vec<String>,
    pub return_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq)]
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtProp {
    pub visibility: Option<Visibility>,
    pub prop_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub accessors: Vec<PropertyAccessor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAccessor {
    pub kind: AccessorKind,
    pub is_const: bool,
    pub attributes: Vec<String>,
    pub body: Option<StatBlock>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessorKind {
    Get,
    Set,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interface {
    pub modifiers: Vec<String>,
    pub name: String,
    pub extends: Vec<String>,
    pub members: Vec<InterfaceMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceMember {
    VirtProp(VirtProp),
    Method(IntfMthd),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntfMthd {
    pub return_type: Type,
    pub is_ref: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub is_const: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mixin {
    pub class: Class,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub modifiers: Vec<String>,
    pub name: String,
    pub extends: Vec<String>,
    pub members: Vec<ClassMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    VirtProp(VirtProp),
    Func(Func),
    Var(Var),
    FuncDef(FuncDef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub visibility: Option<Visibility>,
    pub var_type: Type,
    pub declarations: Vec<VarDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub initializer: Option<VarInit>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarInit {
    Expr(Expr),
    InitList(InitList),
    ArgList(Vec<Arg>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typedef {
    pub prim_type: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Private,
    Protected,
    Public,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub is_const: bool,
    pub scope: Scope,
    pub datatype: DataType,
    pub template_types: Vec<Type>,
    pub modifiers: Vec<TypeModifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeModifier {
    Array,
    Handle,
    ConstHandle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    pub is_global: bool,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Identifier(String),
    PrimType(String),
    Question,
    Auto,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub param_type: Type,
    pub type_mod: Option<TypeMod>,
    pub name: Option<String>,
    pub default_value: Option<Expr>,
    pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeMod {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitList {
    pub items: Vec<InitListItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InitListItem {
    Expr(Expr),
    InitList(InitList),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatBlock {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    If(IfStmt),
    For(ForStmt),
    ForEach(ForEachStmt),
    While(WhileStmt),
    DoWhile(DoWhileStmt),
    Return(ReturnStmt),
    Break,
    Continue,
    Switch(SwitchStmt),
    Block(StatBlock),
    Expr(Option<Expr>),
    Var(Var),
    Using(Using),
    Try(TryStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Box<Statement>,
    pub else_branch: Option<Box<Statement>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub init: ForInit,
    pub condition: Option<Expr>,
    pub increment: Vec<Expr>,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    Var(Var),
    Expr(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForEachStmt {
    pub variables: Vec<(Type, String)>,
    pub iterable: Expr,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Box<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoWhileStmt {
    pub body: Box<Statement>,
    pub condition: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchStmt {
    pub value: Expr,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub pattern: CasePattern,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CasePattern {
    Value(Expr),
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TryStmt {
    pub try_block: StatBlock,
    pub catch_block: StatBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Postfix(Box<Expr>, PostfixOp),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Literal(Literal),
    VarAccess(Scope, String),
    FuncCall(FuncCall),
    ConstructCall(Type, Vec<Arg>),
    Cast(Type, Box<Expr>),
    Lambda(Lambda),
    InitList(InitList),
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncCall {
    pub scope: Scope,
    pub name: String,
    pub template_types: Vec<Type>,
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Not,
    PreInc,
    PreDec,
    BitNot,
    Handle,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PostfixOp {
    PostInc,
    PostDec,
    MemberAccess(String),
    MemberCall(FuncCall),
    Index(Vec<IndexArg>),
    Call(Vec<Arg>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lambda {
    pub params: Vec<LambdaParam>,
    pub body: StatBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub param_type: Option<Type>,
    pub type_mod: Option<TypeMod>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
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
