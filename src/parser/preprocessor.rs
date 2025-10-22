use crate::parser::ast::*;
use crate::parser::error::*;
use crate::parser::parser::Parser;
use crate::parser::script_builder::ScriptBuilder;
use crate::parser::token::*;

pub struct Preprocessor<'a> {
    tokens: Vec<Token>,
    pos: usize,
    builder: &'a mut ScriptBuilder,
}

impl<'a> Preprocessor<'a> {
    pub fn new(tokens: Vec<Token>, builder: &'a mut ScriptBuilder) -> Self {
        Self {
            tokens,
            pos: 0,
            builder,
        }
    }

    pub fn parse(mut self) -> Result<Script> {
        let items = self.parse_items()?;
        Ok(Script { items })
    }

    fn parse_items(&mut self) -> Result<Vec<ScriptNode>> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            // Check for preprocessor directives
            if self.check(&TokenKind::Hash) {
                self.handle_directive(&mut items)?;
            } else {
                // Parse regular script item
                let item_tokens = self.collect_until_next_directive();
                if !item_tokens.is_empty() {
                    let parser = Parser::new(item_tokens);
                    let script = parser.parse()?;
                    items.extend(script.items);
                }
            }
        }

        Ok(items)
    }

    fn handle_directive(&mut self, items: &mut Vec<ScriptNode>) -> Result<()> {
        self.expect(&TokenKind::Hash)?;

        let directive_name = match &self.current().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            TokenKind::If => {
                self.advance();
                "if".to_string()
            }
            TokenKind::Else => {
                self.advance();
                "else".to_string()
            }
            _ => {
                return Err(self.error("Expected directive name after #"));
            }
        };

        match directive_name.as_str() {
            "include" => {
                let path = self.expect_string()?;
                items.push(ScriptNode::Include(Include { path }));
            }
            "pragma" => {
                let content = self.read_until_newline();
                items.push(ScriptNode::Pragma(Pragma { content }));
            }
            "if" => {
                let condition = self.expect_identifier()?;
                let is_defined = self.builder.is_defined(&condition);

                let if_items = self.parse_conditional_block()?;

                let mut found_match = is_defined;
                let mut selected_items = if is_defined { Some(if_items) } else { None };

                // Parse elif and else branches
                loop {
                    if !self.check(&TokenKind::Hash) {
                        break;
                    }

                    let checkpoint = self.pos;
                    self.advance(); // consume #

                    // Get the directive name (elif, else, or endif)
                    let branch_name = match &self.current().kind {
                        TokenKind::Identifier(name) => {
                            let n = name.clone();
                            self.advance();
                            n
                        }
                        TokenKind::Else => {
                            self.advance();
                            "else".to_string()
                        }
                        _ => {
                            // Not a conditional directive, rewind
                            self.pos = checkpoint;
                            break;
                        }
                    };

                    match branch_name.as_str() {
                        "elif" => {
                            let elif_condition = self.expect_identifier()?;
                            let elif_items = self.parse_conditional_block()?;

                            if !found_match && self.builder.is_defined(&elif_condition) {
                                selected_items = Some(elif_items);
                                found_match = true;
                            }
                        }
                        "else" => {
                            let else_items = self.parse_conditional_block()?;

                            if !found_match {
                                selected_items = Some(else_items);
                            }

                            self.expect_directive("endif")?;
                            break;
                        }
                        "endif" => {
                            break;
                        }
                        _ => {
                            // Unknown directive, rewind
                            self.pos = checkpoint;
                            break;
                        }
                    }
                }

                // Add selected items directly to the output
                if let Some(selected) = selected_items {
                    items.extend(selected);
                }
            }
            "define" => {
                let word = self.expect_identifier()?;
                self.builder.define_word(word);
            }
            _ => {
                let content = self.read_until_newline();
                items.push(ScriptNode::CustomDirective(CustomDirective {
                    name: directive_name,
                    content,
                }));
            }
        }

        Ok(())
    }

    fn parse_conditional_block(&mut self) -> Result<Vec<ScriptNode>> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Hash) {
                let checkpoint = self.pos;
                self.advance();

                // Check if this is a conditional end directive
                let is_conditional_end = match &self.current().kind {
                    TokenKind::Identifier(name) => {
                        matches!(name.as_str(), "elif" | "else" | "endif")
                    }
                    TokenKind::Else => true, // ✅ Handle TokenKind::Else
                    _ => false,
                };

                if is_conditional_end {
                    // End of this conditional block
                    self.pos = checkpoint;
                    break;
                }

                // Not a conditional end, rewind and parse as directive
                self.pos = checkpoint;
                self.handle_directive(&mut items)?;
            } else {
                // Parse regular script item
                let item_tokens = self.collect_until_next_directive();
                if !item_tokens.is_empty() {
                    let parser = Parser::new(item_tokens);
                    let script = parser.parse()?;
                    items.extend(script.items);
                }
            }
        }

        Ok(items)
    }

    fn collect_until_next_directive(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Hash) {
                break;
            }

            tokens.push(self.current().clone());
            self.advance();
        }

        // Add EOF token
        if !tokens.is_empty() {
            tokens.push(Token::new(
                TokenKind::Eof,
                Span::new(
                    Position::new(0, 0, 0),
                    Position::new(0, 0, 0),
                    String::new(),
                ),
            ));
        }

        tokens
    }

    fn expect_directive(&mut self, name: &str) -> Result<()> {
        self.expect(&TokenKind::Hash)?;

        if let TokenKind::Identifier(id) = &self.current().kind {
            if id == name {
                self.advance();
                return Ok(());
            }
        }

        Err(self.error(&format!("Expected #{}", name)))
    }

    fn read_until_newline(&mut self) -> String {
        let mut content = String::new();
        let start_line = if self.pos < self.tokens.len() {
            self.tokens[self.pos].span.start.line
        } else {
            0
        };

        while !self.is_at_end() {
            let token = self.current();
            if token.span.start.line != start_line {
                break;
            }

            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(&token.span.source);
            self.advance();
        }

        content.trim().to_string()
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.current().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: format!("{:?}", kind),
                found: format!("{:?}", self.current().kind),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<String> {
        if let TokenKind::Identifier(name) = &self.current().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(self.error("Expected identifier"))
        }
    }

    fn expect_string(&mut self) -> Result<String> {
        if let TokenKind::String(s) = &self.current().kind {
            let s = s.clone();
            self.advance();
            Ok(s)
        } else {
            Err(self.error("Expected string literal"))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError::SyntaxError {
            span: self.current().span.clone(),
            message: message.to_string(),
        }
    }
}
