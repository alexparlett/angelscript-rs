//! For loop compilation.
//!
//! Handles C-style for loops with optional init, condition, and update parts.

use angelscript_core::{DataType, primitives};
use angelscript_parser::ast::{ForInit, ForStmt};

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a for loop.
    ///
    /// For loops have the form: `for (init; condition; update) body`
    /// All parts are optional:
    /// - `for (;;)` is an infinite loop
    /// - `for (; condition;)` is like while
    /// - `for (init;;)` declares a variable and loops forever
    ///
    /// The init variable is scoped to the for loop.
    /// Continue jumps to the update expression, not the condition.
    ///
    /// Bytecode layout:
    /// ```text
    /// [init - var decl or expr]
    /// loop_start:
    /// [condition, if present]
    /// JumpIfFalse -> exit (if condition)
    /// Pop (true path)
    /// [body]
    /// continue_target:
    /// [update expressions]
    /// Loop -> loop_start
    /// exit:
    /// Pop (false path, if condition)
    /// [cleanup scope handles]
    /// ```
    pub fn compile_for<'ast>(&mut self, for_stmt: &ForStmt<'ast>) -> Result<()> {
        // Push scope for init variable
        self.ctx.push_local_scope();

        // Compile init (if present)
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::VarDecl(var_decl) => {
                    self.compile_var_decl(var_decl)?;
                }
                ForInit::Expr(expr) => {
                    let mut expr_compiler = self.expr_compiler();
                    let info = expr_compiler.infer(expr)?;
                    // Pop result if non-void (init evaluated for side effects)
                    if !info.data_type.is_void() {
                        self.emitter.emit_pop();
                    }
                }
            }
        }

        // Mark loop start (at condition)
        let loop_start = self.emitter.current_offset();

        // Enter loop context - continue target will be updated to update expressions
        self.emitter.enter_loop(loop_start);

        // Compile condition (if present)
        let exit_jump = if let Some(condition) = for_stmt.condition {
            let bool_type = DataType::simple(primitives::BOOL);
            let condition_result = {
                let mut expr_compiler = self.expr_compiler();
                expr_compiler.check(condition, &bool_type)
            };

            // If condition compilation fails, clean up before returning error
            if let Err(e) = condition_result {
                self.emitter.exit_loop();
                self.ctx.pop_local_scope();
                return Err(e);
            }

            // Exit loop if condition is false
            let jump = self.emitter.emit_jump(OpCode::JumpIfFalse);
            // True path: pop condition
            self.emitter.emit_pop();
            Some(jump)
        } else {
            // No condition = infinite loop
            None
        };

        // Compile body
        if let Err(e) = self.compile(for_stmt.body) {
            self.emitter.exit_loop();
            self.ctx.pop_local_scope();
            return Err(e);
        }

        // Set continue target to update expressions (after body, before updates)
        let continue_target = self.emitter.current_offset();
        self.emitter.set_continue_target(continue_target);

        // Compile update expressions
        for update_expr in for_stmt.update {
            let mut expr_compiler = self.expr_compiler();
            let info = expr_compiler.infer(update_expr)?;
            // Pop result if non-void (update evaluated for side effects)
            if !info.data_type.is_void() {
                self.emitter.emit_pop();
            }
        }

        // Loop back to condition
        self.emitter.emit_loop(loop_start);

        // Exit path (if we have a condition)
        if let Some(jump) = exit_jump {
            self.emitter.patch_jump(jump);
            // False path: pop condition
            self.emitter.emit_pop();
        }

        // Exit loop context (patches break jumps)
        self.emitter.exit_loop();

        // Pop scope and cleanup handles
        let exiting_vars = self.ctx.pop_local_scope();
        for var in exiting_vars {
            if var.data_type.is_handle {
                let release = self.get_release_behavior(var.data_type.type_hash, for_stmt.span)?;
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit_release(release);
            }
        }

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
    fn for_loop_empty() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (;;) { break; }
        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let for_stmt = ForStmt {
            init: None,
            condition: None,
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode: Jump(3, break) + Loop(3) = 6 bytes
        assert_eq!(chunk.len(), 6);
        assert_eq!(chunk.read_op(0), Some(OpCode::Jump)); // break
        assert_eq!(chunk.read_op(3), Some(OpCode::Loop)); // unconditional loop back
    }

    #[test]
    fn for_loop_with_condition_only() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (; false;) {}
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let for_stmt = ForStmt {
            init: None,
            condition: Some(condition),
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode: PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        assert_eq!(chunk.len(), 9);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushFalse));
        assert_eq!(chunk.read_op(1), Some(OpCode::JumpIfFalse));
        assert_eq!(chunk.read_op(4), Some(OpCode::Pop)); // true path pop
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop));
        assert_eq!(chunk.read_op(8), Some(OpCode::Pop)); // false path pop
    }

    #[test]
    fn for_loop_with_var_init() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (int i = 0; false;) {}
        let init_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(0),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("i", Span::default()),
            init: Some(init_expr),
            span: Span::default(),
        }]);

        let var_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let for_stmt = ForStmt {
            init: Some(ForInit::VarDecl(var_decl)),
            condition: Some(condition),
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        // Variable i should be out of scope after the loop
        assert!(ctx.get_local("i").is_none());

        let chunk = emitter.finish();
        // Bytecode: PushZero(1) + SetLocal(2) + PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 12
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushZero)); // int i = 0
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // slot 0
        assert_eq!(chunk.read_op(3), Some(OpCode::PushFalse)); // condition
    }

    // Note: for_loop_with_expr_init test requires complex AST construction
    // that's better tested in integration tests

    #[test]
    fn for_loop_with_break() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (; true;) { break; }
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

        let for_stmt = ForStmt {
            init: None,
            condition: Some(condition),
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        let chunk = emitter.finish();
        // Bytecode: PushTrue(1) + JumpIfFalse(3) + Pop(1) + Jump(3, break) + Loop(3) + Pop(1) = 12
        assert_eq!(chunk.len(), 12);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(5), Some(OpCode::Jump)); // break
    }

    #[test]
    fn for_loop_with_continue() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (; false;) { continue; }
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

        let for_stmt = ForStmt {
            init: None,
            condition: Some(condition),
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        let chunk = emitter.finish();
        // Continue should be Loop instruction
        // The continue target is set to the update section (offset 5 - after Pop)
        assert_eq!(chunk.read_op(5), Some(OpCode::Loop)); // continue
    }

    #[test]
    fn for_loop_non_bool_condition_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (; 42;) {} - should fail
        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let for_stmt = ForStmt {
            init: None,
            condition: Some(condition),
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_for(&for_stmt);
        assert!(result.is_err());
    }

    #[test]
    fn for_loop_var_scoped_to_loop() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // for (int i = 0;;) { break; }
        let init_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(0),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("i", Span::default()),
            init: Some(init_expr),
            span: Span::default(),
        }]);

        let var_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let body = arena.alloc(Stmt::Block(Block {
            stmts,
            span: Span::default(),
        }));

        let for_stmt = ForStmt {
            init: Some(ForInit::VarDecl(var_decl)),
            condition: None,
            update: &[],
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&for_stmt).unwrap();

        // Variable i should be out of scope after the loop
        assert!(ctx.get_local("i").is_none());
    }

    #[test]
    fn nested_for_loops() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Inner for loop
        let inner_body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let inner_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let inner_for = arena.alloc(ForStmt {
            init: None,
            condition: Some(inner_condition),
            update: &[],
            body: inner_body,
            span: Span::default(),
        });

        // Outer for loop
        let outer_stmts = arena.alloc_slice_copy(&[Stmt::For(inner_for)]);
        let outer_body = arena.alloc(Stmt::Block(Block {
            stmts: outer_stmts,
            span: Span::default(),
        }));

        let outer_condition = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(false),
            span: Span::default(),
        }));

        let outer_for = ForStmt {
            init: None,
            condition: Some(outer_condition),
            update: &[],
            body: outer_body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_for(&outer_for).unwrap();

        let chunk = emitter.finish();
        // Both loops should compile correctly
        // Outer: PushFalse(1) + JumpIfFalse(3) + Pop(1) = 5 bytes before inner
        // Inner: PushFalse(1) + JumpIfFalse(3) + Pop(1) + Loop(3) + Pop(1) = 9 bytes
        // Outer: Loop(3) + Pop(1) = 4 bytes
        // Total: 18 bytes
        assert_eq!(chunk.len(), 18);
    }
}
