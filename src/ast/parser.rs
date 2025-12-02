//! Parser infrastructure for AngelScript.
//!
//! Provides the main [`Parser`] struct with token navigation and basic
//! parsing infrastructure.

use crate::ast::{ParseError, ParseErrorKind, ParseErrors};
use crate::lexer::{Lexer, Span, Token, TokenKind};
use bumpalo::Bump;

/// The main parser for AngelScript source code.
///
/// The parser uses a lookahead approach with buffered tokens, allowing
/// arbitrary peeking ahead without consuming tokens.
///
/// The `'ast` lifetime refers to the arena where AST nodes and token
/// lexemes are allocated. The source string only needs to live during
/// the call to `new()` - after tokenization, all string content is
/// copied into the arena.
pub struct Parser<'ast> {
    /// Buffered tokens for lookahead (lexemes allocated in arena)
    pub(super) buffer: Vec<Token<'ast>>,
    /// Current position in the buffer
    pub(super) position: usize,
    /// Accumulated parse errors
    pub(super) errors: ParseErrors,
    /// Whether we're in panic mode (skipping to synchronization point)
    pub(super) panic_mode: bool,
    /// Arena allocator for AST nodes
    pub(super) arena: &'ast Bump,
}

impl<'ast> Parser<'ast> {
    /// Create a new parser for the given source code.
    ///
    /// This performs eager tokenization - the entire source is tokenized
    /// upfront into a buffer. This eliminates the overhead of lazy tokenization
    /// and provides better performance for complete file parsing.
    ///
    /// The source string is only needed during this call - all token lexemes
    /// are copied into the arena, allowing the source to be freed afterward.
    pub fn new(source: &str, arena: &'ast Bump) -> Self {
        let mut lexer = Lexer::new(source, arena);
        let mut buffer = Vec::with_capacity(Self::estimate_token_count(source));
        let mut errors = ParseErrors::new();

        // Pre-tokenize the entire source
        loop {
            let token = lexer.next_token();

            // Collect any lexer errors immediately
            if token.kind == TokenKind::Error {
                for lexer_error in lexer.take_errors() {
                    let parse_error = ParseError::new(
                        ParseErrorKind::InvalidSyntax,
                        lexer_error.span,
                        format!("lexer error: {}", lexer_error.message),
                    );
                    errors.push(parse_error);
                }
            }

            let is_eof = token.kind == TokenKind::Eof;
            buffer.push(token);

            if is_eof {
                break;
            }
        }

        Self {
            buffer,
            position: 0,
            errors,
            panic_mode: false,
            arena,
        }
    }

    /// Estimate the number of tokens based on source length.
    ///
    /// Uses a heuristic of ~10 characters per token on average.
    /// Clamped to a minimum of 512 and maximum of 16384 to prevent
    /// excessive allocation for very small or very large files.
    fn estimate_token_count(source: &str) -> usize {
        let estimate = source.len() / 10;
        estimate.clamp(512, 16384)
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Take the errors, leaving an empty error collection.
    pub fn take_errors(&mut self) -> ParseErrors {
        std::mem::take(&mut self.errors)
    }

    // ========================================================================
    // Token Navigation
    // ========================================================================

    /// Peek at the current token without consuming it.
    pub fn peek(&mut self) -> &Token<'ast> {
        self.fill_buffer(1);
        &self.buffer[self.position]
    }

