//! Return statement compilation.
//!
//! Handles return statements with proper type checking against the function's
//! declared return type.

use angelscript_core::CompilationError;
use angelscript_parser::ast::ReturnStmt;

use super::{Result, StmtCompiler};

impl<'a, 'ctx> StmtCompiler<'a, 'ctx> {
    /// Compile a return statement.
    ///
    /// Validates that:
    /// - Non-void functions have a return value
    /// - Void functions do not have a return value
    /// - The return value type matches the declared return type
    /// - Reference returns don't reference local variables
    pub fn compile_return<'ast>(&mut self, ret: &ReturnStmt<'ast>) -> Result<()> {
        let is_void = self.return_type.is_void();

        match (&ret.value, is_void) {
            // Return with value in non-void function
            (Some(expr), false) => {
                // Clone return type to avoid borrow conflict
                let return_type = self.return_type;
                let returns_reference = return_type.is_reference();

                // Compile expression with type checking (check calls infer internally)
                let mut expr_compiler = self.expr_compiler();
                let expr_info = expr_compiler.check(expr, &return_type)?;

                // For reference returns, validate the source
                if returns_reference && !expr_info.is_safe_for_ref_return() {
                    return Err(CompilationError::Other {
                        message: "cannot return reference to local variable or parameter"
                            .to_string(),
                        span: ret.span,
                    });
                }

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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);

        let ret = ReturnStmt {
            value: None,
            span: Span::default(),
        };

        compiler.compile_return(&ret).unwrap();

        let chunk = emitter.finish_chunk();
        // Should emit: ReturnVoid (1 byte)
        assert_eq!(chunk.len(), 1);
        assert_eq!(chunk.read_op(0), Some(OpCode::ReturnVoid));
    }

    #[test]
    fn return_value_in_non_void_function() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_return(&ret).unwrap();

        let chunk = emitter.finish_chunk();
        // Literal 42 produces INT32, return type is INT64, so conversion needed
        // Constant(1) + index(1) + I32toI64(1) + Return(1) = 4 bytes
        assert_eq!(chunk.len(), 4);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(0)); // Index 0 in constant pool
        assert_eq!(chunk.read_op(2), Some(OpCode::I32toI64));
        assert_eq!(chunk.read_op(3), Some(OpCode::Return));
    }

    #[test]
    fn return_value_in_void_function_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

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

        compiler.compile_return(&ret).unwrap();

        let chunk = emitter.finish_chunk();
        // Should emit: PushTrue, Return (2 bytes total)
        assert_eq!(chunk.len(), 2);
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
        assert_eq!(chunk.read_op(1), Some(OpCode::Return));
    }

    #[test]
    fn return_float_in_float_function() {
        let arena = Bump::new();
        let (registry, _constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let return_type = DataType::simple(primitives::FLOAT);
        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Float(3.5),
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        compiler.compile_return(&ret).unwrap();

        // Verify the constant pool has 3.5 as Float32
        assert_eq!(emitter.constants().len(), 1);

        let chunk = emitter.finish_chunk();
        // Should emit: Constant(index=0), Return
        // Constant opcode (1) + index (1) + Return (1) = 3 bytes
        assert_eq!(chunk.len(), 3);
        assert_eq!(chunk.read_op(0), Some(OpCode::Constant));
        assert_eq!(chunk.read_byte(1), Some(0)); // Index 0 in constant pool
        assert_eq!(chunk.read_op(2), Some(OpCode::Return));
    }

    #[test]
    fn return_reference_to_local_error() {
        use angelscript_core::RefModifier;
        use angelscript_parser::ast::{Ident, IdentExpr};

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );

        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Return type is int& (reference to int)
        let mut return_type = DataType::simple(primitives::INT32);
        return_type.ref_modifier = RefModifier::InOut;

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let arena = Bump::new();

        // Try to return local variable by reference
        let expr = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        let result = compiler.compile_return(&ret);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, CompilationError::Other { message, .. } if message.contains("cannot return reference to local"))
        );
    }

    #[test]
    fn return_reference_to_global_ok() {
        use angelscript_core::{
            GlobalPropertyEntry, GlobalPropertyImpl, RefModifier, TypeHash, TypeSource,
        };
        use angelscript_parser::ast::{Ident, IdentExpr};

        let mut registry = SymbolRegistry::with_primitives();

        // Register a global variable (non-const)
        let data_type = DataType::simple(primitives::INT32);
        let entry = GlobalPropertyEntry {
            name: "globalVar".to_string(),
            namespace: Vec::new(),
            qualified_name: "globalVar".to_string(),
            type_hash: TypeHash::from_name("globalVar"),
            data_type,
            is_const: false,
            source: TypeSource::ffi_untyped(),
            implementation: GlobalPropertyImpl::Script { slot: 0, data_type },
        };
        registry.register_global(entry).unwrap();

        let mut constants = ConstantPool::new();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Return type is int& (reference to int)
        let mut return_type = DataType::simple(primitives::INT32);
        return_type.ref_modifier = RefModifier::InOut;

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let arena = Bump::new();

        // Return global variable by reference - should be OK
        let expr = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("globalVar", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        // This should succeed
        let result = compiler.compile_return(&ret);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result);
    }

    #[test]
    fn return_value_from_local_ok() {
        use angelscript_parser::ast::{Ident, IdentExpr};

        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a local variable
        let _ = ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false,
            Span::default(),
        );

        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        // Return type is int (by value, not reference)
        let return_type = DataType::simple(primitives::INT32);

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, return_type, None);

        let arena = Bump::new();

        // Return local variable by value - should be OK
        let expr = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));

        let ret = ReturnStmt {
            value: Some(expr),
            span: Span::default(),
        };

        // This should succeed (returning by value is fine)
        let result = compiler.compile_return(&ret);
        assert!(result.is_ok(), "Expected OK, got: {:?}", result);
    }
}
