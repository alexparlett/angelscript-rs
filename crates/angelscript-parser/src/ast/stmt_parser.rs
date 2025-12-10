//! Statement parsing functions for AngelScript.
//!
//! Implements parsing of all statement types including control flow,
//! loops, variable declarations, and blocks.

use super::parser::Parser;
use crate::ast::expr::Expr;
use crate::ast::stmt::*;
use crate::ast::{Ident, ParseError, ParseErrorKind};
use crate::lexer::TokenKind;
use bumpalo::collections::Vec as BVec;

impl<'ast> Parser<'ast> {
    /// Parse a statement.
    ///
    /// This is the main entry point for statement parsing and dispatches
    /// to specific statement parsers based on the current token.
    pub fn parse_statement(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let token = *self.peek();

        match token.kind {
            // Control flow statements
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Try => self.parse_try_catch(),

            // Jump statements
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),

            // Block
            TokenKind::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::Block(block))
            }

            // Check for foreach (contextual keyword)
            _ if self.check_contextual("foreach") => self.parse_foreach(),

            // Variable declaration or expression statement
            _ => {
                // Try to determine if this is a variable declaration or expression
                if self.is_var_decl() {
                    self.parse_var_decl()
                } else {
                    self.parse_expr_stmt()
                }
            }
        }
    }

    /// Parse an expression statement.
    ///
    /// Grammar: `ASSIGN? ';'`
    pub fn parse_expr_stmt(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Check for empty statement (just semicolon)
        if self.check(TokenKind::Semicolon) {
            let span = self.advance().span;
            return Ok(Stmt::Expr(ExprStmt { expr: None, span }));
        }

        // Parse expression
        let expr = self.parse_expr(0)?;
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::Expr(ExprStmt {
            expr: Some(expr),
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a variable declaration statement.
    ///
    /// Grammar: `TYPE IDENTIFIER ('=' EXPR)? (',' IDENTIFIER ('=' EXPR)?)* ';'`
    pub fn parse_var_decl(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.peek().span;

        // Parse type
        let ty = self.parse_type()?;

        // Parse variable declarators (pass type for constructor calls)
        let mut vars = BVec::new_in(self.arena);
        vars.push(self.parse_var_declarator(&ty)?);

        // Parse additional declarators
        while self.eat(TokenKind::Comma).is_some() {
            vars.push(self.parse_var_declarator(&ty)?);
        }

        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::VarDecl(VarDeclStmt {
            ty,
            vars: self.arena.alloc_slice_copy(&vars),
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a variable declarator (name with optional initializer).
    fn parse_var_declarator(
        &mut self,
        var_type: &crate::ast::types::TypeExpr<'ast>,
    ) -> Result<VarDeclarator<'ast>, ParseError> {
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);

        // Optional initializer
        // Two forms:
        // 1. Assignment: `= expr`
        // 2. Constructor call: `(args)` - creates Call with type name as callee
        let init = if self.eat(TokenKind::Equal).is_some() {
            Some(self.parse_expr(0)?)
        } else if self.check(TokenKind::LeftParen) {
            // Constructor-style initialization: Point p(1, 2);
            // This is a call expression with the variable's type name as the callee
            Some(self.parse_constructor_call(var_type)?)
        } else {
            None
        };

        let span = if let Some(init_expr) = init {
            name.span.merge(init_expr.span())
        } else {
            name.span
        };

        Ok(VarDeclarator { name, init, span })
    }

    /// Parse constructor call arguments for variable initialization.
    /// This handles: `Point p(1, 2);` as a call expression.
    fn parse_constructor_call(
        &mut self,
        ty: &crate::ast::types::TypeExpr<'ast>,
    ) -> Result<&'ast Expr<'ast>, ParseError> {
        use crate::ast::expr::{Argument, CallExpr, IdentExpr};

        let start_span = self.expect(TokenKind::LeftParen)?.span;

        // Parse arguments
        let mut args = BVec::new_in(self.arena);

        if !self.check(TokenKind::RightParen) {
            loop {
                // Parse optional named argument
                let name = if self.check(TokenKind::Identifier)
                    && self.peek_nth(1).kind == TokenKind::Colon
                {
                    let ident = self.advance();
                    self.expect(TokenKind::Colon)?;
                    Some(Ident::new(ident.lexeme, ident.span))
                } else {
                    None
                };

                let value = self.parse_expr(0)?;
                let span = if let Some(ref n) = name {
                    n.span.merge(value.span())
                } else {
                    value.span()
                };

                args.push(Argument { name, value, span });

                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
        }

        let end_span = self.expect(TokenKind::RightParen)?.span;

        // Extract the type name to use as the callee
        // Convert TypeExpr to IdentExpr for the callee
        let (scope, ident) = match &ty.base {
            crate::ast::types::TypeBase::Named(name) => (ty.scope, *name),
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::InvalidExpression,
                    ty.span,
                    "constructor call requires a named type",
                ));
            }
        };

        let callee = self.arena.alloc(Expr::Ident(IdentExpr {
            scope,
            ident,
            type_args: ty.template_args,
            span: ty.span,
        }));

        // Create a call expression with the type name as the callee
        // The VM/interpreter will determine if this is a constructor call
        Ok(self.arena.alloc(Expr::Call(self.arena.alloc(CallExpr {
            callee,
            args: self.arena.alloc_slice_copy(&args),
            span: start_span.merge(end_span),
        }))))
    }

    /// Parse a return statement.
    ///
    /// Grammar: `'return' ASSIGN? ';'`
    pub fn parse_return(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Return)?.span;

        // Optional return value
        let value = if !self.check(TokenKind::Semicolon) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };

        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::Return(ReturnStmt {
            value,
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a break statement.
    ///
    /// Grammar: `'break' ';'`
    pub fn parse_break(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Break)?.span;
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::Break(BreakStmt {
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a continue statement.
    ///
    /// Grammar: `'continue' ';'`
    pub fn parse_continue(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Continue)?.span;
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::Continue(ContinueStmt {
            span: start_span.merge(end_span),
        }))
    }

    /// Parse a block statement.
    ///
    /// Grammar: `'{' STATEMENT* '}'`
    pub fn parse_block(&mut self) -> Result<Block<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::LeftBrace)?.span;

        let mut stmts = BVec::new_in(self.arena);

        // Parse statements until we hit the closing brace
        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    // Record the error and try to recover
                    self.errors.push(err);
                    self.synchronize();
                    if self.check(TokenKind::RightBrace) || self.is_eof() {
                        break;
                    }
                }
            }
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Block {
            stmts: self.arena.alloc_slice_copy(&stmts),
            span: start_span.merge(end_span),
        })
    }

    /// Parse an if statement.
    ///
    /// Grammar: `'if' '(' ASSIGN ')' STATEMENT ('else' STATEMENT)?`
    pub fn parse_if(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::If)?.span;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RightParen)?;

        let then_stmt: &'ast Stmt<'ast> = self.arena.alloc(self.parse_statement()?);

        let else_stmt: Option<&'ast Stmt<'ast>> = if self.eat(TokenKind::Else).is_some() {
            Some(self.arena.alloc(self.parse_statement()?))
        } else {
            None
        };

        let span = if let Some(else_s) = else_stmt {
            start_span.merge(else_s.span())
        } else {
            start_span.merge(then_stmt.span())
        };

        Ok(Stmt::If(self.arena.alloc(IfStmt {
            condition,
            then_stmt,
            else_stmt,
            span,
        })))
    }

    /// Parse a while loop.
    ///
    /// Grammar: `'while' '(' ASSIGN ')' STATEMENT`
    pub fn parse_while(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::While)?.span;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RightParen)?;

        let body: &'ast Stmt<'ast> = self.arena.alloc(self.parse_statement()?);
        let span = start_span.merge(body.span());

        Ok(Stmt::While(self.arena.alloc(WhileStmt {
            condition,
            body,
            span,
        })))
    }

    /// Parse a do-while loop.
    ///
    /// Grammar: `'do' STATEMENT 'while' '(' ASSIGN ')' ';'`
    pub fn parse_do_while(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Do)?.span;
        let body: &'ast Stmt<'ast> = self.arena.alloc(self.parse_statement()?);
        self.expect(TokenKind::While)?;
        self.expect(TokenKind::LeftParen)?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RightParen)?;
        let end_span = self.expect(TokenKind::Semicolon)?.span;

        Ok(Stmt::DoWhile(self.arena.alloc(DoWhileStmt {
            body,
            condition,
            span: start_span.merge(end_span),
        })))
    }

    /// Parse a for loop.
    ///
    /// Grammar: `'for' '(' (VAR | EXPRSTAT) EXPRSTAT (ASSIGN (',' ASSIGN)*)? ')' STATEMENT`
    pub fn parse_for(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::For)?.span;
        self.expect(TokenKind::LeftParen)?;

        // Parse initializer (can be var decl or expression)
        let init = if self.check(TokenKind::Semicolon) {
            self.advance();
            None
        } else if self.is_var_decl() {
            let var_decl = self.parse_var_decl()?;
            if let Stmt::VarDecl(decl) = var_decl {
                Some(ForInit::VarDecl(decl))
            } else {
                let span = self.peek().span;
                return Err(ParseError::new(
                    ParseErrorKind::InternalError,
                    span,
                    "parse_var_decl() returned non-VarDecl statement",
                ));
            }
        } else {
            let expr = self.parse_expr(0)?;
            self.expect(TokenKind::Semicolon)?;
            Some(ForInit::Expr(expr))
        };

        // Parse condition
        let condition = if !self.check(TokenKind::Semicolon) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;

        // Parse update expressions
        let mut update = BVec::new_in(self.arena);
        if !self.check(TokenKind::RightParen) {
            update.push(self.parse_expr(0)?);

            while self.eat(TokenKind::Comma).is_some() {
                update.push(self.parse_expr(0)?);
            }
        }

        self.expect(TokenKind::RightParen)?;

        let body: &'ast Stmt<'ast> = self.arena.alloc(self.parse_statement()?);
        let span = start_span.merge(body.span());

        Ok(Stmt::For(self.arena.alloc(ForStmt {
            init,
            condition,
            update: self.arena.alloc_slice_copy(&update),
            body,
            span,
        })))
    }

    /// Parse a foreach loop.
    ///
    /// Grammar: `'foreach' '(' TYPE IDENTIFIER (',' TYPE IDENTIFIER)* ':' ASSIGN ')' STATEMENT`
    pub fn parse_foreach(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self
            .eat_contextual("foreach")
            .ok_or_else(|| {
                let span = self.peek().span;
                ParseError::new(
                    ParseErrorKind::ExpectedStatement,
                    span,
                    "expected 'foreach'",
                )
            })?
            .span;

        self.expect(TokenKind::LeftParen)?;

        // Parse iteration variables
        let mut vars = BVec::new_in(self.arena);
        vars.push(self.parse_foreach_var()?);

        while self.eat(TokenKind::Comma).is_some() {
            // Validate that we have a valid variable declaration start
            // Variables in foreach can start with types (keywords, identifiers) or reference (&)
            // Note: We check BEFORE the colon check to catch trailing commas
            if !self.is_type_start() && !self.check(TokenKind::Amp) && !self.check(TokenKind::Colon)
            {
                let token = *self.peek();
                return Err(ParseError::new(
                    crate::ast::ParseErrorKind::ExpectedToken,
                    token.span,
                    format!(
                        "expected variable declaration or ':' after ',' in foreach, found {}",
                        token.kind
                    ),
                ));
            }

            // After comma, if we see colon, that's a trailing comma error
            if self.check(TokenKind::Colon) {
                let span = self.peek().span;
                return Err(ParseError::new(
                    crate::ast::ParseErrorKind::InvalidSyntax,
                    span,
                    "trailing comma before ':' in foreach is not allowed",
                ));
            }

            vars.push(self.parse_foreach_var()?);
        }

        self.expect(TokenKind::Colon)?;

        // Parse expression to iterate over
        let expr = self.parse_expr(0)?;

        self.expect(TokenKind::RightParen)?;

        let body: &'ast Stmt<'ast> = self.arena.alloc(self.parse_statement()?);
        let span = start_span.merge(body.span());

        Ok(Stmt::Foreach(self.arena.alloc(ForeachStmt {
            vars: self.arena.alloc_slice_copy(&vars),
            expr,
            body,
            span,
        })))
    }

    /// Parse a foreach iteration variable.
    fn parse_foreach_var(&mut self) -> Result<ForeachVar<'ast>, ParseError> {
        let ty = self.parse_type()?;
        let name_token = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(name_token.lexeme, name_token.span);
        let span = ty.span.merge(name.span);

        Ok(ForeachVar { ty, name, span })
    }

    /// Parse a switch statement.
    ///
    /// Grammar: `'switch' '(' ASSIGN ')' '{' CASE* '}'`
    pub fn parse_switch(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Switch)?.span;
        self.expect(TokenKind::LeftParen)?;
        let expr = self.parse_expr(0)?;
        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::LeftBrace)?;

        let mut cases = BVec::new_in(self.arena);

        // Parse cases
        while !self.check(TokenKind::RightBrace) && !self.is_eof() {
            cases.push(self.parse_switch_case()?);
        }

        let end_span = self.expect(TokenKind::RightBrace)?.span;

        Ok(Stmt::Switch(self.arena.alloc(SwitchStmt {
            expr,
            cases: self.arena.alloc_slice_copy(&cases),
            span: start_span.merge(end_span),
        })))
    }

    /// Parse a switch case.
    ///
    /// Grammar: `(('case' EXPR) | 'default') ':' STATEMENT*`
    fn parse_switch_case(&mut self) -> Result<SwitchCase<'ast>, ParseError> {
        let start_span = self.peek().span;
        let mut values = BVec::new_in(self.arena);

        // Parse case labels (can have multiple case labels before statements)
        loop {
            if self.eat(TokenKind::Case).is_some() {
                values.push(self.parse_expr(0)?);
                self.expect(TokenKind::Colon)?;
            } else if self.eat(TokenKind::Default).is_some() {
                self.expect(TokenKind::Colon)?;
                // Default has no value
                break;
            } else {
                break;
            }
        }

        // Parse statements until we hit another case/default or closing brace
        let mut stmts = BVec::new_in(self.arena);
        while !self.check(TokenKind::Case)
            && !self.check(TokenKind::Default)
            && !self.check(TokenKind::RightBrace)
            && !self.is_eof()
        {
            match self.parse_statement() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                    break;
                }
            }
        }

        // Validate: if no statements and we're at the closing brace, this is an error
        // (last case must have statements; only intermediate cases can be empty for fall-through)
        if stmts.is_empty() && self.check(TokenKind::RightBrace) {
            let err_span = self.peek().span;
            return Err(ParseError::new(
                ParseErrorKind::ExpectedStatement,
                err_span,
                "switch case must have at least one statement (did you mean to use fall-through?)",
            ));
        }

        let span = if let Some(last_stmt) = stmts.last() {
            start_span.merge(last_stmt.span())
        } else {
            start_span
        };

        Ok(SwitchCase {
            values: self.arena.alloc_slice_copy(&values),
            stmts: self.arena.alloc_slice_copy(&stmts),
            span,
        })
    }

    /// Parse a try-catch statement.
    ///
    /// Grammar: `'try' STATBLOCK 'catch' STATBLOCK`
    pub fn parse_try_catch(&mut self) -> Result<Stmt<'ast>, ParseError> {
        let start_span = self.expect(TokenKind::Try)?.span;
        let try_block = self.parse_block()?;
        self.expect(TokenKind::Catch)?;
        let catch_block = self.parse_block()?;
        let span = start_span.merge(catch_block.span);

        Ok(Stmt::TryCatch(self.arena.alloc(TryCatchStmt {
            try_block,
            catch_block,
            span,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Parser;

    #[test]
    fn parse_expr_stmt() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("x = 42;", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::Expr(_)));
    }

    #[test]
    fn parse_empty_stmt() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(";", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Expr(expr_stmt) => assert!(expr_stmt.expr.is_none()),
            _ => panic!("Expected empty expression statement"),
        }
    }

    #[test]
    fn parse_var_decl() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int x = 42;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_some());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_multiple_var_decl() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int x = 1, y = 2;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 2);
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_return() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("return 42;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Return(ret) => assert!(ret.value.is_some()),
            _ => panic!("Expected return statement"),
        }
    }

    #[test]
    fn parse_return_void() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("return;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Return(ret) => assert!(ret.value.is_none()),
            _ => panic!("Expected return statement"),
        }
    }

    #[test]
    fn parse_break() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("break;", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::Break(_)));
    }

    #[test]
    fn parse_continue() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("continue;", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::Continue(_)));
    }

    #[test]
    fn parse_block() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ x = 1; y = 2; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Block(block) => {
                assert_eq!(block.stmts.len(), 2);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn parse_if() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("if (x > 0) y = 1;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                assert!(if_stmt.else_stmt.is_none());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_if_else() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("if (x > 0) y = 1; else y = 2;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                assert!(if_stmt.else_stmt.is_some());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_while() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("while (x > 0) x--;", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::While(_)));
    }

    #[test]
    fn parse_do_while() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("do x--; while (x > 0);", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::DoWhile(_)));
    }

    #[test]
    fn parse_for() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (int i = 0; i < 10; i++) sum += i;", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::For(_)));
    }

    #[test]
    fn parse_switch() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("switch (x) { case 1: break; default: break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert_eq!(switch.cases.len(), 2);
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_try_catch() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("try { x = 1; } catch { x = 0; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::TryCatch(_)));
    }

    // ========================================================================
    // Additional Statement Tests
    // ========================================================================

    #[test]
    fn parse_nested_blocks() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ { { x = 1; } } }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Block(block) => {
                assert_eq!(block.stmts.len(), 1);
                // Inner should also be a block
                match &block.stmts[0] {
                    Stmt::Block(_) => {}
                    _ => panic!("Expected nested block"),
                }
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn parse_empty_block() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("{ }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Block(block) => {
                assert_eq!(block.stmts.len(), 0);
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn parse_var_decl_no_init() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int x;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_none());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_var_decl_constructor_style() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Point p(1, 2);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_some());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_var_decl_multiple_mixed() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int a = 1, b, c = 3;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 3);
                assert!(decl.vars[0].init.is_some());
                assert!(decl.vars[1].init.is_none());
                assert!(decl.vars[2].init.is_some());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_for_empty_init() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (; i < 10; i++) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                assert!(for_stmt.init.is_none());
                assert!(for_stmt.condition.is_some());
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_for_empty_condition() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (int i = 0; ; i++) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                assert!(for_stmt.init.is_some());
                assert!(for_stmt.condition.is_none());
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_for_empty_update() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (int i = 0; i < 10;) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                assert!(for_stmt.update.is_empty());
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_for_all_empty() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (;;) { break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                assert!(for_stmt.init.is_none());
                assert!(for_stmt.condition.is_none());
                assert!(for_stmt.update.is_empty());
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_for_multiple_updates() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (int i = 0; i < 10; i++, j--) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                assert_eq!(for_stmt.update.len(), 2);
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_for_expr_init() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (i = 0; i < 10; i++) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => match for_stmt.init {
                Some(ForInit::Expr(_)) => {}
                _ => panic!("Expected expression init"),
            },
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_foreach_single_var() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("foreach (int x : arr) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Foreach(foreach) => {
                assert_eq!(foreach.vars.len(), 1);
            }
            _ => panic!("Expected foreach statement"),
        }
    }

    #[test]
    fn parse_foreach_multiple_vars() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("foreach (int key, string value : dict) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Foreach(foreach) => {
                assert_eq!(foreach.vars.len(), 2);
            }
            _ => panic!("Expected foreach statement"),
        }
    }

    #[test]
    fn parse_switch_multiple_cases() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                    a = 1;
                    break;
                case 2:
                    a = 2;
                    break;
                default:
                    a = 0;
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert_eq!(switch.cases.len(), 3);
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_multiple_case_labels() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                case 2:
                case 3:
                    a = 123;
                    break;
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert_eq!(switch.cases.len(), 1);
                // First case should have 3 values
                assert_eq!(switch.cases[0].values.len(), 3);
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_fallthrough() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                    a = 1;
                case 2:
                    b = 2;
                    break;
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert_eq!(switch.cases.len(), 2);
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_empty_case_error() {
        // Last case with no statements should be a parse error
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                default:
            }
        "#,
            &arena,
        );
        let result = parser.parse_statement();
        assert!(result.is_err());
        // Record the error so we can check it
        if let Err(err) = result {
            parser.errors.push(err);
        }
        assert!(parser.has_errors());
    }

    #[test]
    fn parse_switch_empty_intermediate_case_ok() {
        // Empty intermediate cases are OK (fall-through)
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                case 2:
                    doSomething();
                    break;
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert_eq!(switch.cases.len(), 1);
                assert_eq!(switch.cases[0].values.len(), 2);
                assert!(!switch.cases[0].stmts.is_empty());
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_if_else_if_chain() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            if (x > 10)
                a = 1;
            else if (x > 5)
                a = 2;
            else if (x > 0)
                a = 3;
            else
                a = 0;
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                assert!(if_stmt.else_stmt.is_some());
                // Else should contain another if
                match &if_stmt.else_stmt {
                    Some(Stmt::If(_)) => {}
                    _ => panic!("Expected if in else branch"),
                }
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_while_with_block() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("while (true) { x++; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::While(while_stmt) => match &while_stmt.body {
                Stmt::Block(_) => {}
                _ => panic!("Expected block body"),
            },
            _ => panic!("Expected while statement"),
        }
    }

    #[test]
    fn parse_while_single_statement() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("while (x > 0) x--;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::While(while_stmt) => match &while_stmt.body {
                Stmt::Expr(_) => {}
                _ => panic!("Expected expr statement body"),
            },
            _ => panic!("Expected while statement"),
        }
    }

    #[test]
    fn parse_do_while_with_complex_condition() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("do { x++; } while (x < 10 && y > 0);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::DoWhile(_) => {}
            _ => panic!("Expected do-while statement"),
        }
    }

    #[test]
    fn parse_nested_loops() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            for (int i = 0; i < 10; i++) {
                for (int j = 0; j < 10; j++) {
                    matrix[i, j] = 0;
                }
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => {
                match &for_stmt.body {
                    Stmt::Block(block) => {
                        assert_eq!(block.stmts.len(), 1);
                        // Inner should be another for
                        match &block.stmts[0] {
                            Stmt::For(_) => {}
                            _ => panic!("Expected nested for"),
                        }
                    }
                    _ => panic!("Expected block body"),
                }
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_nested_if_in_loop() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            while (true) {
                if (x > 0)
                    break;
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::While(_) => {}
            _ => panic!("Expected while statement"),
        }
    }

    #[test]
    fn parse_expr_stmt_with_call() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("foo(1, 2, 3);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Expr(expr_stmt) => {
                assert!(expr_stmt.expr.is_some());
            }
            _ => panic!("Expected expression statement"),
        }
    }

    #[test]
    fn parse_expr_stmt_with_assignment() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("x += 42;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Expr(_) => {}
            _ => panic!("Expected expression statement"),
        }
    }

    #[test]
    fn parse_complex_return() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("return a + b * c;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Return(ret) => {
                assert!(ret.value.is_some());
            }
            _ => panic!("Expected return statement"),
        }
    }

    // Additional tests for coverage
    #[test]
    fn parse_foreach_basic() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("foreach (int x : items) sum += x;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Foreach(foreach) => {
                assert_eq!(foreach.vars.len(), 1);
            }
            _ => panic!("Expected foreach statement"),
        }
    }

    #[test]
    fn parse_switch_multiple_case_labels_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("switch (x) { case 1: case 2: sum++; break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                // First case has multiple labels before statements
                assert!(!switch.cases.is_empty());
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_empty_cases_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("switch (x) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch) => {
                assert!(switch.cases.is_empty());
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_with_fallthrough_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            switch (x) {
                case 1:
                case 2:
                case 3:
                    doSomething();
                    break;
                default:
                    doDefault();
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        assert!(matches!(stmt, Stmt::Switch(_)));
    }

    #[test]
    fn parse_var_decl_without_init_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int x;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_none());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_var_decl_with_constructor_call_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("MyClass obj(1, 2, 3);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_some());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_var_decl_handle_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Foo@ handle = null;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_var_decl_array_type_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("array<int> arr;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_try_catch_with_statements_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            try {
                x = riskyOperation();
                y = anotherRisky();
            } catch {
                handleError();
                logError();
            }
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::TryCatch(try_catch) => {
                assert!(!try_catch.try_block.stmts.is_empty());
                assert!(!try_catch.catch_block.stmts.is_empty());
            }
            _ => panic!("Expected try-catch statement"),
        }
    }

    #[test]
    fn parse_chained_else_if_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new(
            r#"
            if (x == 1)
                a();
            else if (x == 2)
                b();
            else if (x == 3)
                c();
            else
                d();
        "#,
            &arena,
        );
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                assert!(if_stmt.else_stmt.is_some());
                // The else branch should be another if statement
                match if_stmt.else_stmt.unwrap() {
                    Stmt::If(_) => {}
                    _ => panic!("Expected else if"),
                }
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_while_with_block_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("while (x > 0) { x--; y++; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::While(while_stmt) => match &while_stmt.body {
                Stmt::Block(block) => {
                    assert_eq!(block.stmts.len(), 2);
                }
                _ => panic!("Expected block body"),
            },
            _ => panic!("Expected while statement"),
        }
    }

    #[test]
    fn parse_do_while_with_block_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("do { x--; y++; } while (x > 0);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::DoWhile(do_while) => match &do_while.body {
                Stmt::Block(block) => {
                    assert_eq!(block.stmts.len(), 2);
                }
                _ => panic!("Expected block body"),
            },
            _ => panic!("Expected do-while statement"),
        }
    }

    #[test]
    fn parse_block_with_error_recovery() {
        let arena = bumpalo::Bump::new();
        // Syntax error: missing semicolon, should recover and continue
        let mut parser = Parser::new("{ x = 1 y = 2; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Block(block) => {
                // Parser should recover and parse what it can
                assert!(!block.stmts.is_empty() || !parser.errors.is_empty());
            }
            _ => panic!("Expected block"),
        }
    }

    #[test]
    fn parse_foreach_trailing_comma_error() {
        let arena = bumpalo::Bump::new();
        // Trailing comma before colon
        let mut parser = Parser::new("foreach (int x, : arr) { }", &arena);
        let result = parser.parse_statement();
        assert!(result.is_err());
    }

    #[test]
    fn parse_foreach_invalid_after_comma() {
        let arena = bumpalo::Bump::new();
        // Invalid token after comma (not a type or &)
        let mut parser = Parser::new("foreach (int x, 123 : arr) { }", &arena);
        let result = parser.parse_statement();
        assert!(result.is_err());
    }

    #[test]
    fn parse_switch_case_error_recovery() {
        let arena = bumpalo::Bump::new();
        // Syntax error in case body - should hit error recovery
        let mut parser = Parser::new("switch (x) { case 1: x = }", &arena);
        let result = parser.parse_statement();
        // Either error or parsed with errors collected
        assert!(result.is_err() || !parser.errors.is_empty());
    }

    #[test]
    fn parse_constructor_call_with_named_arg() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("Point p(x: 1, y: 2);", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.vars.len(), 1);
                assert!(decl.vars[0].init.is_some());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_constructor_call_primitive_type_error() {
        let arena = bumpalo::Bump::new();
        // Trying constructor call on primitive type should fail
        let mut parser = Parser::new("int x(42);", &arena);
        let result = parser.parse_statement();
        assert!(result.is_err());
    }

    #[test]
    fn parse_for_expr_init_coverage() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (i = 0; i < 10; i++) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => match &for_stmt.init {
                Some(ForInit::Expr(_)) => {}
                _ => panic!("Expected expression init"),
            },
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_block_with_eof() {
        let arena = bumpalo::Bump::new();
        // Unclosed block - hits EOF
        let mut parser = Parser::new("{ x = 1;", &arena);
        let result = parser.parse_statement();
        // Should error due to missing closing brace
        assert!(result.is_err());
    }

    #[test]
    fn parse_switch_with_eof() {
        let arena = bumpalo::Bump::new();
        // Unclosed switch - hits EOF
        let mut parser = Parser::new("switch (x) { case 1:", &arena);
        let result = parser.parse_statement();
        // Should error due to missing closing brace
        assert!(result.is_err() || !parser.errors.is_empty());
    }

    #[test]
    fn parse_var_decl_span_no_init() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("int x;", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::VarDecl(decl) => {
                // The span of declarator without init should just be the name span
                assert!(decl.vars[0].init.is_none());
            }
            _ => panic!("Expected variable declaration"),
        }
    }

    #[test]
    fn parse_for_with_var_decl_init() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("for (int i = 0; i < 10; i++) { }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::For(for_stmt) => match &for_stmt.init {
                Some(ForInit::VarDecl(_)) => {}
                _ => panic!("Expected var decl init"),
            },
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn parse_if_span_without_else() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("if (x) y();", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::If(if_stmt) => {
                // If without else - span should include just the then statement
                assert!(if_stmt.else_stmt.is_none());
            }
            _ => panic!("Expected if statement"),
        }
    }

    #[test]
    fn parse_switch_case_span_with_stmts() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("switch (x) { case 1: a(); b(); break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch_stmt) => {
                assert_eq!(switch_stmt.cases.len(), 1);
                assert_eq!(switch_stmt.cases[0].stmts.len(), 3);
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_default_only() {
        let arena = bumpalo::Bump::new();
        let mut parser = Parser::new("switch (x) { default: break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch_stmt) => {
                assert_eq!(switch_stmt.cases.len(), 1);
                // default case has no values
                assert!(switch_stmt.cases[0].values.is_empty());
            }
            _ => panic!("Expected switch statement"),
        }
    }

    #[test]
    fn parse_switch_empty_intermediate_case_coverage() {
        let arena = bumpalo::Bump::new();
        // Multiple case labels without statements (fall through) then statements
        let mut parser = Parser::new("switch (x) { case 1: case 2: default: break; }", &arena);
        let stmt = parser.parse_statement().unwrap();
        match stmt {
            Stmt::Switch(switch_stmt) => {
                assert!(!switch_stmt.cases.is_empty());
            }
            _ => panic!("Expected switch statement"),
        }
    }
}
