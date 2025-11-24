//! Main lexer implementation for AngelScript.
//!
//! The [`Lexer`] converts source text into a stream of [`Token`]s.
//! It uses direct dispatch based on the first character for performance.

use std::collections::VecDeque;

use super::cursor::{is_ident_continue, is_ident_start, Cursor};
use super::error::LexerError;
use super::span::Span;
use super::token::{lookup_keyword, Token, TokenKind};

/// Lexer for AngelScript source code.
///
/// Converts source text into a stream of tokens. Provides lookahead
/// via [`peek`](Self::peek) and [`peek_nth`](Self::peek_nth).
pub struct Lexer<'src> {
    /// Low-level character cursor.
    cursor: Cursor<'src>,
    /// Lookahead buffer for peeking.
    lookahead: VecDeque<Token<'src>>,
    /// Accumulated errors.
    errors: Vec<LexerError>,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source text.
    pub fn new(source: &'src str) -> Self {
        Self {
            cursor: Cursor::new(source),
            lookahead: VecDeque::with_capacity(4),
            errors: Vec::new(),
        }
    }

    /// Take accumulated errors, leaving an empty vec.
    pub fn take_errors(&mut self) -> Vec<LexerError> {
        std::mem::take(&mut self.errors)
    }

    /// Check if any errors occurred.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Consume and return the next token.
    pub fn next_token(&mut self) -> Token<'src> {
        if let Some(token) = self.lookahead.pop_front() {
            return token;
        }
        self.scan_token()
    }

    // =========================================
    // Internal: Token scanning
    // =========================================

    /// Scan the next token from source.
    fn scan_token(&mut self) -> Token<'src> {
        // Skip whitespace
        self.skip_whitespace();

        if self.cursor.is_eof() {
            return self.make_eof();
        }

        let start_line = self.cursor.line();
        let start_col = self.cursor.column();
        let start_offset = self.cursor.offset();

        // Dispatch based on first character
        match self.cursor.peek().unwrap() {
            // Comments or slash operator
            '/' => self.scan_slash(start_line, start_col, start_offset),

            // String literals
            '"' => self.scan_string('"', start_line, start_col, start_offset),
            '\'' => self.scan_string('\'', start_line, start_col, start_offset),

            // Numbers
            c if c.is_ascii_digit() => self.scan_number(start_line, start_col, start_offset),

            // Number starting with dot (e.g., .5)
            '.' if self.cursor.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) => {
                self.scan_number(start_line, start_col, start_offset)
            }

            // Identifiers and keywords
            c if is_ident_start(c) => self.scan_identifier(start_line, start_col, start_offset),

            // Operators and punctuation
            _ => self.scan_operator(start_line, start_col, start_offset),
        }
    }

    /// Skip whitespace and BOM.
    fn skip_whitespace(&mut self) {
        // Check for UTF-8 BOM (EF BB BF)
        if self.cursor.check_str("\u{FEFF}") {
            self.cursor.advance_bytes(3);
        }

        while let Some(c) = self.cursor.peek() {
            if c.is_ascii_whitespace() {
                self.cursor.advance();
            } else {
                break;
            }
        }
    }

    /// Create an EOF token.
    fn make_eof(&self) -> Token<'src> {
        let line = self.cursor.line();
        let col = self.cursor.column();
        Token::new(TokenKind::Eof, "", Span::point(line, col))
    }

    /// Create a token from start position to current position.
    fn make_token(&self, kind: TokenKind, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        let len = self.cursor.offset() - start_offset;
        let span = Span::new(start_line, start_col, len);
        let lexeme = &self.cursor.source()[start_offset as usize..self.cursor.offset() as usize];
        Token::new(kind, lexeme, span)
    }

    /// Create an error token and record the error.
    fn make_error(&mut self, error: LexerError) -> Token<'src> {
        let span = error.span;
        // We need to extract lexeme using the span's line/col info
        // For now, we'll use empty string for errors as we don't have byte offsets
        let lexeme = "";
        self.errors.push(error);
        Token::new(TokenKind::Error, lexeme, span)
    }

    // =========================================
    // Scanning: Comments and slash
    // =========================================

    /// Scan a slash, which could be `/`, `//`, `/*`, `/=`.
    fn scan_slash(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        self.cursor.advance(); // consume '/'

        match self.cursor.peek() {
            // Single-line comment
            Some('/') => {
                self.cursor.advance();
                // Consume until newline
                while let Some(c) = self.cursor.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.cursor.advance();
                }
                // Skip comment, scan next token
                self.scan_token()
            }

            // Multi-line comment
            Some('*') => {
                self.cursor.advance();
                self.scan_block_comment(start_line, start_col, start_offset)
            }

            // Division assignment
            Some('=') => {
                self.cursor.advance();
                self.make_token(TokenKind::SlashEqual, start_line, start_col, start_offset)
            }

            // Just division
            _ => self.make_token(TokenKind::Slash, start_line, start_col, start_offset),
        }
    }

    /// Scan a block comment `/* ... */`.
    fn scan_block_comment(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        loop {
            match self.cursor.peek() {
                None => {
                    // Unterminated comment
                    let len = self.cursor.offset() - start_offset;
                    let error = LexerError::unterminated_comment(Span::new(start_line, start_col, len));
                    return self.make_error(error);
                }
                Some('*') => {
                    self.cursor.advance();
                    if self.cursor.eat('/') {
                        // Comment closed, scan next token
                        return self.scan_token();
                    }
                }
                Some(_) => {
                    self.cursor.advance();
                }
            }
        }
    }

    // =========================================
    // Scanning: Strings
    // =========================================

    /// Scan a string literal starting with the given quote character.
    fn scan_string(&mut self, quote: char, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        self.cursor.advance(); // consume opening quote

        // Check for heredoc `"""`
        if quote == '"' && self.cursor.check_str("\"\"") {
            self.cursor.advance_bytes(2);
            return self.scan_heredoc(start_line, start_col, start_offset);
        }

        // Regular string
        let mut has_newline = false;

        loop {
            match self.cursor.peek() {
                None | Some('\r') | Some('\n') if quote == '\'' => {
                    // Single-quoted strings don't span lines
                    let len = self.cursor.offset() - start_offset;
                    let error = LexerError::unterminated_string(Span::new(start_line, start_col, len));
                    return self.make_error(error);
                }
                None => {
                    let len = self.cursor.offset() - start_offset;
                    let error = LexerError::unterminated_string(Span::new(start_line, start_col, len));
                    return self.make_error(error);
                }
                Some('\n') => {
                    has_newline = true;
                    self.cursor.advance();
                }
                Some('\\') => {
                    self.cursor.advance();
                    // Consume escaped character
                    if self.cursor.peek().is_some() {
                        self.cursor.advance();
                    }
                }
                Some(c) if c == quote => {
                    self.cursor.advance();
                    let kind = if has_newline {
                        TokenKind::StringLiteral // Could differentiate multiline if needed
                    } else {
                        TokenKind::StringLiteral
                    };
                    return self.make_token(kind, start_line, start_col, start_offset);
                }
                Some(_) => {
                    self.cursor.advance();
                }
            }
        }
    }

    /// Scan a heredoc string `"""..."""`.
    fn scan_heredoc(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        loop {
            match self.cursor.peek() {
                None => {
                    let len = self.cursor.offset() - start_offset;
                    let error = LexerError::unterminated_heredoc(Span::new(start_line, start_col, len));
                    return self.make_error(error);
                }
                Some('"') => {
                    if self.cursor.check_str("\"\"\"") {
                        self.cursor.advance_bytes(3);
                        return self.make_token(TokenKind::HeredocLiteral, start_line, start_col, start_offset);
                    }
                    self.cursor.advance();
                }
                Some(_) => {
                    self.cursor.advance();
                }
            }
        }
    }

    // =========================================
    // Scanning: Numbers
    // =========================================

    /// Scan a number literal.
    fn scan_number(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        // Check for radix prefix
        if self.cursor.peek() == Some('0')
            && let Some(radix_char) = self.cursor.peek_nth(1) {
                let radix = match radix_char {
                    'b' | 'B' => Some(2),
                    'o' | 'O' => Some(8),
                    'd' | 'D' => Some(10),
                    'x' | 'X' => Some(16),
                    _ => None,
                };

                if let Some(radix) = radix {
                    return self.scan_radix_number(start_line, start_col, start_offset, radix);
                }
            }

        // Regular decimal number
        self.scan_decimal_number(start_line, start_col, start_offset)
    }

    /// Scan a number with an explicit radix prefix (0x, 0b, 0o, 0d).
    fn scan_radix_number(&mut self, start_line: u32, start_col: u32, start_offset: u32, radix: u32) -> Token<'src> {
        self.cursor.advance(); // '0'
        self.cursor.advance(); // radix letter

        // Consume digits valid for the radix
        let mut has_digits = false;
        while let Some(c) = self.cursor.peek() {
            if is_digit_in_radix(c, radix) {
                has_digits = true;
                self.cursor.advance();
            } else if c == '_' {
                // Allow digit separators
                self.cursor.advance();
            } else {
                break;
            }
        }

        if !has_digits {
            let len = self.cursor.offset() - start_offset;
            let error = LexerError::invalid_number(
                Span::new(start_line, start_col, len),
                "expected digits after radix prefix",
            );
            return self.make_error(error);
        }

        self.make_token(TokenKind::BitsLiteral, start_line, start_col, start_offset)
    }

    /// Scan a decimal number (integer or floating-point).
    fn scan_decimal_number(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        // Integer part (may be empty for `.5`)
        self.consume_decimal_digits();

        let mut is_float = false;

        // Fractional part
        if self.cursor.peek() == Some('.') {
            // Ensure it's not `..` (range) or method call on integer
            if self.cursor.peek_nth(1).is_some_and(|c| c.is_ascii_digit()) {
                self.cursor.advance(); // consume '.'
                self.consume_decimal_digits();
                is_float = true;
            }
        }

        // Exponent part
        if let Some('e' | 'E') = self.cursor.peek() {
            self.cursor.advance();
            // Optional sign
            if matches!(self.cursor.peek(), Some('+' | '-')) {
                self.cursor.advance();
            }
            self.consume_decimal_digits();
            is_float = true;
        }

        // Float suffix
        if let Some('f' | 'F') = self.cursor.peek() {
            self.cursor.advance();
            return self.make_token(TokenKind::FloatLiteral, start_line, start_col, start_offset);
        }

        let kind = if is_float {
            TokenKind::DoubleLiteral
        } else {
            TokenKind::IntLiteral
        };

        self.make_token(kind, start_line, start_col, start_offset)
    }

    /// Consume decimal digits (including underscores as separators).
    fn consume_decimal_digits(&mut self) {
        while let Some(c) = self.cursor.peek() {
            if c.is_ascii_digit() || c == '_' {
                self.cursor.advance();
            } else {
                break;
            }
        }
    }

    // =========================================
    // Scanning: Identifiers and keywords
    // =========================================

    /// Scan an identifier or keyword.
    fn scan_identifier(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        self.cursor.eat_while(is_ident_continue);

        let lexeme = self.cursor.slice_from(start_offset);

        // Check if it's a keyword
        let kind = lookup_keyword(lexeme).unwrap_or(TokenKind::Identifier);

        self.make_token(kind, start_line, start_col, start_offset)
    }

    // =========================================
    // Scanning: Operators
    // =========================================

    /// Scan an operator or punctuation token.
    fn scan_operator(&mut self, start_line: u32, start_col: u32, start_offset: u32) -> Token<'src> {
        let c = self.cursor.advance().unwrap();

        let kind = match c {
            // Single character tokens
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '~' => TokenKind::Tilde,
            '?' => TokenKind::Question,
            '@' => TokenKind::At,

            // Dot (can't be number, we checked earlier)
            '.' => TokenKind::Dot,

            // Colon or ::
            ':' => {
                if self.cursor.eat(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }

            // Plus, ++, +=
            '+' => {
                if self.cursor.eat('+') {
                    TokenKind::PlusPlus
                } else if self.cursor.eat('=') {
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            }

            // Minus, --, -=
            '-' => {
                if self.cursor.eat('-') {
                    TokenKind::MinusMinus
                } else if self.cursor.eat('=') {
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                }
            }

            // Star, **, *=, **=
            '*' => {
                if self.cursor.eat('*') {
                    if self.cursor.eat('=') {
                        TokenKind::StarStarEqual
                    } else {
                        TokenKind::StarStar
                    }
                } else if self.cursor.eat('=') {
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }

            // Percent, %=
            '%' => {
                if self.cursor.eat('=') {
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            }

            // Equal, ==
            '=' => {
                if self.cursor.eat('=') {
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }

            // Bang, !=, !is
            '!' => {
                if self.cursor.eat('=') {
                    TokenKind::BangEqual
                } else if self.cursor.check_str("is")
                    && !self.cursor.peek_nth(2).is_some_and(is_ident_continue)
                {
                    self.cursor.advance_bytes(2);
                    TokenKind::NotIs
                } else {
                    TokenKind::Bang
                }
            }

            // Less, <=, <<, <<=
            '<' => {
                if self.cursor.eat('=') {
                    TokenKind::LessEqual
                } else if self.cursor.eat('<') {
                    if self.cursor.eat('=') {
                        TokenKind::LessLessEqual
                    } else {
                        TokenKind::LessLess
                    }
                } else {
                    TokenKind::Less
                }
            }

            // Greater, >=, >>, >>>, >>=, >>>=
            '>' => {
                if self.cursor.eat('=') {
                    TokenKind::GreaterEqual
                } else if self.cursor.eat('>') {
                    if self.cursor.eat('>') {
                        if self.cursor.eat('=') {
                            TokenKind::GreaterGreaterGreaterEqual
                        } else {
                            TokenKind::GreaterGreaterGreater
                        }
                    } else if self.cursor.eat('=') {
                        TokenKind::GreaterGreaterEqual
                    } else {
                        TokenKind::GreaterGreater
                    }
                } else {
                    TokenKind::Greater
                }
            }

            // Amp, &=, &&
            '&' => {
                if self.cursor.eat('=') {
                    TokenKind::AmpEqual
                } else if self.cursor.eat('&') {
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Amp
                }
            }

            // Pipe, |=, ||
            '|' => {
                if self.cursor.eat('=') {
                    TokenKind::PipeEqual
                } else if self.cursor.eat('|') {
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }

            // Caret, ^=, ^^
            '^' => {
                if self.cursor.eat('=') {
                    TokenKind::CaretEqual
                } else if self.cursor.eat('^') {
                    TokenKind::CaretCaret
                } else {
                    TokenKind::Caret
                }
            }

            // Unrecognized character
            _ => {
                let len = self.cursor.offset() - start_offset;
                let error = LexerError::unexpected_char(c, Span::new(start_line, start_col, len));
                return self.make_error(error);
            }
        };

        self.make_token(kind, start_line, start_col, start_offset)
    }
}

/// Implement Iterator for convenient token streaming.
impl<'src> Iterator for Lexer<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token.kind == TokenKind::Eof {
            None
        } else {
            Some(token)
        }
    }
}

/// Check if a character is a valid digit in the given radix.
fn is_digit_in_radix(c: char, radix: u32) -> bool {
    match radix {
        2 => matches!(c, '0' | '1'),
        8 => matches!(c, '0'..='7'),
        10 => c.is_ascii_digit(),
        16 => c.is_ascii_hexdigit(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to collect all tokens from source.
    fn tokenize(source: &str) -> Vec<(TokenKind, &str)> {
        Lexer::new(source)
            .map(|t| (t.kind, t.lexeme))
            .collect()
    }

    /// Helper to get token kinds only.
    fn token_kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(source).map(|t| t.kind).collect()
    }

    // =========================================
    // Basic tokens
    // =========================================

    #[test]
    fn empty_source() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.next_token().kind, TokenKind::Eof);
    }

    #[test]
    fn whitespace_only() {
        let mut lexer = Lexer::new("   \t\n\r  ");
        assert_eq!(lexer.next_token().kind, TokenKind::Eof);
    }

    #[test]
    fn bom_handling() {
        let source = "\u{FEFF}hello";
        let tokens = tokenize(source);
        assert_eq!(tokens, vec![(TokenKind::Identifier, "hello")]);
    }

    // =========================================
    // Identifiers and keywords
    // =========================================

    #[test]
    fn identifiers() {
        assert_eq!(
            tokenize("hello world _foo bar123"),
            vec![
                (TokenKind::Identifier, "hello"),
                (TokenKind::Identifier, "world"),
                (TokenKind::Identifier, "_foo"),
                (TokenKind::Identifier, "bar123"),
            ]
        );
    }

    #[test]
    fn keywords() {
        assert_eq!(
            token_kinds("if else while for return"),
            vec![
                TokenKind::If,
                TokenKind::Else,
                TokenKind::While,
                TokenKind::For,
                TokenKind::Return,
            ]
        );
    }

    #[test]
    fn type_keywords() {
        assert_eq!(
            token_kinds("int int32 uint uint32 float double"),
            vec![
                TokenKind::Int,
                TokenKind::Int,
                TokenKind::UInt,
                TokenKind::UInt,
                TokenKind::Float,
                TokenKind::Double,
            ]
        );
    }

    #[test]
    fn keyword_vs_identifier() {
        // "iffy" should be identifier, not "if" + "fy"
        assert_eq!(tokenize("iffy"), vec![(TokenKind::Identifier, "iffy")]);
    }

    // =========================================
    // Numbers
    // =========================================

    #[test]
    fn integer_literals() {
        assert_eq!(
            tokenize("42 0 12345"),
            vec![
                (TokenKind::IntLiteral, "42"),
                (TokenKind::IntLiteral, "0"),
                (TokenKind::IntLiteral, "12345"),
            ]
        );
    }

    #[test]
    fn float_literals() {
        assert_eq!(
            tokenize("3.14 1.0f 2.5F"),
            vec![
                (TokenKind::DoubleLiteral, "3.14"),
                (TokenKind::FloatLiteral, "1.0f"),
                (TokenKind::FloatLiteral, "2.5F"),
            ]
        );
    }

    #[test]
    fn scientific_notation() {
        assert_eq!(
            tokenize("1e10 2.5e-3 3E+4f"),
            vec![
                (TokenKind::DoubleLiteral, "1e10"),
                (TokenKind::DoubleLiteral, "2.5e-3"),
                (TokenKind::FloatLiteral, "3E+4f"),
            ]
        );
    }

    #[test]
    fn radix_numbers() {
        assert_eq!(
            tokenize("0xFF 0b1010 0o77 0d99"),
            vec![
                (TokenKind::BitsLiteral, "0xFF"),
                (TokenKind::BitsLiteral, "0b1010"),
                (TokenKind::BitsLiteral, "0o77"),
                (TokenKind::BitsLiteral, "0d99"),
            ]
        );
    }

    #[test]
    fn number_with_dot() {
        // .5 is a valid float
        assert_eq!(tokenize(".5"), vec![(TokenKind::DoubleLiteral, ".5")]);

        // 1. followed by non-digit is int then dot
        assert_eq!(
            tokenize("1.x"),
            vec![
                (TokenKind::IntLiteral, "1"),
                (TokenKind::Dot, "."),
                (TokenKind::Identifier, "x"),
            ]
        );
    }

    // =========================================
    // Strings
    // =========================================

    #[test]
    fn string_literals() {
        assert_eq!(
            tokenize(r#""hello" 'a'"#),
            vec![
                (TokenKind::StringLiteral, r#""hello""#),
                (TokenKind::StringLiteral, "'a'"),
            ]
        );
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(
            tokenize(r#""hello\nworld" "tab\there""#),
            vec![
                (TokenKind::StringLiteral, r#""hello\nworld""#),
                (TokenKind::StringLiteral, r#""tab\there""#),
            ]
        );
    }

    #[test]
    fn heredoc_string() {
        let source = r#""""
multiline
string
""""#;
        let tokens = tokenize(source);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, TokenKind::HeredocLiteral);
    }

    #[test]
    fn unterminated_string() {
        let mut lexer = Lexer::new(r#""hello"#);
        let token = lexer.next_token();
        assert_eq!(token.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    // =========================================
    // Comments
    // =========================================

    #[test]
    fn line_comment() {
        assert_eq!(
            tokenize("a // comment\nb"),
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::Identifier, "b"),
            ]
        );
    }

    #[test]
    fn block_comment() {
        assert_eq!(
            tokenize("a /* comment */ b"),
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::Identifier, "b"),
            ]
        );
    }

    #[test]
    fn multiline_block_comment() {
        assert_eq!(
            tokenize("a /* multi\nline\ncomment */ b"),
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::Identifier, "b"),
            ]
        );
    }

    #[test]
    fn unterminated_comment() {
        let mut lexer = Lexer::new("a /* unterminated");
        let _ = lexer.next_token(); // 'a'
        let token = lexer.next_token();
        assert_eq!(token.kind, TokenKind::Error);
        assert!(lexer.has_errors());
    }

    // =========================================
    // Operators
    // =========================================

    #[test]
    fn arithmetic_operators() {
        assert_eq!(
            token_kinds("+ - * / % **"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::StarStar,
            ]
        );
    }

    #[test]
    fn comparison_operators() {
        assert_eq!(
            token_kinds("== != < <= > >="),
            vec![
                TokenKind::EqualEqual,
                TokenKind::BangEqual,
                TokenKind::Less,
                TokenKind::LessEqual,
                TokenKind::Greater,
                TokenKind::GreaterEqual,
            ]
        );
    }

    #[test]
    fn assignment_operators() {
        assert_eq!(
            token_kinds("= += -= *= /= %= **="),
            vec![
                TokenKind::Equal,
                TokenKind::PlusEqual,
                TokenKind::MinusEqual,
                TokenKind::StarEqual,
                TokenKind::SlashEqual,
                TokenKind::PercentEqual,
                TokenKind::StarStarEqual,
            ]
        );
    }

    #[test]
    fn bitwise_operators() {
        assert_eq!(
            token_kinds("& | ^ ~ << >> >>>"),
            vec![
                TokenKind::Amp,
                TokenKind::Pipe,
                TokenKind::Caret,
                TokenKind::Tilde,
                TokenKind::LessLess,
                TokenKind::GreaterGreater,
                TokenKind::GreaterGreaterGreater,
            ]
        );
    }

    #[test]
    fn bitwise_assignment_operators() {
        assert_eq!(
            token_kinds("&= |= ^= <<= >>= >>>="),
            vec![
                TokenKind::AmpEqual,
                TokenKind::PipeEqual,
                TokenKind::CaretEqual,
                TokenKind::LessLessEqual,
                TokenKind::GreaterGreaterEqual,
                TokenKind::GreaterGreaterGreaterEqual,
            ]
        );
    }

    #[test]
    fn logical_operators() {
        assert_eq!(
            token_kinds("&& || ^^ ! and or xor not"),
            vec![
                TokenKind::AmpAmp,
                TokenKind::PipePipe,
                TokenKind::CaretCaret,
                TokenKind::Bang,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Xor,
                TokenKind::Not,
            ]
        );
    }

    #[test]
    fn increment_decrement() {
        assert_eq!(
            token_kinds("++ --"),
            vec![TokenKind::PlusPlus, TokenKind::MinusMinus]
        );
    }

    #[test]
    fn delimiters() {
        assert_eq!(
            token_kinds("( ) [ ] { } ; ,"),
            vec![
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Semicolon,
                TokenKind::Comma,
            ]
        );
    }

    #[test]
    fn other_operators() {
        assert_eq!(
            token_kinds(". : :: ? @ ~"),
            vec![
                TokenKind::Dot,
                TokenKind::Colon,
                TokenKind::ColonColon,
                TokenKind::Question,
                TokenKind::At,
                TokenKind::Tilde,
            ]
        );
    }

    // =========================================
    // Special: !is token
    // =========================================

    #[test]
    fn not_is_token() {
        // !is as single token
        assert_eq!(token_kinds("!is"), vec![TokenKind::NotIs]);

        // ! followed by is (with space)
        assert_eq!(
            token_kinds("! is"),
            vec![TokenKind::Bang, TokenKind::Is]
        );

        // !island should be ! + identifier
        assert_eq!(
            token_kinds("!island"),
            vec![TokenKind::Bang, TokenKind::Identifier]
        );
    }

    // =========================================
    // Error recovery
    // =========================================

    #[test]
    fn unexpected_character() {
        let mut lexer = Lexer::new("a $ b");
        let tokens: Vec<_> = lexer.by_ref().collect();

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].kind, TokenKind::Error);
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert!(lexer.has_errors());
    }

    // =========================================
    // Integration: real code
    // =========================================

    #[test]
    fn simple_function() {
        let source = r#"
            int add(int a, int b) {
                return a + b;
            }
        "#;

        let tokens: Vec<_> = Lexer::new(source).collect();
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::LeftParen,
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::Comma,
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::Return,
                TokenKind::Identifier,
                TokenKind::Plus,
                TokenKind::Identifier,
                TokenKind::Semicolon,
                TokenKind::RightBrace,
            ]
        );
    }

    #[test]
    fn class_definition() {
        let source = r#"
            class Enemy {
                int health;
                void takeDamage(int amount) {
                    health -= amount;
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.by_ref().collect();

        assert!(!lexer.has_errors());
        assert!(tokens.len() > 10);
    }

    // =========================================
    // Public API methods
    // =========================================

    #[test]
    fn has_errors_method() {
        let mut lexer = Lexer::new("int x = 42;");
        assert!(!lexer.has_errors());

        while lexer.next_token().kind != TokenKind::Eof {}
        assert!(!lexer.has_errors());

        let mut lexer_with_error = Lexer::new("$");
        assert!(!lexer_with_error.has_errors());

        lexer_with_error.next_token();
        assert!(lexer_with_error.has_errors());
    }
}
