//! Literal expression compilation.
//!
//! Compiles literal values (integers, floats, strings, booleans, null).

use angelscript_core::{CompilationError, DataType, Span, primitives};
use angelscript_parser::ast::LiteralKind;

use super::{ExprCompiler, Result};
use crate::expr_info::ExprInfo;

/// Compile a literal expression.
pub fn compile_literal(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    kind: &LiteralKind,
    span: Span,
) -> Result<ExprInfo> {
    match kind {
        LiteralKind::Int(value) => compile_int(compiler, *value),
        LiteralKind::Float(value) => compile_float(compiler, *value),
        LiteralKind::Double(value) => compile_double(compiler, *value),
        LiteralKind::Bool(value) => compile_bool(compiler, *value),
        LiteralKind::String(bytes) => compile_string(compiler, bytes, span),
        LiteralKind::Null => compile_null(compiler),
    }
}

fn compile_int(compiler: &mut ExprCompiler<'_, '_, '_>, value: i64) -> Result<ExprInfo> {
    // All integer literals are emitted as int64, VM handles narrowing
    compiler.emitter().emit_int(value);

    // Determine the type based on value range
    let type_hash = if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
        primitives::INT32
    } else {
        primitives::INT64
    };

    Ok(ExprInfo::rvalue(DataType::simple(type_hash)))
}

fn compile_float(compiler: &mut ExprCompiler<'_, '_, '_>, value: f32) -> Result<ExprInfo> {
    compiler.emitter().emit_f32(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::FLOAT)))
}

fn compile_double(compiler: &mut ExprCompiler<'_, '_, '_>, value: f64) -> Result<ExprInfo> {
    compiler.emitter().emit_f64(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::DOUBLE)))
}

fn compile_bool(compiler: &mut ExprCompiler<'_, '_, '_>, value: bool) -> Result<ExprInfo> {
    compiler.emitter().emit_bool(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}

fn compile_string(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    bytes: &[u8],
    span: Span,
) -> Result<ExprInfo> {
    // Get string type from context's string factory
    let string_type = compiler
        .ctx()
        .string_type_hash()
        .ok_or(CompilationError::NoStringFactory { span })?;

    // Emit the string bytes - the VM will call the string factory to create the value
    compiler.emitter().emit_string_bytes(bytes.to_vec());

    Ok(ExprInfo::rvalue(DataType::simple(string_type)))
}

fn compile_null(compiler: &mut ExprCompiler<'_, '_, '_>) -> Result<ExprInfo> {
    compiler.emitter().emit_null();
    // Null has a special "null handle" type that can convert to any handle type
    Ok(ExprInfo::rvalue(DataType::simple(primitives::NULL)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{ConstantPool, OpCode};
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_registry::SymbolRegistry;

    fn create_test_compiler<'a, 'ctx, 'pool>(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
    ) -> ExprCompiler<'a, 'ctx, 'pool> {
        ExprCompiler::new(ctx, emitter, None)
    }

    #[test]
    fn compile_int_literal_small() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_int(&mut compiler, 42);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT32);
        assert!(!info.is_lvalue);
    }

    #[test]
    fn compile_int_literal_large() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let large_value = i64::MAX;
        let result = compile_int(&mut compiler, large_value);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::INT64);
    }

    #[test]
    fn compile_float_literal() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_float(&mut compiler, 3.14);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::FLOAT);
    }

    #[test]
    fn compile_double_literal() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_double(&mut compiler, 2.71828);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::DOUBLE);
    }

    #[test]
    fn compile_bool_true() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_bool(&mut compiler, true);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::BOOL);

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushTrue));
    }

    #[test]
    fn compile_bool_false() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_bool(&mut compiler, false);
        assert!(result.is_ok());

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushFalse));
    }

    #[test]
    fn compile_null_literal() {
        let registry = SymbolRegistry::with_primitives();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut constants = ConstantPool::new();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let mut compiler = create_test_compiler(&mut ctx, &mut emitter);

        let result = compile_null(&mut compiler);
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.data_type.type_hash, primitives::NULL);

        let chunk = emitter.finish();
        assert_eq!(chunk.read_op(0), Some(OpCode::PushNull));
    }
}
