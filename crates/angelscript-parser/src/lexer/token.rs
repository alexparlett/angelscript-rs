//! Token types and definitions for the AngelScript lexer.
//!
//! Based on token definitions from the C++ AngelScript implementation
//! (`as_tokendef.h`).

use angelscript_core::Span;
use std::fmt;

/// A token from the source code.
///
/// The `'ast` lifetime refers to the arena where the lexeme string is allocated.
/// This allows the source string to be freed after lexing, since all string
/// content is copied into the arena.
#[derive(Clone, Copy, PartialEq)]
pub struct Token<'ast> {
    /// The type of token.
    pub kind: TokenKind,
    /// The source text of this token (allocated in arena).
    pub lexeme: &'ast str,
    /// Location in source.
    pub span: Span,
}

impl<'ast> Token<'ast> {
    /// Create a new token.
    #[inline]
    pub fn new(kind: TokenKind, lexeme: &'ast str, span: Span) -> Self {
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
    /// `using`
    Using,
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
                | Using
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
            Using => "'using'",
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
        "using" => Using,
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

    #[test]
    fn token_kind_description_all_types() {
        // Cover all type keywords
        assert_eq!(TokenKind::Int8.description(), "'int8'");
        assert_eq!(TokenKind::Int16.description(), "'int16'");
        assert_eq!(TokenKind::Int64.description(), "'int64'");
        assert_eq!(TokenKind::UInt8.description(), "'uint8'");
        assert_eq!(TokenKind::UInt16.description(), "'uint16'");
        assert_eq!(TokenKind::UInt64.description(), "'uint64'");
        assert_eq!(TokenKind::Auto.description(), "'auto'");
    }

    #[test]
    fn token_kind_description_values() {
        assert_eq!(TokenKind::True.description(), "'true'");
        assert_eq!(TokenKind::False.description(), "'false'");
        assert_eq!(TokenKind::Null.description(), "'null'");
    }

    #[test]
    fn token_kind_description_control_flow() {
        assert_eq!(TokenKind::Do.description(), "'do'");
        assert_eq!(TokenKind::Switch.description(), "'switch'");
        assert_eq!(TokenKind::Case.description(), "'case'");
        assert_eq!(TokenKind::Default.description(), "'default'");
        assert_eq!(TokenKind::Break.description(), "'break'");
        assert_eq!(TokenKind::Continue.description(), "'continue'");
        assert_eq!(TokenKind::Try.description(), "'try'");
        assert_eq!(TokenKind::Catch.description(), "'catch'");
    }

    #[test]
    fn token_kind_description_declarations() {
        assert_eq!(TokenKind::Interface.description(), "'interface'");
        assert_eq!(TokenKind::Enum.description(), "'enum'");
        assert_eq!(TokenKind::FuncDef.description(), "'funcdef'");
        assert_eq!(TokenKind::Namespace.description(), "'namespace'");
        assert_eq!(TokenKind::Mixin.description(), "'mixin'");
        assert_eq!(TokenKind::Typedef.description(), "'typedef'");
        assert_eq!(TokenKind::Import.description(), "'import'");
        assert_eq!(TokenKind::Const.description(), "'const'");
        assert_eq!(TokenKind::Private.description(), "'private'");
        assert_eq!(TokenKind::Protected.description(), "'protected'");
    }

    #[test]
    fn token_kind_description_word_operators() {
        assert_eq!(TokenKind::And.description(), "'and'");
        assert_eq!(TokenKind::Or.description(), "'or'");
        assert_eq!(TokenKind::Xor.description(), "'xor'");
        assert_eq!(TokenKind::Not.description(), "'not'");
        assert_eq!(TokenKind::Is.description(), "'is'");
        assert_eq!(TokenKind::NotIs.description(), "'!is'");
        assert_eq!(TokenKind::In.description(), "'in'");
        assert_eq!(TokenKind::Out.description(), "'out'");
        assert_eq!(TokenKind::InOut.description(), "'inout'");
        assert_eq!(TokenKind::Cast.description(), "'cast'");
        assert_eq!(TokenKind::Super.description(), "'super'");
        assert_eq!(TokenKind::This.description(), "'this'");
    }

    #[test]
    fn token_kind_description_compound_assignment() {
        assert_eq!(TokenKind::Equal.description(), "'='");
        assert_eq!(TokenKind::PlusEqual.description(), "'+='");
        assert_eq!(TokenKind::MinusEqual.description(), "'-='");
        assert_eq!(TokenKind::StarEqual.description(), "'*='");
        assert_eq!(TokenKind::SlashEqual.description(), "'/='");
        assert_eq!(TokenKind::PercentEqual.description(), "'%='");
        assert_eq!(TokenKind::StarStarEqual.description(), "'**='");
    }

    #[test]
    fn token_kind_description_bitwise() {
        assert_eq!(TokenKind::Amp.description(), "'&'");
        assert_eq!(TokenKind::Pipe.description(), "'|'");
        assert_eq!(TokenKind::Caret.description(), "'^'");
        assert_eq!(TokenKind::Tilde.description(), "'~'");
        assert_eq!(TokenKind::LessLess.description(), "'<<'");
        assert_eq!(TokenKind::GreaterGreater.description(), "'>>'");
        assert_eq!(TokenKind::GreaterGreaterGreater.description(), "'>>>'");
    }

    #[test]
    fn token_kind_description_bitwise_assignment() {
        assert_eq!(TokenKind::AmpEqual.description(), "'&='");
        assert_eq!(TokenKind::PipeEqual.description(), "'|='");
        assert_eq!(TokenKind::CaretEqual.description(), "'^='");
        assert_eq!(TokenKind::LessLessEqual.description(), "'<<='");
        assert_eq!(TokenKind::GreaterGreaterEqual.description(), "'>>='");
        assert_eq!(
            TokenKind::GreaterGreaterGreaterEqual.description(),
            "'>>>='"
        );
    }

    #[test]
    fn token_kind_description_comparison() {
        assert_eq!(TokenKind::LessEqual.description(), "'<='");
        assert_eq!(TokenKind::GreaterEqual.description(), "'>='");
    }

    #[test]
    fn token_kind_description_logical() {
        assert_eq!(TokenKind::AmpAmp.description(), "'&&'");
        assert_eq!(TokenKind::PipePipe.description(), "'||'");
        assert_eq!(TokenKind::CaretCaret.description(), "'^^'");
        assert_eq!(TokenKind::Bang.description(), "'!'");
    }

    #[test]
    fn token_kind_description_increment_decrement() {
        assert_eq!(TokenKind::PlusPlus.description(), "'++'");
        assert_eq!(TokenKind::MinusMinus.description(), "'--'");
    }

    #[test]
    fn token_kind_description_other_operators() {
        assert_eq!(TokenKind::Percent.description(), "'%'");
        assert_eq!(TokenKind::StarStar.description(), "'**'");
        assert_eq!(TokenKind::Question.description(), "'?'");
        assert_eq!(TokenKind::Colon.description(), "':'");
        assert_eq!(TokenKind::ColonColon.description(), "'::'");
        assert_eq!(TokenKind::Dot.description(), "'.'");
        assert_eq!(TokenKind::At.description(), "'@'");
    }

    #[test]
    fn token_kind_description_brackets() {
        assert_eq!(TokenKind::LeftBracket.description(), "'['");
        assert_eq!(TokenKind::RightBracket.description(), "']'");
        assert_eq!(TokenKind::Comma.description(), "','");
    }

    #[test]
    fn token_kind_description_error() {
        assert_eq!(TokenKind::Error.description(), "error");
    }

    #[test]
    fn token_kind_display_trait() {
        // Test Display implementation which calls description()
        assert_eq!(format!("{}", TokenKind::If), "'if'");
        assert_eq!(format!("{}", TokenKind::Plus), "'+'");
        assert_eq!(format!("{}", TokenKind::IntLiteral), "integer literal");
    }

    #[test]
    fn keyword_lookup_all_types() {
        // Type aliases
        assert_eq!(lookup_keyword("int"), Some(TokenKind::Int));
        assert_eq!(lookup_keyword("int8"), Some(TokenKind::Int8));
        assert_eq!(lookup_keyword("int16"), Some(TokenKind::Int16));
        assert_eq!(lookup_keyword("int64"), Some(TokenKind::Int64));
        assert_eq!(lookup_keyword("uint"), Some(TokenKind::UInt));
        assert_eq!(lookup_keyword("uint8"), Some(TokenKind::UInt8));
        assert_eq!(lookup_keyword("uint16"), Some(TokenKind::UInt16));
        assert_eq!(lookup_keyword("uint64"), Some(TokenKind::UInt64));
        assert_eq!(lookup_keyword("void"), Some(TokenKind::Void));
        assert_eq!(lookup_keyword("bool"), Some(TokenKind::Bool));
        assert_eq!(lookup_keyword("float"), Some(TokenKind::Float));
        assert_eq!(lookup_keyword("double"), Some(TokenKind::Double));
        assert_eq!(lookup_keyword("auto"), Some(TokenKind::Auto));
    }

    #[test]
    fn keyword_lookup_values() {
        assert_eq!(lookup_keyword("true"), Some(TokenKind::True));
        assert_eq!(lookup_keyword("false"), Some(TokenKind::False));
        assert_eq!(lookup_keyword("null"), Some(TokenKind::Null));
    }

    #[test]
    fn keyword_lookup_control_flow() {
        assert_eq!(lookup_keyword("if"), Some(TokenKind::If));
        assert_eq!(lookup_keyword("else"), Some(TokenKind::Else));
        assert_eq!(lookup_keyword("for"), Some(TokenKind::For));
        assert_eq!(lookup_keyword("while"), Some(TokenKind::While));
        assert_eq!(lookup_keyword("do"), Some(TokenKind::Do));
        assert_eq!(lookup_keyword("switch"), Some(TokenKind::Switch));
        assert_eq!(lookup_keyword("case"), Some(TokenKind::Case));
        assert_eq!(lookup_keyword("default"), Some(TokenKind::Default));
        assert_eq!(lookup_keyword("break"), Some(TokenKind::Break));
        assert_eq!(lookup_keyword("continue"), Some(TokenKind::Continue));
        assert_eq!(lookup_keyword("return"), Some(TokenKind::Return));
        assert_eq!(lookup_keyword("try"), Some(TokenKind::Try));
        assert_eq!(lookup_keyword("catch"), Some(TokenKind::Catch));
    }

    #[test]
    fn keyword_lookup_declarations() {
        assert_eq!(lookup_keyword("class"), Some(TokenKind::Class));
        assert_eq!(lookup_keyword("interface"), Some(TokenKind::Interface));
        assert_eq!(lookup_keyword("enum"), Some(TokenKind::Enum));
        assert_eq!(lookup_keyword("funcdef"), Some(TokenKind::FuncDef));
        assert_eq!(lookup_keyword("namespace"), Some(TokenKind::Namespace));
        assert_eq!(lookup_keyword("mixin"), Some(TokenKind::Mixin));
        assert_eq!(lookup_keyword("typedef"), Some(TokenKind::Typedef));
        assert_eq!(lookup_keyword("import"), Some(TokenKind::Import));
        assert_eq!(lookup_keyword("const"), Some(TokenKind::Const));
        assert_eq!(lookup_keyword("private"), Some(TokenKind::Private));
        assert_eq!(lookup_keyword("protected"), Some(TokenKind::Protected));
    }

    #[test]
    fn keyword_lookup_word_operators() {
        assert_eq!(lookup_keyword("and"), Some(TokenKind::And));
        assert_eq!(lookup_keyword("or"), Some(TokenKind::Or));
        assert_eq!(lookup_keyword("xor"), Some(TokenKind::Xor));
        assert_eq!(lookup_keyword("not"), Some(TokenKind::Not));
        assert_eq!(lookup_keyword("is"), Some(TokenKind::Is));
        assert_eq!(lookup_keyword("in"), Some(TokenKind::In));
        assert_eq!(lookup_keyword("out"), Some(TokenKind::Out));
        assert_eq!(lookup_keyword("inout"), Some(TokenKind::InOut));
        assert_eq!(lookup_keyword("cast"), Some(TokenKind::Cast));
        assert_eq!(lookup_keyword("super"), Some(TokenKind::Super));
        assert_eq!(lookup_keyword("this"), Some(TokenKind::This));
    }

    #[test]
    fn keyword_lookup_not_found() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("bar"), None);
        assert_eq!(lookup_keyword(""), None);
        assert_eq!(lookup_keyword("Int"), None); // Case sensitive
        assert_eq!(lookup_keyword("IF"), None);
    }
}
