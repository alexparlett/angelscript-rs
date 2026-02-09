use crate::token::{keyword_lookup, Span, Token, TokenKind};

/// Lexer that tokenizes AngelScript source code.
pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    /// Tokenize the entire source into a Vec, skipping whitespace/comments.
    pub fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok.kind == TokenKind::Eof {
                tokens.push(tok);
                break;
            }
            if !tok.kind.is_trivia() {
                tokens.push(tok);
            }
        }
        tokens
    }

    /// Tokenize including whitespace and comments (for formatting tools).
    pub fn tokenize_all(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Token {
        if self.pos >= self.bytes.len() {
            return Token::new(TokenKind::Eof, Span::new(self.pos, self.pos));
        }

        let start = self.pos;

        // Try each token type in order
        if let Some(tok) = self.try_whitespace(start) {
            return tok;
        }
        if let Some(tok) = self.try_comment(start) {
            return tok;
        }
        if let Some(tok) = self.try_string(start) {
            return tok;
        }
        if let Some(tok) = self.try_number(start) {
            return tok;
        }
        if let Some(tok) = self.try_identifier_or_keyword(start) {
            return tok;
        }
        if let Some(tok) = self.try_operator(start) {
            return tok;
        }

        // Unrecognized character — advance one byte
        self.pos += 1;
        Token::new(TokenKind::Error, Span::new(start, self.pos))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> u8 {
        let b = self.bytes[self.pos];
        self.pos += 1;
        b
    }

    fn try_whitespace(&mut self, start: usize) -> Option<Token> {
        let b = self.bytes[self.pos];
        if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
            self.pos += 1;
            while self.pos < self.bytes.len() {
                let b = self.bytes[self.pos];
                if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            return Some(Token::new(
                TokenKind::WhiteSpace,
                Span::new(start, self.pos),
            ));
        }
        // UTF-8 BOM (EF BB BF)
        if b == 0xEF && self.peek_at(1) == Some(0xBB) && self.peek_at(2) == Some(0xBF) {
            self.pos += 3;
            return Some(Token::new(
                TokenKind::WhiteSpace,
                Span::new(start, self.pos),
            ));
        }
        None
    }

    fn try_comment(&mut self, start: usize) -> Option<Token> {
        if self.bytes[self.pos] != b'/' {
            return None;
        }
        match self.peek_at(1) {
            Some(b'/') => {
                // Single-line comment
                self.pos += 2;
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
                Some(Token::new(
                    TokenKind::LineComment,
                    Span::new(start, self.pos),
                ))
            }
            Some(b'*') => {
                // Multi-line comment
                self.pos += 2;
                let mut depth = 1;
                while self.pos < self.bytes.len() && depth > 0 {
                    if self.bytes[self.pos] == b'*' && self.peek_at(1) == Some(b'/') {
                        depth -= 1;
                        self.pos += 2;
                    } else if self.bytes[self.pos] == b'/' && self.peek_at(1) == Some(b'*') {
                        depth += 1;
                        self.pos += 2;
                    } else {
                        self.pos += 1;
                    }
                }
                Some(Token::new(
                    TokenKind::BlockComment,
                    Span::new(start, self.pos),
                ))
            }
            _ => None,
        }
    }

    fn try_string(&mut self, start: usize) -> Option<Token> {
        let b = self.bytes[self.pos];

        if b == b'"' {
            // Check for heredoc string """..."""
            if self.peek_at(1) == Some(b'"') && self.peek_at(2) == Some(b'"') {
                self.pos += 3;
                loop {
                    if self.pos + 2 >= self.bytes.len() {
                        return Some(Token::new(
                            TokenKind::NonTerminatedStringConstant,
                            Span::new(start, self.pos),
                        ));
                    }
                    if self.bytes[self.pos] == b'"'
                        && self.peek_at(1) == Some(b'"')
                        && self.peek_at(2) == Some(b'"')
                    {
                        self.pos += 3;
                        return Some(Token::new(
                            TokenKind::HeredocStringConstant,
                            Span::new(start, self.pos),
                        ));
                    }
                    self.pos += 1;
                }
            }

            // Regular string "..."
            self.pos += 1; // skip opening quote
            let mut multiline = false;
            while self.pos < self.bytes.len() {
                match self.bytes[self.pos] {
                    b'"' => {
                        self.pos += 1;
                        let kind = if multiline {
                            TokenKind::MultilineStringConstant
                        } else {
                            TokenKind::StringConstant
                        };
                        return Some(Token::new(kind, Span::new(start, self.pos)));
                    }
                    b'\\' => {
                        self.pos += 1; // skip backslash
                        if self.pos < self.bytes.len() {
                            self.pos += 1; // skip escaped char
                        }
                    }
                    b'\n' => {
                        multiline = true;
                        self.pos += 1;
                    }
                    _ => {
                        self.pos += 1;
                    }
                }
            }
            return Some(Token::new(
                TokenKind::NonTerminatedStringConstant,
                Span::new(start, self.pos),
            ));
        }

        // Single-quoted strings (character literals treated as strings in AS)
        if b == b'\'' {
            self.pos += 1;
            while self.pos < self.bytes.len() {
                match self.bytes[self.pos] {
                    b'\'' => {
                        self.pos += 1;
                        return Some(Token::new(
                            TokenKind::StringConstant,
                            Span::new(start, self.pos),
                        ));
                    }
                    b'\\' => {
                        self.pos += 1;
                        if self.pos < self.bytes.len() {
                            self.pos += 1;
                        }
                    }
                    _ => {
                        self.pos += 1;
                    }
                }
            }
            return Some(Token::new(
                TokenKind::NonTerminatedStringConstant,
                Span::new(start, self.pos),
            ));
        }

        None
    }

    fn try_number(&mut self, start: usize) -> Option<Token> {
        let b = self.bytes[self.pos];

        if !b.is_ascii_digit() {
            // Check for .digit (float starting with dot)
            if b == b'.' && self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
                return Some(self.lex_float_from_dot(start));
            }
            return None;
        }

        // 0x, 0o, 0b, 0d prefixed constants
        if b == b'0' {
            match self.peek_at(1) {
                Some(b'x') | Some(b'X') => {
                    self.pos += 2;
                    while self.pos < self.bytes.len()
                        && (self.bytes[self.pos].is_ascii_hexdigit()
                            || self.bytes[self.pos] == b'_')
                    {
                        self.pos += 1;
                    }
                    return Some(Token::new(
                        TokenKind::BitsConstant,
                        Span::new(start, self.pos),
                    ));
                }
                Some(b'o') | Some(b'O') => {
                    self.pos += 2;
                    while self.pos < self.bytes.len()
                        && (self.bytes[self.pos] >= b'0' && self.bytes[self.pos] <= b'7'
                            || self.bytes[self.pos] == b'_')
                    {
                        self.pos += 1;
                    }
                    return Some(Token::new(
                        TokenKind::BitsConstant,
                        Span::new(start, self.pos),
                    ));
                }
                Some(b'b') | Some(b'B') => {
                    self.pos += 2;
                    while self.pos < self.bytes.len()
                        && (self.bytes[self.pos] == b'0'
                            || self.bytes[self.pos] == b'1'
                            || self.bytes[self.pos] == b'_')
                    {
                        self.pos += 1;
                    }
                    return Some(Token::new(
                        TokenKind::BitsConstant,
                        Span::new(start, self.pos),
                    ));
                }
                Some(b'd') | Some(b'D') => {
                    self.pos += 2;
                    while self.pos < self.bytes.len()
                        && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
                    {
                        self.pos += 1;
                    }
                    return Some(Token::new(
                        TokenKind::BitsConstant,
                        Span::new(start, self.pos),
                    ));
                }
                _ => {}
            }
        }

        // Decimal integer or float
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
        {
            self.pos += 1;
        }

        // Check for fractional part
        if self.pos < self.bytes.len()
            && self.bytes[self.pos] == b'.'
            && self.peek_at(1).map_or(false, |c| c.is_ascii_digit())
        {
            return Some(self.lex_float_from_dot(start));
        }

        // Check for exponent (without decimal point)
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'e' || self.bytes[self.pos] == b'E')
        {
            return Some(self.lex_float_exponent(start));
        }

        // Check for float suffix
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'f' {
            self.pos += 1;
            return Some(Token::new(
                TokenKind::FloatConstant,
                Span::new(start, self.pos),
            ));
        }

        Some(Token::new(
            TokenKind::IntConstant,
            Span::new(start, self.pos),
        ))
    }

    fn lex_float_from_dot(&mut self, start: usize) -> Token {
        // We're at the dot
        self.pos += 1; // skip dot
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'_')
        {
            self.pos += 1;
        }
        // Exponent?
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'e' || self.bytes[self.pos] == b'E')
        {
            return self.lex_float_exponent(start);
        }
        // Float suffix?
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'f' {
            self.pos += 1;
            return Token::new(TokenKind::FloatConstant, Span::new(start, self.pos));
        }
        // No suffix = double
        Token::new(TokenKind::DoubleConstant, Span::new(start, self.pos))
    }

    fn lex_float_exponent(&mut self, start: usize) -> Token {
        self.pos += 1; // skip 'e'/'E'
        if self.pos < self.bytes.len()
            && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-')
        {
            self.pos += 1; // skip sign
        }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        // Float suffix?
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'f' {
            self.pos += 1;
            return Token::new(TokenKind::FloatConstant, Span::new(start, self.pos));
        }
        Token::new(TokenKind::DoubleConstant, Span::new(start, self.pos))
    }

    fn try_identifier_or_keyword(&mut self, start: usize) -> Option<Token> {
        let b = self.bytes[self.pos];
        if !is_ident_start(b) {
            return None;
        }

        self.pos += 1;
        while self.pos < self.bytes.len() && is_ident_continue(self.bytes[self.pos]) {
            self.pos += 1;
        }

        let word = &self.source[start..self.pos];

        // Check for keywords
        if let Some(kw) = keyword_lookup(word) {
            return Some(Token::new(kw, Span::new(start, self.pos)));
        }

        Some(Token::new(
            TokenKind::Identifier,
            Span::new(start, self.pos),
        ))
    }

    fn try_operator(&mut self, start: usize) -> Option<Token> {
        let b = self.advance();
        let kind = match b {
            b'+' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::AddAssign
                }
                Some(b'+') => {
                    self.pos += 1;
                    TokenKind::Inc
                }
                _ => TokenKind::Plus,
            },
            b'-' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::SubAssign
                }
                Some(b'-') => {
                    self.pos += 1;
                    TokenKind::Dec
                }
                _ => TokenKind::Minus,
            },
            b'*' => match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    if self.peek() == Some(b'=') {
                        self.pos += 1;
                        TokenKind::PowAssign
                    } else {
                        TokenKind::StarStar
                    }
                }
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::MulAssign
                }
                _ => TokenKind::Star,
            },
            // Note: '/' is handled in try_comment first; if we get here it's division
            b'/' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::DivAssign
                }
                _ => TokenKind::Slash,
            },
            b'%' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::ModAssign
                }
                _ => TokenKind::Percent,
            },
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::Equal
                } else {
                    TokenKind::Assign
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    TokenKind::NotEqual
                } else if self.peek() == Some(b'i')
                    && self.peek_at(1) == Some(b's')
                    && !self.peek_at(2).map_or(false, |c| is_ident_continue(c))
                {
                    self.pos += 2;
                    TokenKind::NotIs
                } else {
                    TokenKind::Not
                }
            }
            b'<' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::LessEqual
                }
                Some(b'<') => {
                    self.pos += 1;
                    if self.peek() == Some(b'=') {
                        self.pos += 1;
                        TokenKind::ShiftLeftAssign
                    } else {
                        TokenKind::ShiftLeft
                    }
                }
                _ => TokenKind::Less,
            },
            b'>' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::GreaterEqual
                }
                Some(b'>') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'>') => {
                            self.pos += 1;
                            if self.peek() == Some(b'=') {
                                self.pos += 1;
                                TokenKind::ShiftRightAAssign
                            } else {
                                TokenKind::ShiftRightArith
                            }
                        }
                        Some(b'=') => {
                            self.pos += 1;
                            TokenKind::ShiftRightLAssign
                        }
                        _ => TokenKind::ShiftRight,
                    }
                }
                _ => TokenKind::Greater,
            },
            b'|' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::OrAssign
                }
                Some(b'|') => {
                    self.pos += 1;
                    TokenKind::Or
                }
                _ => TokenKind::Pipe,
            },
            b'&' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::AndAssign
                }
                Some(b'&') => {
                    self.pos += 1;
                    TokenKind::And
                }
                _ => TokenKind::Amp,
            },
            b'^' => match self.peek() {
                Some(b'=') => {
                    self.pos += 1;
                    TokenKind::XorAssign
                }
                Some(b'^') => {
                    self.pos += 1;
                    TokenKind::Xor
                }
                _ => TokenKind::Caret,
            },
            b'~' => TokenKind::Tilde,
            b'.' => TokenKind::Dot,
            b':' => {
                if self.peek() == Some(b':') {
                    self.pos += 1;
                    TokenKind::Scope
                } else {
                    TokenKind::Colon
                }
            }
            b';' => TokenKind::Semicolon,
            b',' => TokenKind::Comma,
            b'{' => TokenKind::OpenBrace,
            b'}' => TokenKind::CloseBrace,
            b'(' => TokenKind::OpenParen,
            b')' => TokenKind::CloseParen,
            b'[' => TokenKind::OpenBracket,
            b']' => TokenKind::CloseBracket,
            b'?' => TokenKind::Question,
            b'@' => TokenKind::At,
            _ => {
                // Not an operator — back up
                self.pos = start;
                return None;
            }
        };

        Some(Token::new(kind, Span::new(start, self.pos)))
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<(TokenKind, &str)> {
        let tokens = Lexer::tokenize(src);
        tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| (t.kind, t.text(src)))
            .collect()
    }

    #[test]
    fn empty_source() {
        let tokens = Lexer::tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn identifiers_and_keywords() {
        let result = lex("int foo = 42;");
        assert_eq!(
            result,
            vec![
                (TokenKind::Int, "int"),
                (TokenKind::Identifier, "foo"),
                (TokenKind::Assign, "="),
                (TokenKind::IntConstant, "42"),
                (TokenKind::Semicolon, ";"),
            ]
        );
    }

    #[test]
    fn operators() {
        let result = lex("a + b * c");
        assert_eq!(
            result,
            vec![
                (TokenKind::Identifier, "a"),
                (TokenKind::Plus, "+"),
                (TokenKind::Identifier, "b"),
                (TokenKind::Star, "*"),
                (TokenKind::Identifier, "c"),
            ]
        );
    }

    #[test]
    fn compound_operators() {
        let result = lex("+= -= *= /= %= **= |= &= ^= <<= >>= >>>=");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::AddAssign,
                TokenKind::SubAssign,
                TokenKind::MulAssign,
                TokenKind::DivAssign,
                TokenKind::ModAssign,
                TokenKind::PowAssign,
                TokenKind::OrAssign,
                TokenKind::AndAssign,
                TokenKind::XorAssign,
                TokenKind::ShiftLeftAssign,
                TokenKind::ShiftRightLAssign,
                TokenKind::ShiftRightAAssign,
            ]
        );
    }

    #[test]
    fn comparison_operators() {
        let result = lex("== != < > <= >=");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Equal,
                TokenKind::NotEqual,
                TokenKind::Less,
                TokenKind::Greater,
                TokenKind::LessEqual,
                TokenKind::GreaterEqual,
            ]
        );
    }

    #[test]
    fn logical_operators() {
        let result = lex("&& || ^^ ! and or xor not");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Xor,
                TokenKind::Not,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Xor,
                TokenKind::Not,
            ]
        );
    }

    #[test]
    fn bitwise_operators() {
        let result = lex("& | ^ ~ << >> >>>");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Amp,
                TokenKind::Pipe,
                TokenKind::Caret,
                TokenKind::Tilde,
                TokenKind::ShiftLeft,
                TokenKind::ShiftRight,
                TokenKind::ShiftRightArith,
            ]
        );
    }

    #[test]
    fn integer_literals() {
        let result = lex("0 42 1234567890");
        assert_eq!(
            result,
            vec![
                (TokenKind::IntConstant, "0"),
                (TokenKind::IntConstant, "42"),
                (TokenKind::IntConstant, "1234567890"),
            ]
        );
    }

    #[test]
    fn hex_literal() {
        let result = lex("0xFF 0x1234ABCD");
        assert_eq!(
            result,
            vec![
                (TokenKind::BitsConstant, "0xFF"),
                (TokenKind::BitsConstant, "0x1234ABCD"),
            ]
        );
    }

    #[test]
    fn binary_literal() {
        let result = lex("0b1010 0B1111_0000");
        assert_eq!(
            result,
            vec![
                (TokenKind::BitsConstant, "0b1010"),
                (TokenKind::BitsConstant, "0B1111_0000"),
            ]
        );
    }

    #[test]
    fn float_literals() {
        let result = lex("3.14 3.14f 1.0e10 1.5e-3f");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::DoubleConstant, // 3.14 (no suffix = double)
                TokenKind::FloatConstant,  // 3.14f
                TokenKind::DoubleConstant, // 1.0e10
                TokenKind::FloatConstant,  // 1.5e-3f
            ]
        );
    }

    #[test]
    fn string_literals() {
        let result = lex(r#""hello" 'c'"#);
        assert_eq!(
            result,
            vec![
                (TokenKind::StringConstant, "\"hello\""),
                (TokenKind::StringConstant, "'c'"),
            ]
        );
    }

    #[test]
    fn heredoc_string() {
        let result = lex(r#""""heredoc string""""#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, TokenKind::HeredocStringConstant);
    }

    #[test]
    fn escaped_string() {
        let result = lex(r#""hello\nworld""#);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, TokenKind::StringConstant);
    }

    #[test]
    fn comments() {
        let all = Lexer::tokenize_all("// comment\nint x; /* block */");
        let kinds: Vec<TokenKind> = all
            .iter()
            .filter(|t| t.kind != TokenKind::WhiteSpace)
            .map(|t| t.kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::LineComment,
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::Semicolon,
                TokenKind::BlockComment,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn handle_and_scope() {
        let result = lex("Player@ p; Game::Entities::Player");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::At,
                TokenKind::Identifier,
                TokenKind::Semicolon,
                TokenKind::Identifier,
                TokenKind::Scope,
                TokenKind::Identifier,
                TokenKind::Scope,
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn power_operator() {
        let result = lex("a ** b **= c");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::StarStar,
                TokenKind::Identifier,
                TokenKind::PowAssign,
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn inc_dec() {
        let result = lex("i++ j--");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Inc,
                TokenKind::Identifier,
                TokenKind::Dec,
            ]
        );
    }

    #[test]
    fn is_and_not_is() {
        let result = lex("a is b !is c");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Is,
                TokenKind::Identifier,
                TokenKind::NotIs,
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn full_statement() {
        let result = lex("void main() { int x = 10; if (x > 5) return; }");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Void,
                TokenKind::Identifier,
                TokenKind::OpenParen,
                TokenKind::CloseParen,
                TokenKind::OpenBrace,
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::Assign,
                TokenKind::IntConstant,
                TokenKind::Semicolon,
                TokenKind::If,
                TokenKind::OpenParen,
                TokenKind::Identifier,
                TokenKind::Greater,
                TokenKind::IntConstant,
                TokenKind::CloseParen,
                TokenKind::Return,
                TokenKind::Semicolon,
                TokenKind::CloseBrace,
            ]
        );
    }

    #[test]
    fn class_declaration() {
        let result = lex("class Player : IEntity { private int health; }");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Class,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
                TokenKind::OpenBrace,
                TokenKind::Private,
                TokenKind::Int,
                TokenKind::Identifier,
                TokenKind::Semicolon,
                TokenKind::CloseBrace,
            ]
        );
    }

    #[test]
    fn ternary_operator() {
        let result = lex("a ? b : c");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Identifier,
                TokenKind::Question,
                TokenKind::Identifier,
                TokenKind::Colon,
                TokenKind::Identifier,
            ]
        );
    }

    #[test]
    fn type_keywords() {
        let result = lex("int8 int16 int64 uint8 uint16 uint64 float double bool auto");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Int8,
                TokenKind::Int16,
                TokenKind::Int64,
                TokenKind::UInt8,
                TokenKind::UInt16,
                TokenKind::UInt64,
                TokenKind::Float,
                TokenKind::Double,
                TokenKind::Bool,
                TokenKind::Auto,
            ]
        );
    }

    #[test]
    fn dot_not_float() {
        // "a.b" should be identifier, dot, identifier (not a float)
        let result = lex("a.b");
        let kinds: Vec<TokenKind> = result.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds,
            vec![TokenKind::Identifier, TokenKind::Dot, TokenKind::Identifier]
        );
    }
}
