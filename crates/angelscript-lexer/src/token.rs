use std::fmt;

/// Byte offset span in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Start byte offset (inclusive).
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one covering both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A single token produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    /// Extract the source text for this token from the original source.
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.span.start..self.span.end]
    }
}

/// All token types recognized by the AngelScript lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ── Special ──────────────────────────────────────────────────────
    /// End of file.
    Eof,
    /// Unrecognized character.
    Error,

    // ── Whitespace & comments ────────────────────────────────────────
    /// Whitespace (space, tab, CR, LF, BOM).
    WhiteSpace,
    /// Single-line comment `// ...`.
    LineComment,
    /// Multi-line comment `/* ... */`.
    BlockComment,

    // ── Literals ─────────────────────────────────────────────────────
    /// Identifier `abc123`.
    Identifier,
    /// Integer constant `1234`.
    IntConstant,
    /// Float constant `12.34f`.
    FloatConstant,
    /// Double constant `12.34`.
    DoubleConstant,
    /// String constant `"..."`.
    StringConstant,
    /// Multiline string constant.
    MultilineStringConstant,
    /// Heredoc string `"""..."""`.
    HeredocStringConstant,
    /// Unterminated string.
    NonTerminatedStringConstant,
    /// Hex/octal/binary bits constant `0xFFFF`.
    BitsConstant,

    // ── Arithmetic operators ─────────────────────────────────────────
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `**`
    StarStar,

    // ── Handle ───────────────────────────────────────────────────────
    /// `@`
    At,

    // ── Assignment operators ─────────────────────────────────────────
    /// `=`
    Assign,
    /// `+=`
    AddAssign,
    /// `-=`
    SubAssign,
    /// `*=`
    MulAssign,
    /// `/=`
    DivAssign,
    /// `%=`
    ModAssign,
    /// `**=`
    PowAssign,
    /// `|=`
    OrAssign,
    /// `&=`
    AndAssign,
    /// `^=`
    XorAssign,
    /// `<<=`
    ShiftLeftAssign,
    /// `>>=`
    ShiftRightLAssign,
    /// `>>>=`
    ShiftRightAAssign,

    // ── Increment/Decrement ──────────────────────────────────────────
    /// `++`
    Inc,
    /// `--`
    Dec,

    // ── Punctuation ──────────────────────────────────────────────────
    /// `.`
    Dot,
    /// `::`
    Scope,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `?`
    Question,
    /// `:`
    Colon,

    // ── Bitwise operators ────────────────────────────────────────────
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `~`
    Tilde,
    /// `^`
    Caret,
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    ShiftRightArith,

    // ── Comparison operators ─────────────────────────────────────────
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    Less,
    /// `>`
    Greater,
    /// `<=`
    LessEqual,
    /// `>=`
    GreaterEqual,

    // ── Logical operators ────────────────────────────────────────────
    /// `!` or `not`
    Not,
    /// `&&` or `and`
    And,
    /// `||` or `or`
    Or,
    /// `^^` or `xor`
    Xor,

    // ── Identity operators ───────────────────────────────────────────
    /// `is`
    Is,
    /// `!is`
    NotIs,

    // ── Keywords ─────────────────────────────────────────────────────
    /// `if`
    If,
    /// `else`
    Else,
    /// `for`
    For,
    /// `while`
    While,
    /// `do`
    Do,
    /// `switch`
    Switch,
    /// `case`
    Case,
    /// `default`
    Default,
    /// `break`
    Break,
    /// `continue`
    Continue,
    /// `return`
    Return,
    /// `try`
    Try,
    /// `catch`
    Catch,

    // ── Type keywords ────────────────────────────────────────────────
    /// `void`
    Void,
    /// `bool`
    Bool,
    /// `int` or `int32`
    Int,
    /// `int8`
    Int8,
    /// `int16`
    Int16,
    /// `int64`
    Int64,
    /// `uint` or `uint32`
    UInt,
    /// `uint8`
    UInt8,
    /// `uint16`
    UInt16,
    /// `uint64`
    UInt64,
    /// `float`
    Float,
    /// `double`
    Double,
    /// `auto`
    Auto,

    // ── Constant keywords ────────────────────────────────────────────
    /// `true`
    True,
    /// `false`
    False,
    /// `null`
    Null,

    // ── Declaration keywords ─────────────────────────────────────────
    /// `class`
    Class,
    /// `interface`
    Interface,
    /// `enum`
    Enum,
    /// `funcdef`
    Funcdef,
    /// `typedef`
    Typedef,
    /// `namespace`
    Namespace,
    /// `mixin`
    Mixin,
    /// `import`
    Import,
    /// `cast`
    Cast,

    // ── Modifier keywords ────────────────────────────────────────────
    /// `const`
    Const,
    /// `in`
    In,
    /// `out`
    Out,
    /// `inout`
    InOut,
    /// `private`
    Private,
    /// `protected`
    Protected,
}

