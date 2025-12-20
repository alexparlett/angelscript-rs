//! Binary operator expression compilation.
//!
//! Compiles binary operators (+, -, *, /, %, etc.) including
//! short-circuit evaluation for logical operators.

use angelscript_core::{DataType, primitives};
use angelscript_parser::ast::{BinaryExpr, BinaryOp};

use super::{ExprCompiler, Result};
use crate::bytecode::OpCode;
use crate::expr_info::ExprInfo;
use crate::operators::{OperatorResolution, resolve_binary};

/// Compile a binary expression.
pub fn compile_binary<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    // Handle short-circuit operators specially (not overloadable)
    match expr.op {
        BinaryOp::LogicalAnd => return compile_logical_and(compiler, expr),
        BinaryOp::LogicalOr => return compile_logical_or(compiler, expr),
        BinaryOp::LogicalXor => return compile_logical_xor(compiler, expr),
        _ => {}
    }

    // First compile left operand to infer its type
    let left_info = compiler.infer(expr.left)?;

    // Compile right operand to infer its type
    let right_info = compiler.infer(expr.right)?;

    // Resolve operator to find out what operation to perform
    let resolution = resolve_binary(
        &left_info.data_type,
        &right_info.data_type,
        expr.op,
        compiler.ctx(),
        expr.span,
    )?;

    // Emit bytecode based on resolution
    match resolution {
        OperatorResolution::Primitive {
            opcode,
            result_type,
            ..
        } => {
            // For primitive operations, operands are already on the stack
            compiler.emitter().emit(opcode);

            // Handle NotEqual by negating the result
            if matches!(expr.op, BinaryOp::NotEqual) {
                compiler.emitter().emit(OpCode::Not);
            }

            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::MethodOnLeft {
            method_hash,
            result_type,
            ..
        } => {
            // Get parameter type and re-compile with type checking
            let param_type = compiler
                .ctx()
                .get_function(method_hash)
                .and_then(|f| f.def.params.first())
                .map(|p| p.data_type)
                .ok_or_else(|| angelscript_core::CompilationError::Other {
                    message: "Method signature missing parameter".to_string(),
                    span: expr.span,
                })?;

            // Re-compile: left (receiver) and right (with conversion)
            compiler.infer(expr.left)?;
            compiler.check(expr.right, &param_type)?;

            compiler.emitter().emit_call_method(method_hash, 1);

            if matches!(expr.op, BinaryOp::NotEqual | BinaryOp::NotIs) {
                compiler.emitter().emit(OpCode::Not);
            }

            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::MethodOnRight {
            method_hash,
            result_type,
            ..
        } => {
            // Get parameter type and re-compile with type checking
            let param_type = compiler
                .ctx()
                .get_function(method_hash)
                .and_then(|f| f.def.params.first())
                .map(|p| p.data_type)
                .ok_or_else(|| angelscript_core::CompilationError::Other {
                    message: "Method signature missing parameter".to_string(),
                    span: expr.span,
                })?;

            // Re-compile: right (receiver) and left (with conversion)
            compiler.infer(expr.right)?;
            compiler.check(expr.left, &param_type)?;

            compiler.emitter().emit_call_method(method_hash, 1);

            if matches!(expr.op, BinaryOp::NotEqual | BinaryOp::NotIs) {
                compiler.emitter().emit(OpCode::Not);
            }

            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::HandleComparison { negate } => {
            // Operands already on the stack
            compiler.emitter().emit(OpCode::EqHandle);
            if negate {
                compiler.emitter().emit(OpCode::Not);
            }
            Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
        }
    }
}

/// Compile && with short-circuit evaluation.
fn compile_logical_and<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    // Compile left, coerce to bool
    compiler.check(expr.left, &bool_type)?;

    // If false, skip right (result is false)
    let jump_to_end = compiler.emitter().emit_jump(OpCode::JumpIfFalse);

    // Pop left result (was true, continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right, coerce to bool
    compiler.check(expr.right, &bool_type)?;

    // Patch jump to here
    compiler.emitter().patch_jump(jump_to_end);

    Ok(ExprInfo::rvalue(bool_type))
}

/// Compile || with short-circuit evaluation.
fn compile_logical_or<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    // Compile left, coerce to bool
    compiler.check(expr.left, &bool_type)?;

    // If true, skip right (result is true)
    let jump_to_end = compiler.emitter().emit_jump(OpCode::JumpIfTrue);

    // Pop left result (was false, continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right, coerce to bool
    compiler.check(expr.right, &bool_type)?;

    // Patch jump
    compiler.emitter().patch_jump(jump_to_end);

    Ok(ExprInfo::rvalue(bool_type))
}

