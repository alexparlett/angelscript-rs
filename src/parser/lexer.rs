use crate::parser::error::*;
use crate::parser::token::*;

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments()?;

        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::Eof));
        }

        let start = self.current_position();
        let ch = self.current_char();

        match ch {
            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier(),

            // Numbers
            '0'..='9' => self.read_number(),

            // Strings
            '"' | '\'' => self.read_string(ch),

            // Preprocessor
            '#' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::Hash, start))
            }

            // Operators and punctuation
            '+' => self.read_plus(),
            '-' => self.read_minus(),
            '*' => self.read_star(),
            '/' => self.read_slash(),
            '%' => self.read_percent(),
            '=' => self.read_equals(),
            '!' => self.read_bang(),
            '<' => self.read_less(),
            '>' => self.read_greater(),
            '&' => self.read_ampersand(),
            '|' => self.read_pipe(),
            '^' => self.read_caret(),
            '~' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::BitNot, start))
            }
            '@' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::At, start))
            }
            '?' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::Question, start))
            }

            // Delimiters
            '(' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::LParen, start))
            }
            ')' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::RParen, start))
            }
            '[' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::LBracket, start))
            }
            ']' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::RBracket, start))
            }
            '{' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::LBrace, start))
            }
            '}' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::RBrace, start))
            }
            ',' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::Comma, start))
            }
            ';' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::Semicolon, start))
            }
            '.' => {
                self.advance();
                Ok(self.make_token_from(TokenKind::Dot, start))
            }
            ':' => self.read_colon(),

            _ => Err(ParseError::SyntaxError {
                span: self.make_span_from(start),
                message: format!("Unexpected character: '{}'", ch),
            }),
        }
    }

    // Operator readers

    fn read_plus(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('+') {
            Ok(self.make_token_from(TokenKind::Inc, start))
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::AddAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::Add, start))
        }
    }

    fn read_minus(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('-') {
            Ok(self.make_token_from(TokenKind::Dec, start))
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::SubAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::Sub, start))
        }
    }

    fn read_star(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('*') {
            if self.match_char('=') {
                Ok(self.make_token_from(TokenKind::PowAssign, start))
            } else {
                Ok(self.make_token_from(TokenKind::Pow, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::MulAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::Mul, start))
        }
    }

    fn read_slash(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::DivAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::Div, start))
        }
    }

    fn read_percent(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::ModAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::Mod, start))
        }
    }

    fn read_equals(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::Eq, start))
        } else {
            Ok(self.make_token_from(TokenKind::Assign, start))
        }
    }

    fn read_bang(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::Ne, start))
        } else if self.peek_str("is") {
            self.advance();
            self.advance();
            Ok(self.make_token_from(TokenKind::IsNot, start))
        } else {
            Ok(self.make_token_from(TokenKind::Not, start))
        }
    }

    fn read_less(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('<') {
            if self.match_char('=') {
                Ok(self.make_token_from(TokenKind::ShlAssign, start))
            } else {
                Ok(self.make_token_from(TokenKind::Shl, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::Le, start))
        } else {
            Ok(self.make_token_from(TokenKind::Lt, start))
        }
    }

    fn read_greater(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('>') {
            if self.match_char('>') {
                if self.match_char('=') {
                    Ok(self.make_token_from(TokenKind::UShrAssign, start))
                } else {
                    Ok(self.make_token_from(TokenKind::UShr, start))
                }
            } else if self.match_char('=') {
                Ok(self.make_token_from(TokenKind::ShrAssign, start))
            } else {
                Ok(self.make_token_from(TokenKind::Shr, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::Ge, start))
        } else {
            Ok(self.make_token_from(TokenKind::Gt, start))
        }
    }

    fn read_ampersand(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('&') {
            Ok(self.make_token_from(TokenKind::And, start))
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::BitAndAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::BitAnd, start))
        }
    }

    fn read_pipe(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('|') {
            Ok(self.make_token_from(TokenKind::Or, start))
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::BitOrAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::BitOr, start))
        }
    }

    fn read_caret(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char('^') {
            Ok(self.make_token_from(TokenKind::Xor, start))
        } else if self.match_char('=') {
            Ok(self.make_token_from(TokenKind::BitXorAssign, start))
        } else {
            Ok(self.make_token_from(TokenKind::BitXor, start))
        }
    }

    fn read_colon(&mut self) -> Result<Token> {
        let start = self.current_position();
        self.advance();

        if self.match_char(':') {
            Ok(self.make_token_from(TokenKind::DoubleColon, start))
        } else {
            Ok(self.make_token_from(TokenKind::Colon, start))
        }
    }

    // Complex token readers

    fn read_identifier(&mut self) -> Result<Token> {
        let start = self.current_position();
        let mut text = String::new();

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                text.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = TokenKind::keyword(&text).unwrap_or_else(|| TokenKind::Identifier(text));

        Ok(self.make_token_from(kind, start))
    }

    fn read_number(&mut self) -> Result<Token> {
        let start = self.current_position();
        let mut text = String::new();

        // Handle hex, binary, octal
        if self.current_char() == '0' && !self.is_at_end() {
            text.push(self.current_char());
            self.advance();

            if !self.is_at_end() {
                let next = self.current_char();
                match next {
                    'x' | 'X' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
                            text.push(self.current_char());
                            self.advance();
                        }
                        return Ok(self.make_token_from(TokenKind::Number(text), start));
                    }
                    'b' | 'B' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() && matches!(self.current_char(), '0' | '1') {
                            text.push(self.current_char());
                            self.advance();
                        }
                        return Ok(self.make_token_from(TokenKind::Bits(text), start));
                    }
                    'o' | 'O' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() && matches!(self.current_char(), '0'..='7') {
                            text.push(self.current_char());
                            self.advance();
                        }
                        return Ok(self.make_token_from(TokenKind::Number(text), start));
                    }
                    'd' | 'D' => {
                        // Decimal bits notation
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() && self.current_char().is_ascii_digit() {
                            text.push(self.current_char());
                            self.advance();
                        }
                        return Ok(self.make_token_from(TokenKind::Bits(text), start));
                    }
                    _ => {}
                }
            }
        }

        // Regular decimal number
        while !self.is_at_end() && self.current_char().is_ascii_digit() {
            text.push(self.current_char());
            self.advance();
        }

        // Decimal point
        if !self.is_at_end() && self.current_char() == '.' {
            let next_pos = self.pos + 1;
            if next_pos < self.chars.len() && self.chars[next_pos].is_ascii_digit() {
                text.push('.');
                self.advance();
                while !self.is_at_end() && self.current_char().is_ascii_digit() {
                    text.push(self.current_char());
                    self.advance();
                }
            }
        }

        // Exponent
        if !self.is_at_end() && matches!(self.current_char(), 'e' | 'E') {
            text.push(self.current_char());
            self.advance();
            if !self.is_at_end() && matches!(self.current_char(), '+' | '-') {
                text.push(self.current_char());
                self.advance();
            }
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                text.push(self.current_char());
                self.advance();
            }
        }

        Ok(self.make_token_from(TokenKind::Number(text), start))
    }

    fn read_string(&mut self, quote: char) -> Result<Token> {
        let start = self.current_position();
        let mut text = String::new();

        self.advance(); // Skip opening quote

        while !self.is_at_end() {
            let ch = self.current_char();

            if ch == quote {
                self.advance(); // Skip closing quote
                return Ok(self.make_token_from(TokenKind::String(text), start));
            }

            if ch == '\\' {
                self.advance();
                if !self.is_at_end() {
                    let escaped = self.current_char();
                    text.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        '0' => '\0',
                        _ => escaped,
                    });
                    self.advance();
                }
            } else {
                text.push(ch);
                self.advance();
            }
        }

        Err(ParseError::InvalidString {
            span: self.make_span_from(start),
            message: "Unterminated string literal".to_string(),
        })
    }

    // Helper methods

    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        loop {
            if self.is_at_end() {
                break;
            }

            let ch = self.current_char();

            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            // Single-line comment
            if ch == '/' && self.peek() == Some('/') {
                while !self.is_at_end() && self.current_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            // Multi-line comment
            if ch == '/' && self.peek() == Some('*') {
                self.advance(); // /
                self.advance(); // *

                while !self.is_at_end() {
                    if self.current_char() == '*' && self.peek() == Some('/') {
                        self.advance(); // *
                        self.advance(); // /
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            break;
        }

        Ok(())
    }

    fn current_char(&self) -> char {
        self.chars[self.pos]
    }

    fn peek(&self) -> Option<char> {
        if self.pos + 1 < self.chars.len() {
            Some(self.chars[self.pos + 1])
        } else {
            None
        }
    }

    fn peek_str(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            if self.pos + i >= self.chars.len() || self.chars[self.pos + i] != ch {
                return false;
            }
        }
        true
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.current_char() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn current_position(&self) -> Position {
        Position::new(self.line, self.column, self.pos)
    }

    fn make_token(&self, kind: TokenKind) -> Token {
        Token::new(
            kind,
            Span::new(
                self.current_position(),
                self.current_position(),
                String::new(),
            ),
        )
    }

    fn make_token_from(&self, kind: TokenKind, start: Position) -> Token {
        let end = self.current_position();
        let source = self.source[start.offset..end.offset].to_string();
        Token::new(kind, Span::new(start, end, source))
    }

    fn make_span_from(&self, start: Position) -> Span {
        let end = self.current_position();
        let source = self.source[start.offset..end.offset].to_string();
        Span::new(start, end, source)
    }
}
