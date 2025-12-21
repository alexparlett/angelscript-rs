//! Block statement compilation.
//!
//! Handles block statements `{ ... }` with proper scope management.

use angelscript_parser::ast::Block;

use super::{Result, StmtCompiler};

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
    /// Compile a block statement.
    ///
    /// Creates a new scope for the block, compiles all statements within it,
    /// then pops the scope when done. Variables declared in the block are
    /// only visible within it. Handle variables get Release calls emitted.
    pub fn compile_block<'ast>(&mut self, block: &Block<'ast>) -> Result<()> {
        // Push a new scope for this block
        self.ctx.push_local_scope();

        // Compile each statement in the block
        for stmt in block.stmts {
            self.compile(stmt)?;
        }

        // Pop the scope and get variables that went out of scope
        let exiting_vars = self.ctx.pop_local_scope();

        // Emit Release for handle variables
        for var in exiting_vars {
            if var.data_type.is_handle {
                // Look up the release behavior for this type
                // Use block span as fallback since LocalVariable doesn't have span
                let release_hash =
                    self.get_release_behavior(var.data_type.type_hash, block.span)?;
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit_release(release_hash);
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
    use angelscript_core::{DataType, Span};
    use angelscript_parser::ast::{
        Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, Stmt, TypeExpr, VarDeclStmt,
        VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn empty_block() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let block = Block {
            stmts: &[],
            span: Span::default(),
        };

        compiler.compile_block(&block).unwrap();

        // Empty block emits no bytecode
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 0);
    }

    #[test]
    fn block_creates_scope() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Create a variable declaration: int x = 42;
        let init_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init_expr),
            span: Span::default(),
        }]);

        let var_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(var_decl)]);

        let block = Block {
            stmts,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_block(&block).unwrap();

        // After block, x should no longer be in scope
        assert!(ctx.get_local("x").is_none());

        // Bytecode: Constant(0) [2 bytes] + SetLocal(0) [2 bytes] = 4 bytes
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(0)); // Constant pool index
        assert_eq!(chunk.read_op(2), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(3), Some(0)); // Slot 0
    }

    #[test]
    fn nested_blocks() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Outer block variable: int x = 1;
        let outer_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let outer_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(outer_init),
            span: Span::default(),
        }]);

        let outer_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: outer_vars,
            span: Span::default(),
        };

        // Inner block variable: int y = 2;
        let inner_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let inner_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("y", Span::default()),
            init: Some(inner_init),
            span: Span::default(),
        }]);

        let inner_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: inner_vars,
            span: Span::default(),
        };

        let inner_stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(inner_decl)]);
        let inner_block = arena.alloc(Block {
            stmts: inner_stmts,
            span: Span::default(),
        });

        // Outer block: { int x = 1; { int y = 2; } }
        let outer_stmts =
            arena.alloc_slice_copy(&[Stmt::VarDecl(outer_decl), Stmt::Block(*inner_block)]);

        let outer_block = Block {
            stmts: outer_stmts,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_block(&outer_block).unwrap();

        // Both variables should be out of scope after the outer block
        assert!(ctx.get_local("x").is_none());
        assert!(ctx.get_local("y").is_none());

        // Bytecode layout:
        // int x = 1: PushOne(1 byte) + SetLocal(1 byte) + slot(1 byte) = 3 bytes
        // int y = 2: Constant(1 byte) + index(1 byte) + SetLocal(1 byte) + slot(1 byte) = 4 bytes
        // Total: 7 bytes
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 7);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushOne));
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // Slot 0 for x
        assert_eq!(chunk.read_op(3), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(4), Some(0)); // Constant pool index for 2
        assert_eq!(chunk.read_op(5), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(6), Some(1)); // Slot 1 for y
    }

    #[test]
    fn variable_shadowing_in_nested_block() {
        use crate::bytecode::OpCode;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Outer x: int x = 1;
        let outer_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let outer_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(outer_init),
            span: Span::default(),
        }]);

        let outer_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: outer_vars,
            span: Span::default(),
        };

        // Inner x (shadows): float x = 2.0;
        let inner_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(2.0),
            span: Span::default(),
        }));

        let inner_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(inner_init),
            span: Span::default(),
        }]);

        let inner_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Float, Span::default()),
            vars: inner_vars,
            span: Span::default(),
        };

        let inner_stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(inner_decl)]);
        let inner_block = arena.alloc(Block {
            stmts: inner_stmts,
            span: Span::default(),
        });

        let outer_stmts =
            arena.alloc_slice_copy(&[Stmt::VarDecl(outer_decl), Stmt::Block(*inner_block)]);

        let outer_block = Block {
            stmts: outer_stmts,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        compiler.compile_block(&outer_block).unwrap();

        // Bytecode layout:
        // int x = 1: PushOne(1) + SetLocal(1) + slot(1) = 3 bytes
        // float x = 2.0: Constant(1) + index(1) + SetLocal(1) + slot(1) = 4 bytes
        // Total: 7 bytes
        let chunk = emitter.finish_chunk();
        assert_eq!(chunk.len(), 7);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushOne));
        assert_eq!(chunk.read_op(1), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(2), Some(0)); // Slot 0 for outer x
        assert_eq!(chunk.read_op(3), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(4), Some(0)); // Constant pool index for 2.0
        assert_eq!(chunk.read_op(5), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_byte(6), Some(1)); // Slot 1 for inner x (shadow)
    }
}
