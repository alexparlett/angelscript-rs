use crate::parser::ast::*;
use crate::parser::error::*;
use crate::parser::token::*;

pub struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
    eof_token: Token,
}

impl ExprParser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        // Ensure there's always an EOF token
        if tokens.is_empty() || !matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)) {
            tokens.push(Token::new(
                TokenKind::Eof,
                Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0), String::new()),
            ));
        }

        Self {
            tokens,
            pos: 0,
            eof_token: Token::new(
                TokenKind::Eof,
                Span::new(Position::new(0, 0, 0), Position::new(0, 0, 0), String::new()),
            ),
        }
    }

    pub fn parse(mut self) -> Result<Expr> {
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

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr> {
        // Parse prefix expression
        let mut lhs = self.parse_prefix()?;

        // Parse infix/postfix expressions
        loop {
            if self.is_at_end() {
                break;
            }

            // Try postfix operators first
            if let Some(postfix_bp) = self.postfix_binding_power() {
                if postfix_bp < min_bp {
                    break;
                }
                lhs = self.parse_postfix(lhs)?;
                continue;
            }

            // Try infix operators
            if let Some((l_bp, r_bp)) = self.infix_binding_power() {
                if l_bp < min_bp {
                    break;
                }
                lhs = self.parse_infix(lhs, r_bp)?;
                continue;
            }

            // No more operators
            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr> {
        let token = self.current().clone();

        match &token.kind {
            // Literals
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

            // Prefix operators
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

            // Parenthesized expression
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }

            // Identifier or function call
            // Identifier - could be variable, function call, or templated type
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();

                // Check if this is a templated type (for constructor calls)
                // e.g., array<int>(), MyClass<T>()
                if self.check(&TokenKind::Lt) {
                    // Could be template args or less-than operator
                    // Heuristic: if followed by a type-like token, it's probably a template
                    let next_pos = self.pos + 1;
                    let looks_like_template = if next_pos < self.tokens.len() {
                        matches!(
                        self.tokens[next_pos].kind,
                        TokenKind::Identifier(_) | TokenKind::Int | TokenKind::Int8 |
                        TokenKind::Int16 | TokenKind::Int32 | TokenKind::Int64 |
                        TokenKind::Uint | TokenKind::Uint8 | TokenKind::Uint16 |
                        TokenKind::Uint32 | TokenKind::Uint64 | TokenKind::Float |
                        TokenKind::Double | TokenKind::Bool | TokenKind::Void |
                        TokenKind::Const | TokenKind::Auto
                    )
                    } else {
                        false
                    };

                    if looks_like_template {
                        // Parse as templated type
                        self.advance(); // consume <

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

                        self.expect(&TokenKind::Gt)?;

                        // Check if this is a constructor call
                        if self.check(&TokenKind::LParen) {
                            self.advance();
                            let args = self.parse_arg_list_inner()?;
                            self.expect(&TokenKind::RParen)?;

                            let typ = Type {
                                is_const: false,
                                scope: Scope { is_global: false, path: Vec::new() },
                                datatype: DataType::Identifier(name),
                                template_types,
                                modifiers: Vec::new(),
                            };

                            return Ok(Expr::ConstructCall(typ, args));
                        } else {
                            // Just a type reference (unusual in expression context)
                            // Treat as variable access for now
                            return Ok(Expr::VarAccess(
                                Scope { is_global: false, path: Vec::new() },
                                name
                            ));
                        }
                    }
                }

                // Check for scope resolution
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
            
            // Cast expression
            TokenKind::Cast => self.parse_cast(),

            // Lambda
            TokenKind::Function => self.parse_lambda(),

            // Init list
            TokenKind::LBrace => self.parse_init_list(),

            TokenKind::This => {
                self.advance();
                Ok(Expr::VarAccess(
                    Scope { is_global: false, path: Vec::new() },
                    "this".to_string()
                ))
            }
            TokenKind::Super => {
                self.advance();
                Ok(Expr::VarAccess(
                    Scope { is_global: false, path: Vec::new() },
                    "super".to_string()
                ))
            }

            _ => Err(ParseError::UnexpectedToken {
                span: token.span.clone(),
                expected: "expression".to_string(),
                found: format!("{:?}", token.kind),
            }),
        }
    }

    fn parse_infix(&mut self, lhs: Expr, r_bp: u8) -> Result<Expr> {
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
                // Ternary operator
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

    fn parse_postfix(&mut self, lhs: Expr) -> Result<Expr> {
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
                // Function call
                self.advance();
                let args = self.parse_arg_list_inner()?;
                self.expect(&TokenKind::RParen)?;
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::Call(args)))
            }
            TokenKind::LBracket => {
                // Array indexing
                self.advance();
                let indices = self.parse_index_args()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Postfix(Box::new(lhs), PostfixOp::Index(indices)))
            }
            TokenKind::Dot => {
                // Member access
                self.advance();

                // Check if it's a method call or property access
                if let TokenKind::Identifier(name) = &self.current().kind {
                    let name = name.clone();
                    self.advance();

                    // Check for function call
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

    // Helper methods

    fn parse_scope(&mut self, initial_name: String) -> Result<(Scope, String)> {
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

    fn parse_cast(&mut self) -> Result<Expr> {
        self.expect(&TokenKind::Cast)?;
        self.expect(&TokenKind::Lt)?;

        // Parse type - simplified
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

        self.expect(&TokenKind::Gt)?;
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

    fn parse_lambda(&mut self) -> Result<Expr> {
        self.expect(&TokenKind::Function)?;
        self.expect(&TokenKind::LParen)?;

        // Parse lambda parameters
        let mut params = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            // Parse parameter type (optional for lambdas)
            let param_type = if self.check_type_token() {
                Some(self.parse_type_in_expr()?)
            } else {
                None
            };

            // Check for & modifier
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
                    TypeMod::InOut // Default
                });
            }

            // Parse parameter name (optional)
            let name = if let TokenKind::Identifier(n) = &self.current().kind {
                let n = n.clone();
                self.advance();
                Some(n)
            } else {
                None
            };

            params.push(LambdaParam {
                param_type,
                type_mod,
                name,
            });

            if !self.check(&TokenKind::RParen) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::RParen)?;

        // Parse lambda body
        self.expect(&TokenKind::LBrace)?;

        // We need to parse the body, but we're in the expression parser
        // and don't have access to statement parsing.
        // For now, collect all tokens until the matching }
        let body_tokens = self.collect_lambda_body()?;

        self.expect(&TokenKind::RBrace)?;

        // Create a simplified lambda with the body tokens
        // The actual statement parsing would happen later if needed
        Ok(Expr::Lambda(Lambda {
            params,
            body: StatBlock {
                statements: Vec::new() // Simplified - would need full statement parser
            },
        }))
    }

    fn collect_lambda_body(&mut self) -> Result<Vec<Token>> {
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
        TokenKind::Void | TokenKind::Int | TokenKind::Int8 | TokenKind::Int16 |
        TokenKind::Int32 | TokenKind::Int64 | TokenKind::Uint | TokenKind::Uint8 |
        TokenKind::Uint16 | TokenKind::Uint32 | TokenKind::Uint64 | TokenKind::Float |
        TokenKind::Double | TokenKind::Bool | TokenKind::Auto | TokenKind::Const |
        TokenKind::Identifier(_)
    )
    }

    fn parse_type_in_expr(&mut self) -> Result<Type> {
        let is_const = self.check(&TokenKind::Const);
        if is_const {
            self.advance();
        }

        // Simplified scope parsing
        let scope = Scope {
            is_global: false,
            path: Vec::new(),
        };

        // Parse datatype
        let datatype = match &self.current().kind {
            TokenKind::Void => {
                self.advance();
                DataType::PrimType("void".to_string())
            }
            TokenKind::Int => {
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
            TokenKind::Int32 => {
                self.advance();
                DataType::PrimType("int32".to_string())
            }
            TokenKind::Int64 => {
                self.advance();
                DataType::PrimType("int64".to_string())
            }
            TokenKind::Uint => {
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
            TokenKind::Uint32 => {
                self.advance();
                DataType::PrimType("uint32".to_string())
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
            _ => return Err(ParseError::SyntaxError {
                span: self.current().span.clone(),
                message: "Expected type".to_string(),
            }),
        };

        // Parse template args if present
        let template_types = if self.check(&TokenKind::Lt) {
            self.advance();
            let mut types = vec![self.parse_type_in_expr()?];

            while self.check(&TokenKind::Comma) {
                self.advance();
                types.push(self.parse_type_in_expr()?);
            }

            self.expect(&TokenKind::Gt)?;
            types
        } else {
            Vec::new()
        };

        // Parse type modifiers
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

    fn parse_init_list(&mut self) -> Result<Expr> {
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

    fn parse_arg_list_inner(&mut self) -> Result<Vec<Arg>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            // Check for named argument
            let name = if let TokenKind::Identifier(n) = &self.current().kind {
                let n = n.clone();
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len()
                    && self.tokens[next_pos].kind == TokenKind::Colon
                {
                    self.advance();
                    self.advance();
                    Some(n)
                } else {
                    None
                }
            } else {
                None
            };

            let value = self.parse_expr(0)?;
            args.push(Arg { name, value });

            if !self.check(&TokenKind::RParen) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(args)
    }

    fn parse_index_args(&mut self) -> Result<Vec<IndexArg>> {
        let mut args = Vec::new();

        while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
            // Check for named index
            let name = if let TokenKind::Identifier(n) = &self.current().kind {
                let n = n.clone();
                let next_pos = self.pos + 1;
                if next_pos < self.tokens.len()
                    && self.tokens[next_pos].kind == TokenKind::Colon
                {
                    self.advance();
                    self.advance();
                    Some(n)
                } else {
                    None
                }
            } else {
                None
            };

            let value = self.parse_expr(0)?;
            args.push(IndexArg { name, value });

            if !self.check(&TokenKind::RBracket) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        Ok(args)
    }

    // Binding power functions

    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        if self.is_at_end() {
            return None;
        }

        let token = self.current();
        Some(match token.kind {
            // Assignment (right associative)
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

            // Ternary (right associative)
            TokenKind::Question => (4, 3),

            // Logical OR
            TokenKind::Or => (5, 6),

            // Logical XOR
            TokenKind::Xor => (7, 8),

            // Logical AND
            TokenKind::And => (9, 10),

            // Bitwise OR
            TokenKind::BitOr => (11, 12),

            // Bitwise XOR
            TokenKind::BitXor => (13, 14),

            // Bitwise AND
            TokenKind::BitAnd => (15, 16),

            // Equality
            TokenKind::Eq | TokenKind::Ne | TokenKind::Is | TokenKind::IsNot => (17, 18),

            // Relational
            TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge => (19, 20),

            // Shift
            TokenKind::Shl | TokenKind::Shr | TokenKind::UShr => (21, 22),

            // Additive
            TokenKind::Add | TokenKind::Sub => (23, 24),

            // Multiplicative
            TokenKind::Mul | TokenKind::Div | TokenKind::Mod => (25, 26),

            // Power (right associative)
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

    // Token navigation

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
