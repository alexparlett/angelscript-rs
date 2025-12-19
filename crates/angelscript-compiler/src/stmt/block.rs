//! Block statement compilation.
//!
//! Handles block statements `{ ... }` with proper scope management.

use angelscript_parser::ast::Block;

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
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
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit(OpCode::Release);
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
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let block = Block {
            stmts: &[],
            span: Span::default(),
        };

        assert!(compiler.compile_block(&block).is_ok());
    }

    #[test]
    fn block_creates_scope() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

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

        // Variable x should be visible inside block compilation
        assert!(compiler.compile_block(&block).is_ok());

        // After block, x should no longer be in scope
        assert!(ctx.get_local("x").is_none());
    }

    #[test]
    fn nested_blocks() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

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

        assert!(compiler.compile_block(&outer_block).is_ok());

        // Both variables should be out of scope after the outer block
        assert!(ctx.get_local("x").is_none());
        assert!(ctx.get_local("y").is_none());
    }

    #[test]
    fn variable_shadowing_in_nested_block() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

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

        // This should compile successfully - shadowing is allowed
        assert!(compiler.compile_block(&outer_block).is_ok());
    }
}
