//! Return statement compilation.
//!
//! Handles return statements with proper type checking against the function's
//! declared return type.

use angelscript_core::CompilationError;
use angelscript_parser::ast::ReturnStmt;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a return statement.
    ///
    /// Validates that:
    /// - Non-void functions have a return value
    /// - Void functions do not have a return value
    /// - The return value type matches the declared return type
    pub fn compile_return<'ast>(&mut self, ret: &ReturnStmt<'ast>) -> Result<()> {
        let is_void = self.return_type.is_void();

        match (&ret.value, is_void) {
            // Return with value in non-void function
            (Some(expr), false) => {
                // Clone return type to avoid borrow conflict
                let return_type = self.return_type;

                // Compile expression and check against return type
                let mut expr_compiler = self.expr_compiler();
                expr_compiler.check(expr, &return_type)?;

                self.emitter.emit_return();
                Ok(())
            }

            // Return without value in void function
            (None, true) => {
                self.emitter.emit_return_void();
                Ok(())
            }

            // Error: return value in void function
            (Some(_), true) => Err(CompilationError::TypeMismatch {
                message: "void function cannot return a value".to_string(),
                span: ret.span,
            }),

            // Error: missing return value in non-void function
            (None, false) => {
                let type_name = self
                    .ctx
                    .get_type(self.return_type.type_hash)
                    .map(|e| e.qualified_name().to_string())
                    .unwrap_or_else(|| format!("{:?}", self.return_type.type_hash));

                Err(CompilationError::TypeMismatch {
                    message: format!(
                        "non-void function must return a value of type '{}'",
                        type_name
                    ),
                    span: ret.span,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{ConstantPool, OpCode};
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{DataType, Span, primitives};
    use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn return_void_in_void_function() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let ret = ReturnStmt {
            value: None,
            span: Span::default(),
        };

        assert!(compiler.compile_return(&ret).is_ok());

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::ReturnVoid));
    }

    #[test]
    fn return_value_in_non_void_function() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let return_type = DataType::simple(primitives::INT64);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        assert!(compiler.compile_return(&ret).is_ok());

        let chunk = emitter.finish();
        // Should have constant load and return
        assert!(chunk.len() > 0);
    }

    #[test]
    fn return_value_in_void_function_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        let result = compiler.compile_return(&ret);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompilationError::TypeMismatch { .. })));
    }

    #[test]
    fn return_void_in_non_void_function_error() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let return_type = DataType::simple(primitives::INT32);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let ret = ReturnStmt {
            value: None,
            span: Span::default(),
        };

        let result = compiler.compile_return(&ret);
        assert!(result.is_err());
        assert!(matches!(result, Err(CompilationError::TypeMismatch { .. })));
    }

    #[test]
    fn return_wrong_type_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // Function returns bool, but we return int
        let return_type = DataType::simple(primitives::BOOL);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        let result = compiler.compile_return(&ret);
        assert!(result.is_err());
    }

    #[test]
    fn return_bool_in_bool_function() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let return_type = DataType::simple(primitives::BOOL);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        assert!(compiler.compile_return(&ret).is_ok());
    }

    #[test]
    fn return_float_in_float_function() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let return_type = DataType::simple(primitives::DOUBLE);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.14),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        assert!(compiler.compile_return(&ret).is_ok());
    }
}
