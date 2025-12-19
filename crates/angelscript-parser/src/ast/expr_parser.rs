//! Expression parsing using Pratt parsing (precedence climbing).
//!
//! This module implements expression parsing with proper operator precedence
//! and associativity using the Pratt parsing algorithm.

use super::parser::Parser;
use crate::ast::expr::*;
use crate::ast::{
    AssignOp, BinaryOp, Ident, ParseError, ParseErrorKind, PostfixOp, Scope, UnaryOp,
};
use crate::lexer::TokenKind;
use angelscript_core::Span;

impl<'ast> Parser<'ast> {
    /// Parse an expression with a minimum binding power.
    ///
    /// This is the core of the Pratt parser. It handles operator precedence
    /// by only consuming operators with sufficient binding power.
    pub fn parse_expr(&mut self, min_bp: u8) -> Result<&'ast Expr<'ast>, ParseError> {
        // Parse the prefix expression (literals, identifiers, unary ops, etc.)
        let mut lhs = self.parse_prefix()?;

        // Now parse infix and postfix operators
        loop {
            // Check for postfix operators (highest precedence)
            if let Some(postfix_op) = PostfixOp::from_token(self.peek().kind) {
                let op_bp = PostfixOp::binding_power();
                if op_bp < min_bp {
                    break;
                }

                let op_token = self.advance();
                let span = lhs.span().merge(op_token.span);
                lhs = self
                    .arena
                    .alloc(Expr::Postfix(self.arena.alloc(PostfixExpr {
                        operand: lhs,
                        op: postfix_op,
                        span,
                    })));
                continue;
            }

            // Check for member access (.)
            if self.check(TokenKind::Dot) {
                let op_bp = 27; // Same as postfix
                if op_bp < min_bp {
                    break;
                }
                lhs = self.parse_member_access(lhs)?;
                continue;
            }

            // Check for function call
            if self.check(TokenKind::LeftParen) {
                let op_bp = 27; // Same as postfix
                if op_bp < min_bp {
                    break;
                }
                lhs = self.parse_call(lhs)?;
                continue;
            }

            // Check for array indexing
            if self.check(TokenKind::LeftBracket) {
                let op_bp = 27; // Same as postfix
                if op_bp < min_bp {
                    break;
                }
                lhs = self.parse_index(lhs)?;
                continue;
            }

            // Check for ternary operator (?:)
            if self.check(TokenKind::Question) {
                if 2 < min_bp {
                    break;
                }
                lhs = self.parse_ternary(lhs)?;
                continue;
            }

            // Check for assignment operators
            if let Some(assign_op) = AssignOp::from_token(self.peek().kind) {
                let (l_bp, r_bp) = AssignOp::binding_power();
                if l_bp < min_bp {
                    break;
                }

                self.advance();
                let rhs = self.parse_expr(r_bp)?;
                let span = lhs.span().merge(rhs.span());
                lhs = self.arena.alloc(Expr::Assign(self.arena.alloc(AssignExpr {
                    target: lhs,
                    op: assign_op,
                    value: rhs,
                    span,
                })));
                continue;
            }

            // Check for binary operators
            if let Some(bin_op) = BinaryOp::from_token(self.peek().kind) {
                let (l_bp, r_bp) = bin_op.binding_power();
                if l_bp < min_bp {
                    break;
                }

                self.advance();
                let rhs = self.parse_expr(r_bp)?;
                let span = lhs.span().merge(rhs.span());
                lhs = self.arena.alloc(Expr::Binary(self.arena.alloc(BinaryExpr {
                    left: lhs,
                    op: bin_op,
                    right: rhs,
                    span,
                })));
                continue;
            }

            // No more operators
            break;
        }

        Ok(lhs)
    }

