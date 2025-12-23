//! Do-while loop compilation.
//!
//! Handles do-while loops where the body executes at least once before
//! the condition is checked.

use angelscript_core::{DataType, primitives};
use angelscript_parser::ast::DoWhileStmt;

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
    /// Compile a do-while loop.
    ///
    /// The body executes first, then the condition is checked. If true,
    /// the loop repeats. Continue jumps to the condition check.
    ///
    /// Bytecode layout:
    /// ```text
    /// loop_start:
    /// [body]
    /// condition_start:  <- continue target
    /// [condition]
    /// JumpIfFalse -> exit
    /// Pop (true path - pop condition)
    /// Loop -> loop_start
    /// exit:
    /// Pop (false path - pop condition)
    /// ```
    pub fn compile_do_while<'ast>(&mut self, do_while: &DoWhileStmt<'ast>) -> Result<()> {
        // Mark loop start
        let loop_start = self.emitter.current_offset();

        // Enter loop context with deferred continue target
        // (continue target is the condition, which comes after the body)
        self.emitter.enter_loop_deferred();

        // Compile body first (executes at least once)
        self.compile(do_while.body)?;

        // Set continue target to condition (after body)
        // This also patches any continue statements emitted during body compilation
        let condition_start = self.emitter.current_offset();
        self.emitter.set_continue_target(condition_start);

        // Compile condition - must be bool
        let bool_type = DataType::simple(primitives::BOOL);
        let condition_result = {
            let mut expr_compiler = self.expr_compiler();
            expr_compiler.check(do_while.condition, &bool_type)
        };

        // If condition compilation fails, clean up loop context before returning error
        if let Err(e) = condition_result {
            self.emitter.exit_loop();
            return Err(e);
        }

        // Exit loop if condition is false
        let exit_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // True path: pop condition and loop back
        self.emitter.emit_pop();
        self.emitter.emit_loop(loop_start);

        // Exit target
        self.emitter.patch_jump(exit_jump);

        // False path: pop condition
        self.emitter.emit_pop();

        // Exit loop context (patches break jumps)
        self.emitter.exit_loop();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{CompilationError, Span};
    use angelscript_parser::ast::{
        Block, BreakStmt, ContinueStmt, Expr, LiteralExpr, LiteralKind, Stmt,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn do_while_basic() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // do {} while (false);
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let do_while = DoWhileStmt {
            body,
            condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_do_while(&do_while).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode layout:
        // (empty body)
        // PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        // JumpIfFalse jumps over Pop + Loop = 4 bytes
        assert_eq!(chunk.read_u16(2), Some(4));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop));
    }

    #[test]
    fn do_while_with_true_condition() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // do {} while (true); - infinite loop
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let do_while = DoWhileStmt {
            body,
            condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_do_while(&do_while).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode: PushTrue(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop));
        // Loop jumps back 8 bytes to start (offset 0)
        assert_eq!(chunk.read_u16(6), Some(8));
    }

    #[test]
    fn do_while_with_break() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // do { break; } while (true);
        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let do_while = DoWhileStmt {
            body,
            condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_do_while(&do_while).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode: Jump(3, break) + PushTrue(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 12
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::Jump)); // break
        assert_eq!(chunk.read_op(3), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(4), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(7), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(8), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(11), Some(OpCode::Pop));
    }

    #[test]
    fn do_while_with_continue() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // do { continue; } while (false);
        let continue_stmt = Stmt::Continue(ContinueStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[continue_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let do_while = DoWhileStmt {
            body,
            condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_do_while(&do_while).unwrap();

        let chunk = emitter.finish_chunk();
        // Bytecode: Jump(3, continue - forward to condition) + PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 12
        assert_eq!(chunk.len(), 12);
        // Continue is a forward Jump (patched to condition) since continue target is deferred
        assert_eq!(chunk.read_op(0), Some(OpCode::Jump));
        // Jump distance should point to offset 3 (condition start): 3 - 3 = 0
        assert_eq!(chunk.read_u16(1), Some(0)); // Jump 0 bytes forward to offset 3
        assert_eq!(chunk.read_op(3), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(7), Some(OpCode::Pop));
        assert_eq!(chunk.read_op(8), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(11), Some(OpCode::Pop));
    }

    #[test]
    fn do_while_non_bool_condition_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // do {} while (42); - should fail
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let do_while = DoWhileStmt {
            body,
            condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_do_while(&do_while);
        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TypeMismatch { .. } => {}
            other => panic!(
                "Expected TypeMismatch error for non-bool condition, got: {:?}",
                other
            ),
        }
    }

    #[test]
    fn nested_do_while() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Inner do-while
        let inner_body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let inner_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let inner_do_while = arena.alloc(DoWhileStmt {
            body: inner_body,
            condition: inner_condition,
            span: Span::default(),
        });

        // Outer do-while
        let outer_stmts = arena.alloc_slice_copy(&[Stmt::DoWhile(inner_do_while)]);
        let outer_body = arena.alloc(Stmt::Block(Block {
            stmts: outer_stmts,
            span: Span::default(),
        }));

        let outer_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let outer_do_while = DoWhileStmt {
            body: outer_body,
            condition: outer_condition,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_do_while(&outer_do_while).unwrap();

        let chunk = emitter.finish_chunk();
        // Inner: PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        // Outer: (inner 9) + PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 18 bytes
        assert_eq!(chunk.len(), 18);
        // Inner loop at offset 0
        assert_eq!(chunk.read_op(0), Some(OpCode::PushFalse));
        // Outer loop condition at offset 9
        assert_eq!(chunk.read_op(9), Some(OpCode::PushFalse));
    }
}
