//! Try-catch statement compilation.
//!
//! Handles exception handling statements with proper control flow.
//!
//! ## Bytecode Layout
//!
//! ```text
//! TryBegin -> catch_offset   ; Push exception handler with catch address
//! [try body]                 ; May throw, jumping to catch
//! [try cleanup]              ; Release handles from try scope (normal path)
//! TryEnd                     ; Pop exception handler (success)
//! Jump -> after_catch        ; Skip catch block
//! catch:
//! [catch body]               ; Executed on exception
//! [catch cleanup]            ; Release handles from catch scope
//! after_catch:
//! ```
//!
//! ## Exception Unwinding
//!
//! When an exception occurs (via `throw()` or runtime error), the VM is
//! responsible for:
//! 1. Finding the nearest TryBegin handler on the handler stack
//! 2. Cleaning up local variables between throw site and catch block
//! 3. Jumping to the catch block address stored in TryBegin
//!
//! The cleanup emitted here only runs on the normal (non-exception) path.
//! The VM must track stack frame state at TryBegin to properly release
//! handles during exception unwinding.

use angelscript_parser::ast::TryCatchStmt;

use crate::bytecode::OpCode;

use super::{Result, StmtCompiler};

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
    /// Compile a try-catch statement.
    ///
    /// The try block is compiled within an exception handler context.
    /// If an exception is thrown during the try block, execution jumps
    /// to the catch block. If the try block completes normally, the
    /// catch block is skipped.
    ///
    /// AngelScript's simple try-catch doesn't have typed exceptions or
    /// exception variables - it's a basic catch-all mechanism.
    pub fn compile_try_catch<'ast>(&mut self, try_catch: &TryCatchStmt<'ast>) -> Result<()> {
        // Emit TryBegin - will be patched with catch offset
        let try_begin = self.emitter.emit_jump(OpCode::TryBegin);

        // Push scope for try block
        self.ctx.push_local_scope();

        // Compile try block statements
        for stmt in try_catch.try_block.stmts {
            self.compile(stmt)?;
        }

        // Pop try block scope and emit cleanup
        let try_exiting_vars = self.ctx.pop_local_scope();
        for var in &try_exiting_vars {
            if var.data_type.is_handle {
                let release_hash =
                    self.get_release_behavior(var.data_type.type_hash, try_catch.try_block.span)?;
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit_release(release_hash);
            }
        }

        // End try block - pops exception handler
        self.emitter.emit(OpCode::TryEnd);

        // Jump past catch block on successful try completion
        let success_jump = self.emitter.emit_jump(OpCode::Jump);

        // Patch TryBegin to point to catch block
        self.emitter.patch_jump(try_begin);

        // Push scope for catch block
        self.ctx.push_local_scope();

        // Compile catch block statements
        for stmt in try_catch.catch_block.stmts {
            self.compile(stmt)?;
        }

        // Pop catch block scope and emit cleanup
        let catch_exiting_vars = self.ctx.pop_local_scope();
        for var in &catch_exiting_vars {
            if var.data_type.is_handle {
                let release_hash =
                    self.get_release_behavior(var.data_type.type_hash, try_catch.catch_block.span)?;
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit_release(release_hash);
            }
        }

        // Patch success jump to after catch
        self.emitter.patch_jump(success_jump);

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
        Block, Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, Stmt, TypeExpr, VarDeclStmt,
        VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn empty_try_catch() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let try_catch = TryCatchStmt {
            try_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            catch_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_try_catch(&try_catch).unwrap();

        let chunk = emitter.finish_chunk();

        // Bytecode layout:
        // TryBegin + offset(2) = 3 bytes (offset 0-2)
        // TryEnd = 1 byte (offset 3)
        // Jump + offset(2) = 3 bytes (offset 4-6)
        // (empty catch block)
        // Total: 7 bytes
        assert_eq!(chunk.len(), 7);
        assert_eq!(chunk.read_op(0), Some(OpCode::TryBegin));
        // TryBegin offset: points to catch block at offset 7 (after success jump)
        // Actually the catch block starts at offset 7, jump offset = 7 - 1 - 2 = 4
        assert_eq!(chunk.read_u16(1), Some(4));
        assert_eq!(chunk.read_op(3), Some(OpCode::TryEnd));
        assert_eq!(chunk.read_op(4), Some(OpCode::Jump));
        // Jump offset: end is at 7, so 7 - 5 - 2 = 0
        assert_eq!(chunk.read_u16(5), Some(0));
    }

    #[test]
    fn try_catch_with_statements() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Create try block: { int x = 1; }
        let try_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let try_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(try_init),
            span: Span::default(),
        }]);

        let try_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: try_vars,
            span: Span::default(),
        };

        let try_stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(try_decl)]);

        // Create catch block: { int y = 2; }
        let catch_init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let catch_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("y", Span::default()),
            init: Some(catch_init),
            span: Span::default(),
        }]);

        let catch_decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: catch_vars,
            span: Span::default(),
        };

        let catch_stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(catch_decl)]);

        let try_catch = TryCatchStmt {
            try_block: Block {
                stmts: try_stmts,
                span: Span::default(),
            },
            catch_block: Block {
                stmts: catch_stmts,
                span: Span::default(),
            },
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_try_catch(&try_catch).unwrap();

        // Variables should be out of scope after try-catch
        assert!(ctx.get_local("x").is_none());
        assert!(ctx.get_local("y").is_none());

        let chunk = emitter.finish_chunk();

        // Bytecode layout:
        // TryBegin + offset(2) = 3 bytes
        // int x = 1: PushOne(1) + SetLocal(2) = 3 bytes
        // TryEnd = 1 byte
        // Jump + offset(2) = 3 bytes
        // int y = 2: Constant(2) + SetLocal(2) = 4 bytes
        // Total: 14 bytes
        assert_eq!(chunk.len(), 14);
        assert_eq!(chunk.read_op(0), Some(OpCode::TryBegin));
        assert_eq!(chunk.read_op(3), Some(OpCode::PushOne));
        assert_eq!(chunk.read_op(4), Some(OpCode::SetLocal));
        assert_eq!(chunk.read_op(6), Some(OpCode::TryEnd));
        assert_eq!(chunk.read_op(7), Some(OpCode::Jump));
        assert_eq!(chunk.read_op(10), Some(OpCode::Constant));
        assert_eq!(chunk.read_op(12), Some(OpCode::SetLocal));
    }

    #[test]
    fn nested_try_catch() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Inner try-catch
        let inner_try_catch = arena.alloc(TryCatchStmt {
            try_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            catch_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            span: Span::default(),
        });

        let inner_stmt = arena.alloc(Stmt::TryCatch(inner_try_catch));
        let inner_stmts = arena.alloc_slice_copy(&[*inner_stmt]);

        // Outer try-catch with inner try-catch in try block
        let outer_try_catch = TryCatchStmt {
            try_block: Block {
                stmts: inner_stmts,
                span: Span::default(),
            },
            catch_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_try_catch(&outer_try_catch).unwrap();

        let chunk = emitter.finish_chunk();

        // Outer: TryBegin(3) + [inner](7) + TryEnd(1) + Jump(3) + (catch)(0) = 14 bytes
        assert_eq!(chunk.len(), 14);
        assert_eq!(chunk.read_op(0), Some(OpCode::TryBegin)); // Outer TryBegin
        assert_eq!(chunk.read_op(3), Some(OpCode::TryBegin)); // Inner TryBegin
        assert_eq!(chunk.read_op(6), Some(OpCode::TryEnd)); // Inner TryEnd
        assert_eq!(chunk.read_op(7), Some(OpCode::Jump)); // Inner success jump
        assert_eq!(chunk.read_op(10), Some(OpCode::TryEnd)); // Outer TryEnd
        assert_eq!(chunk.read_op(11), Some(OpCode::Jump)); // Outer success jump
    }

    #[test]
    fn try_catch_with_handle_emits_release() {
        use angelscript_core::entries::ClassEntry;
        use angelscript_core::{TypeHash, TypeKind};
        use angelscript_parser::ast::{TypeBase, TypeSuffix};

        let arena = Bump::new();
        let mut registry = SymbolRegistry::with_primitives();

        // Register a reference type with release behavior
        let release_hash = TypeHash::from_name("Foo::Release");
        let mut foo_class = ClassEntry::ffi("Foo", TypeKind::reference());
        foo_class.behaviors.release = Some(release_hash);
        registry.register_type(foo_class.into()).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Try block: Foo@ f; (default init to null - no AddRef needed)
        let try_vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("f", Span::default()),
            init: None, // Default init to null
            span: Span::default(),
        }]);

        // Create handle type: Foo@
        let suffixes = arena.alloc_slice_copy(&[TypeSuffix::Handle { is_const: false }]);
        let try_ty = TypeExpr::new(
            false,
            None,
            TypeBase::Named(Ident::new("Foo", Span::default())),
            &[],
            suffixes,
            Span::default(),
        );

        let try_decl = VarDeclStmt {
            ty: try_ty,
            vars: try_vars,
            span: Span::default(),
        };

        let try_stmts = arena.alloc_slice_copy(&[Stmt::VarDecl(try_decl)]);

        let try_catch = TryCatchStmt {
            try_block: Block {
                stmts: try_stmts,
                span: Span::default(),
            },
            catch_block: Block {
                stmts: &[],
                span: Span::default(),
            },
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_try_catch(&try_catch).unwrap();

        // Variable should be out of scope
        assert!(ctx.get_local("f").is_none());

        // Verify Release was emitted for the handle when exiting try scope
        let chunk = emitter.finish_chunk();
        let mut found_release = false;
        for i in 0..chunk.len() {
            if chunk.read_op(i) == Some(OpCode::Release) {
                found_release = true;
                break;
            }
        }
        assert!(
            found_release,
            "Expected Release opcode for handle variable in try block"
        );
    }
}
