//! Lambda expression compilation.
//!
//! Handles `function(params) { body }` syntax for anonymous functions.
//!
//! ## Restrictions
//!
//! Per AngelScript semantics, lambdas:
//! - **Cannot access outer scope variables** (no closures)
//! - Must have types inferable from expected funcdef or explicit parameter types
//! - When parameter types are untyped AND multiple overloads could match, require explicit types
//!
//! ## Type Inference
//!
//! Lambda parameter types can be inferred when:
//! 1. Explicitly typed: `function(int a, int b) { ... }`
//! 2. Single expected funcdef: `Callback @cb = function(a, b) { ... }` where `Callback` is unambiguous
//!
//! Compilation fails when parameters are untyped and:
//! - No expected type is available
//! - Multiple funcdef overloads could match

use angelscript_core::{CompilationError, DataType, TypeHash};
use angelscript_parser::ast::LambdaExpr;

use super::{ExprCompiler, Result};
use crate::bytecode::OpCode;
use crate::context::CompilationContext;
use crate::emit::BytecodeEmitter;
use crate::expr_info::ExprInfo;
use crate::return_checker::ReturnChecker;
use crate::stmt::StmtCompiler;
use crate::type_resolver::TypeResolver;

/// Compile a lambda expression: `function(params) { body }`
///
/// Lambdas are compiled as anonymous functions with isolated scope.
/// They cannot capture variables from the enclosing scope.
///
/// # Parameters
///
/// * `expected` - Expected funcdef type (for type inference)
///
/// # Returns
///
/// * `Ok(ExprInfo)` - Function pointer to the compiled lambda
/// * `Err(CompilationError)` - If types cannot be inferred or body has errors
pub fn compile_lambda<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    expr: &LambdaExpr<'ast>,
    expected: Option<&DataType>,
) -> Result<ExprInfo> {
    let span = expr.span;

    // 1. Check for untyped parameters without expected type
    let has_untyped_params = expr.params.iter().any(|p| p.ty.is_none());

    // Get the expected funcdef for type inference (if available)
    let funcdef = expected.and_then(|e| {
        compiler
            .ctx()
            .get_type(e.type_hash)
            .and_then(|t| t.as_funcdef().cloned())
    });

    if has_untyped_params && funcdef.is_none() {
        return Err(CompilationError::TypeMismatch {
            message: "lambda with untyped parameters requires expected funcdef type for inference"
                .to_string(),
            span,
        });
    }

    // 2. Validate parameter count matches expected funcdef (if present)
    if let Some(ref fd) = funcdef
        && expr.params.len() != fd.params.len()
    {
        return Err(CompilationError::TypeMismatch {
            message: format!(
                "lambda has {} parameters but expected funcdef '{}' has {}",
                expr.params.len(),
                fd.name,
                fd.params.len()
            ),
            span,
        });
    }

    // 3. Resolve parameter types (from explicit types or expected funcdef)
    let param_types = resolve_param_types(compiler, expr, funcdef.as_ref())?;

    // 4. Resolve return type
    let return_type = if let Some(ret_ty) = &expr.return_type {
        let mut resolver = TypeResolver::new(compiler.ctx_mut());
        resolver.resolve(&ret_ty.ty)?
    } else if let Some(ref fd) = funcdef {
        fd.return_type
    } else {
        // Infer void for lambdas without return type and no expected type
        DataType::void()
    };

    // 5. Generate lambda hash and name
    let lambda_hash =
        TypeHash::from_name(&generate_lambda_funcdef_name(&param_types, &return_type));
    let lambda_name = format!("$lambda_{:x}", lambda_hash.as_u64());

    // 6. Determine the result funcdef type
    let result_type = if let Some(expected) = expected {
        // Use the expected type if it's a funcdef
        *expected
    } else {
        // Use the generated lambda hash for anonymous funcdefs
        DataType::simple(lambda_hash)
    };

    // 7. Compile the lambda body using inner references
    // We need to access ctx and emitter separately to avoid double-borrow
    compile_lambda_body(
        compiler.ctx,
        compiler.emitter,
        expr,
        &param_types,
        return_type,
        lambda_hash,
        lambda_name,
        span,
    )?;

    Ok(ExprInfo::rvalue(result_type))
}

