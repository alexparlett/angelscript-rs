//! Token types and definitions for the AngelScript lexer.
//!
//! Based on token definitions from the C++ AngelScript implementation
//! (`as_tokendef.h`).

use super::span::Span;
use std::fmt;

/// A token from the source code.
#[derive(Clone, Copy, PartialEq)]
pub struct Token<'src> {
    /// The type of token.
    pub kind: TokenKind,
    /// The source text of this token.
    pub lexeme: &'src str,
    /// Location in source.
    pub span: Span,
}

impl<'src> Token<'src> {
    /// Create a new token.
    #[inline]
    pub fn new(kind: TokenKind, lexeme: &'src str, span: Span) -> Self {
        Self { kind, lexeme, span }
    }
}

impl fmt::Debug for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}({:?} @ {:?})", self.kind, self.lexeme, self.span)
    }
}

/// All possible token types in AngelScript.
///
/// Organized by category for clarity. Matches the token types from
/// the C++ implementation (`as_tokendef.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // =========================================
    // Literals
    // =========================================
    /// Integer literal: `42`, `1234`
    IntLiteral,
    /// Float literal: `3.14f`, `1.0F`
    FloatLiteral,
    /// Double literal: `3.14`, `1.0e10`
    DoubleLiteral,
    /// String literal: `"hello"`, `'a'`
    StringLiteral,
    /// Heredoc string: `"""multi\nline"""`
    HeredocLiteral,
    /// Bits literal: `0xFF`, `0b1010`, `0o77`, `0d99`
    BitsLiteral,

    // =========================================
    // Identifiers
    // =========================================
    /// User-defined identifier
    Identifier,

    // =========================================
    // Keywords - Types
    // =========================================
    /// `void`
    Void,
    /// `bool`
    Bool,
    /// `int` (alias for int32)
    Int,
    /// `int8`
    Int8,
    /// `int16`
    Int16,
    /// `int64`
    Int64,
    /// `uint` (alias for uint32)
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

    // =========================================
    // Keywords - Values
    // =========================================
    /// `true`
    True,
    /// `false`
    False,
    /// `null`
    Null,

    // =========================================
    // Keywords - Control Flow
    // =========================================
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

    // =========================================
    // Keywords - Declarations
    // =========================================
    /// `class`
    Class,
    /// `interface`
    Interface,
    /// `enum`
    Enum,
    /// `funcdef`
    FuncDef,
    /// `namespace`
    Namespace,
    /// `mixin`
    Mixin,
    /// `typedef`
    Typedef,
    /// `import`
    Import,
    /// `const`
    Const,
    /// `private`
    Private,
    /// `protected`
    Protected,

    // =========================================
    // Keywords - Operators (word form)
    // =========================================
    /// `and` (same as `&&`)
    And,
    /// `or` (same as `||`)
    Or,
    /// `xor` (same as `^^`)
    Xor,
    /// `not` (same as `!`)
    Not,
    /// `is`
    Is,
    /// `!is`
    NotIs,
    /// `in`
    In,
    /// `out`
    Out,
    /// `inout`
    InOut,
    /// `cast`
    Cast,
    /// `super` (for calling base class constructor)
    Super,
    /// `this` (reference to current object in methods)
    This,

    // =========================================
    // Operators - Arithmetic
    // =========================================
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

    // =========================================
    // Operators - Compound Assignment
    // =========================================
    /// `=`
    Equal,
    /// `+=`
    PlusEqual,
    /// `-=`
    MinusEqual,
    /// `*=`
    StarEqual,
    /// `/=`
    SlashEqual,
    /// `%=`
    PercentEqual,
    /// `**=`
    StarStarEqual,

    // =========================================
    // Operators - Bitwise
    // =========================================
    /// `&`
    Amp,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `~`
    Tilde,
    /// `<<`
    LessLess,
    /// `>>`
    GreaterGreater,
    /// `>>>`
    GreaterGreaterGreater,

    // =========================================
    // Operators - Bitwise Assignment
    // =========================================
    /// `&=`
    AmpEqual,
    /// `|=`
    PipeEqual,
    /// `^=`
    CaretEqual,
    /// `<<=`
    LessLessEqual,
    /// `>>=`
    GreaterGreaterEqual,
    /// `>>>=`
    GreaterGreaterGreaterEqual,

    // =========================================
    // Operators - Comparison
    // =========================================
    /// `==`
    EqualEqual,
    /// `!=`
    BangEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,

    // =========================================
    // Operators - Logical (symbolic)
    // =========================================
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `^^`
    CaretCaret,
    /// `!`
    Bang,

    // =========================================
    // Operators - Increment/Decrement
    // =========================================
    /// `++`
    PlusPlus,
    /// `--`
    MinusMinus,

    // =========================================
    // Operators - Other
    // =========================================
    /// `?`
    Question,
    /// `:`
    Colon,
    /// `::`
    ColonColon,
    /// `.`
    Dot,
    /// `@`
    At,

    // =========================================
    // Delimiters
    // =========================================
    /// `(`
    LeftParen,
    /// `)`
    RightParen,
    /// `[`
    LeftBracket,
    /// `]`
    RightBracket,
    /// `{`
    LeftBrace,
    /// `}`
    RightBrace,
    /// `;`
    Semicolon,
    /// `,`
    Comma,

    // =========================================
    // Special
    // =========================================
    /// End of file
    Eof,
    /// Lexer error (unrecognized input)
    Error,
}

