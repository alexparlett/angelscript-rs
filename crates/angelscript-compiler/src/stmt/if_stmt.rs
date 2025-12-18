//! If/else statement compilation.
//!
//! Handles if statements with optional else branches, including:
//! - Simple if: `if (cond) stmt`
//! - If-else: `if (cond) stmt else stmt`
//! - Chained else-if: `if (cond) stmt else if (cond2) stmt2 else stmt3`

use angelscript_core::{DataType, primitives};
use angelscript_parser::ast::IfStmt;

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile an if statement.
    ///
    /// The condition must evaluate to bool. Generates appropriate jump
    /// instructions for control flow.
    pub fn compile_if<'ast>(&mut self, if_stmt: &IfStmt<'ast>) -> Result<()> {
        // Compile condition - must be bool
        let bool_type = DataType::simple(primitives::BOOL);
        let mut expr_compiler = self.expr_compiler();
        expr_compiler.check(if_stmt.condition, &bool_type)?;

        if let Some(else_stmt) = if_stmt.else_stmt {
            // if-else form
            self.compile_if_else(if_stmt.then_stmt, else_stmt)
        } else {
            // if-only form
            self.compile_if_only(if_stmt.then_stmt)
        }
    }

    /// Compile if without else branch.
    ///
    /// Bytecode layout:
    /// ```text
    /// [condition]
    /// JumpIfFalse -> end
    /// Pop (true path - pop condition)
    /// [then branch]
    /// end:
    /// Pop (false path - pop condition)
    /// ```
    fn compile_if_only<'ast>(
        &mut self,
        then_stmt: &angelscript_parser::ast::Stmt<'ast>,
    ) -> Result<()> {
        // Jump to end if false
        let end_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // True path: pop condition and execute then branch
        self.emitter.emit_pop();
        self.compile(then_stmt)?;

        // Jump over the false-path pop
        let skip_false_pop = self.emitter.emit_jump(OpCode::Jump);

        // False path target
        self.emitter.patch_jump(end_jump);

        // False path: pop condition
        self.emitter.emit_pop();

        // Patch skip jump to here
        self.emitter.patch_jump(skip_false_pop);

        Ok(())
    }

    /// Compile if with else branch.
    ///
    /// Bytecode layout:
    /// ```text
    /// [condition]
    /// JumpIfFalse -> else
    /// Pop (true path - pop condition)
    /// [then branch]
    /// Jump -> end
    /// else:
    /// Pop (false path - pop condition)
    /// [else branch]
    /// end:
    /// ```
    fn compile_if_else<'ast>(
        &mut self,
        then_stmt: &angelscript_parser::ast::Stmt<'ast>,
        else_stmt: &angelscript_parser::ast::Stmt<'ast>,
    ) -> Result<()> {
        // Jump to else if false
        let else_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // True path: pop condition and execute then branch
        self.emitter.emit_pop();
        self.compile(then_stmt)?;

        // Jump over else branch
        let end_jump = self.emitter.emit_jump(OpCode::Jump);

        // Else target: pop condition and execute else branch
        self.emitter.patch_jump(else_jump);
        self.emitter.emit_pop();
        self.compile(else_stmt)?;

        // End target
        self.emitter.patch_jump(end_jump);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::Span;
    use angelscript_parser::ast::{
        Block, Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, Stmt, TypeExpr, VarDeclStmt,
        VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn if_only_true_condition() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let then_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let if_stmt = IfStmt {
            condition,
            then_stmt,
            else_stmt: None,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_if(&if_stmt).is_ok());

        let chunk = emitter.finish();
        // Should have: PushTrue, JumpIfFalse, Pop, Jump, Pop
        assert!(chunk.len() > 0);
    }

    #[test]
    fn if_else() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let then_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let else_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let if_stmt = IfStmt {
            condition,
            then_stmt,
            else_stmt: Some(else_stmt),
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_if(&if_stmt).is_ok());

        let chunk = emitter.finish();
        // Should have bytecode for condition, jumps, and branches
        assert!(chunk.len() > 0);
    }

    #[test]
    fn if_non_bool_condition_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Use an integer as condition - should fail
        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let then_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let if_stmt = IfStmt {
            condition,
            then_stmt,
            else_stmt: None,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let result = compiler.compile_if(&if_stmt);
        assert!(result.is_err());
    }

    #[test]
    fn nested_if() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Inner if
        let inner_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let inner_then = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let inner_if = arena.alloc(IfStmt {
            condition: inner_condition,
            then_stmt: inner_then,
            else_stmt: None,
            span: Span::default(),
        });

        // Outer if with inner if as then branch
        let outer_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let outer_then = arena.alloc(Stmt::If(inner_if));

        let outer_if = IfStmt {
            condition: outer_condition,
            then_stmt: outer_then,
            else_stmt: None,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_if(&outer_if).is_ok());
    }

    #[test]
    fn if_else_if_chain() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // else if (false) {}
        let else_if_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let else_if_then = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let else_block = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let else_if = arena.alloc(IfStmt {
            condition: else_if_condition,
            then_stmt: else_if_then,
            else_stmt: Some(else_block),
            span: Span::default(),
        });

        // if (true) {} else if (false) {} else {}
        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let then_stmt = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let else_stmt = arena.alloc(Stmt::If(else_if));

        let if_stmt = IfStmt {
            condition,
            then_stmt,
            else_stmt: Some(else_stmt),
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_if(&if_stmt).is_ok());
    }

    #[test]
    fn if_with_var_decl_in_body() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let var_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(var_decl)]);

        let then_block = Block {
            stmts,
            span: Span::default(),
        };

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let then_stmt = arena.alloc(Stmt::Block(then_block));

        let if_stmt = IfStmt {
            condition,
            then_stmt,
            else_stmt: None,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_if(&if_stmt).is_ok());

        // Variable x should be out of scope after the if block
        assert!(ctx.get_local("x").is_none());
    }
}