/// Compile the lambda body with proper scope and bytecode generation.
///
/// This is a separate function to work around borrow checker limitations
/// when accessing both ctx and emitter from ExprCompiler.
#[allow(clippy::too_many_arguments)]
fn compile_lambda_body<'ast>(
    ctx: &mut CompilationContext<'_>,
    emitter: &mut BytecodeEmitter,
    expr: &LambdaExpr<'ast>,
    param_types: &[DataType],
    return_type: DataType,
    lambda_hash: TypeHash,
    lambda_name: String,
    span: angelscript_core::Span,
) -> Result<()> {
    // Start new chunk for lambda body (parent chunk stays on stack)
    emitter.start_chunk();

    // Start lambda's isolated scope (pushed onto scope stack)
    ctx.begin_function();

    // Declare parameters in lambda scope
    for (i, (param, ty)) in expr.params.iter().zip(param_types).enumerate() {
        let name = param
            .name
            .map(|n| n.name.to_string())
            .unwrap_or_else(|| format!("_{}", i));
        ctx.declare_param(name, *ty, ty.is_const, param.span)?;
    }

    // Compile lambda body
    {
        let mut stmt_compiler = StmtCompiler::new(
            ctx,
            emitter,
            return_type,
            None, // No 'this' in lambdas
        );
        for stmt in expr.body.stmts {
            stmt_compiler.compile(stmt)?;
        }
    }

    // Check if we need implicit return
    let has_explicit_return = {
        let checker = ReturnChecker::new();
        checker.all_paths_return(emitter.chunk())
    };

    // Add implicit return for void lambdas without explicit return
    if return_type.is_void() && !has_explicit_return {
        emitter.emit(OpCode::ReturnVoid);
    }

    // Verify non-void lambdas have returns on all paths
    if !return_type.is_void() && !has_explicit_return {
        // Clean up before returning error
        emitter.finish_chunk(); // Discard lambda chunk
        ctx.end_function(); // Pop lambda scope
        return Err(CompilationError::Other {
            message: "not all code paths return a value in lambda".to_string(),
            span,
        });
    }

    // End lambda scope (pop from scope stack)
    ctx.end_function();

    // Register the compiled lambda
    emitter.finish_function(lambda_hash, lambda_name);

    // Emit FuncPtr in parent's chunk (now current again)
    emitter.emit_func_ptr(lambda_hash);

    Ok(())
}

/// Resolve parameter types from explicit types or expected funcdef.
fn resolve_param_types(
    compiler: &mut ExprCompiler<'_, '_>,
    expr: &LambdaExpr<'_>,
    funcdef: Option<&angelscript_core::entries::FuncdefEntry>,
) -> Result<Vec<DataType>> {
    let mut types = Vec::with_capacity(expr.params.len());

    for (i, param) in expr.params.iter().enumerate() {
        let param_type = if let Some(ref ty) = param.ty {
            // Explicit type
            let mut resolver = TypeResolver::new(compiler.ctx_mut());
            resolver.resolve(&ty.ty)?
        } else if let Some(fd) = funcdef {
            // Infer from expected funcdef
            fd.params[i]
        } else {
            return Err(CompilationError::TypeMismatch {
                message: format!(
                    "cannot infer type for lambda parameter {}",
                    param
                        .name
                        .as_ref()
                        .map(|n| n.name)
                        .unwrap_or(&format!("{}", i))
                ),
                span: param.span,
            });
        };
        types.push(param_type);
    }

    Ok(types)
}

