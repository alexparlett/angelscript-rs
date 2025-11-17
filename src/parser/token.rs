use crate::core::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Option<Span>,
    pub lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Option<Span>, lexeme: String) -> Self {
        Self { kind, span, lexeme }
    }

    pub fn eof() -> Self {
        Self {
            kind: TokenKind::Eof,
            span: None,
            lexeme: String::new(),
        }
    }

    pub fn line(&self) -> usize {
        self.span.as_ref().map(|s| s.start_line).unwrap_or(0)
    }

    pub fn lexeme(&self) -> &str {
        &self.lexeme
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(String),
    String(String),
    Bits(String),
    True,
    False,
    Null,

    Identifier(String),

    Class,
    Enum,
    Interface,
    Namespace,
    Typedef,
    Funcdef,
    Import,
    Mixin,
    Void,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float,
    Double,
    Bool,
    Auto,
    Const,
    Private,
    Protected,
    If,
    Else,
    For,
    ForEach,
    While,
    Do,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Return,
    Try,
    Catch,
    In,
    Out,
    InOut,
    Cast,

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
    Not,

    BitAnd,
    BitOr,
    BitXor,
    BitNot,
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

    Inc,
    Dec,
    At,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    Dot,
    Comma,
    Semicolon,
    Colon,
    Question,
    DoubleColon,

    Hash,

    Eof,
}

impl TokenKind {
    pub fn keyword(s: &str) -> Option<TokenKind> {
        Some(match s {
            "class" => TokenKind::Class,
            "enum" => TokenKind::Enum,
            "interface" => TokenKind::Interface,
            "namespace" => TokenKind::Namespace,
            "typedef" => TokenKind::Typedef,
            "funcdef" => TokenKind::Funcdef,
            "import" => TokenKind::Import,
            "mixin" => TokenKind::Mixin,
            "void" => TokenKind::Void,
            "int" => TokenKind::Int,
            "int8" => TokenKind::Int8,
            "int16" => TokenKind::Int16,
            "int32" => TokenKind::Int32,
            "int64" => TokenKind::Int64,
            "uint" => TokenKind::Uint,
            "uint8" => TokenKind::Uint8,
            "uint16" => TokenKind::Uint16,
            "uint32" => TokenKind::Uint32,
            "uint64" => TokenKind::Uint64,
            "float" => TokenKind::Float,
            "double" => TokenKind::Double,
            "bool" => TokenKind::Bool,
            "auto" => TokenKind::Auto,
            "const" => TokenKind::Const,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "foreach" => TokenKind::ForEach,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "in" => TokenKind::In,
            "out" => TokenKind::Out,
            "inout" => TokenKind::InOut,
            "cast" => TokenKind::Cast,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "xor" => TokenKind::Xor,
            "not" => TokenKind::Not,
            "is" => TokenKind::Is,
            _ => return None,
        })
    }

    pub fn is_contextual_keyword(s: &str) -> bool {
        matches!(
            s,
            "this"
                | "super"
                | "from"
                | "shared"
                | "final"
                | "override"
                | "get"
                | "set"
                | "abstract"
                | "function"
                | "external"
                | "explicit"
                | "property"
                | "delete"
        )
    }
}
