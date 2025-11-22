use crate::core::error::*;
use crate::core::span::SpanBuilder;
use crate::parser::token::*;

pub struct Lexer {
    source: String,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    span_builder: SpanBuilder,
    include_spans: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self::new_with_name("<input>", source, false)
    }

    pub fn new_with_name(source_name: &str, source: &str, include_spans: bool) -> Self {
        let span_builder = SpanBuilder::new(source_name.to_string(), source);

        Self {
            source: source.to_string(),
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            span_builder,
            include_spans,
        }
    }

    pub fn tokenize(mut self) -> ParseResult<Vec<Token>> {
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

    fn next_token(&mut self) -> ParseResult<Token> {
        self.skip_whitespace_and_comments()?;

        if self.is_at_end() {
            return Ok(Token::eof());
        }

        let start_offset = self.pos;
        let ch = self.current_char();

        let token = match ch {
            'a'..='z' | 'A'..='Z' | '_' => self.read_identifier(start_offset)?,
            '0'..='9' => self.read_number(start_offset)?,
            '"' | '\'' => self.read_string(ch, start_offset)?,
            '#' => {
                self.advance();
                self.make_token(TokenKind::Hash, start_offset)
            }
            '+' => self.read_plus(start_offset)?,
            '-' => self.read_minus(start_offset)?,
            '*' => self.read_star(start_offset)?,
            '/' => self.read_slash(start_offset)?,
            '%' => self.read_percent(start_offset)?,
            '=' => self.read_equals(start_offset)?,
            '!' => self.read_bang(start_offset)?,
            '<' => self.read_less(start_offset)?,
            '>' => self.read_greater(start_offset)?,
            '&' => self.read_ampersand(start_offset)?,
            '|' => self.read_pipe(start_offset)?,
            '^' => self.read_caret(start_offset)?,
            '~' => {
                self.advance();
                self.make_token(TokenKind::BitNot, start_offset)
            }
            '@' => {
                self.advance();
                self.make_token(TokenKind::At, start_offset)
            }
            '?' => {
                self.advance();
                self.make_token(TokenKind::Question, start_offset)
            }
            '(' => {
                self.advance();
                self.make_token(TokenKind::LParen, start_offset)
            }
            ')' => {
                self.advance();
                self.make_token(TokenKind::RParen, start_offset)
            }
            '[' => {
                self.advance();
                self.make_token(TokenKind::LBracket, start_offset)
            }
            ']' => {
                self.advance();
                self.make_token(TokenKind::RBracket, start_offset)
            }
            '{' => {
                self.advance();
                self.make_token(TokenKind::LBrace, start_offset)
            }
            '}' => {
                self.advance();
                self.make_token(TokenKind::RBrace, start_offset)
            }
            ',' => {
                self.advance();
                self.make_token(TokenKind::Comma, start_offset)
            }
            ';' => {
                self.advance();
                self.make_token(TokenKind::Semicolon, start_offset)
            }
            '.' => {
                self.advance();
                self.make_token(TokenKind::Dot, start_offset)
            }
            ':' => self.read_colon(start_offset)?,
            _ => {
                return Err(ParseError::SyntaxError {
                    span: if self.include_spans {
                        Some(self.span_builder.span(start_offset, self.pos))
                    } else {
                        None
                    },
                    message: format!("Unexpected character: '{}'", ch),
                });
            }
        };

        Ok(token)
    }

    fn read_plus(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('+') {
            Ok(self.make_token(TokenKind::Inc, start))
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::AddAssign, start))
        } else {
            Ok(self.make_token(TokenKind::Add, start))
        }
    }

    fn read_minus(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('-') {
            Ok(self.make_token(TokenKind::Dec, start))
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::SubAssign, start))
        } else {
            Ok(self.make_token(TokenKind::Sub, start))
        }
    }

    fn read_star(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('*') {
            if self.match_char('=') {
                Ok(self.make_token(TokenKind::PowAssign, start))
            } else {
                Ok(self.make_token(TokenKind::Pow, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::MulAssign, start))
        } else {
            Ok(self.make_token(TokenKind::Mul, start))
        }
    }

    fn read_slash(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('=') {
            Ok(self.make_token(TokenKind::DivAssign, start))
        } else {
            Ok(self.make_token(TokenKind::Div, start))
        }
    }

    fn read_percent(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('=') {
            Ok(self.make_token(TokenKind::ModAssign, start))
        } else {
            Ok(self.make_token(TokenKind::Mod, start))
        }
    }

    fn read_equals(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('=') {
            Ok(self.make_token(TokenKind::Eq, start))
        } else {
            Ok(self.make_token(TokenKind::Assign, start))
        }
    }

    fn read_bang(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('=') {
            Ok(self.make_token(TokenKind::Ne, start))
        } else if self.peek_keyword("is") {
            self.advance();
            self.advance();
            Ok(self.make_token(TokenKind::IsNot, start))
        } else {
            Ok(self.make_token(TokenKind::Not, start))
        }
    }

    fn read_less(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('<') {
            if self.match_char('=') {
                Ok(self.make_token(TokenKind::ShlAssign, start))
            } else {
                Ok(self.make_token(TokenKind::Shl, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::Le, start))
        } else {
            Ok(self.make_token(TokenKind::Lt, start))
        }
    }

    fn read_greater(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('>') {
            if self.match_char('>') {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::UShrAssign, start))
                } else {
                    Ok(self.make_token(TokenKind::UShr, start))
                }
            } else if self.match_char('=') {
                Ok(self.make_token(TokenKind::ShrAssign, start))
            } else {
                Ok(self.make_token(TokenKind::Shr, start))
            }
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::Ge, start))
        } else {
            Ok(self.make_token(TokenKind::Gt, start))
        }
    }

    fn read_ampersand(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('&') {
            Ok(self.make_token(TokenKind::And, start))
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::BitAndAssign, start))
        } else {
            Ok(self.make_token(TokenKind::BitAnd, start))
        }
    }

    fn read_pipe(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('|') {
            Ok(self.make_token(TokenKind::Or, start))
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::BitOrAssign, start))
        } else {
            Ok(self.make_token(TokenKind::BitOr, start))
        }
    }

    fn read_caret(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char('^') {
            Ok(self.make_token(TokenKind::Xor, start))
        } else if self.match_char('=') {
            Ok(self.make_token(TokenKind::BitXorAssign, start))
        } else {
            Ok(self.make_token(TokenKind::BitXor, start))
        }
    }

    fn read_colon(&mut self, start: usize) -> ParseResult<Token> {
        self.advance();
        if self.match_char(':') {
            Ok(self.make_token(TokenKind::DoubleColon, start))
        } else {
            Ok(self.make_token(TokenKind::Colon, start))
        }
    }

    fn read_identifier(&mut self, start: usize) -> ParseResult<Token> {
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
        Ok(self.make_token(kind, start))
    }

    fn read_number(&mut self, start: usize) -> ParseResult<Token> {
        let mut text = String::new();

        // Handle hex, binary, octal, decimal prefixes
        if self.current_char() == '0' && !self.is_at_end() {
            text.push(self.current_char());
            self.advance();

            if !self.is_at_end() {
                let next = self.current_char();
                match next {
                    'x' | 'X' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() {
                            let ch = self.current_char();
                            if ch.is_ascii_hexdigit() {
                                text.push(ch);
                                self.advance();
                            } else if ch == '\'' {
                                // Skip separator
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        return Ok(self.make_token(TokenKind::Number(text), start));
                    }
                    'b' | 'B' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() {
                            let ch = self.current_char();
                            if matches!(ch, '0' | '1') {
                                text.push(ch);
                                self.advance();
                            } else if ch == '\'' {
                                // Skip separator
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        return Ok(self.make_token(TokenKind::Bits(text), start));
                    }
                    'o' | 'O' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() {
                            let ch = self.current_char();
                            if matches!(ch, '0'..='7') {
                                text.push(ch);
                                self.advance();
                            } else if ch == '\'' {
                                // Skip separator
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        return Ok(self.make_token(TokenKind::Number(text), start));
                    }
                    'd' | 'D' => {
                        text.push(next);
                        self.advance();
                        while !self.is_at_end() {
                            let ch = self.current_char();
                            if ch.is_ascii_digit() {
                                text.push(ch);
                                self.advance();
                            } else if ch == '\'' {
                                // Skip separator
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        return Ok(self.make_token(TokenKind::Number(text), start));
                    }
                    _ => {}
                }
            }
        }

        // Regular decimal number with separators
        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                text.push(ch);
                self.advance();
            } else if ch == '\'' {
                // Skip separator, continue
                self.advance();
            } else {
                break;
            }
        }

        // Decimal point
        if !self.is_at_end() && self.current_char() == '.' {
            let next_pos = self.pos + 1;
            if next_pos < self.chars.len() && self.chars[next_pos].is_ascii_digit() {
                text.push('.');
                self.advance();
                while !self.is_at_end() {
                    let ch = self.current_char();
                    if ch.is_ascii_digit() {
                        text.push(ch);
                        self.advance();
                    } else if ch == '\'' {
                        // Skip separator in fractional part
                        self.advance();
                    } else {
                        break;
                    }
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
            while !self.is_at_end() {
                let ch = self.current_char();
                if ch.is_ascii_digit() {
                    text.push(ch);
                    self.advance();
                } else if ch == '\'' {
                    // Skip separator in exponent
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Ok(self.make_token(TokenKind::Number(text), start))
    }

    fn read_string(&mut self, quote: char, start_offset: usize) -> ParseResult<Token> {
        let mut text = String::new();

        self.advance(); // Skip opening quote

        // Check for heredoc (triple quotes)
        if quote == '"' && !self.is_at_end() && self.current_char() == '"' {
            self.advance();
            if !self.is_at_end() && self.current_char() == '"' {
                self.advance();
                // Heredoc mode: read until """
                return self.read_heredoc(start_offset);
            } else {
                // Just two quotes, return empty string
                return Ok(self.make_token(TokenKind::String(String::new()), start_offset));
            }
        }

        while !self.is_at_end() {
            let ch = self.current_char();

            if ch == quote {
                self.advance(); // Skip closing quote
                return Ok(self.make_token(TokenKind::String(text), start_offset));
            }

            if ch == '\\' {
                self.advance();
                if !self.is_at_end() {
                    let escaped = self.current_char();
                    let result = match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        '0' => '\0',
                        'a' => '\x07', // alert/bell
                        'b' => '\x08', // backspace
                        'f' => '\x0C', // form feed
                        'v' => '\x0B', // vertical tab
                        'x' => {
                            self.advance();
                            let mut hex = String::new();
                            for _ in 0..2 {
                                if !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
                                    hex.push(self.current_char());
                                    self.advance();
                                }
                            }
                            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                text.push(byte as char);
                                continue; // Skip the advance at line 535
                            } else {
                                '?'
                            }
                        }
                        'u' => {
                            self.advance();
                            let mut hex = String::new();
                            for _ in 0..4 {
                                if !self.is_at_end() && self.current_char().is_ascii_hexdigit() {
                                    hex.push(self.current_char());
                                    self.advance();
                                }
                            }
                            if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                if let Some(ch) = char::from_u32(code) {
                                    text.push(ch);
                                    continue; // Skip the advance at line 535
                                }
                            }
                            '?'
                        }
                        _ => escaped,
                    };
                    text.push(result);
                    self.advance();
                }
            } else {
                text.push(ch);
                self.advance();
            }
        }

        Err(ParseError::InvalidString {
            span: if self.include_spans {
                Some(self.span_builder.span(start_offset, self.pos))
            } else {
                None
            },
            message: "Unterminated string literal".to_string(),
        })
    }

    fn read_heredoc(&mut self, start_offset: usize) -> ParseResult<Token> {
        let mut text = String::new();

        // Read until we find """
        while !self.is_at_end() {
            if self.current_char() == '"' {
                // Check for """
                if self.pos + 2 < self.chars.len()
                    && self.chars[self.pos + 1] == '"'
                    && self.chars[self.pos + 2] == '"'
                {
                    // Found closing """
                    self.advance(); // First "
                    self.advance(); // Second "
                    self.advance(); // Third "
                    return Ok(self.make_token(TokenKind::String(text), start_offset));
                }
            }

            // Regular character
            text.push(self.current_char());
            self.advance();
        }

        Err(ParseError::InvalidString {
            span: if self.include_spans {
                Some(self.span_builder.span(start_offset, self.pos))
            } else {
                None
            },
            message: "Unterminated heredoc string (missing closing \"\"\")".to_string(),
        })
    }

    fn skip_whitespace_and_comments(&mut self) -> ParseResult<()> {
        loop {
            if self.is_at_end() {
                break;
            }

            let ch = self.current_char();

            if ch.is_whitespace() {
                self.advance();
                continue;
            }

            if ch == '/' && self.peek() == Some('/') {
                while !self.is_at_end() && self.current_char() != '\n' {
                    self.advance();
                }
                continue;
            }

            if ch == '/' && self.peek() == Some('*') {
                self.advance();
                self.advance();

                while !self.is_at_end() {
                    if self.current_char() == '*' && self.peek() == Some('/') {
                        self.advance();
                        self.advance();
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

    fn peek_keyword(&self, keyword: &str) -> bool {
        let chars: Vec<char> = keyword.chars().collect();

        if self.pos + chars.len() > self.chars.len() {
            return false;
        }

        for (i, &ch) in chars.iter().enumerate() {
            if self.chars[self.pos + i] != ch {
                return false;
            }
        }

        let next_pos = self.pos + chars.len();
        if next_pos < self.chars.len() {
            let next_char = self.chars[next_pos];
            if next_char.is_alphanumeric() || next_char == '_' {
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

    fn make_token(&self, kind: TokenKind, start_offset: usize) -> Token {
        let lexeme = self.source[start_offset..self.pos].to_string();
        let span = if self.include_spans {
            Some(self.span_builder.span(start_offset, self.pos))
        } else {
            None
        };

        Token::new(kind, span, lexeme)
    }
}
