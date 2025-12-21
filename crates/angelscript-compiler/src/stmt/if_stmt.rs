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

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_if(&if_stmt).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // PushTrue: 1 byte (offset 0)
        // JumpIfFalse + offset: 3 bytes (offset 1-3)
        // Pop: 1 byte (offset 4)
        // (empty then block)
        // Jump + offset: 3 bytes (offset 5-7)
        // Pop: 1 byte (offset 8)
        // Total: 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        // Jump distance: len(8) - offset(2) - 2 = 4
        assert_eq!(chunk.read_u16(2), Some(4));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(5), Some(OpCode::Jump));
        // Skip jump distance: len(9) - offset(6) - 2 = 1
        assert_eq!(chunk.read_u16(6), Some(1));
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop));
    }

    #[test]
    fn if_else() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_if(&if_stmt).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // PushTrue: 1 byte (offset 0)
        // JumpIfFalse + offset: 3 bytes (offset 1-3)
        // Pop: 1 byte (offset 4)
        // (empty then block)
        // Jump + offset: 3 bytes (offset 5-7)
        // Pop: 1 byte (offset 8)
        // (empty else block)
        // Total: 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        // Jump distance to else: len(8) - offset(2) - 2 = 4
        assert_eq!(chunk.read_u16(2), Some(4));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(5), Some(OpCode::Jump));
        // Jump distance to end: len(9) - offset(6) - 2 = 1
        assert_eq!(chunk.read_u16(6), Some(1));
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop));
    }

    #[test]
    fn if_non_bool_condition_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_if(&outer_if).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // Outer: PushTrue(1) + JumpIfFalse(3) + Pop(1) = 5 bytes before inner
        // Inner: PushFalse(1) + JumpIfFalse(3) + Pop(1) + Jump(3) + Pop(1) = 9 bytes
        // Outer: Jump(3) + Pop(1) = 4 bytes
        // Total: 18 bytes
        assert_eq!(chunk.len(), 18);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Inner if starts at offset 5
        assert_eq!(chunk.read_op(5), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(6), Some(OpCode::JumpIfFalse));
    }

    #[test]
    fn if_else_if_chain() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_if(&if_stmt).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // 0: PushTrue
        // 1-3: JumpIfFalse -> 8 (else-if condition)
        // 4: Pop (true path)
        // 5-7: Jump -> 18 (end, skips ALL else branches)
        // 8: Pop (false path from first condition)
        // 9: PushFalse
        // 10-12: JumpIfFalse -> 17 (else block)
        // 13: Pop (true path)
        // 14-16: Jump -> 18 (end)
        // 17: Pop (false path, else block is empty)
        // Total: 18 bytes
        assert_eq!(chunk.len(), 18);

        // First if condition
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        // If first condition false, jump to else-if at offset 8
        // Jump distance: 8 - 2 - 2 = 4
        assert_eq!(chunk.read_u16(2), Some(4));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));

        // After first then-block, jump to END (offset 18), skipping else-if AND else
        assert_eq!(chunk.read_op(5), Some(OpCode::Jump));
        // Jump distance: 18 - 6 - 2 = 10
        assert_eq!(chunk.read_u16(6), Some(10));

        // Else-if condition starts at offset 8
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(9), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(10), Some(OpCode::JumpIfFalse));
        // If else-if condition false, jump to else at offset 17
        // Jump distance: 17 - 11 - 2 = 4
        assert_eq!(chunk.read_u16(11), Some(4));
        assert_eq!(chunk.read_op(13), Some(OpCode::Pop));

        // After else-if then-block, jump to END (offset 18)
        assert_eq!(chunk.read_op(14), Some(OpCode::Jump));
        // Jump distance: 18 - 15 - 2 = 1
        assert_eq!(chunk.read_u16(15), Some(1));

        // Final else block (just pops the condition)
        assert_eq!(chunk.read_op(17), Some(OpCode::Pop));
    }

    #[test]
    fn if_with_var_decl_in_body() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_if(&if_stmt).unwrap();

        // Variable x should be out of scope after the if block
        assert!(ctx.get_local("x").is_none());

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // PushTrue(1) + JumpIfFalse(3) + Pop(1) = 5 bytes
        // var decl: Constant(2) + SetLocal(2) = 4 bytes
        // Jump(3) + Pop(1) = 4 bytes
        // Total: 13 bytes
        assert_eq!(chunk.len(), 13);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Var decl starts at offset 5
        assert_eq!(chunk.read_op(5), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(7), Some(OpCode::SetLocal));
    }
}