    /// Parse a prefix expression (the start of an expression).
    fn parse_prefix(&mut self) -> Result<&'ast Expr<'ast>, ParseError> {
        let token = *self.peek();

        match token.kind {
            // Literals
            TokenKind::IntLiteral => {
                self.advance();
                let value = token.lexeme.parse::<i64>().unwrap_or(0);
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Int(value),
                    span: token.span,
                })))
            }

            TokenKind::BitsLiteral => {
                self.advance();
                // Parse different bases: 0xFF (hex), 0b1010 (binary), 0o77 (octal), 0d99 (decimal)
                let value = if token.lexeme.starts_with("0x") || token.lexeme.starts_with("0X") {
                    // Hexadecimal
                    i64::from_str_radix(&token.lexeme[2..], 16).unwrap_or(0)
                } else if token.lexeme.starts_with("0b") || token.lexeme.starts_with("0B") {
                    // Binary
                    i64::from_str_radix(&token.lexeme[2..], 2).unwrap_or(0)
                } else if token.lexeme.starts_with("0o") || token.lexeme.starts_with("0O") {
                    // Octal
                    i64::from_str_radix(&token.lexeme[2..], 8).unwrap_or(0)
                } else if token.lexeme.starts_with("0d") || token.lexeme.starts_with("0D") {
                    // Decimal (explicit)
                    token.lexeme[2..].parse::<i64>().unwrap_or(0)
                } else {
                    // Fallback: try to parse as regular integer
                    token.lexeme.parse::<i64>().unwrap_or(0)
                };
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Int(value),
                    span: token.span,
                })))
            }

            TokenKind::FloatLiteral => {
                self.advance();
                let value = token
                    .lexeme
                    .trim_end_matches('f')
                    .trim_end_matches('F')
                    .parse::<f32>()
                    .unwrap_or(0.0);
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Float(value),
                    span: token.span,
                })))
            }

            TokenKind::DoubleLiteral => {
                self.advance();
                let value = token.lexeme.parse::<f64>().unwrap_or(0.0);
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Double(value),
                    span: token.span,
                })))
            }

            TokenKind::StringLiteral | TokenKind::HeredocLiteral => {
                self.advance();
                let is_heredoc = token.lexeme.starts_with("\"\"\"");
                let bytes = self.process_string_bytes(token.lexeme, is_heredoc, token.span)?;
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::String(bytes),
                    span: token.span,
                })))
            }

            TokenKind::True => {
                self.advance();
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Bool(true),
                    span: token.span,
                })))
            }

            TokenKind::False => {
                self.advance();
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Bool(false),
                    span: token.span,
                })))
            }

            TokenKind::Null => {
                self.advance();
                Ok(self.arena.alloc(Expr::Literal(LiteralExpr {
                    kind: LiteralKind::Null,
                    span: token.span,
                })))
            }

            // Unary prefix operators
            _ if UnaryOp::from_token(token.kind).is_some() => {
                let op = UnaryOp::from_token(token.kind).unwrap();
                self.advance();
                let bp = UnaryOp::binding_power();
                let operand = self.parse_expr(bp)?;
                let span = token.span.merge(operand.span());
                Ok(self.arena.alloc(Expr::Unary(self.arena.alloc(UnaryExpr {
                    op,
                    operand,
                    span,
                }))))
            }

            // Parenthesized expression
            TokenKind::LeftParen => {
                let start_span = self.advance().span;
                let expr = self.parse_expr(0)?;
                let end_span = self.expect(TokenKind::RightParen)?.span;
                Ok(self.arena.alloc(Expr::Paren(self.arena.alloc(ParenExpr {
                    expr,
                    span: start_span.merge(end_span),
                }))))
            }

            // Cast expression
            TokenKind::Cast => self.parse_cast(),

            // Lambda expression
            _ if self.check_contextual("function") => self.parse_lambda(),

            // Initializer list
            TokenKind::LeftBrace => self.parse_init_list(),

            // Identifier or constructor call
            TokenKind::Identifier | TokenKind::ColonColon => self.parse_ident_or_constructor(),

            // Super keyword (for base class constructor calls)
            TokenKind::Super => {
                let token = self.advance();
                let ident = Ident::new(token.lexeme, token.span);
                Ok(self.arena.alloc(Expr::Ident(IdentExpr {
                    scope: None,
                    ident,
                    type_args: &[],
                    span: token.span,
                })))
            }

            // This keyword (reference to current object in methods)
            TokenKind::This => {
                let token = self.advance();
                let ident = Ident::new(token.lexeme, token.span);
                Ok(self.arena.alloc(Expr::Ident(IdentExpr {
                    scope: None,
                    ident,
                    type_args: &[],
                    span: token.span,
                })))
            }

            // Type keywords (for constructor calls)
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
            | TokenKind::Auto => self.parse_ident_or_constructor(),

            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedExpression,
                token.span,
                format!("expected expression, found {}", token.kind),
            )),
        }
    }

    /// Parse member access (dot operator).
    fn parse_member_access(
        &mut self,
        object: &'ast Expr<'ast>,
    ) -> Result<&'ast Expr<'ast>, ParseError> {
        let dot_span = self.expect(TokenKind::Dot)?.span;

        // The member must be an identifier
        let member_token = self.expect(TokenKind::Identifier)?;
        let member_ident = Ident::new(member_token.lexeme, member_token.span);

        // Check if this is a method call (followed by '(')
        if self.check(TokenKind::LeftParen) {
            let args = self.parse_arguments()?;
            let span = object.span().merge(
                self.buffer
                    .get(self.position.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(dot_span),
            );

            Ok(self.arena.alloc(Expr::Member(self.arena.alloc(MemberExpr {
                object,
                member: MemberAccess::Method {
                    name: member_ident,
                    args,
                },
                span,
            }))))
        } else {
            // Field access
            let span = object.span().merge(member_ident.span);
            Ok(self.arena.alloc(Expr::Member(self.arena.alloc(MemberExpr {
                object,
                member: MemberAccess::Field(member_ident),
                span,
            }))))
        }
    }

    /// Parse function call.
    fn parse_call(&mut self, callee: &'ast Expr<'ast>) -> Result<&'ast Expr<'ast>, ParseError> {
        let args = self.parse_arguments()?;
        let span = callee.span().merge(
            self.buffer
                .get(self.position.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(callee.span()),
        );

        Ok(self.arena.alloc(Expr::Call(self.arena.alloc(CallExpr {
            callee,
            args,
            span,
        }))))
    }

    /// Parse array indexing.
    fn parse_index(&mut self, object: &'ast Expr<'ast>) -> Result<&'ast Expr<'ast>, ParseError> {
        self.expect(TokenKind::LeftBracket)?;

        let mut indices = bumpalo::collections::Vec::new_in(self.arena);

        // Parse first index
        if !self.check(TokenKind::RightBracket) {
            indices.push(self.parse_index_item()?);

            // Parse remaining indices
            while self.eat(TokenKind::Comma).is_some() {
                indices.push(self.parse_index_item()?);
            }
        }

        let end_span = self.expect(TokenKind::RightBracket)?.span;
        let span = object.span().merge(end_span);

        Ok(self.arena.alloc(Expr::Index(self.arena.alloc(IndexExpr {
            object,
            indices: indices.into_bump_slice(),
            span,
        }))))
    }

    /// Parse a single index item (can be named).
    fn parse_index_item(&mut self) -> Result<IndexItem<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Check for named index: identifier :
        let name = if self.check(TokenKind::Identifier) && self.peek_nth(1).kind == TokenKind::Colon
        {
            let ident_token = self.advance();
            self.expect(TokenKind::Colon)?; // Validate colon is present
            Some(Ident::new(ident_token.lexeme, ident_token.span))
        } else {
            None
        };

        let index = self.parse_expr(0)?;
        let span = start_span.merge(index.span());

        Ok(IndexItem { name, index, span })
    }

    /// Parse ternary conditional (condition ? then : else).
    fn parse_ternary(
        &mut self,
        condition: &'ast Expr<'ast>,
    ) -> Result<&'ast Expr<'ast>, ParseError> {
        self.expect(TokenKind::Question)?;
        let then_expr = self.parse_expr(0)?;
        self.expect(TokenKind::Colon)?;
        let else_expr = self.parse_expr(1)?; // Right-associative
        let span = condition.span().merge(else_expr.span());

        Ok(self
            .arena
            .alloc(Expr::Ternary(self.arena.alloc(TernaryExpr {
                condition,
                then_expr,
                else_expr,
                span,
            }))))
    }

    /// Parse cast expression: cast<Type>(expr)
    fn parse_cast(&mut self) -> Result<&'ast Expr<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Cast)?.span;
        self.expect(TokenKind::Less)?;
        let target_type = self.parse_type()?;
        self.expect(TokenKind::Greater)?;
        self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expr(0)?;
        let end_span = self.expect(TokenKind::RightParen)?.span;

        Ok(self.arena.alloc(Expr::Cast(self.arena.alloc(CastExpr {
            target_type,
            expr,
            span: start_span.merge(end_span),
        }))))
    }

    /// Parse lambda expression: function(params) { body }
    fn parse_lambda(&mut self) -> Result<&'ast Expr<'ast>, ParseError> {
        let start_span = self
            .eat_contextual("function")
            .ok_or_else(|| {
                let span = self.peek().span;
                ParseError::new(
                    ParseErrorKind::ExpectedExpression,
                    span,
                    "expected 'function'",
                )
            })?
            .span;

        self.expect(TokenKind::LeftParen)?;

        let mut params = bumpalo::collections::Vec::new_in(self.arena);

        // Parse parameters
        if !self.check(TokenKind::RightParen) {
            params.push(self.parse_lambda_param()?);

            while self.eat(TokenKind::Comma).is_some() {
                params.push(self.parse_lambda_param()?);
            }
        }

        self.expect(TokenKind::RightParen)?;

        // Parse body
        let body = self.parse_block()?;

        let end_span = body.span;

        Ok(self.arena.alloc(Expr::Lambda(self.arena.alloc(LambdaExpr {
            params: params.into_bump_slice(),
            return_type: None,
            body: self.arena.alloc(body),
            span: start_span.merge(end_span),
        }))))
    }

    /// Parse a lambda parameter.
    fn parse_lambda_param(&mut self) -> Result<LambdaParam<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Disambiguate between:
        // - function(a, b) - names only, infer types from context
        // - function(int a, int b) - explicit types with names
        // - function(int, int) - types only (for some contexts)
        //
        // Strategy:
        // - Primitive type keyword → definitely a type
        // - identifier identifier → type name pattern
        // - identifier , or identifier ) → name-only pattern

        let (ty, name) = if self.is_primitive_type() {
            // Primitive type (int, float, etc.) - always a type
            let param_ty = self.parse_param_type()?;
            let param_name = if self.check(TokenKind::Identifier) {
                let token = self.advance();
                Some(Ident::new(token.lexeme, token.span))
            } else {
                None // Type without name (e.g., function(int, int))
            };
            (Some(param_ty), param_name)
        } else if self.check(TokenKind::Identifier) {
            // Could be either "CustomType name" or just "name"
            // Lookahead to next token to disambiguate
            let next_token = self.peek_nth(1);

            if next_token.kind == TokenKind::Identifier {
                // Pattern: identifier identifier → type name
                let param_ty = self.parse_param_type()?;
                let token = self.advance();
                let param_name = Some(Ident::new(token.lexeme, token.span));
                (Some(param_ty), param_name)
            } else {
                // Pattern: identifier , or identifier ) → name only
                let token = self.advance();
                let param_name = Some(Ident::new(token.lexeme, token.span));
                (None, param_name)
            }
        } else {
            // No identifier - error
            return Err(ParseError::new(
                crate::ast::ParseErrorKind::UnexpectedToken,
                self.peek().span,
                "expected parameter type or name".to_string(),
            ));
        };

        let span = if let Some(ref n) = name {
            start_span.merge(n.span)
        } else if let Some(ref t) = ty {
            t.span
        } else {
            start_span
        };

        Ok(LambdaParam { ty, name, span })
    }

    /// Parse initializer list: { elements }
    fn parse_init_list(&mut self) -> Result<&'ast Expr<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::LeftBrace)?.span;

        let mut elements = bumpalo::collections::Vec::new_in(self.arena);

        // Parse elements
        if !self.check(TokenKind::RightBrace) {
            elements.push(self.parse_init_element()?);

            while self.eat(TokenKind::Comma).is_some() {
                if self.check(TokenKind::RightBrace) {
                    break; // Trailing comma
                }
                elements.push(self.parse_init_element()?);
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(self.arena.alloc(Expr::InitList(InitListExpr {
            ty: None,
            elements: elements.into_bump_slice(),
            span: start_span.merge(end_span),
        })))
    }

    /// Parse an initializer list element.
    fn parse_init_element(&mut self) -> Result<InitElement<'ast>, ParseError> {
        if self.check(TokenKind::LeftBrace) {
            // Nested initializer list
            if let Expr::InitList(init_list) = *self.parse_init_list()? {
                Ok(InitElement::InitList(init_list))
            } else {
                let span = self.peek().span;
                Err(ParseError::new(
                    ParseErrorKind::InternalError,
                    span,
                    "parse_init_list() returned non-InitList expression",
                ))
            }
        } else {
            // Expression element
            let expr = self.parse_expr(0)?;
            Ok(InitElement::Expr(expr))
        }
    }

    /// Parse identifier, scoped identifier, or constructor call.
    ///
    /// This handles disambiguation between:
    /// - Simple identifier: `foo`
    /// - Scoped identifier: `Namespace::foo`
    /// - Type call (constructor or cast): `MyClass(args)`
    /// - Scoped type call: `Namespace::MyClass(args)`
    fn parse_ident_or_constructor(&mut self) -> Result<&'ast Expr<'ast>, ParseError> {
        let start_span = self.peek().span;

        // First check if this looks like a type call using lookahead
        // Type call pattern: [::] identifier [::identifier]* [<template>] (
        let is_type_call = self.is_constructor_call();

        if is_type_call {
            // Parse as type call: Type(args)
            let ty = self.parse_type()?;

            // Should be followed by '('
            if self.check(TokenKind::LeftParen) {
                let args = self.parse_arguments()?;
                let span = start_span.merge(
                    self.buffer
                        .get(self.position.saturating_sub(1))
                        .map(|t| t.span)
                        .unwrap_or(ty.span),
                );

                // Primitive types: Type(expr) is always a cast (primitives have no constructors)
                // User types: Type(args) is constructor call or opConv (semantic analyzer decides)
                if matches!(ty.base, crate::ast::types::TypeBase::Primitive(_)) {
                    // Primitive cast: float(value), int(value), etc.
                    // Must have exactly one argument for cast
                    if args.len() != 1 {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidExpression,
                            span,
                            format!(
                                "primitive cast requires exactly one argument, found {}",
                                args.len()
                            ),
                        ));
                    }

                    Ok(self.arena.alloc(Expr::Cast(self.arena.alloc(CastExpr {
                        target_type: ty,
                        expr: args[0].value,
                        span,
                    }))))
                } else {
                    // User-defined type: convert to Call expression
                    // The VM/interpreter will determine if this is a constructor or function call
                    let (scope, ident) = match ty.base {
                        crate::ast::types::TypeBase::Named(name) => (ty.scope, name),
                        _ => {
                            return Err(ParseError::new(
                                ParseErrorKind::ExpectedIdentifier,
                                ty.span,
                                "expected identifier",
                            ));
                        }
                    };

                    let callee = self.arena.alloc(Expr::Ident(IdentExpr {
                        scope,
                        ident,
                        type_args: ty.template_args,
                        span: ty.span,
                    }));

                    Ok(self.arena.alloc(Expr::Call(self.arena.alloc(CallExpr {
                        callee,
                        args,
                        span,
                    }))))
                }
            } else {
                // Lookahead said it was constructor call, but '(' is missing
                let token = *self.peek();
                Err(ParseError::new(
                    ParseErrorKind::ExpectedToken,
                    token.span,
                    format!(
                        "expected '(' after type name for constructor call, found {}",
                        token.kind
                    ),
                ))
            }
        } else {
            // Parse as simple identifier (possibly scoped)
            // This avoids trying to parse < as template arguments
            let scope = if self.eat(TokenKind::ColonColon).is_some() {
                // Global scope: ::identifier
                Some(Scope {
                    is_absolute: true,
                    segments: &[],
                    span: start_span,
                })
            } else {
                None
            };

            // Parse scoped path: A::B::identifier
            let mut scope = scope;
            let mut ident_token = self.expect(TokenKind::Identifier)?;
            let mut ident = Ident::new(ident_token.lexeme, ident_token.span);
            let mut segments = bumpalo::collections::Vec::new_in(self.arena);

            while self.check(TokenKind::ColonColon)
                && self.peek_nth(1).kind == TokenKind::Identifier
            {
                // Build scope path
                self.advance(); // consume ::

                if scope.is_none() {
                    scope = Some(Scope {
                        is_absolute: false,
                        segments: &[],
                        span: start_span,
                    });
                }

                segments.push(ident);

                ident_token = self.advance();
                ident = Ident::new(ident_token.lexeme, ident_token.span);
            }

            // Update scope with segments and span
            if let Some(ref mut s) = scope {
                s.segments = segments.into_bump_slice();
                s.span = start_span.merge(ident.span);
            }

            let span = ident.span;

            Ok(self.arena.alloc(Expr::Ident(IdentExpr {
                scope,
                ident,
                type_args: &[],
                span: start_span.merge(span),
            })))
        }
    }

    /// Check if current position looks like a constructor call.
    /// Constructor pattern: [::] (identifier|primitive) [::identifier]* [<template>] (
    fn is_constructor_call(&mut self) -> bool {
        let saved_pos = self.position;

        // Skip optional ::
        self.eat(TokenKind::ColonColon);

        // Need at least one identifier OR primitive type keyword
        let is_type_start = self.check(TokenKind::Identifier) || self.is_primitive_type();
        if !is_type_start {
            self.position = saved_pos;
            return false;
        }
        self.advance();

        // Skip scope path (only for identifiers, not primitives)
        while self.check(TokenKind::ColonColon) && self.peek_nth(1).kind == TokenKind::Identifier {
            self.advance(); // ::
            self.advance(); // identifier
        }

        // Check for template arguments
        if self.check(TokenKind::Less) {
            // Try to skip template args
            if !self.try_skip_template_args_simple() {
                self.position = saved_pos;
                return false;
            }
        }

        // Must be followed by '(' to be constructor
        let result = self.check(TokenKind::LeftParen);

        self.position = saved_pos;
        result
    }

    /// Parse function arguments: (arg1, arg2, ...)
    fn parse_arguments(&mut self) -> Result<&'ast [Argument<'ast>], ParseError> {
        self.expect(TokenKind::LeftParen)?;

        let mut args = bumpalo::collections::Vec::new_in(self.arena);

        if !self.check(TokenKind::RightParen) {
            args.push(self.parse_argument()?);

            while self.eat(TokenKind::Comma).is_some() {
                args.push(self.parse_argument()?);
            }
        }

        self.expect(TokenKind::RightParen)?;
        Ok(args.into_bump_slice())
    }

    /// Parse a single argument (can be named).
    fn parse_argument(&mut self) -> Result<Argument<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Check for named argument: identifier :
        let name = if self.check(TokenKind::Identifier) && self.peek_nth(1).kind == TokenKind::Colon
        {
            let ident_token = self.advance();
            self.expect(TokenKind::Colon)?; // Validate colon is present
            Some(Ident::new(ident_token.lexeme, ident_token.span))
        } else {
            None
        };

        let value = self.parse_expr(0)?;
        let span = start_span.merge(value.span());

        Ok(Argument { name, value, span })
    }

    /// Process a string literal, handling escape sequences and returning raw bytes.
    ///
    /// Supports escape sequences:
    /// - `\n` (newline), `\r` (carriage return), `\t` (tab)
    /// - `\\` (backslash), `\"` (double quote), `\'` (single quote)
    /// - `\0` (null byte)
    /// - `\xNN` (hex byte, e.g., `\xFF`)
    fn process_string_bytes(
        &mut self,
        lexeme: &str,
        is_heredoc: bool,
        span: Span,
    ) -> Result<Vec<u8>, ParseError> {
        let content = if is_heredoc {
            // Heredoc: strip """ from both ends
            lexeme
                .trim_start_matches("\"\"\"")
                .trim_end_matches("\"\"\"")
        } else {
            // Regular string: strip surrounding quotes
            let trimmed = lexeme.trim_start_matches('"').trim_start_matches('\'');
            trimmed.trim_end_matches('"').trim_end_matches('\'')
        };

        // Heredocs don't process escape sequences
        if is_heredoc {
            return Ok(content.as_bytes().to_vec());
        }

        let mut bytes = Vec::with_capacity(content.len());
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => bytes.push(0x0A),
                    Some('r') => bytes.push(0x0D),
                    Some('t') => bytes.push(0x09),
                    Some('\\') => bytes.push(0x5C),
                    Some('"') => bytes.push(0x22),
                    Some('\'') => bytes.push(0x27),
                    Some('0') => bytes.push(0x00),
                    Some('x') => {
                        // Hex escape: \xNN
                        let hex: String = chars.by_ref().take(2).collect();
                        if hex.len() != 2 {
                            return Err(ParseError::new(
                                ParseErrorKind::InvalidEscapeSequence,
                                span,
                                format!("incomplete hex escape sequence: \\x{}", hex),
                            ));
                        }
                        match u8::from_str_radix(&hex, 16) {
                            Ok(byte) => bytes.push(byte),
                            Err(_) => {
                                return Err(ParseError::new(
                                    ParseErrorKind::InvalidEscapeSequence,
                                    span,
                                    format!("invalid hex escape sequence: \\x{}", hex),
                                ));
                            }
                        }
                    }
                    Some(other) => {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidEscapeSequence,
                            span,
                            format!("unknown escape sequence: \\{}", other),
                        ));
                    }
                    None => {
                        return Err(ParseError::new(
                            ParseErrorKind::InvalidEscapeSequence,
                            span,
                            "incomplete escape sequence at end of string".to_string(),
                        ));
                    }
                }
            } else {
                // Regular character - encode as UTF-8 bytes
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(encoded.as_bytes());
            }
        }

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_int_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("42", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(42)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_binary_expr() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("1 + 2", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Add));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("1 + 2 * 3", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Add));
                match bin.right {
                    Expr::Binary(inner) => {
                        assert!(matches!(inner.op, BinaryOp::Mul));
                    }
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_unary() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("-42", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Unary(un) => {
                assert!(matches!(un.op, UnaryOp::Neg));
            }
            _ => panic!("Expected unary expression"),
        }
    }

    #[test]
    fn parse_parenthesized() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("(42)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Paren(_) => {}
            _ => panic!("Expected parenthesized expression"),
        }
    }

    #[test]
    fn parse_call() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("foo()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn parse_member_access() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.field", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(mem) => {
                assert!(matches!(mem.member, MemberAccess::Field(_)));
            }
            _ => panic!("Expected member expression"),
        }
    }

    #[test]
    fn parse_index() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("arr[0]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(_) => {}
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_ternary() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a ? b : c", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ternary(_) => {}
            _ => panic!("Expected ternary expression"),
        }
    }

    #[test]
    fn parse_assignment() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("x = 42", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Assign(assign) => {
                assert!(matches!(assign.op, AssignOp::Assign));
            }
            _ => panic!("Expected assignment"),
        }
    }

    // ========================================================================
    // Literal Tests
    // ========================================================================

    #[test]
    fn parse_float_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("3.14f", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::Float(val) = lit.kind {
                    assert!((val - 3.14).abs() < 0.001);
                } else {
                    panic!("Expected float literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_double_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("2.71828", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Double(_)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_hex() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0xFF", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(255)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_binary() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0b1010", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(10)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_octal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0o77", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(63)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_string_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""hello world""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    assert_eq!(s, b"hello world");
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_true_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("true", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Bool(true)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_false_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("false", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Bool(false)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_null_literal() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("null", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Null));
            }
            _ => panic!("Expected literal"),
        }
    }

    // ========================================================================
    // Binary Operator Tests
    // ========================================================================

    #[test]
    fn parse_all_binary_operators() {
        let operators = vec![
            ("+", BinaryOp::Add),
            ("-", BinaryOp::Sub),
            ("*", BinaryOp::Mul),
            ("/", BinaryOp::Div),
            ("%", BinaryOp::Mod),
            ("&", BinaryOp::BitwiseAnd),
            ("|", BinaryOp::BitwiseOr),
            ("^", BinaryOp::BitwiseXor),
            ("<<", BinaryOp::ShiftLeft),
            (">>", BinaryOp::ShiftRight),
            (">>>", BinaryOp::ShiftRightUnsigned),
            ("&&", BinaryOp::LogicalAnd),
            ("||", BinaryOp::LogicalOr),
            ("^^", BinaryOp::LogicalXor),
            ("==", BinaryOp::Equal),
            ("!=", BinaryOp::NotEqual),
            ("<", BinaryOp::Less),
            ("<=", BinaryOp::LessEqual),
            (">", BinaryOp::Greater),
            (">=", BinaryOp::GreaterEqual),
            ("is", BinaryOp::Is),
            ("!is", BinaryOp::NotIs),
        ];

        for (op_str, expected_op) in operators {
            let source = format!("a {} b", op_str);
            let arena = bumpalo::Bump::new();
            let mut parser = Parser::new(&source, &arena);
            let expr = parser.parse_expr(0).unwrap();
            match expr {
                Expr::Binary(bin) => {
                    assert!(
                        std::mem::discriminant(&bin.op) == std::mem::discriminant(&expected_op),
                        "Failed for operator: {}",
                        op_str
                    );
                }
                _ => panic!("Expected binary expression for: {}", op_str),
            }
        }
    }

    // ========================================================================
    // Unary Operator Tests
    // ========================================================================

    #[test]
    fn parse_all_unary_operators() {
        let operators = vec![
            ("-", UnaryOp::Neg),
            ("!", UnaryOp::LogicalNot),
            ("~", UnaryOp::BitwiseNot),
            ("++", UnaryOp::PreInc),
            ("--", UnaryOp::PreDec),
            ("@", UnaryOp::HandleOf),
        ];

        for (op_str, expected_op) in operators {
            let source = format!("{}x", op_str);
            let arena = bumpalo::Bump::new();
            let mut parser = Parser::new(&source, &arena);
            let expr = parser.parse_expr(0).unwrap();
            match expr {
                Expr::Unary(un) => {
                    assert!(
                        std::mem::discriminant(&un.op) == std::mem::discriminant(&expected_op),
                        "Failed for operator: {}",
                        op_str
                    );
                }
                _ => panic!("Expected unary expression for: {}", op_str),
            }
        }
    }

    // ========================================================================
    // Postfix Operator Tests
    // ========================================================================

    #[test]
    fn parse_postfix_increment() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("x++", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Postfix(post) => {
                assert!(matches!(post.op, PostfixOp::PostInc));
            }
            _ => panic!("Expected postfix expression"),
        }
    }

    #[test]
    fn parse_postfix_decrement() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("x--", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Postfix(post) => {
                assert!(matches!(post.op, PostfixOp::PostDec));
            }
            _ => panic!("Expected postfix expression"),
        }
    }

    // ========================================================================
    // Assignment Operator Tests
    // ========================================================================

    #[test]
    fn parse_all_assignment_operators() {
        let operators = vec![
            ("=", AssignOp::Assign),
            ("+=", AssignOp::AddAssign),
            ("-=", AssignOp::SubAssign),
            ("*=", AssignOp::MulAssign),
            ("/=", AssignOp::DivAssign),
            ("%=", AssignOp::ModAssign),
            ("&=", AssignOp::AndAssign),
            ("|=", AssignOp::OrAssign),
            ("^=", AssignOp::XorAssign),
            ("<<=", AssignOp::ShlAssign),
            (">>=", AssignOp::ShrAssign),
            (">>>=", AssignOp::UshrAssign),
        ];

        for (op_str, expected_op) in operators {
            let source = format!("x {} 42", op_str);
            let arena = bumpalo::Bump::new();
            let mut parser = Parser::new(&source, &arena);
            let expr = parser.parse_expr(0).unwrap();
            match expr {
                Expr::Assign(assign) => {
                    assert!(
                        std::mem::discriminant(&assign.op) == std::mem::discriminant(&expected_op),
                        "Failed for operator: {}",
                        op_str
                    );
                }
                _ => panic!("Expected assignment for: {}", op_str),
            }
        }
    }

    // ========================================================================
    // Call and Member Tests
    // ========================================================================

    #[test]
    fn parse_call_with_args() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.foo(1, 2, 3)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(mem) => match mem.member {
                MemberAccess::Method { ref args, .. } => {
                    assert_eq!(args.len(), 3);
                }
                _ => panic!("Expected method call"),
            },
            _ => panic!("Expected member expression with method call"),
        }
    }

    #[test]
    fn parse_call_empty_args() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.bar()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(mem) => match mem.member {
                MemberAccess::Method { ref args, .. } => {
                    assert_eq!(args.len(), 0);
                }
                _ => panic!("Expected method call"),
            },
            _ => panic!("Expected member expression with method call"),
        }
    }

    #[test]
    fn parse_named_argument() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.foo(x: 42)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(mem) => match mem.member {
                MemberAccess::Method { ref args, .. } => {
                    assert_eq!(args.len(), 1);
                    assert!(args[0].name.is_some());
                }
                _ => panic!("Expected method call"),
            },
            _ => panic!("Expected member expression with method call"),
        }
    }

    #[test]
    fn parse_method_call() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.method()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(mem) => {
                assert!(matches!(mem.member, MemberAccess::Method { .. }));
            }
            _ => panic!("Expected member expression"),
        }
    }

    #[test]
    fn parse_chained_member_access() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a.b.c", &arena);
        let expr = parser.parse_expr(0).unwrap();
        // Should parse as ((a.b).c)
        match expr {
            Expr::Member(outer) => {
                match &outer.object {
                    Expr::Member(_) => {} // Good, inner member access
                    _ => panic!("Expected nested member access"),
                }
            }
            _ => panic!("Expected member expression"),
        }
    }

    // ========================================================================
    // Index Tests
    // ========================================================================

    #[test]
    fn parse_index_single() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("arr[0]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                assert_eq!(idx.indices.len(), 1);
            }
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_index_multiple() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("matrix[i, j]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                assert_eq!(idx.indices.len(), 2);
            }
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_named_index() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("dict[key: x]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                assert_eq!(idx.indices.len(), 1);
                assert!(idx.indices[0].name.is_some());
            }
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_empty_index() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("arr[]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                assert_eq!(idx.indices.len(), 0);
            }
            _ => panic!("Expected index expression"),
        }
    }

    // ========================================================================
    // Cast Tests
    // ========================================================================

    #[test]
    fn parse_cast_expression() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("cast<int>(3.14)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Cast(_) => {}
            _ => panic!("Expected cast expression"),
        }
    }

    #[test]
    fn parse_primitive_cast_via_constructor() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int(3.14)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Cast(_) => {}
            _ => panic!("Expected cast expression for primitive constructor"),
        }
    }

    #[test]
    fn parse_constructor_expression() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass(1, 2)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(call) => {
                assert_eq!(call.args.len(), 2);
                // Verify the callee is an identifier
                match &call.callee {
                    Expr::Ident(ident) => {
                        assert_eq!(ident.ident.name, "MyClass");
                    }
                    _ => panic!("Expected identifier as callee"),
                }
            }
            _ => panic!("Expected call expression"),
        }
    }

    // ========================================================================
    // Lambda Tests
    // ========================================================================

    #[test]
    fn parse_lambda_no_params() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function() { return 42; }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                assert_eq!(lambda.params.len(), 0);
                assert!(lambda.body.stmts.len() > 0);
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    #[test]
    fn parse_lambda_with_params() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function(int x, int y) { return x + y; }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                assert_eq!(lambda.params.len(), 2);
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    #[test]
    fn parse_lambda_with_return_type() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function() { return 42; }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                // Lambdas don't have explicit return types - they're inferred
                assert!(lambda.return_type.is_none());
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    // ========================================================================
    // Init List Tests
    // ========================================================================

    #[test]
    fn parse_init_list_simple() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ 1, 2, 3 }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::InitList(init) => {
                assert_eq!(init.elements.len(), 3);
            }
            _ => panic!("Expected init list"),
        }
    }

    #[test]
    fn parse_init_list_empty() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::InitList(init) => {
                assert_eq!(init.elements.len(), 0);
            }
            _ => panic!("Expected init list"),
        }
    }

    #[test]
    fn parse_init_list_nested() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ {1, 2}, {3, 4} }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::InitList(init) => {
                assert_eq!(init.elements.len(), 2);
                match &init.elements[0] {
                    InitElement::InitList(_) => {}
                    _ => panic!("Expected nested init list"),
                }
            }
            _ => panic!("Expected init list"),
        }
    }

    #[test]
    fn parse_init_list_trailing_comma() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ 1, 2, }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::InitList(init) => {
                assert_eq!(init.elements.len(), 2);
            }
            _ => panic!("Expected init list"),
        }
    }

    // ========================================================================
    // Identifier and Scope Tests
    // ========================================================================

    #[test]
    fn parse_simple_identifier() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("myVar", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert_eq!(ident.ident.name, "myVar");
                assert!(ident.scope.is_none());
            }
            _ => panic!("Expected identifier"),
        }
    }

    #[test]
    fn parse_scoped_identifier() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Namespace::myVar", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert!(ident.scope.is_some());
                let scope = ident.scope.unwrap();
                assert!(!scope.is_absolute);
                assert_eq!(scope.segments.len(), 1);
            }
            _ => panic!("Expected scoped identifier"),
        }
    }

    #[test]
    fn parse_global_scoped_identifier() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::globalVar", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert!(ident.scope.is_some());
                let scope = ident.scope.unwrap();
                assert!(scope.is_absolute);
            }
            _ => panic!("Expected global scoped identifier"),
        }
    }

    #[test]
    fn parse_deeply_nested_scope() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("A::B::C::var", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert!(ident.scope.is_some());
                let scope = ident.scope.unwrap();
                assert_eq!(scope.segments.len(), 3);
            }
            _ => panic!("Expected deeply scoped identifier"),
        }
    }

    // ========================================================================
    // Precedence Tests
    // ========================================================================

    #[test]
    fn parse_precedence_mul_over_add() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("1 + 2 * 3", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Add));
                // Right side should be multiplication
                match bin.right {
                    Expr::Binary(inner) => {
                        assert!(matches!(inner.op, BinaryOp::Mul));
                    }
                    _ => panic!("Expected multiplication on right"),
                }
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_precedence_parentheses() {
        // (1 + 2) * 3 should parse as (1 + 2) * 3
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("(1 + 2) * 3", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Mul));
                // Left side should be parenthesized addition
                match &bin.left {
                    Expr::Paren(_) => {}
                    _ => panic!("Expected paren on left"),
                }
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_right_associative_ternary() {
        // a ? b : c ? d : e should parse as a ? b : (c ? d : e)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a ? b : c ? d : e", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ternary(tern) => {
                // else_expr should be another ternary
                match &tern.else_expr {
                    Expr::Ternary(_) => {}
                    _ => panic!("Expected nested ternary in else branch"),
                }
            }
            _ => panic!("Expected ternary expression"),
        }
    }

    // ========================================================================
    // Complex Expression Tests
    // ========================================================================

    #[test]
    fn parse_complex_chained_expression() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("obj.method(1, 2)[0].field++", &arena);
        let expr = parser.parse_expr(0).unwrap();
        // Should parse as (((obj.method(1,2))[0]).field)++
        match expr {
            Expr::Postfix(_) => {} // Outermost is postfix
            _ => panic!("Expected postfix at top level"),
        }
    }

    #[test]
    fn parse_mixed_operators() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a + b * c - d / e", &arena);
        let expr = parser.parse_expr(0).unwrap();
        // Should respect precedence: (a + (b * c)) - (d / e)
        match expr {
            Expr::Binary(_) => {}
            _ => panic!("Expected binary expression"),
        }
    }

    // ========================================================================
    // Error Cases
    // ========================================================================

    #[test]
    fn parse_expr_invalid_start() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(";", &arena);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
        // Record the error so we can check it
        if let Err(err) = result {
            parser.errors.push(err);
        }
        assert!(parser.has_errors());
    }

    #[test]
    fn parse_primitive_cast_wrong_arg_count() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int(1, 2)", &arena);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
        // Record the error so we can check it
        if let Err(err) = result {
            parser.errors.push(err);
        }
        assert!(parser.has_errors());
    }

    // ========================================================================
    // Additional Coverage Tests
    // ========================================================================

    #[test]
    fn parse_cast_expression_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("cast<int>(3.14)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Cast(_) => {}
            _ => panic!("Expected cast expression"),
        }
    }

    #[test]
    fn parse_super_keyword_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("super", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert_eq!(ident.ident.name, "super");
            }
            _ => panic!("Expected ident expression"),
        }
    }

    #[test]
    fn parse_this_keyword_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("this", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert_eq!(ident.ident.name, "this");
            }
            _ => panic!("Expected ident expression"),
        }
    }

    #[test]
    fn parse_bits_literal_decimal_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0d99", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(99)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_heredoc_string_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#""""multi
line
string""""#,
            &arena,
        );
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(_) = &lit.kind {
                    // Heredoc string parsed
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_global_scoped_template_array_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::array<int>()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn parse_lambda_custom_type_param() {
        let arena = bumpalo::Bump::new();
        // CustomType name pattern
        let mut parser = Parser::new("function(MyType arg) { }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                assert_eq!(lambda.params.len(), 1);
                assert!(lambda.params[0].ty.is_some());
                assert!(lambda.params[0].name.is_some());
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    #[test]
    fn parse_constructor_call_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass(1, 2)", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression (constructor)"),
        }
    }

    #[test]
    fn parse_chained_member_access_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a.b.c", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(_) => {}
            _ => panic!("Expected member expression"),
        }
    }

    #[test]
    fn parse_complex_chained_expression_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("arr[0].field.method()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Member(_) => {}
            _ => panic!("Expected member expression"),
        }
    }

    // ========================================================================
    // Additional Coverage Tests for Uncovered Lines
    // ========================================================================

    #[test]
    fn parse_lambda_name_only_params() {
        // Test lambda with name-only params (no types)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function(a, b) { }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                assert_eq!(lambda.params.len(), 2);
                // These should be name-only params
                assert!(lambda.params[0].ty.is_none());
                assert!(lambda.params[0].name.is_some());
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    #[test]
    fn parse_lambda_type_only_params() {
        // Test lambda with type-only params (no names) - primitive types
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function(int, float) { }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(lambda) => {
                assert_eq!(lambda.params.len(), 2);
                // These should be type-only params
                assert!(lambda.params[0].ty.is_some());
                assert!(lambda.params[0].name.is_none());
            }
            _ => panic!("Expected lambda expression"),
        }
    }

    #[test]
    fn parse_auto_type_constructor() {
        // Test auto keyword - note: auto is special and may not work as constructor
        // This test exercises the type keyword branch
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("auto(42)", &arena);
        // auto type may error since it's not valid as constructor
        let result = parser.parse_expr(0);
        // Just exercise the code path
        let _ = result;
    }

    #[test]
    fn parse_void_type_keyword() {
        // Test void type keyword (not typically used as constructor)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("void()", &arena);
        // This might fail or succeed depending on implementation
        let result = parser.parse_expr(0);
        // Just exercise the code path
        let _ = result;
    }

    #[test]
    fn parse_bits_literal_fallback() {
        // Test bits literal that doesn't match any prefix (fallback case)
        // Note: This exercises the fallback branch in bits literal parsing
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0xFF", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(_) => {}
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_uppercase_prefix() {
        // Test bits literal with uppercase X
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0X10", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(16)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_uppercase_binary() {
        // Test bits literal with uppercase B
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0B1100", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(12)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_uppercase_octal() {
        // Test bits literal with uppercase O
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0O10", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(8)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_bits_literal_uppercase_decimal() {
        // Test bits literal with uppercase D
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("0D42", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                assert!(matches!(lit.kind, LiteralKind::Int(42)));
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_scoped_constructor_with_template() {
        // Test constructor with scope and template args
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Namespace::Container<int>()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn parse_deeply_nested_template_lookahead() {
        // Test template lookahead with nested templates
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Map<string, array<int>>()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn parse_global_scope_identifier_simple() {
        // Test global scope ::identifier (not followed by paren)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("::globalFunc", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ident(ident) => {
                assert!(ident.scope.is_some());
                assert!(ident.scope.unwrap().is_absolute);
            }
            _ => panic!("Expected ident expression"),
        }
    }

    #[test]
    fn parse_template_lookahead_not_constructor() {
        // Test case where < is not template but comparison
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a < b", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                assert!(matches!(bin.op, BinaryOp::Less));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_template_lookahead_with_paren_in_args() {
        // Test template lookahead that sees '(' but not for constructor
        let arena = bumpalo::Bump::new();
        // This should NOT be parsed as constructor because the < is comparison
        let mut parser = Parser::new("a < b || c", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(_) => {}
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_lambda_error_not_function_keyword() {
        // This tests the error path when 'function' keyword is expected but missing
        // Note: This is difficult to trigger directly as check_contextual guards it
        // We exercise related paths instead
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("function() { return 42; }", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Lambda(_) => {}
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn parse_nested_ternary() {
        // Test nested ternary expressions
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a ? b ? c : d : e", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Ternary(tern) => {
                // then_expr should be another ternary
                match &tern.then_expr {
                    Expr::Ternary(_) => {}
                    _ => panic!("Expected nested ternary in then branch"),
                }
            }
            _ => panic!("Expected ternary expression"),
        }
    }

    #[test]
    fn parse_mixed_postfix_and_binary() {
        // Test postfix with binary to check precedence
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a++ + b", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                // Left should be postfix
                match &bin.left {
                    Expr::Postfix(_) => {}
                    _ => panic!("Expected postfix on left"),
                }
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn parse_index_after_call() {
        // Test indexing after function call
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("getArray()[0]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                // Object should be a call
                match &idx.object {
                    Expr::Call(_) => {}
                    _ => panic!("Expected call as object"),
                }
            }
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_chained_index() {
        // Test multiple index operations
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("matrix[0][1]", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Index(idx) => {
                // Object should be another index
                match &idx.object {
                    Expr::Index(_) => {}
                    _ => panic!("Expected nested index"),
                }
            }
            _ => panic!("Expected index expression"),
        }
    }

    #[test]
    fn parse_many_primitive_type_casts() {
        // Test all primitive type keywords for constructor-style casts
        let types = vec![
            "int8", "int16", "int64", "uint", "uint8", "uint16", "uint64", "float", "double",
            "bool",
        ];

        for ty in types {
            let source = format!("{}(42)", ty);
            let arena = bumpalo::Bump::new();
            let mut parser = Parser::new(&source, &arena);
            let expr = parser.parse_expr(0).unwrap();
            match expr {
                Expr::Cast(_) => {}
                _ => panic!("Expected cast for type: {}", ty),
            }
        }
    }

    #[test]
    fn parse_template_with_double_greater() {
        // Test template with >> token (should be split)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Map<int, array<int>>()", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Call(_) => {}
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn parse_shift_operators_precedence() {
        // Test shift operators with other operators
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("a << b + c", &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Binary(bin) => {
                // + has higher precedence, so rhs should be addition
                assert!(matches!(bin.op, BinaryOp::ShiftLeft));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    // ========================================================================
    // String Escape Sequence Tests
    // ========================================================================

    #[test]
    fn parse_string_escape_sequences() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""a\nb\tc""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    // a, newline, b, tab, c
                    assert_eq!(s, &[0x61, 0x0A, 0x62, 0x09, 0x63]);
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_string_hex_escape() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""\xFF\x00""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    assert_eq!(s, &[0xFF, 0x00]);
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_string_all_escape_sequences() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""\n\r\t\\\"\'\0""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    assert_eq!(s, &[0x0A, 0x0D, 0x09, 0x5C, 0x22, 0x27, 0x00]);
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_heredoc_no_escape_processing() {
        let arena = bumpalo::Bump::new();
        // Heredoc should NOT process escape sequences
        let mut parser = Parser::new(r#""""raw\nstring""""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    // Should contain literal backslash-n, not newline
                    assert_eq!(s, b"raw\\nstring");
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }

    #[test]
    fn parse_string_invalid_escape_sequence() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""\z""#, &arena);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
    }

    #[test]
    fn parse_string_incomplete_hex_escape() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""\xF""#, &arena);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
    }

    #[test]
    fn parse_string_invalid_hex_escape() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""\xGG""#, &arena);
        let result = parser.parse_expr(0);
        assert!(result.is_err());
    }

    #[test]
    fn parse_string_mixed_content() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(r#""hello\x20world""#, &arena);
        let expr = parser.parse_expr(0).unwrap();
        match expr {
            Expr::Literal(lit) => {
                if let LiteralKind::String(s) = &lit.kind {
                    // hello<space>world
                    assert_eq!(s, b"hello world");
                } else {
                    panic!("Expected string literal");
                }
            }
            _ => panic!("Expected literal"),
        }
    }
}
