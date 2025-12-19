//! Unary operator expression compilation.
//!
//! Compiles unary operators (-, +, !, ~, ++, --, @).

use angelscript_core::{CompilationError, DataType};
use angelscript_parser::ast::{PostfixExpr, PostfixOp, UnaryExpr, UnaryOp};

use super::{ExprCompiler, Result};
use crate::bytecode::OpCode;
use crate::expr_info::ExprInfo;
use crate::operators::{UnaryResolution, resolve_unary};

/// Compile a unary prefix expression.
pub fn compile_unary<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &UnaryExpr<'ast>,
) -> Result<ExprInfo> {
    match expr.op {
        UnaryOp::PreInc | UnaryOp::PreDec => compile_prefix_inc_dec(compiler, expr),
        UnaryOp::HandleOf => compile_handle_of(compiler, expr),
        _ => compile_simple_unary(compiler, expr),
    }
}

/// Compile a postfix expression (++, --).
pub fn compile_postfix<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &PostfixExpr<'ast>,
) -> Result<ExprInfo> {
    compile_postfix_inc_dec(compiler, expr)
}

fn compile_simple_unary<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &UnaryExpr<'ast>,
) -> Result<ExprInfo> {
    // Compile operand
    let operand_info = compiler.infer(expr.operand)?;

    // Resolve operator
    let resolution = resolve_unary(&operand_info.data_type, expr.op, compiler.ctx(), expr.span)?;

    // Emit bytecode based on resolution
    match resolution {
        UnaryResolution::Primitive {
            opcode,
            result_type,
        } => {
            compiler.emitter().emit(opcode);
            Ok(ExprInfo::rvalue(result_type))
        }
        UnaryResolution::NoOp { result_type } => {
            // Unary plus is a no-op for numeric types
            Ok(ExprInfo::rvalue(result_type))
        }
        UnaryResolution::Method {
            method_hash,
            result_type,
        } => {
            compiler.emitter().emit_call_method(method_hash, 0);
            Ok(ExprInfo::rvalue(result_type))
        }
    }
}

fn compile_prefix_inc_dec<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &UnaryExpr<'ast>,
) -> Result<ExprInfo> {
    // Compile operand as lvalue
    let operand_info = compiler.infer(expr.operand)?;

    // Must be an lvalue
    if !operand_info.is_lvalue {
        return Err(CompilationError::NotAnLvalue { span: expr.span });
    }

    // Must be mutable
    if !operand_info.is_mutable {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot modify const value with increment/decrement operator".to_string(),
            span: expr.span,
        });
    }

    // Emit the appropriate opcode
    let opcode = match expr.op {
        UnaryOp::PreInc => OpCode::PreInc,
        UnaryOp::PreDec => OpCode::PreDec,
        _ => unreachable!(),
    };

    compiler.emitter().emit(opcode);

    // Result is an lvalue with same type, preserving the source for ref return validation
    // (e.g., ++x still refers to the same storage location as x)
    Ok(ExprInfo::lvalue(operand_info.data_type).with_source(operand_info.source))
}

fn compile_postfix_inc_dec<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &PostfixExpr<'ast>,
) -> Result<ExprInfo> {
    // Compile operand as lvalue
    let operand_info = compiler.infer(expr.operand)?;

    // Must be an lvalue
    if !operand_info.is_lvalue {
        return Err(CompilationError::NotAnLvalue { span: expr.span });
    }

    // Must be mutable
    if !operand_info.is_mutable {
        return Err(CompilationError::CannotModifyConst {
            message: "cannot modify const value with increment/decrement operator".to_string(),
            span: expr.span,
        });
    }

    // Emit the appropriate opcode
    let opcode = match expr.op {
        PostfixOp::PostInc => OpCode::PostInc,
        PostfixOp::PostDec => OpCode::PostDec,
    };

    compiler.emitter().emit(opcode);

    // Result is an rvalue (the old value)
    Ok(ExprInfo::rvalue(operand_info.data_type))
}

fn compile_handle_of<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &UnaryExpr<'ast>,
) -> Result<ExprInfo> {
    // Compile operand
    let operand_info = compiler.infer(expr.operand)?;

    // Handle-of (@) creates a handle from a value
    // The operand must be a value type (not already a handle)
    if operand_info.data_type.is_handle {
        return Err(CompilationError::Other {
            message: "Cannot take handle of a handle type".to_string(),
            span: expr.span,
        });
    }

    compiler.emitter().emit(OpCode::HandleOf);

    // Result is a handle to the type
    let handle_type = DataType::with_handle(operand_info.data_type.type_hash, false);
    Ok(ExprInfo::rvalue(handle_type))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::{Span, primitives};
    use angelscript_parser::ast::{Expr, LiteralExpr, LiteralKind};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_compiler<'a, 'ctx, 'pool>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
    ) -> ExprCompiler<'a, 'ctx, 'pool> {
        ExprCompiler::new(ctx, emitter, None)
    }

    fn make_int_literal(arena: &Bump, value: i64) -> &Expr<'_> {
        arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(value),
            span: Span::new(1, 1, 1),
        }))
    }

    #[test]
    fn compile_negation() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let operand = make_int_literal(&arena, 42);
        let unary_expr = arena.alloc(UnaryExpr {
            op: UnaryOp::Neg,
            operand,
            span: Span::new(1, 1, 3),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_unary(&mut compiler, unary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn compile_logical_not() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let operand = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::new(1, 1, 4),
        }));
        let unary_expr = arena.alloc(UnaryExpr {
            op: UnaryOp::LogicalNot,
            operand,
            span: Span::new(1, 1, 5),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_unary(&mut compiler, unary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::BOOL);
    }

    #[test]
    fn compile_unary_plus() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let operand = make_int_literal(&arena, 42);
        let unary_expr = arena.alloc(UnaryExpr {
            op: UnaryOp::Plus,
            operand,
            span: Span::new(1, 1, 3),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_unary(&mut compiler, unary_expr);
        assert!(result.is_ok());

        // Unary plus is a no-op
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
    }
}