impl TokenKind {
    /// Whether this is a whitespace or comment token.
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenKind::WhiteSpace | TokenKind::LineComment | TokenKind::BlockComment
        )
    }

    /// Whether this is a keyword token.
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            TokenKind::If
                | TokenKind::Else
                | TokenKind::For
                | TokenKind::While
                | TokenKind::Do
                | TokenKind::Switch
                | TokenKind::Case
                | TokenKind::Default
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::Try
                | TokenKind::Catch
                | TokenKind::Void
                | TokenKind::Bool
                | TokenKind::Int
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int64
                | TokenKind::UInt
                | TokenKind::UInt8
                | TokenKind::UInt16
                | TokenKind::UInt64
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Auto
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Enum
                | TokenKind::Funcdef
                | TokenKind::Typedef
                | TokenKind::Namespace
                | TokenKind::Mixin
                | TokenKind::Import
                | TokenKind::Cast
                | TokenKind::Const
                | TokenKind::In
                | TokenKind::Out
                | TokenKind::InOut
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Xor
                | TokenKind::Not
                | TokenKind::Is
                | TokenKind::NotIs
        )
    }

    /// Whether this is a type keyword (can start a type expression).
    pub fn is_type_keyword(self) -> bool {
        matches!(
            self,
            TokenKind::Void
                | TokenKind::Bool
                | TokenKind::Int
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int64
                | TokenKind::UInt
                | TokenKind::UInt8
                | TokenKind::UInt16
                | TokenKind::UInt64
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Auto
        )
    }

    /// Whether this is a literal constant.
    pub fn is_literal(self) -> bool {
        matches!(
            self,
            TokenKind::IntConstant
                | TokenKind::FloatConstant
                | TokenKind::DoubleConstant
                | TokenKind::StringConstant
                | TokenKind::MultilineStringConstant
                | TokenKind::HeredocStringConstant
                | TokenKind::BitsConstant
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
        )
    }

    /// Whether this is an assignment operator.
    pub fn is_assignment_op(self) -> bool {
        matches!(
            self,
            TokenKind::Assign
                | TokenKind::AddAssign
                | TokenKind::SubAssign
                | TokenKind::MulAssign
                | TokenKind::DivAssign
                | TokenKind::ModAssign
                | TokenKind::PowAssign
                | TokenKind::OrAssign
                | TokenKind::AndAssign
                | TokenKind::XorAssign
                | TokenKind::ShiftLeftAssign
                | TokenKind::ShiftRightLAssign
                | TokenKind::ShiftRightAAssign
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Eof => write!(f, "<eof>"),
            TokenKind::Error => write!(f, "<error>"),
            TokenKind::WhiteSpace => write!(f, "<whitespace>"),
            TokenKind::LineComment => write!(f, "<line-comment>"),
            TokenKind::BlockComment => write!(f, "<block-comment>"),
            TokenKind::Identifier => write!(f, "<identifier>"),
            TokenKind::IntConstant => write!(f, "<int>"),
            TokenKind::FloatConstant => write!(f, "<float>"),
            TokenKind::DoubleConstant => write!(f, "<double>"),
            TokenKind::StringConstant => write!(f, "<string>"),
            TokenKind::MultilineStringConstant => write!(f, "<multiline-string>"),
            TokenKind::HeredocStringConstant => write!(f, "<heredoc>"),
            TokenKind::NonTerminatedStringConstant => write!(f, "<unterminated-string>"),
            TokenKind::BitsConstant => write!(f, "<bits>"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::StarStar => write!(f, "**"),
            TokenKind::At => write!(f, "@"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::AddAssign => write!(f, "+="),
            TokenKind::SubAssign => write!(f, "-="),
            TokenKind::MulAssign => write!(f, "*="),
            TokenKind::DivAssign => write!(f, "/="),
            TokenKind::ModAssign => write!(f, "%="),
            TokenKind::PowAssign => write!(f, "**="),
            TokenKind::OrAssign => write!(f, "|="),
            TokenKind::AndAssign => write!(f, "&="),
            TokenKind::XorAssign => write!(f, "^="),
            TokenKind::ShiftLeftAssign => write!(f, "<<="),
            TokenKind::ShiftRightLAssign => write!(f, ">>="),
            TokenKind::ShiftRightAAssign => write!(f, ">>>="),
            TokenKind::Inc => write!(f, "++"),
            TokenKind::Dec => write!(f, "--"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Scope => write!(f, "::"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::OpenBrace => write!(f, "{{"),
            TokenKind::CloseBrace => write!(f, "}}"),
            TokenKind::OpenParen => write!(f, "("),
            TokenKind::CloseParen => write!(f, ")"),
            TokenKind::OpenBracket => write!(f, "["),
            TokenKind::CloseBracket => write!(f, "]"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Amp => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::ShiftLeft => write!(f, "<<"),
            TokenKind::ShiftRight => write!(f, ">>"),
            TokenKind::ShiftRightArith => write!(f, ">>>"),
            TokenKind::Equal => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Not => write!(f, "!"),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Xor => write!(f, "^^"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::NotIs => write!(f, "!is"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Switch => write!(f, "switch"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Default => write!(f, "default"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::Catch => write!(f, "catch"),
            TokenKind::Void => write!(f, "void"),
            TokenKind::Bool => write!(f, "bool"),
            TokenKind::Int => write!(f, "int"),
            TokenKind::Int8 => write!(f, "int8"),
            TokenKind::Int16 => write!(f, "int16"),
            TokenKind::Int64 => write!(f, "int64"),
            TokenKind::UInt => write!(f, "uint"),
            TokenKind::UInt8 => write!(f, "uint8"),
            TokenKind::UInt16 => write!(f, "uint16"),
            TokenKind::UInt64 => write!(f, "uint64"),
            TokenKind::Float => write!(f, "float"),
            TokenKind::Double => write!(f, "double"),
            TokenKind::Auto => write!(f, "auto"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Class => write!(f, "class"),
            TokenKind::Interface => write!(f, "interface"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Funcdef => write!(f, "funcdef"),
            TokenKind::Typedef => write!(f, "typedef"),
            TokenKind::Namespace => write!(f, "namespace"),
            TokenKind::Mixin => write!(f, "mixin"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Cast => write!(f, "cast"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Out => write!(f, "out"),
            TokenKind::InOut => write!(f, "inout"),
            TokenKind::Private => write!(f, "private"),
            TokenKind::Protected => write!(f, "protected"),
        }
    }
}

/// Lookup keyword from identifier string.
pub fn keyword_lookup(word: &str) -> Option<TokenKind> {
    match word {
        "and" => Some(TokenKind::And),
        "auto" => Some(TokenKind::Auto),
        "bool" => Some(TokenKind::Bool),
        "break" => Some(TokenKind::Break),
        "case" => Some(TokenKind::Case),
        "cast" => Some(TokenKind::Cast),
        "catch" => Some(TokenKind::Catch),
        "class" => Some(TokenKind::Class),
        "const" => Some(TokenKind::Const),
        "continue" => Some(TokenKind::Continue),
        "default" => Some(TokenKind::Default),
        "do" => Some(TokenKind::Do),
        "double" => Some(TokenKind::Double),
        "else" => Some(TokenKind::Else),
        "enum" => Some(TokenKind::Enum),
        "false" => Some(TokenKind::False),
        "float" => Some(TokenKind::Float),
        "for" => Some(TokenKind::For),
        "funcdef" => Some(TokenKind::Funcdef),
        "if" => Some(TokenKind::If),
        "import" => Some(TokenKind::Import),
        "in" => Some(TokenKind::In),
        "inout" => Some(TokenKind::InOut),
        "int" | "int32" => Some(TokenKind::Int),
        "int8" => Some(TokenKind::Int8),
        "int16" => Some(TokenKind::Int16),
        "int64" => Some(TokenKind::Int64),
        "interface" => Some(TokenKind::Interface),
        "is" => Some(TokenKind::Is),
        "mixin" => Some(TokenKind::Mixin),
        "namespace" => Some(TokenKind::Namespace),
        "not" => Some(TokenKind::Not),
        "null" => Some(TokenKind::Null),
        "or" => Some(TokenKind::Or),
        "out" => Some(TokenKind::Out),
        "private" => Some(TokenKind::Private),
        "protected" => Some(TokenKind::Protected),
        "return" => Some(TokenKind::Return),
        "switch" => Some(TokenKind::Switch),
        "true" => Some(TokenKind::True),
        "try" => Some(TokenKind::Try),
        "typedef" => Some(TokenKind::Typedef),
        "uint" | "uint32" => Some(TokenKind::UInt),
        "uint8" => Some(TokenKind::UInt8),
        "uint16" => Some(TokenKind::UInt16),
        "uint64" => Some(TokenKind::UInt64),
        "void" => Some(TokenKind::Void),
        "while" => Some(TokenKind::While),
        "xor" => Some(TokenKind::Xor),
        _ => None,
    }
}

/// Context-sensitive keywords — treated as identifiers unless in specific contexts.
pub const CONTEXT_KEYWORDS: &[&str] = &[
    "this",
    "super",
    "from",
    "shared",
    "final",
    "override",
    "get",
    "set",
    "abstract",
    "function",
    "if_handle_then_const",
    "external",
    "explicit",
    "property",
    "delete",
    "using",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup_basic() {
        assert_eq!(keyword_lookup("if"), Some(TokenKind::If));
        assert_eq!(keyword_lookup("class"), Some(TokenKind::Class));
        assert_eq!(keyword_lookup("int32"), Some(TokenKind::Int));
        assert_eq!(keyword_lookup("uint32"), Some(TokenKind::UInt));
        assert_eq!(keyword_lookup("notakeyword"), None);
        assert_eq!(keyword_lookup("Player"), None);
    }

    #[test]
    fn keyword_lookup_aliases() {
        // "and", "or", "xor", "not" are keyword forms of logical operators
        assert_eq!(keyword_lookup("and"), Some(TokenKind::And));
        assert_eq!(keyword_lookup("or"), Some(TokenKind::Or));
        assert_eq!(keyword_lookup("xor"), Some(TokenKind::Xor));
        assert_eq!(keyword_lookup("not"), Some(TokenKind::Not));
    }

    #[test]
    fn token_kind_categories() {
        assert!(TokenKind::If.is_keyword());
        assert!(TokenKind::Int.is_type_keyword());
        assert!(TokenKind::Assign.is_assignment_op());
        assert!(TokenKind::WhiteSpace.is_trivia());
        assert!(TokenKind::IntConstant.is_literal());
        assert!(!TokenKind::Identifier.is_keyword());
    }

    #[test]
    fn span_merge() {
        let a = Span::new(5, 10);
        let b = Span::new(8, 15);
        let merged = a.merge(b);
        assert_eq!(merged, Span::new(5, 15));
    }
}