/// Generate a unique name for an anonymous funcdef based on signature.
fn generate_lambda_funcdef_name(params: &[DataType], return_type: &DataType) -> String {
    let param_str = params
        .iter()
        .map(|p| format!("{:x}", p.type_hash.as_u64()))
        .collect::<Vec<_>>()
        .join("_");
    format!("$lambda_{}_{}", param_str, return_type.type_hash.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::Span;
    use angelscript_parser::ast::{Block, LambdaParam};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
    ) -> ExprCompiler<'a, 'ctx> {
        ExprCompiler::new(ctx, emitter, None)
    }

    #[test]
    fn lambda_untyped_params_without_expected_fails() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with untyped parameter: function(a) { }
        let params = arena.alloc_slice_copy(&[LambdaParam {
            ty: None, // Untyped!
            name: Some(angelscript_parser::ast::Ident::new(
                "a",
                Span::new(1, 10, 1),
            )),
            span: Span::new(1, 10, 1),
        }]);

        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 13, 2),
        });

        let lambda_expr = LambdaExpr {
            params,
            return_type: None,
            body,
            span: Span::new(1, 1, 15),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, None);

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(message.contains("untyped parameters"));
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn lambda_empty_params_empty_body_succeeds() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with no parameters and empty body: function() { }
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 12, 2),
        });

        let lambda_expr = LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::new(1, 1, 14),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, None);

        // Empty body lambda with all typed params (no params = all typed) succeeds
        assert!(
            result.is_ok(),
            "Empty body lambda should succeed: {:?}",
            result
        );
    }

    #[test]
    fn lambda_with_expected_funcdef_succeeds() {
        use angelscript_core::entries::{FuncdefEntry, TypeEntry};

        let mut registry = SymbolRegistry::with_primitives();

        // Register a funcdef type: funcdef void Callback()
        let funcdef_hash = TypeHash::from_name("Callback");
        let funcdef = FuncdefEntry::ffi("Callback", vec![], DataType::void());
        registry.register_type(TypeEntry::Funcdef(funcdef)).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with no parameters and empty body: function() { }
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 12, 2),
        });

        let lambda_expr = LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::new(1, 1, 14),
        };

        let expected_type = DataType::simple(funcdef_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "Lambda with expected funcdef should succeed: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, funcdef_hash);
    }

    #[test]
    fn lambda_untyped_params_with_expected_void_funcdef_succeeds() {
        use angelscript_core::entries::{FuncdefEntry, TypeEntry};
        use angelscript_core::primitives;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a funcdef type: funcdef void BinaryOp(int, int)
        // Using void return so empty body is valid
        let funcdef_hash = TypeHash::from_name("BinaryOp");
        let funcdef = FuncdefEntry::ffi(
            "BinaryOp",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::void(),
        );
        registry.register_type(TypeEntry::Funcdef(funcdef)).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with untyped parameters: function(a, b) { }
        let params = arena.alloc_slice_copy(&[
            LambdaParam {
                ty: None, // Untyped - will be inferred from funcdef
                name: Some(angelscript_parser::ast::Ident::new(
                    "a",
                    Span::new(1, 10, 1),
                )),
                span: Span::new(1, 10, 1),
            },
            LambdaParam {
                ty: None, // Untyped - will be inferred from funcdef
                name: Some(angelscript_parser::ast::Ident::new(
                    "b",
                    Span::new(1, 13, 1),
                )),
                span: Span::new(1, 13, 1),
            },
        ]);

        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 16, 2),
        });

        let lambda_expr = LambdaExpr {
            params,
            return_type: None,
            body,
            span: Span::new(1, 1, 18),
        };

        let expected_type = DataType::simple(funcdef_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, Some(&expected_type));

        assert!(
            result.is_ok(),
            "Lambda with inferred params should succeed: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, funcdef_hash);
    }

    #[test]
    fn lambda_param_count_mismatch_fails() {
        use angelscript_core::entries::{FuncdefEntry, TypeEntry};
        use angelscript_core::primitives;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a funcdef type: funcdef void UnaryOp(int)
        let funcdef_hash = TypeHash::from_name("UnaryOp");
        let funcdef = FuncdefEntry::ffi(
            "UnaryOp",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        registry.register_type(TypeEntry::Funcdef(funcdef)).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with 2 parameters but funcdef expects 1
        let params = arena.alloc_slice_copy(&[
            LambdaParam {
                ty: None,
                name: Some(angelscript_parser::ast::Ident::new(
                    "a",
                    Span::new(1, 10, 1),
                )),
                span: Span::new(1, 10, 1),
            },
            LambdaParam {
                ty: None,
                name: Some(angelscript_parser::ast::Ident::new(
                    "b",
                    Span::new(1, 13, 1),
                )),
                span: Span::new(1, 13, 1),
            },
        ]);

        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 16, 2),
        });

        let lambda_expr = LambdaExpr {
            params,
            return_type: None,
            body,
            span: Span::new(1, 1, 18),
        };

        let expected_type = DataType::simple(funcdef_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, Some(&expected_type));

        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::TypeMismatch { message, .. } => {
                assert!(message.contains("parameters"));
            }
            _ => panic!("Expected TypeMismatch error, got {:?}", err),
        }
    }

    #[test]
    fn lambda_with_body_compiles_successfully() {
        use angelscript_parser::ast::{Expr, ExprStmt, LiteralExpr, LiteralKind, Stmt};

        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Create a non-empty body with a statement: function() { 42; }
        let lit_expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::new(1, 15, 2),
        }));
        let stmt = arena.alloc(Stmt::Expr(ExprStmt {
            expr: Some(lit_expr),
            span: Span::new(1, 15, 3),
        }));

        let body = arena.alloc(Block {
            stmts: std::slice::from_ref(stmt),
            span: Span::new(1, 12, 6),
        });

        let lambda_expr = LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::new(1, 1, 18),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, None);

        // Lambda body compilation should now succeed
        assert!(
            result.is_ok(),
            "Lambda with body should compile successfully: {:?}",
            result
        );

        // Verify the lambda was registered as a compiled function
        assert_eq!(emitter.compiled_functions().len(), 1);
    }

    #[test]
    fn lambda_non_void_without_return_fails() {
        use angelscript_core::entries::{FuncdefEntry, TypeEntry};
        use angelscript_core::primitives;

        let mut registry = SymbolRegistry::with_primitives();

        // Register a funcdef type: funcdef int IntProducer()
        let funcdef_hash = TypeHash::from_name("IntProducer");
        let funcdef = FuncdefEntry::ffi("IntProducer", vec![], DataType::simple(primitives::INT32));
        registry.register_type(TypeEntry::Funcdef(funcdef)).unwrap();

        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();

        // Lambda with empty body but non-void return type
        let body = arena.alloc(Block {
            stmts: &[],
            span: Span::new(1, 12, 2),
        });

        let lambda_expr = LambdaExpr {
            params: &[],
            return_type: None,
            body,
            span: Span::new(1, 1, 14),
        };

        let expected_type = DataType::simple(funcdef_hash);
        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_lambda(&mut compiler, &lambda_expr, Some(&expected_type));

        // Should fail because non-void lambda needs return
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            CompilationError::Other { message, .. } => {
                assert!(
                    message.contains("not all code paths return"),
                    "Expected 'not all code paths return' error, got: {}",
                    message
                );
            }
            _ => panic!("Expected Other error about missing return, got {:?}", err),
        }
    }
}
