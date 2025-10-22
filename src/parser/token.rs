use crate::parser::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(String),
    String(String),
    Bits(String),
    True,
    False,
    Null,

    // Identifiers
    Identifier(String),

    // Keywords - ONLY actual reserved keywords from as_tokendef.h
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

    // Operators - Binary
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Mod, // %
    Pow, // **

    // Comparison
    Eq,    // ==
    Ne,    // !=
    Lt,    // <
    Le,    // <=
    Gt,    // >
    Ge,    // >=
    Is,    // is
    IsNot, // !is

    // Logical
    And, // && or 'and'
    Or,  // || or 'or'
    Xor, // ^^ or 'xor'
    Not, // ! or 'not'

    // Bitwise
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    BitNot, // ~
    Shl,    // <<
    Shr,    // >>
    UShr,   // >>>

    // Assignment
    Assign,       // =
    AddAssign,    // +=
    SubAssign,    // -=
    MulAssign,    // *=
    DivAssign,    // /=
    ModAssign,    // %=
    PowAssign,    // **=
    BitAndAssign, // &=
    BitOrAssign,  // |=
    BitXorAssign, // ^=
    ShlAssign,    // <<=
    ShrAssign,    // >>=
    UShrAssign,   // >>>=

    // Unary
    Inc, // ++
    Dec, // --
    At,  // @

    // Delimiters
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // Other
    Dot,         // .
    Comma,       // ,
    Semicolon,   // ;
    Colon,       // :
    Question,    // ?
    DoubleColon, // ::

    // Preprocessor
    Hash, // #

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

    /// Check if an identifier string is a contextual keyword
    /// These are NOT tokens but are checked during parsing
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
                | "if_handle_then_const"
        )
    }
}
