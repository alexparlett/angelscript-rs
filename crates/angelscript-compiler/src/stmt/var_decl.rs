//! Variable declaration compilation.
//!
//! Handles variable declarations including:
//! - Explicit type declarations: `int x = 5;`
//! - Multiple declarators: `int x = 1, y = 2;`
//! - Auto type inference: `auto x = someFunction();`
//! - Default initialization: `int x;` (zero-initialized)
//! - Const variables: `const int x = 42;`

use angelscript_core::{CompilationError, DataType, Span};
use angelscript_parser::ast::{TypeBase, VarDeclStmt};

use crate::bytecode::OpCode;
use crate::type_resolver::TypeResolver;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a variable declaration statement.
    ///
    /// Supports multiple declarators, auto type inference, and default initialization.
    pub fn compile_var_decl<'ast>(&mut self, decl: &VarDeclStmt<'ast>) -> Result<()> {
        // Check if this is an auto type declaration
        let is_auto = matches!(decl.ty.base, TypeBase::Auto);

        // For non-auto types, resolve the base type once
        let base_type = if is_auto {
            None
        } else {
            Some(TypeResolver::new(self.ctx).resolve(&decl.ty)?)
        };

        // Process each declarator
        for var in decl.vars {
            let var_type = if is_auto {
                // Auto type - must have initializer
                let Some(init) = &var.init else {
                    return Err(CompilationError::Other {
                        message: "auto variable must have an initializer".to_string(),
                        span: var.span,
                    });
                };

                // Infer type from initializer
                let mut expr_compiler = self.expr_compiler();
                let info = expr_compiler.infer(init)?;
                info.data_type
            } else {
                base_type.clone().unwrap()
            };

            // Check if the type is const from the declaration
            let is_const = decl.ty.is_const;

            // Declare the variable in the current scope
            let slot = self.ctx.declare_local(
                var.name.name.to_string(),
                var_type.clone(),
                is_const,
                var.span,
            )?;

            // Compile initializer or default initialization
            if let Some(init) = &var.init {
                // For auto type, the expression was already compiled during type inference
                // For explicit type, compile with type check
                if !is_auto {
                    let mut expr_compiler = self.expr_compiler();
                    expr_compiler.check(init, &var_type)?;
                }

                // Store the result in the local slot
                self.emitter.emit_set_local(slot);
            } else {
                // Default initialization
                self.emit_default_init(&var_type, slot, var.span)?;
            }

            // Mark the variable as initialized
            self.ctx.mark_local_initialized(var.name.name);
        }

        Ok(())
    }

    /// Emit default initialization for a variable.
    ///
    /// - Primitives are zero-initialized
    /// - Handles are null-initialized
    /// - Value types require a default constructor (not yet implemented)
    fn emit_default_init(&mut self, var_type: &DataType, slot: u32, span: Span) -> Result<()> {
        // Check if it's a primitive type
        if var_type.is_primitive() {
            // Zero-initialize primitives
            self.emitter.emit(OpCode::PushZero);
            self.emitter.emit_set_local(slot);
            Ok(())
        } else if var_type.is_handle {
            // Null-initialize handles
            self.emitter.emit_null();
            self.emitter.emit_set_local(slot);
            Ok(())
        } else {
            // Value types need default constructor - not yet implemented
            // TODO: Task 45 or later - call default constructor
            Err(CompilationError::Other {
                message: "default construction of value types not yet implemented; provide an initializer".to_string(),
                span,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::primitives;
    use angelscript_parser::ast::{
        Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, TypeExpr, VarDeclarator,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn var_decl_with_int_initializer() {
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

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        // Variable should be declared
        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::INT32);
        assert!(var.unwrap().is_initialized);
    }

    #[test]
    fn var_decl_default_init_primitive() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x;
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert!(var.unwrap().is_initialized);
    }

    #[test]
    fn var_decl_multiple_declarators() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // int x = 1, y = 2;
        let init1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));
        let init2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[
            VarDeclarator {
                name: Ident::new("x", Span::default()),
                init: Some(init1),
                span: Span::default(),
            },
            VarDeclarator {
                name: Ident::new("y", Span::default()),
                init: Some(init2),
                span: Span::default(),
            },
        ]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        // Both variables should be declared
        assert!(ctx.get_local("x").is_some());
        assert!(ctx.get_local("y").is_some());

        // Check slots are different
        assert_eq!(ctx.get_local("x").unwrap().slot, 0);
        assert_eq!(ctx.get_local("y").unwrap().slot, 1);
    }

    #[test]
    fn var_decl_auto_type_inference() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // auto x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        // Variable should be inferred as int (small ints are INT32)
        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn var_decl_auto_without_initializer_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // auto x; - should error
        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: None,
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::new(false, None, TypeBase::Auto, &[], &[], Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let result = compiler.compile_var_decl(&decl);
        assert!(result.is_err());
    }

    #[test]
    fn var_decl_const() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // const int x = 42;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let mut ty = TypeExpr::primitive(PrimitiveType::Int, Span::default());
        ty.is_const = true;

        let decl = VarDeclStmt {
            ty,
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert!(var.unwrap().is_const);
    }

    #[test]
    fn var_decl_redeclaration_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // First declaration: int x = 1;
        let init1 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));

        let vars1 = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init1),
            span: Span::default(),
        }]);

        let decl1 = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: vars1,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        assert!(compiler.compile_var_decl(&decl1).is_ok());

        // Second declaration at same scope: int x = 2; - should error
        let init2 = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));

        let vars2 = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init2),
            span: Span::default(),
        }]);

        let decl2 = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            vars: vars2,
            span: Span::default(),
        };

        let result = compiler.compile_var_decl(&decl2);
        assert!(matches!(
            result,
            Err(CompilationError::VariableRedeclaration { .. })
        ));
    }

    #[test]
    fn var_decl_float() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // float x = 3.14;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.14),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Float, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::FLOAT);
    }

    #[test]
    fn var_decl_bool() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // bool x = true;
        let init = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let vars = arena.alloc_slice_copy(&[VarDeclarator {
            name: Ident::new("x", Span::default()),
            init: Some(init),
            span: Span::default(),
        }]);

        let decl = VarDeclStmt {
            ty: TypeExpr::primitive(PrimitiveType::Bool, Span::default()),
            vars,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        assert!(compiler.compile_var_decl(&decl).is_ok());

        let var = ctx.get_local("x");
        assert!(var.is_some());
        assert_eq!(var.unwrap().data_type.type_hash, primitives::BOOL);
    }
}