    /// Peek ahead n tokens without consuming.
    pub fn peek_nth(&mut self, n: usize) -> &Token<'ast> {
        self.fill_buffer(n + 1);
        &self.buffer[self.position + n]
    }

    /// Get the current token and advance to the next.
    pub fn advance(&mut self) -> Token<'ast> {
        self.fill_buffer(1);
        let token = self.buffer[self.position];
        self.position += 1;
        token
    }

    /// Check if the current token matches the given kind.
    pub fn check(&mut self, kind: TokenKind) -> bool {
        self.peek().kind == kind
    }

    /// Check if the current token is EOF.
    pub fn is_eof(&mut self) -> bool {
        self.check(TokenKind::Eof)
    }

    /// If the current token matches the given kind, consume it and return Some.
    /// Otherwise, return None without consuming.
    pub fn eat(&mut self, kind: TokenKind) -> Option<Token<'ast>> {
        if self.check(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    /// Expect the current token to be of the given kind.
    /// If it matches, consume and return it. Otherwise, return an error.
    pub fn expect(&mut self, kind: TokenKind) -> Result<Token<'ast>, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = *self.peek();
            Err(ParseError::new(
                ParseErrorKind::ExpectedToken,
                token.span,
                format!("expected {}, found {}", kind, token.kind),
            ))
        }
    }

    /// Check if the current token is an identifier with the given name.
    /// This is used for contextual keywords.
    pub fn check_contextual(&mut self, name: &str) -> bool {
        let token = self.peek();
        token.kind == TokenKind::Identifier && token.lexeme == name
    }

    /// Consume an identifier if it matches the given contextual keyword.
    pub fn eat_contextual(&mut self, name: &str) -> Option<Token<'ast>> {
        if self.check_contextual(name) {
            Some(self.advance())
        } else {
            None
        }
    }

    /// Fill the token buffer to have at least `needed` tokens available.
    ///
    /// With eager tokenization, this is now a no-op since all tokens are
    /// already buffered during Parser::new(). We keep this method for API
    /// compatibility and to avoid panics if code tries to read past EOF.
    #[inline]
    fn fill_buffer(&mut self, _needed: usize) {
        // No-op: all tokens are pre-loaded during construction
    }

    // ========================================================================
    // Error Handling
    // ========================================================================

    /// Record a parse error.
    pub fn error(&mut self, kind: ParseErrorKind, span: Span, message: impl Into<String>) {
        self.errors.push(ParseError::new(kind, span, message));
        self.panic_mode = true;
    }

    /// Synchronize after an error by skipping tokens until a safe point.
    ///
    /// Safe synchronization points are:
    /// - Semicolons
    /// - Closing braces
    /// - Statement keywords (if, while, for, return, etc.)
    /// - Declaration keywords (class, function, etc.)
    pub fn synchronize(&mut self) {
        self.panic_mode = false;

        // CRITICAL: Always advance at least once to prevent infinite loops
        // If we're already at a sync point and don't advance, the caller
        // will try parsing again, fail immediately, call synchronize() again,
        // and we're stuck in an infinite loop.
        let start_pos = self.position;

        while !self.is_eof() {
            // If we just passed a semicolon, we're at a statement boundary
            if self.buffer.get(self.position.saturating_sub(1))
                .is_some_and(|t| t.kind == TokenKind::Semicolon)
            {
                // Only stop if we've advanced at least once
                if self.position > start_pos {
                    return;
                }
            }

            // Check if we're at a safe synchronization point
            match self.peek().kind {
                TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Enum
                | TokenKind::FuncDef
                | TokenKind::Namespace
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Switch
                | TokenKind::Try => {
                    // Only stop at sync point if we've advanced at least once
                    if self.position > start_pos {
                        return;
                    }
                    // Otherwise, advance past this sync point token
                    self.advance();
                }
                
                // For RightBrace, always advance past it
                TokenKind::RightBrace => {
                    self.advance();
                    return;
                }
                
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ========================================================================
    // Disambiguation Helpers (Phase 6)
    // ========================================================================

    /// Check if the current position starts a type expression.
    ///
    /// This is used to disambiguate contexts where types can appear.
    /// Returns true if the current token can start a type.
    pub fn is_type_start(&mut self) -> bool {
        match self.peek().kind {
            // Primitive types
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
            | TokenKind::Double => true,

            // Const modifier
            TokenKind::Const => true,

            // Identifier (could be class/typedef name)
            TokenKind::Identifier => true,

            // Scope resolution
            TokenKind::ColonColon => true,

            // Auto type
            TokenKind::Auto => true,

            _ => false,
        }
    }

    /// Check if the current position looks like a variable declaration.
    ///
    /// This helps disambiguate between variable declarations and other
    /// statements in contexts where both are possible.
    pub fn is_var_decl(&mut self) -> bool {
        if !self.is_type_start() {
            return false;
        }

        // Save position
        let saved_pos = self.position;

        // Try to skip past a type expression
        let is_var = self.try_skip_type();

        // Restore position
        self.position = saved_pos;

        is_var
    }

    /// Try to skip past a type expression and check if it's followed by an identifier.
    ///
    /// This is a lookahead helper for variable declaration detection.
    fn try_skip_type(&mut self) -> bool {
        // Skip optional const
        self.eat(TokenKind::Const);

        // Skip optional scope
        if self.eat(TokenKind::ColonColon).is_some() {
            // Global scope
            if !self.check(TokenKind::Identifier) {
                return false;
            }
            self.advance();
        }

        // Skip the base type (either primitive keyword or identifier)
        if self.is_primitive_type() {
            // Primitive type keyword: int, float, bool, etc.
            self.advance();
        } else if self.check(TokenKind::Identifier) {
            // Named type: Handle scope::identifier
            while self.check(TokenKind::Identifier) {
                self.advance();
                if self.eat(TokenKind::ColonColon).is_some() {
                    continue;
                }
                break;
            }
        } else {
            // Not a valid type
            return false;
        }

        // Skip template arguments
        if self.check(TokenKind::Less)
            && !self.try_skip_template_args() {
                return false;
            }

        // Skip type suffixes (@, &)
        // Note: [] is NOT a type suffix - it's only an index operator
        while matches!(
            self.peek().kind,
            TokenKind::At | TokenKind::Amp
        ) {
            if self.eat(TokenKind::At).is_some() {
                self.eat(TokenKind::Const);
            } else if self.eat(TokenKind::Amp).is_some() {
                // Could be reference with in/out/inout
                if self.check(TokenKind::Identifier) {
                    let lexeme = self.peek().lexeme;
                    if lexeme == "in" || lexeme == "out" || lexeme == "inout" {
                        self.advance();
                    }
                }
            }
        }

        // Should be followed by an identifier (variable name)
        self.check(TokenKind::Identifier)
    }

    /// Check if current token is a primitive type keyword.
    pub fn is_primitive_type(&mut self) -> bool {
        matches!(
            self.peek().kind,
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

    /// Try to skip past template arguments in angle brackets.
    fn try_skip_template_args(&mut self) -> bool {
        if self.eat(TokenKind::Less).is_none() {
            return false;
        }

        let mut depth = 1;
        while depth > 0 && !self.is_eof() {
            match self.peek().kind {
                TokenKind::Less => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Greater => {
                    depth -= 1;
                    self.advance();
                }
                TokenKind::GreaterGreater => {
                    // >> should be treated as two >
                    depth -= 2;
                    self.advance();
                }
                TokenKind::GreaterGreaterGreater => {
                    // >>> should be treated as three >
                    depth -= 3;
                    self.advance();
                }
                TokenKind::Comma => {
                    self.advance();
                }
                _ => {
                    // Skip anything else inside template args
                    self.advance();
                }
            }
        }

        depth == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_creation() {
        let source = "int x = 42;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert_eq!(parser.peek().kind, TokenKind::Int);
    }

    #[test]
    fn token_navigation() {
        let source = "int x = 42;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert_eq!(parser.peek().kind, TokenKind::Int);
        assert_eq!(parser.peek_nth(1).kind, TokenKind::Identifier);
        assert_eq!(parser.peek_nth(2).kind, TokenKind::Equal);

        let token = parser.advance();
        assert_eq!(token.kind, TokenKind::Int);
        assert_eq!(parser.peek().kind, TokenKind::Identifier);
    }

    #[test]
    fn check_and_eat() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert!(parser.check(TokenKind::Int));
        assert!(!parser.check(TokenKind::Float));

        let int_token = parser.eat(TokenKind::Int);
        assert!(int_token.is_some());
        assert_eq!(int_token.unwrap().kind, TokenKind::Int);

        let float_token = parser.eat(TokenKind::Float);
        assert!(float_token.is_none());
    }

    #[test]
    fn expect_success() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let result = parser.expect(TokenKind::Int);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind, TokenKind::Int);
    }

    #[test]
    fn expect_failure() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let result = parser.expect(TokenKind::Float);
        assert!(result.is_err());
        // Record the error so we can check it
        if let Err(err) = result {
            parser.errors.push(err);
        }
        assert!(parser.has_errors());
    }

    #[test]
    fn contextual_keywords() {
        let source = "shared class";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert!(parser.check_contextual("shared"));
        let token = parser.eat_contextual("shared");
        assert!(token.is_some());
        assert_eq!(token.unwrap().lexeme, "shared");

        assert!(!parser.check_contextual("shared"));
    }

    #[test]
    fn error_accumulation() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        parser.error(ParseErrorKind::ExpectedToken, Span::new(1, 1, 3), "test error 1");
        parser.error(ParseErrorKind::ExpectedToken, Span::new(1, 5, 1), "test error 2");

        assert_eq!(parser.errors.len(), 2);
    }

    #[test]
    fn is_type_start_primitives() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_type_start());

        let source = "float x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_type_start());

        let source = "void func();";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_type_start());
    }

    #[test]
    fn is_type_start_const() {
        let source = "const int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_type_start());
    }

    #[test]
    fn is_type_start_identifier() {
        let source = "MyClass obj;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_type_start());
    }

    #[test]
    fn is_type_start_not_type() {
        let source = "if (x)";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(!parser.is_type_start());

        let source = "return x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(!parser.is_type_start());
    }

    #[test]
    fn is_var_decl_simple() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_var_decl());
    }

    #[test]
    fn is_var_decl_complex_type() {
        let source = "const array<int>@ x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_var_decl());
    }

    #[test]
    fn synchronize_on_semicolon() {
        let source = "error tokens here ; int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        // Trigger panic mode
        parser.panic_mode = true;
        let start_pos = parser.position;

        parser.synchronize();

        // Should have advanced past the semicolon
        assert!(parser.position > start_pos);
        assert!(!parser.panic_mode);
    }

    #[test]
    fn synchronize_on_keyword() {
        let source = "error tokens if (x) { }";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        parser.panic_mode = true;
        parser.synchronize();

        // Should stop at 'if' keyword
        assert!(parser.check(TokenKind::If));
        assert!(!parser.panic_mode);
    }

    #[test]
    fn synchronize_at_eof() {
        let source = "error tokens";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        parser.panic_mode = true;
        parser.synchronize();

        // Should reach EOF
        assert!(parser.is_eof());
    }

    #[test]
    fn synchronize_advances_at_least_once() {
        // Critical test: ensure we don't infinite loop
        let source = "if while for";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        parser.panic_mode = true;
        let start_pos = parser.position;

        parser.synchronize();

        // Must have advanced at least once
        assert!(parser.position > start_pos);
    }

    #[test]
    fn peek_nth_multiple() {
        let source = "int x = 42;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert_eq!(parser.peek_nth(0).kind, TokenKind::Int);
        assert_eq!(parser.peek_nth(1).kind, TokenKind::Identifier);
        assert_eq!(parser.peek_nth(2).kind, TokenKind::Equal);
        assert_eq!(parser.peek_nth(3).kind, TokenKind::IntLiteral);
        assert_eq!(parser.peek_nth(4).kind, TokenKind::Semicolon);
    }

    #[test]
    fn is_eof_check() {
        let source = "int";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert!(!parser.is_eof());
        parser.advance(); // int
        assert!(parser.is_eof());
    }

    #[test]
    fn take_errors() {
        let source = "int x;";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        parser.error(ParseErrorKind::ExpectedToken, Span::new(1, 1, 1), "error 1");
        parser.error(ParseErrorKind::ExpectedToken, Span::new(1, 2, 1), "error 2");

        assert_eq!(parser.errors.len(), 2);

        let errors = parser.take_errors();
        assert_eq!(errors.len(), 2);
        assert_eq!(parser.errors.len(), 0);
    }

    #[test]
    fn is_primitive_type_all_types() {
        let types = vec![
            "void", "bool", "int", "int8", "int16", "int64",
            "uint", "uint8", "uint16", "uint64", "float", "double", "auto"
        ];

        for ty in types {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(ty, &arena);
            assert!(parser.is_primitive_type(), "Failed for type: {}", ty);
        }
    }

    #[test]
    fn is_primitive_type_negative() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass", &arena);
        assert!(!parser.is_primitive_type());

        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("if", &arena);
        assert!(!parser.is_primitive_type());
    }

    #[test]
    fn try_skip_template_args() {
        let source = "<int, float> rest";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let saved = parser.position;
        assert!(parser.try_skip_template_args());

        // Should have consumed template args
        assert!(parser.position > saved);
        assert_eq!(parser.peek().lexeme, "rest");
    }

    #[test]
    fn try_skip_nested_template_args() {
        let source = "<array<int>> rest";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        assert!(parser.try_skip_template_args());
        assert_eq!(parser.peek().lexeme, "rest");
    }

    #[test]
    fn is_var_decl_simple_types() {
        let tests = vec![
            "int x",
            "float y",
            "const int z",
            "MyClass obj",
            "array<int> arr",
        ];

        for test in tests {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(test, &arena);
            assert!(parser.is_var_decl(), "Failed for: {}", test);
        }
    }

    #[test]
    fn is_var_decl_with_reference() {
        let source = "int& x";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_var_decl());
    }

    #[test]
    fn is_var_decl_with_handle() {
        let source = "MyClass@ obj";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);
        assert!(parser.is_var_decl());
    }

    #[test]
    fn is_var_decl_not_a_declaration() {
        let tests = vec![
            "if (x)",
            "return x;",
            "break;",
        ];

        for test in tests {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(test, &arena);
            assert!(!parser.is_var_decl(), "Incorrectly identified as var decl: {}", test);
        }
    }

    #[test]
    fn eat_contextual_success() {
        let source = "shared class";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let token = parser.eat_contextual("shared");
        assert!(token.is_some());
        assert_eq!(token.unwrap().lexeme, "shared");
        assert_eq!(parser.peek().kind, TokenKind::Class);
    }

    #[test]
    fn eat_contextual_wrong_name() {
        let source = "shared class";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let token = parser.eat_contextual("external");
        assert!(token.is_none());
        assert_eq!(parser.peek().lexeme, "shared");
    }

    #[test]
    fn eat_contextual_not_identifier() {
        let source = "class Foo";
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(source, &arena);

        let token = parser.eat_contextual("class");
        assert!(token.is_none());
    }

    #[test]
    fn multiple_errors_accumulate() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("test", &arena);

        for i in 0..5 {
            parser.error(
                ParseErrorKind::ExpectedToken,
                Span::new(1, i, 1),
                format!("error {}", i),
            );
        }

        assert_eq!(parser.errors.len(), 5);
        assert!(parser.has_errors());
    }
}