/// Compile ^^ (logical XOR) - no short circuit, both sides always evaluated.
fn compile_logical_xor<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    compiler.check(expr.left, &bool_type)?;
    compiler.check(expr.right, &bool_type)?;

    // XOR for booleans: a != b
    compiler.emitter().emit(OpCode::EqBool);
    compiler.emitter().emit(OpCode::Not);

    Ok(ExprInfo::rvalue(bool_type))
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

    fn make_bool_literal(arena: &Bump, value: bool) -> &Expr<'_> {
        arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(value),
            span: Span::new(1, 1, 4),
        }))
    }

    #[test]
    fn compile_int_addition() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let left = make_int_literal(&arena, 1);
        let right = make_int_literal(&arena, 2);
        let binary_expr = arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::Add,
            right,
            span: Span::new(1, 1, 5),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_binary(&mut compiler, binary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);

        let chunk = emitter.finish();
        // Bytecode: PushOne (left=1), Constant (right=2), AddI32
        chunk.assert_opcodes(&[OpCode::PushOne, OpCode::Constant, OpCode::AddI32]);
    }

    #[test]
    fn compile_int_comparison() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let left = make_int_literal(&arena, 5);
        let right = make_int_literal(&arena, 3);
        let binary_expr = arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::Less,
            right,
            span: Span::new(1, 1, 5),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_binary(&mut compiler, binary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        // Comparison should return bool
        assert_eq!(info.data_type.type_hash, primitives::BOOL);

        let chunk = emitter.finish();
        // Bytecode: Constant (left), Constant (right), LtI32
        chunk.assert_opcodes(&[OpCode::Constant, OpCode::Constant, OpCode::LtI32]);
    }

    #[test]
    fn compile_logical_and() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let left = make_bool_literal(&arena, true);
        let right = make_bool_literal(&arena, false);
        let binary_expr = arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::LogicalAnd,
            right,
            span: Span::new(1, 1, 11),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = super::compile_logical_and(&mut compiler, binary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::BOOL);

        let chunk = emitter.finish();
        // Bytecode: PushTrue (left), JumpIfFalse, Pop, PushFalse (right)
        chunk.assert_contains_opcodes(&[
            OpCode::PushTrue,
            OpCode::JumpIfFalse,
            OpCode::Pop,
            OpCode::PushFalse,
        ]);
    }

    #[test]
    fn compile_logical_or() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let left = make_bool_literal(&arena, false);
        let right = make_bool_literal(&arena, true);
        let binary_expr = arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::LogicalOr,
            right,
            span: Span::new(1, 1, 11),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = super::compile_logical_or(&mut compiler, binary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::BOOL);

        let chunk = emitter.finish();
        // Bytecode: PushFalse (left), JumpIfTrue, Pop, PushTrue (right)
        chunk.assert_contains_opcodes(&[
            OpCode::PushFalse,
            OpCode::JumpIfTrue,
            OpCode::Pop,
            OpCode::PushTrue,
        ]);
    }

    #[test]
    fn compile_logical_xor() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let arena = Bump::new();
        let left = make_bool_literal(&arena, true);
        let right = make_bool_literal(&arena, true);
        let binary_expr = arena.alloc(BinaryExpr {
            left,
            op: BinaryOp::LogicalXor,
            right,
            span: Span::new(1, 1, 11),
        });

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = super::compile_logical_xor(&mut compiler, binary_expr);
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::BOOL);

        let chunk = emitter.finish();
        // Bytecode: PushTrue (left), PushTrue (right), EqBool, Not
        chunk.assert_opcodes(&[
            OpCode::PushTrue,
            OpCode::PushTrue,
            OpCode::EqBool,
            OpCode::Not,
        ]);
    }
}
