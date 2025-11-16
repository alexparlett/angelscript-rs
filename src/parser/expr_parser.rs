use crate::core::error::*;
use crate::parser::ast::*;
use crate::parser::token::*;

pub struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
    eof_token: Token,
}

impl ExprParser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() || !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
            tokens.push(Token::new(
                TokenKind::Eof,
                Span::new(
                    Position::new(0, 0, 0),
                    Position::new(0, 0, 0),
                    String::new(),
                ),
            ));
        }

        Self {
            tokens,
            pos: 0,
            eof_token: Token::new(
                TokenKind::Eof,
                Span::new(
                    Position::new(0, 0, 0),
                    Position::new(0, 0, 0),
                    String::new(),
                ),
            ),
        }
    }

    pub fn parse(mut self) -> ParseResult<Expr> {
        let expr = self.parse_expr(0)?;

        if !self.is_at_end() {
            return Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: "end of expression".to_string(),
                found: format!("{:?}", self.current().kind),
            });
        }

        Ok(expr)
    }

    fn parse_expr(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.is_at_end() {
                break;
            }

            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;
                continue;
            }

            if let Some((l_bp, r_bp)) = self.infix_binding_power() {
                if l_bp < min_bp {
                    break;
                }
                lhs = self.parse_infix(lhs, r_bp)?;
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        let token = self.current().clone();

        match &token.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(n.clone())))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s.clone())))
            }
            TokenKind::Bits(b) => {
                self.advance();
                Ok(Expr::Literal(Literal::Bits(b.clone())))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Literal(Literal::Null))
            }
            TokenKind::Void => {
                self.advance();
                Ok(Expr::Void)
            }

            TokenKind::Sub => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::Neg);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(rhs)))
            }
            TokenKind::Add => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::Plus);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::Plus, Box::new(rhs)))
            }
            TokenKind::Not => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::Not);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(rhs)))
            }
            TokenKind::BitNot => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::BitNot);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::BitNot, Box::new(rhs)))
            }
            TokenKind::Inc => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::PreInc);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::PreInc, Box::new(rhs)))
            }
            TokenKind::Dec => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::PreDec);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::PreDec, Box::new(rhs)))
            }
            TokenKind::At => {
                self.advance();
                let ((), r_bp) = prefix_binding_power(UnaryOp::Handle);
                let rhs = self.parse_expr(r_bp)?;
                Ok(Expr::Unary(UnaryOp::Handle, Box::new(rhs)))
            }

            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }

            TokenKind::Identifier(name) => {
                // Check if this is "function" keyword for lambda (contextual keyword)
                if name == "function" {
                    // Look ahead to determine if this is a lambda or function call
                    // Lambda: function(...) { ... }
                    // Call: function(...)
                    let checkpoint = self.pos;
                    self.advance(); // skip "function"

                    if self.check(&TokenKind::LParen) {
                        // Find the matching ) and check if { follows
                        let mut paren_depth = 1;
                        let mut scan_pos = self.pos + 1; // skip the (

                        while paren_depth > 0 && scan_pos < self.tokens.len() {
                            match &self.tokens[scan_pos].kind {
                                TokenKind::LParen => paren_depth += 1,
                                TokenKind::RParen => paren_depth -= 1,
                                TokenKind::Eof => break,
                                _ => {}
                            }
                            scan_pos += 1;
                        }

                        // Check if { follows the )
                        let is_lambda = if scan_pos < self.tokens.len() {
                            matches!(self.tokens[scan_pos].kind, TokenKind::LBrace)
                        } else {
                            false
                        };

                        self.pos = checkpoint; // rewind

                        if is_lambda {
                            return self.parse_lambda();
                        }
                    } else {
                        self.pos = checkpoint; // rewind
                    }
                }

                let name = name.clone();
                self.advance();

                if self.check(&TokenKind::Lt) {
                    let next_pos = self.pos + 1;
                    let looks_like_template = if next_pos < self.tokens.len() {
                        matches!(
                            self.tokens[next_pos].kind,
                            TokenKind::Identifier(_)
                                | TokenKind::Int
                                | TokenKind::Int8
                                | TokenKind::Int16
                                | TokenKind::Int32
                                | TokenKind::Int64
                                | TokenKind::Uint
                                | TokenKind::Uint8
                                | TokenKind::Uint16
                                | TokenKind::Uint32
                                | TokenKind::Uint64
                                | TokenKind::Float
                                | TokenKind::Double
                                | TokenKind::Bool
                                | TokenKind::Void
                                | TokenKind::Const
                                | TokenKind::Auto
                        )
                    } else {
                        false
                    };

                    if looks_like_template {
                        self.advance();

                        let mut template_types = Vec::new();

                        loop {
                            let typ = self.parse_type_in_expr()?;
                            template_types.push(typ);

                            if self.check(&TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        self.expect_gt_in_template()?;

                        if self.check(&TokenKind::LParen) {
                            self.advance();
                            let args = self.parse_arg_list_inner()?;
                            self.expect(&TokenKind::RParen)?;

                            let typ = Type {
                                is_const: false,
                                scope: Scope {
                                    is_global: false,
                                    path: Vec::new(),
                                },
                                datatype: DataType::Identifier(name),
                                template_types,
                                modifiers: Vec::new(),
                            };

                            return Ok(Expr::ConstructCall(typ, args));
                        } else {
                            return Ok(Expr::VarAccess(
                                Scope {
                                    is_global: false,
                                    path: Vec::new(),
                                },
                                name,
                            ));
                        }
                    }
                }

                let (scope, final_name) = self.parse_scope(name)?;

                Ok(Expr::VarAccess(scope, final_name))
            }

            TokenKind::DoubleColon => {
                self.advance();

                if let TokenKind::Identifier(name) = &self.current().kind {
                    let name = name.clone();
                    self.advance();

                    let (mut scope, final_name) = self.parse_scope(name)?;
                    scope.is_global = true;

                    Ok(Expr::VarAccess(scope, final_name))
                } else {
                    Err(ParseError::UnexpectedToken {
                        span: self.current().span.clone(),
                        expected: "identifier after ::".to_string(),
                        found: format!("{:?}", self.current().kind),
                    })
                }
            }

            TokenKind::Cast => self.parse_cast(),

            TokenKind::LBrace => self.parse_init_list(),

            _ => Err(ParseError::UnexpectedToken {
                span: token.span.clone(),
                expected: "expression".to_string(),
                found: format!("{:?}", token.kind),
            }),
        }
    }

    fn parse_infix(&mut self, lhs: Expr, r_bp: u8) -> ParseResult<Expr> {
        let token = self.current().clone();
        let op = match &token.kind {
            TokenKind::Add => BinaryOp::Add,
            TokenKind::Sub => BinaryOp::Sub,
            TokenKind::Mul => BinaryOp::Mul,
            TokenKind::Div => BinaryOp::Div,
            TokenKind::Mod => BinaryOp::Mod,
            TokenKind::Pow => BinaryOp::Pow,

            TokenKind::Eq => BinaryOp::Eq,
            TokenKind::Ne => BinaryOp::Ne,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::Le => BinaryOp::Le,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::Ge => BinaryOp::Ge,
            TokenKind::Is => BinaryOp::Is,
            TokenKind::IsNot => BinaryOp::IsNot,

            TokenKind::And => BinaryOp::And,
            TokenKind::Or => BinaryOp::Or,
            TokenKind::Xor => BinaryOp::Xor,

            TokenKind::BitAnd => BinaryOp::BitAnd,
            TokenKind::BitOr => BinaryOp::BitOr,
            TokenKind::BitXor => BinaryOp::BitXor,
            TokenKind::Shl => BinaryOp::Shl,
            TokenKind::Shr => BinaryOp::Shr,
            TokenKind::UShr => BinaryOp::UShr,

            TokenKind::Assign => BinaryOp::Assign,
            TokenKind::AddAssign => BinaryOp::AddAssign,
            TokenKind::SubAssign => BinaryOp::SubAssign,
            TokenKind::MulAssign => BinaryOp::MulAssign,
            TokenKind::DivAssign => BinaryOp::DivAssign,
            TokenKind::ModAssign => BinaryOp::ModAssign,
            TokenKind::PowAssign => BinaryOp::PowAssign,
            TokenKind::BitAndAssign => BinaryOp::BitAndAssign,
            TokenKind::BitOrAssign => BinaryOp::BitOrAssign,
            TokenKind::BitXorAssign => BinaryOp::BitXorAssign,
            TokenKind::ShlAssign => BinaryOp::ShlAssign,
            TokenKind::ShrAssign => BinaryOp::ShrAssign,
            TokenKind::UShrAssign => BinaryOp::UShrAssign,

            TokenKind::Question => {
                self.advance();
                let then_expr = self.parse_expr(0)?;
                self.expect(&TokenKind::Colon)?;
                let else_expr = self.parse_expr(r_bp)?;
                return Ok(Expr::Ternary(
                    Box::new(lhs),
                    Box::new(then_expr),
                    Box::new(else_expr),
                ));
            }

            _ => {
                return Err(ParseError::InvalidOperator {
                    span: token.span.clone(),
                    operator: format!("{:?}", token.kind),
                });
            }
        };

        self.advance();
        let rhs = self.parse_expr(r_bp)?;
        Ok(Expr::Binary(Box::new(lhs), op, Box::new(rhs)))
    }

    fn parse_postfix(&mut self, lhs: Expr) -> ParseResult<Expr> {
        let token = self.current().clone();

        match &token.kind {
            TokenKind::Inc => {
                self.advance();
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::PostInc))
            }
            TokenKind::Dec => {
                self.advance();
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::PostDec))
            }
            TokenKind::LParen => {
                self.advance();
                let args = self.parse_arg_list_inner()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::Call(args)))
            }
            TokenKind::LBracket => {
                self.advance();
                let indices = self.parse_index_args()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::Index(indices)))
            }
            TokenKind::Dot => {
                self.advance();

                if let TokenKind::Identifier(name) = &self.current().kind {
                    let name = name.clone();
                    self.advance();

                    if self.check(&TokenKind::LParen) {
                        self.advance();
                        let args = self.parse_arg_list_inner()?;
                        self.expect(&TokenKind::RParen)?;

                        let func_call = FuncCall {
                            scope: Scope {
                                is_global: false,
                                path: Vec::new(),
                            },
                            name,
                            template_types: Vec::new(),
                            args,
                        };
                        Ok(Expr::Postfix(
                            Box::new(lhs),
                            PostfixOp::MemberCall(func_call),
                        ))
                    } else {
                        Ok(Expr::Postfix(Box::new(lhs), PostfixOp::MemberAccess(name)))
                    }
                } else {
                    Err(ParseError::UnexpectedToken {
                        span: self.current().span.clone(),
                        expected: "identifier".to_string(),
                        found: format!("{:?}", self.current().kind),
                    })
                }
            }
            _ => Err(ParseError::InvalidExpression {
                span: token.span.clone(),
                message: format!("Expected postfix operator, found {:?}", token.kind),
            }),
        }
    }

    fn parse_scope(&mut self, initial_name: String) -> ParseResult<(Scope, String)> {
        let mut path = Vec::new();
        let mut current_name = initial_name;
        let is_global = false;

        while self.check(&TokenKind::DoubleColon) {
            self.advance();
            path.push(current_name);

            if let TokenKind::Identifier(name) = &self.current().kind {
                current_name = name.clone();
                self.advance();
            } else {
                return Err(ParseError::UnexpectedToken {
                    span: self.current().span.clone(),
                    expected: "identifier after ::".to_string(),
                    found: format!("{:?}", self.current().kind),
                });
            }
        }

        Ok((Scope { is_global, path }, current_name))
    }

    fn parse_cast(&mut self) -> ParseResult<Expr> {
        self.expect(&TokenKind::Cast)?;
        self.expect(&TokenKind::Lt)?;

        let type_name = if let TokenKind::Identifier(name) = &self.current().kind {
            name.clone()
        } else {
            return Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: "type name".to_string(),
                found: format!("{:?}", self.current().kind),
            });
        };
        self.advance();

        self.expect_gt_in_template()?;
        self.expect(&TokenKind::LParen)?;

        let expr = self.parse_expr(0)?;

        self.expect(&TokenKind::RParen)?;

        let cast_type = Type {
            is_const: false,
            scope: Scope {
                is_global: false,
                path: Vec::new(),
            },
            datatype: DataType::Identifier(type_name),
            template_types: Vec::new(),
            modifiers: Vec::new(),
        };

        Ok(Expr::Cast(cast_type, Box::new(expr)))
    }

    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        if let TokenKind::Identifier(name) = &self.current().kind {
            if name != "function" {
                return Err(ParseError::UnexpectedToken {
                    span: self.current().span.clone(),
                    expected: "'function'".to_string(),
                    found: name.clone(),
                });
            }
        }
        self.advance();

        self.expect(&TokenKind::LParen)?;

        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let checkpoint = self.pos;

            if self.check(&TokenKind::Const) {
                self.advance();
            }

            let param_type = if self.check_type_token() {
                self.pos = checkpoint;
                let typ = self.parse_type_in_expr()?;

                if self.check_identifier()
                    || self.check(&TokenKind::Comma)
                    || self.check(&TokenKind::RParen)
                {
                    Some(typ)
                } else {
                    self.pos = checkpoint;
                    None
                }
            } else {
                self.pos = checkpoint;
                None
            };

            let mut type_mod = None;
            if self.check(&TokenKind::BitAnd) {
                self.advance();

                type_mod = Some(if self.check(&TokenKind::In) {
                    self.advance();
                    TypeMod::In
                } else if self.check(&TokenKind::Out) {
                    self.advance();
                    TypeMod::Out
                } else if self.check(&TokenKind::InOut) {
                    self.advance();
                    TypeMod::InOut
                } else {
                    TypeMod::InOut
                });
            }

            let name = if self.check_identifier() {
                Some(self.expect_identifier()?)
            } else {
                None
            };

            params.push(LambdaParam {
                param_type,
                type_mod,
                name,
            });

            if self.check(&TokenKind::Comma) {
                self.advance();
            } else if !self.check(&TokenKind::RParen) {
                return Err(ParseError::UnexpectedToken {
                    span: self.current().span.clone(),
                    expected: "',' or ')'".to_string(),
                    found: format!("{:?}", self.current().kind),
                });
            }
        }

        self.expect(&TokenKind::RParen)?;

        self.expect(&TokenKind::LBrace)?;

        let _body_tokens = self.collect_lambda_body()?;

        self.expect(&TokenKind::RBrace)?;

        Ok(Expr::Lambda(Lambda {
            params,
            body: StatBlock {
                statements: Vec::new(),
            },
        }))
    }

    fn collect_lambda_body(&mut self) -> ParseResult<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut brace_depth = 1;

        while brace_depth > 0 && !self.is_at_end() {
            let token = self.current().clone();

            match &token.kind {
                TokenKind::LBrace => {
                    brace_depth += 1;
                    tokens.push(token);
                    self.advance();
                }
                TokenKind::RBrace => {
                    brace_depth -= 1;
                    if brace_depth > 0 {
                        tokens.push(token);
                        self.advance();
                    }
                }
                _ => {
                    tokens.push(token);
                    self.advance();
                }
            }
        }

        Ok(tokens)
    }

    fn check_type_token(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Void
                | TokenKind::Int
                | TokenKind::Int8
                | TokenKind::Int16
                | TokenKind::Int32
                | TokenKind::Int64
                | TokenKind::Uint
                | TokenKind::Uint8
                | TokenKind::Uint16
                | TokenKind::Uint32
                | TokenKind::Uint64
                | TokenKind::Float
                | TokenKind::Double
                | TokenKind::Bool
                | TokenKind::Auto
                | TokenKind::Const
                | TokenKind::Identifier(_)
        )
    }

    fn parse_type_in_expr(&mut self) -> ParseResult<Type> {
        let is_const = self.check(&TokenKind::Const);
        if is_const {
            self.advance();
        }

        let scope = Scope {
            is_global: false,
            path: Vec::new(),
        };

        let datatype = match &self.current().kind {
            TokenKind::Void => {
                self.advance();
                DataType::PrimType("void".to_string())
            }
            TokenKind::Int | TokenKind::Int32 => {
                self.advance();
                DataType::PrimType("int".to_string())
            }
            TokenKind::Int8 => {
                self.advance();
                DataType::PrimType("int8".to_string())
            }
            TokenKind::Int16 => {
                self.advance();
                DataType::PrimType("int16".to_string())
            }
            TokenKind::Int64 => {
                self.advance();
                DataType::PrimType("int64".to_string())
            }
            TokenKind::Uint | TokenKind::Uint32 => {
                self.advance();
                DataType::PrimType("uint".to_string())
            }
            TokenKind::Uint8 => {
                self.advance();
                DataType::PrimType("uint8".to_string())
            }
            TokenKind::Uint16 => {
                self.advance();
                DataType::PrimType("uint16".to_string())
            }
            TokenKind::Uint64 => {
                self.advance();
                DataType::PrimType("uint64".to_string())
            }
            TokenKind::Float => {
                self.advance();
                DataType::PrimType("float".to_string())
            }
            TokenKind::Double => {
                self.advance();
                DataType::PrimType("double".to_string())
            }
            TokenKind::Bool => {
                self.advance();
                DataType::PrimType("bool".to_string())
            }
            TokenKind::Auto => {
                self.advance();
                DataType::Auto
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                DataType::Identifier(name)
            }
            _ => {
                return Err(ParseError::SyntaxError {
                    span: self.current().span.clone(),
                    message: "Expected type".to_string(),
                });
            }
        };

        let template_types = if self.check(&TokenKind::Lt) {
            self.advance();
            let mut types = vec![self.parse_type_in_expr()?];

            while self.check(&TokenKind::Comma) {
                self.advance();
                types.push(self.parse_type_in_expr()?);
            }

            self.expect_gt_in_template()?;
            types
        } else {
            Vec::new()
        };

        let mut modifiers = Vec::new();

        loop {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                self.expect(&TokenKind::RBracket)?;
                modifiers.push(TypeModifier::Array);
            } else if self.check(&TokenKind::At) {
                self.advance();
                if self.check(&TokenKind::Const) {
                    self.advance();
                    modifiers.push(TypeModifier::ConstHandle);
                } else {
                    modifiers.push(TypeModifier::Handle);
                }
            } else {
                break;
            }
        }

        Ok(Type {
            is_const,
            scope,
            datatype,
            template_types,
            modifiers,
        })
    }

    fn parse_init_list(&mut self) -> ParseResult<Expr> {
        self.expect(&TokenKind::LBrace)?;

        let mut items = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let expr = self.parse_expr(0)?;
            items.push(InitListItem::Expr(expr));

            if !self.check(&TokenKind::RBrace) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::RBrace)?;

        Ok(Expr::InitList(InitList { items }))
    }

    fn parse_arg_list_inner(&mut self) -> ParseResult<Vec<Arg>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            let value = self.parse_expr(0)?;
            args.push(Arg { name: None, value });

            if !self.check(&TokenKind::RParen) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(args)
    }

    fn parse_index_args(&mut self) -> ParseResult<Vec<IndexArg>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
            let value = self.parse_expr(0)?;
            args.push(IndexArg { name: None, value });

            if !self.check(&TokenKind::RBracket) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(args)
    }

    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        if self.is_at_end() {
            return None;
        }

        let token = self.current();
        Some(match token.kind {
            TokenKind::Assign
            | TokenKind::AddAssign
            | TokenKind::SubAssign
            | TokenKind::MulAssign
            | TokenKind::DivAssign
            | TokenKind::ModAssign
            | TokenKind::PowAssign
            | TokenKind::BitAndAssign
            | TokenKind::BitOrAssign
            | TokenKind::BitXorAssign
            | TokenKind::ShlAssign
            | TokenKind::ShrAssign
            | TokenKind::UShrAssign => (2, 1),

            TokenKind::Question => (4, 3),
            TokenKind::Or => (5, 6),
            TokenKind::Xor => (7, 8),
            TokenKind::And => (9, 10),
            TokenKind::BitOr => (11, 12),
            TokenKind::BitXor => (13, 14),
            TokenKind::BitAnd => (15, 16),
            TokenKind::Eq | TokenKind::Ne | TokenKind::Is | TokenKind::IsNot => (17, 18),
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge => (19, 20),
            TokenKind::Shl | TokenKind::Shr | TokenKind::UShr => (21, 22),
            TokenKind::Add | TokenKind::Sub => (23, 24),
            TokenKind::Mul | TokenKind::Div | TokenKind::Mod => (25, 26),
            TokenKind::Pow => (28, 27),

            _ => return None,
        })
    }

    fn postfix_binding_power(&self) -> Option<u8> {
        if self.is_at_end() {
            return None;
        }

        let token = self.current();
        Some(match token.kind {
            TokenKind::Inc
            | TokenKind::Dec
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::Dot => 29,
            _ => return None,
        })
    }

    /// Expect > in template context, handling >>, >>>, etc. as multiple >
    fn expect_gt_in_template(&mut self) -> ParseResult<()> {
        match &self.current().kind {
            TokenKind::Gt => {
                self.advance();
                Ok(())
            }
            TokenKind::Shr => {
                let shr_token = self.current().clone();
                let gt_token = Token::new(
                    TokenKind::Gt,
                    Span::new(
                        Position::new(
                            shr_token.span.start.line,
                            shr_token.span.start.column + 1,
                            shr_token.span.start.offset + 1,
                        ),
                        shr_token.span.end.clone(),
                        ">".to_string(),
                    ),
                );
                self.tokens[self.pos] = gt_token;
                Ok(())
            }
            TokenKind::UShr => {
                let ushr_token = self.current().clone();
                let shr_token = Token::new(
                    TokenKind::Shr,
                    Span::new(
                        Position::new(
                            ushr_token.span.start.line,
                            ushr_token.span.start.column + 1,
                            ushr_token.span.start.offset + 1,
                        ),
                        ushr_token.span.end.clone(),
                        ">>".to_string(),
                    ),
                );
                self.tokens[self.pos] = shr_token;
                Ok(())
            }
            _ => Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: "'>'".to_string(),
                found: format!("{:?}", self.current().kind),
            }),
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.eof_token)
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() - 1 || self.current().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.current().kind, TokenKind::Identifier(_))
    }

    fn expect(&mut self, kind: &TokenKind) -> ParseResult<()> {
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

    fn expect_identifier(&mut self) -> ParseResult<String> {
        if let TokenKind::Identifier(name) = &self.current().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(ParseError::UnexpectedToken {
                span: self.current().span.clone(),
                expected: "identifier".to_string(),
                found: format!("{:?}", self.current().kind),
            })
        }
    }
}

fn prefix_binding_power(op: UnaryOp) -> ((), u8) {
    match op {
        UnaryOp::Neg
        | UnaryOp::Plus
        | UnaryOp::Not
        | UnaryOp::BitNot
        | UnaryOp::PreInc
        | UnaryOp::PreDec
        | UnaryOp::Handle => ((), 27),
    }
}
