//! Ternary conditional expression compilation.
//!
//! Handles `condition ? then_expr : else_expr` syntax with short-circuit evaluation.

use angelscript_core::{CompilationError, DataType, primitives};
use angelscript_parser::ast::TernaryExpr;

use super::{ExprCompiler, Result};
use crate::bytecode::OpCode;
use crate::conversion::find_conversion;
use crate::expr_info::ExprInfo;

/// Compile a ternary conditional expression: `cond ? then : else`
///
/// The condition is evaluated first. If true, the then branch is evaluated;
/// otherwise the else branch. Only one branch is evaluated at runtime
/// (short-circuit evaluation).
pub fn compile_ternary<'ast>(
    compiler: &mut ExprCompiler<'_, '_>,
    expr: &TernaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    // 1. Compile condition, coerce to bool
    compiler.check(expr.condition, &bool_type)?;

    // 2. If false, jump to else branch
    let jump_to_else = compiler.emitter().emit_jump(OpCode::JumpIfFalse);

    // 3. Pop condition (was true), compile then branch
    compiler.emitter().emit(OpCode::Pop);
    let then_info = compiler.infer(expr.then_expr)?;

    // 4. Jump over else branch
    let jump_to_end = compiler.emitter().emit_jump(OpCode::Jump);

    // 5. Patch else jump target, pop condition (was false), compile else branch
    compiler.emitter().patch_jump(jump_to_else);
    compiler.emitter().emit(OpCode::Pop);
    let else_info = compiler.infer(expr.else_expr)?;

    // 6. Patch end jump target
    compiler.emitter().patch_jump(jump_to_end);

    // 7. Unify types of then and else branches
    let result_type = unify_types(
        &then_info.data_type,
        &else_info.data_type,
        compiler,
        expr.span,
    )?;

    Ok(ExprInfo::rvalue(result_type))
}

/// Find a common type for the two branches of a ternary expression.
///
/// Type unification rules:
/// 1. Same type - no conversion needed
/// 2. One can implicitly convert to the other - use that type
/// 3. Neither can convert - error
fn unify_types(
    then_type: &DataType,
    else_type: &DataType,
    compiler: &ExprCompiler<'_, '_>,
    span: angelscript_core::Span,
) -> Result<DataType> {
    // Same type - no conversion needed
    if then_type.type_hash == else_type.type_hash {
        return Ok(*then_type);
    }

    // Check if else can convert to then
    if find_conversion(else_type, then_type, compiler.ctx(), true).is_some() {
        // else converts to then type
        return Ok(*then_type);
    }

    // Check if then can convert to else
    if find_conversion(then_type, else_type, compiler.ctx(), true).is_some() {
        // then converts to else type
        return Ok(*else_type);
    }

    // No common type found
    let then_name = compiler
        .ctx()
        .get_type(then_type.type_hash)
        .map(|t| t.qualified_name().to_string())
        .unwrap_or_else(|| format!("{:?}", then_type.type_hash));

    let else_name = compiler
        .ctx()
        .get_type(else_type.type_hash)
        .map(|t| t.qualified_name().to_string())
        .unwrap_or_else(|| format!("{:?}", else_type.type_hash));

    Err(CompilationError::TypeMismatch {
        message: format!(
            "ternary branches have incompatible types: '{}' and '{}'",
            then_name, else_name
        ),
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::Span;
    use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter,
    ) -> ExprCompiler<'a, 'ctx> {
        ExprCompiler::new(ctx, emitter, None)
    }

    fn make_bool_literal(arena: &Bump, value: bool) -> &Expr<'_> {
        arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(value),
            span: Span::new(1, 1, 4),
        }))
    }

    fn make_int_literal(arena: &Bump, value: i64) -> &Expr<'_> {
        arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(value),
            span: Span::new(1, 1, 1),
        }))
    }

    #[test]
    fn ternary_same_type() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let cond = make_bool_literal(&arena, true);
        let then_expr = make_int_literal(&arena, 1);
        let else_expr = make_int_literal(&arena, 2);

        let ternary_expr = TernaryExpr {
            condition: cond,
            then_expr,
            else_expr,
            span: Span::new(1, 1, 15),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_ternary(&mut compiler, &ternary_expr);

        assert!(result.is_ok(), "Ternary should compile: {:?}", result);
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn ternary_generates_jumps() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let cond = make_bool_literal(&arena, true);
        let then_expr = make_int_literal(&arena, 1);
        let else_expr = make_int_literal(&arena, 2);

        let ternary_expr = TernaryExpr {
            condition: cond,
            then_expr,
            else_expr,
            span: Span::new(1, 1, 15),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let _ = compile_ternary(&mut compiler, &ternary_expr);

        let chunk = emitter.finish_chunk();
        // Should contain JumpIfFalse and Jump opcodes
        let mut found_jump_if_false = false;
        let mut found_jump = false;
        let mut pos = 0;
        while let Some(op) = chunk.read_op(pos) {
            if matches!(op, OpCode::JumpIfFalse) {
                found_jump_if_false = true;
            }
            if matches!(op, OpCode::Jump) {
                found_jump = true;
            }
            pos += 1;
        }
        assert!(found_jump_if_false, "Should emit JumpIfFalse");
        assert!(found_jump, "Should emit Jump");
    }

    #[test]
    fn ternary_incompatible_types_fails() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new();
        emitter.start_chunk();

        let arena = Bump::new();
        let cond = make_bool_literal(&arena, true);
        let then_expr = make_int_literal(&arena, 1);
        let else_expr = make_bool_literal(&arena, false);

        let ternary_expr = TernaryExpr {
            condition: cond,
            then_expr,
            else_expr,
            span: Span::new(1, 1, 15),
        };

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);
        let result = compile_ternary(&mut compiler, &ternary_expr);

        assert!(
            result.is_err(),
            "Ternary with incompatible types should fail"
        );
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::TypeMismatch { .. }
        ));
    }
}
