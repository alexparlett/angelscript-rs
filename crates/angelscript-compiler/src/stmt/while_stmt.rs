//! While loop compilation.
//!
//! Handles while loops with proper loop context for break/continue statements.

use angelscript_core::{DataType, primitives};
use angelscript_parser::ast::WhileStmt;

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a while loop.
    ///
    /// The condition must evaluate to bool. Creates a loop context for
    /// break/continue support.
    ///
    /// Bytecode layout:
    /// ```text
    /// loop_start:
    /// [condition]
    /// JumpIfFalse -> exit
    /// Pop (true path - pop condition)
    /// [body]
    /// Loop -> loop_start
    /// exit:
    /// Pop (false path - pop condition)
    /// ```
    pub fn compile_while<'ast>(&mut self, while_stmt: &WhileStmt<'ast>) -> Result<()> {
        // Mark loop start for backward jump
        let loop_start = self.emitter.current_offset();

        // Enter loop context (enables break/continue)
        self.emitter.enter_loop(loop_start);

        // Compile condition - must be bool
        let bool_type = DataType::simple(primitives::BOOL);
        let condition_result = {
            let mut expr_compiler = self.expr_compiler();
            expr_compiler.check(while_stmt.condition, &bool_type)
        };

        // If condition compilation fails, clean up loop context before returning error
        if let Err(e) = condition_result {
            self.emitter.exit_loop();
            return Err(e);
        }

        // Exit loop if false
        let exit_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // True path: pop condition
        self.emitter.emit_pop();

        // Compile loop body
        self.compile(while_stmt.body)?;

        // Jump back to start (loop)
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
    use angelscript_core::Span;
    use angelscript_parser::ast::{
        Block, BreakStmt, ContinueStmt, Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, Stmt,
        TypeExpr, VarDeclStmt, VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn while_loop_basic() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let while_stmt = WhileStmt {
            condition,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&while_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode layout:
        // PushTrue(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        // JumpIfFalse distance: len(8) - offset(2) - 2 = 4
        assert_eq!(chunk.read_u16(2), Some(4));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Empty body
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop));
        // Loop back offset calculation in emit_loop
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop));
    }

    #[test]
    fn while_non_bool_condition_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Use an integer as condition - should fail
        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let while_stmt = WhileStmt {
            condition,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let result = compiler.compile_while(&while_stmt);
        assert!(result.is_err());
    }

    #[test]
    fn while_with_break() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        // while (true) { break; }
        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });

        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let while_stmt = WhileStmt {
            condition,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&while_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode layout:
        // PushTrue(1) + JumpIfFalse(3) + Pop(1) + Jump(3, break) + Loop(3) + Pop(1) = 12 bytes
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Break is a Jump
        assert_eq!(chunk.read_op(5), Some(OpCode::Jump));
        // Break jump distance: patched at exit_loop
        assert_eq!(chunk.read_op(8), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(11), Some(OpCode::Pop));
    }

    #[test]
    fn while_with_continue() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        // while (true) { continue; }
        let continue_stmt = Stmt::Continue(ContinueStmt {
            span: Span::default(),
        });

        let stmts = arena.alloc_slice_copy(&[continue_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let while_stmt = WhileStmt {
            condition,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&while_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode layout:
        // PushTrue(1) + JumpIfFalse(3) + Pop(1) + Loop(3, continue) + Loop(3) + Pop(1) = 12 bytes
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Continue is a Loop back to start
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop));
        // Continue loop offset back to 0
        assert_eq!(chunk.read_u16(6), Some(8)); // 8 bytes back
        // Regular loop at end
        assert_eq!(chunk.read_op(8), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(11), Some(OpCode::Pop));
    }

    #[test]
    fn nested_while_loops() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Inner while
        let inner_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let inner_body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let inner_while = arena.alloc(WhileStmt {
            condition: inner_condition,
            body: inner_body,
            span: Span::default(),
        });

        // Outer while
        let outer_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let inner_while_stmt = arena.alloc_slice_copy(&[Stmt::While(inner_while)]);
        let outer_body = arena.alloc(Stmt::Block(Block {
            stmts: inner_while_stmt,
            span: Span::default(),
        }));

        let outer_while = WhileStmt {
            condition: outer_condition,
            body: outer_body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&outer_while).unwrap();

        let chunk = emitter.finish();
        // Bytecode layout:
        // Outer: PushTrue(1) + JumpIfFalse(3) + Pop(1) = 5 bytes before inner
        // Inner: PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        // Outer: Loop(3) + Pop(1) = 4 bytes
        // Total: 18 bytes
        assert_eq!(chunk.len(), 18);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Inner loop starts at offset 5
        assert_eq!(chunk.read_op(5), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(6), Some(OpCode::JumpIfFalse));
        // Outer loop at end
        assert_eq!(chunk.read_op(14), Some(OpCode::Loop));
    }

    #[test]
    fn while_with_var_decl_in_body() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        // int x = 0;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(0),
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

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let while_stmt = WhileStmt {
            condition,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&while_stmt).unwrap();

        // Variable x should be out of scope after the loop
        assert!(ctx.get_local("x").is_none());

        let chunk = emitter.finish();
        // Bytecode layout:
        // PushFalse(1) + JumpIfFalse(3) + Pop(1) = 5 bytes
        // var decl: PushZero(1) + SetLocal(2) = 3 bytes
        // Loop(3) + Pop(1) = 4 bytes
        // Total: 12 bytes
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop));
        // Var decl: int x = 0 uses PushZero optimization
        assert_eq!(chunk.read_op(5), Some(OpCode::PushZero));
        assert_eq!(chunk.read_op(6), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(7), Some(0)); // Slot 0
        assert_eq!(chunk.read_op(8), Some(OpCode::Loop));
    }

    #[test]
    fn break_targets_innermost_loop() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Inner while with break
        let inner_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let inner_stmts = arena.alloc_slice_copy(&[break_stmt]);

        let inner_body = arena.alloc(Stmt::Block(Block {
            stmts: inner_stmts,
            span: Span::default(),
        }));

        let inner_while = arena.alloc(WhileStmt {
            condition: inner_condition,
            body: inner_body,
            span: Span::default(),
        });

        // Outer while
        let outer_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let outer_stmts = arena.alloc_slice_copy(&[Stmt::While(inner_while)]);
        let outer_body = arena.alloc(Stmt::Block(Block {
            stmts: outer_stmts,
            span: Span::default(),
        }));

        let outer_while = WhileStmt {
            condition: outer_condition,
            body: outer_body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_while(&outer_while).unwrap();

        let chunk = emitter.finish();
        // Bytecode layout:
        // Outer: PushTrue(1) + JumpIfFalse(3) + Pop(1) = 5 bytes
        // Inner: PushTrue(1) + JumpIfFalse(3) + Pop(1) + Jump(3, break) + Loop(3) + Pop(1) = 12 bytes
        // Outer: Loop(3) + Pop(1) = 4 bytes
        // Total: 21 bytes
        assert_eq!(chunk.len(), 21);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        // Inner loop starts at offset 5
        assert_eq!(chunk.read_op(5), Some(OpCode::PushTrue));
        // Inner break at offset 10
        assert_eq!(chunk.read_op(10), Some(OpCode::Jump));
        // Outer loop at offset 17
        assert_eq!(chunk.read_op(17), Some(OpCode::Loop));
    }
}