impl TokenKind {
    /// Check if this token kind is a keyword.
    pub fn is_keyword(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Void | Bool
                | Int
                | Int8
                | Int16
                | Int64
                | UInt
                | UInt8
                | UInt16
                | UInt64
                | Float
                | Double
                | Auto
                | True
                | False
                | Null
                | If
                | Else
                | For
                | While
                | Do
                | Switch
                | Case
                | Default
                | Break
                | Continue
                | Return
                | Try
                | Catch
                | Class
                | Interface
                | Enum
                | FuncDef
                | Namespace
                | Mixin
                | Typedef
                | Import
                | Const
                | Private
                | Protected
                | And
                | Or
                | Xor
                | Not
                | Is
                | NotIs
                | In
                | Out
                | InOut
                | Cast
        )
    }

    /// Check if this token kind is a literal.
    pub fn is_literal(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            IntLiteral
                | FloatLiteral
                | DoubleLiteral
                | StringLiteral
                | HeredocLiteral
                | BitsLiteral
                | True
                | False
                | Null
        )
    }

    /// Check if this token kind is an operator.
    pub fn is_operator(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Plus | Minus
                | Star
                | Slash
                | Percent
                | StarStar
                | Equal
                | PlusEqual
                | MinusEqual
                | StarEqual
                | SlashEqual
                | PercentEqual
                | StarStarEqual
                | Amp
                | Pipe
                | Caret
                | Tilde
                | LessLess
                | GreaterGreater
                | GreaterGreaterGreater
                | AmpEqual
                | PipeEqual
                | CaretEqual
                | LessLessEqual
                | GreaterGreaterEqual
                | GreaterGreaterGreaterEqual
                | EqualEqual
                | BangEqual
                | Less
                | LessEqual
                | Greater
                | GreaterEqual
                | AmpAmp
                | PipePipe
                | CaretCaret
                | Bang
                | PlusPlus
                | MinusMinus
                | Question
                | Colon
                | ColonColon
                | Dot
                | At
                | And
                | Or
                | Xor
                | Not
                | Is
                | NotIs
        )
    }

    /// Check if this token kind is a delimiter.
    pub fn is_delimiter(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            LeftParen
                | RightParen
                | LeftBracket
                | RightBracket
                | LeftBrace
                | RightBrace
                | Semicolon
                | Comma
        )
    }

    /// Get the string representation of this token kind for error messages.
    pub fn description(self) -> &'static str {
        use TokenKind::*;
        match self {
            IntLiteral => "integer literal",
            FloatLiteral => "float literal",
            DoubleLiteral => "double literal",
            StringLiteral => "string literal",
            HeredocLiteral => "heredoc string",
            BitsLiteral => "bits literal",
            Identifier => "identifier",
            Void => "'void'",
            Bool => "'bool'",
            Int => "'int'",
            Int8 => "'int8'",
            Int16 => "'int16'",
            Int64 => "'int64'",
            UInt => "'uint'",
            UInt8 => "'uint8'",
            UInt16 => "'uint16'",
            UInt64 => "'uint64'",
            Float => "'float'",
            Double => "'double'",
            Auto => "'auto'",
            True => "'true'",
            False => "'false'",
            Null => "'null'",
            If => "'if'",
            Else => "'else'",
            For => "'for'",
            While => "'while'",
            Do => "'do'",
            Switch => "'switch'",
            Case => "'case'",
            Default => "'default'",
            Break => "'break'",
            Continue => "'continue'",
            Return => "'return'",
            Try => "'try'",
            Catch => "'catch'",
            Class => "'class'",
            Interface => "'interface'",
            Enum => "'enum'",
            FuncDef => "'funcdef'",
            Namespace => "'namespace'",
            Mixin => "'mixin'",
            Typedef => "'typedef'",
            Import => "'import'",
            Const => "'const'",
            Private => "'private'",
            Protected => "'protected'",
            And => "'and'",
            Or => "'or'",
            Xor => "'xor'",
            Not => "'not'",
            Is => "'is'",
            NotIs => "'!is'",
            In => "'in'",
            Out => "'out'",
            InOut => "'inout'",
            Cast => "'cast'",
            Super => "'super'",
            This => "'this'",
            Plus => "'+'",
            Minus => "'-'",
            Star => "'*'",
            Slash => "'/'",
            Percent => "'%'",
            StarStar => "'**'",
            Equal => "'='",
            PlusEqual => "'+='",
            MinusEqual => "'-='",
            StarEqual => "'*='",
            SlashEqual => "'/='",
            PercentEqual => "'%='",
            StarStarEqual => "'**='",
            Amp => "'&'",
            Pipe => "'|'",
            Caret => "'^'",
            Tilde => "'~'",
            LessLess => "'<<'",
            GreaterGreater => "'>>'",
            GreaterGreaterGreater => "'>>>'",
            AmpEqual => "'&='",
            PipeEqual => "'|='",
            CaretEqual => "'^='",
            LessLessEqual => "'<<='",
            GreaterGreaterEqual => "'>>='",
            GreaterGreaterGreaterEqual => "'>>>='",
            EqualEqual => "'=='",
            BangEqual => "'!='",
            Less => "'<'",
            LessEqual => "'<='",
            Greater => "'>'",
            GreaterEqual => "'>='",
            AmpAmp => "'&&'",
            PipePipe => "'||'",
            CaretCaret => "'^^'",
            Bang => "'!'",
            PlusPlus => "'++'",
            MinusMinus => "'--'",
            Question => "'?'",
            Colon => "':'",
            ColonColon => "'::'",
            Dot => "'.'",
            At => "'@'",
            LeftParen => "'('",
            RightParen => "')'",
            LeftBracket => "'['",
            RightBracket => "']'",
            LeftBrace => "'{'",
            RightBrace => "'}'",
            Semicolon => "';'",
            Comma => "','",
            Eof => "end of file",
            Error => "error",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Map a keyword string to its [`TokenKind`], or `None` if not a keyword.
///
/// This handles all AngelScript keywords including type aliases
/// (`int32` → `Int`, `uint32` → `UInt`).
pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match ident {
        // Types
        "void" => Void,
        "bool" => Bool,
        "int" | "int32" => Int,
        "int8" => Int8,
        "int16" => Int16,
        "int64" => Int64,
        "uint" | "uint32" => UInt,
        "uint8" => UInt8,
        "uint16" => UInt16,
        "uint64" => UInt64,
        "float" => Float,
        "double" => Double,
        "auto" => Auto,

        // Values
        "true" => True,
        "false" => False,
        "null" => Null,

        // Control flow
        "if" => If,
        "else" => Else,
        "for" => For,
        "while" => While,
        "do" => Do,
        "switch" => Switch,
        "case" => Case,
        "default" => Default,
        "break" => Break,
        "continue" => Continue,
        "return" => Return,
        "try" => Try,
        "catch" => Catch,

        // Declarations
        "class" => Class,
        "interface" => Interface,
        "enum" => Enum,
        "funcdef" => FuncDef,
        "namespace" => Namespace,
        "mixin" => Mixin,
        "typedef" => Typedef,
        "import" => Import,
        "const" => Const,
        "private" => Private,
        "protected" => Protected,

        // Word operators
        "and" => And,
        "or" => Or,
        "xor" => Xor,
        "not" => Not,
        "is" => Is,
        "in" => In,
        "out" => Out,
        "inout" => InOut,
        "cast" => Cast,
        "super" => Super,
        "this" => This,

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup() {
        assert_eq!(lookup_keyword("if"), Some(TokenKind::If));
        assert_eq!(lookup_keyword("int32"), Some(TokenKind::Int));
        assert_eq!(lookup_keyword("uint32"), Some(TokenKind::UInt));
        assert_eq!(lookup_keyword("notakeyword"), None);
    }

    #[test]
    fn token_categories() {
        assert!(TokenKind::If.is_keyword());
        assert!(TokenKind::IntLiteral.is_literal());
        assert!(TokenKind::Plus.is_operator());
        assert!(TokenKind::LeftParen.is_delimiter());
    }

    #[test]
    fn token_new() {
        let token = Token::new(TokenKind::Identifier, "foo", Span::new(1, 5, 3));
        assert_eq!(token.kind, TokenKind::Identifier);
        assert_eq!(token.lexeme, "foo");
        assert_eq!(token.span, Span::new(1, 5, 3));
    }

    #[test]
    fn token_debug_format() {
        let token = Token::new(TokenKind::Identifier, "foo", Span::new(1, 5, 3));
        let debug = format!("{:?}", token);
        assert!(debug.contains("Identifier"));
        assert!(debug.contains("foo"));
        assert!(debug.contains("1:5"));
    }

    #[test]
    fn token_kind_description_literals() {
        assert_eq!(TokenKind::IntLiteral.description(), "integer literal");
        assert_eq!(TokenKind::FloatLiteral.description(), "float literal");
        assert_eq!(TokenKind::DoubleLiteral.description(), "double literal");
        assert_eq!(TokenKind::StringLiteral.description(), "string literal");
        assert_eq!(TokenKind::HeredocLiteral.description(), "heredoc string");
        assert_eq!(TokenKind::BitsLiteral.description(), "bits literal");
    }

    #[test]
    fn token_kind_description_keywords() {
        assert_eq!(TokenKind::If.description(), "'if'");
        assert_eq!(TokenKind::Else.description(), "'else'");
        assert_eq!(TokenKind::For.description(), "'for'");
        assert_eq!(TokenKind::While.description(), "'while'");
        assert_eq!(TokenKind::Return.description(), "'return'");
        assert_eq!(TokenKind::Class.description(), "'class'");
    }

    #[test]
    fn token_kind_description_types() {
        assert_eq!(TokenKind::Void.description(), "'void'");
        assert_eq!(TokenKind::Bool.description(), "'bool'");
        assert_eq!(TokenKind::Int.description(), "'int'");
        assert_eq!(TokenKind::UInt.description(), "'uint'");
        assert_eq!(TokenKind::Float.description(), "'float'");
        assert_eq!(TokenKind::Double.description(), "'double'");
    }

    #[test]
    fn token_kind_description_operators() {
        assert_eq!(TokenKind::Plus.description(), "'+'");
        assert_eq!(TokenKind::Minus.description(), "'-'");
        assert_eq!(TokenKind::Star.description(), "'*'");
        assert_eq!(TokenKind::Slash.description(), "'/'");
        assert_eq!(TokenKind::EqualEqual.description(), "'=='");
        assert_eq!(TokenKind::BangEqual.description(), "'!='");
    }

    #[test]
    fn token_kind_description_delimiters() {
        assert_eq!(TokenKind::LeftParen.description(), "'('");
        assert_eq!(TokenKind::RightParen.description(), "')'");
        assert_eq!(TokenKind::LeftBrace.description(), "'{'");
        assert_eq!(TokenKind::RightBrace.description(), "'}'");
        assert_eq!(TokenKind::Semicolon.description(), "';'");
    }

    #[test]
    fn token_kind_description_special() {
        assert_eq!(TokenKind::Identifier.description(), "identifier");
        assert_eq!(TokenKind::Eof.description(), "end of file");
    }

    #[test]
    fn is_keyword_comprehensive() {
        // Should be keywords
        assert!(TokenKind::If.is_keyword());
        assert!(TokenKind::Else.is_keyword());
        assert!(TokenKind::Class.is_keyword());
        assert!(TokenKind::Return.is_keyword());
        assert!(TokenKind::Void.is_keyword());

        // Should NOT be keywords
        assert!(!TokenKind::Identifier.is_keyword());
        assert!(!TokenKind::IntLiteral.is_keyword());
        assert!(!TokenKind::Plus.is_keyword());
        assert!(!TokenKind::LeftParen.is_keyword());
        assert!(!TokenKind::Eof.is_keyword());
    }

    #[test]
    fn is_literal_comprehensive() {
        // Should be literals
        assert!(TokenKind::IntLiteral.is_literal());
        assert!(TokenKind::FloatLiteral.is_literal());
        assert!(TokenKind::DoubleLiteral.is_literal());
        assert!(TokenKind::StringLiteral.is_literal());
        assert!(TokenKind::HeredocLiteral.is_literal());
        assert!(TokenKind::BitsLiteral.is_literal());

        // Should NOT be literals
        assert!(!TokenKind::Identifier.is_literal());
        assert!(!TokenKind::If.is_literal());
        assert!(!TokenKind::Plus.is_literal());
        assert!(!TokenKind::Eof.is_literal());
    }

    #[test]
    fn is_operator_comprehensive() {
        // Arithmetic operators
        assert!(TokenKind::Plus.is_operator());
        assert!(TokenKind::Minus.is_operator());
        assert!(TokenKind::Star.is_operator());
        assert!(TokenKind::Slash.is_operator());

        // Comparison operators
        assert!(TokenKind::EqualEqual.is_operator());
        assert!(TokenKind::BangEqual.is_operator());
        assert!(TokenKind::Less.is_operator());
        assert!(TokenKind::Greater.is_operator());

        // Logical operators
        assert!(TokenKind::AmpAmp.is_operator());
        assert!(TokenKind::PipePipe.is_operator());

        // Should NOT be operators
        assert!(!TokenKind::Identifier.is_operator());
        assert!(!TokenKind::If.is_operator());
        assert!(!TokenKind::IntLiteral.is_operator());
    }

    #[test]
    fn is_delimiter_comprehensive() {
        // Should be delimiters
        assert!(TokenKind::LeftParen.is_delimiter());
        assert!(TokenKind::RightParen.is_delimiter());
        assert!(TokenKind::LeftBrace.is_delimiter());
        assert!(TokenKind::RightBrace.is_delimiter());
        assert!(TokenKind::LeftBracket.is_delimiter());
        assert!(TokenKind::RightBracket.is_delimiter());
        assert!(TokenKind::Semicolon.is_delimiter());
        assert!(TokenKind::Comma.is_delimiter());

        // Should NOT be delimiters
        assert!(!TokenKind::Plus.is_delimiter());
        assert!(!TokenKind::If.is_delimiter());
        assert!(!TokenKind::Identifier.is_delimiter());
    }
}
