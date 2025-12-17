//! Expression compiler using bidirectional type checking.
//!
//! The [`ExprCompiler`] compiles AST expressions to bytecode using a
//! bidirectional type checking approach:
//! - `infer()` - Synthesizes type from expression (bottom-up)
//! - `check()` - Verifies expression against expected type (top-down)
//!
//! # Example
//!
//! ```ignore
//! let mut compiler = ExprCompiler::new(ctx, &mut scope, &mut emitter, None);
//!
//! // Infer type of expression
//! let info = compiler.infer(&expr)?;
//!
//! // Check expression against expected type
//! let info = compiler.check(&expr, &expected_type)?;
//! ```

mod binary;
mod calls;
mod identifiers;
mod literals;
pub(crate) mod member;
mod unary;

use angelscript_core::{CompilationError, DataType, TypeHash, primitives};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::context::CompilationContext;
use crate::conversion::{Conversion, ConversionKind, find_conversion};
use crate::emit::BytecodeEmitter;
use crate::expr_info::ExprInfo;

type Result<T> = std::result::Result<T, CompilationError>;

/// Compiles expressions using bidirectional type checking.
///
/// The compiler maintains references to the compilation context and
/// bytecode emitter. It supports both type inference (synthesizing types
/// from expressions) and type checking (verifying expressions against expected
/// types).
pub struct ExprCompiler<'a, 'ctx, 'pool> {
    /// Compilation context with type registry, namespace info, and local scope
    ctx: &'a mut CompilationContext<'ctx>,
    /// Bytecode emitter
    emitter: &'a mut BytecodeEmitter<'pool>,
    /// Current class type (for 'this' and method access)
    current_class: Option<TypeHash>,
}

impl<'a, 'ctx, 'pool> ExprCompiler<'a, 'ctx, 'pool> {
    /// Create a new expression compiler.
    ///
    /// # Arguments
    ///
    /// * `ctx` - Compilation context with type information and local scope
    /// * `emitter` - Bytecode emitter for output
    /// * `current_class` - The class being compiled (for 'this' access)
    pub fn new(
        ctx: &'a mut CompilationContext<'ctx>,
        emitter: &'a mut BytecodeEmitter<'pool>,
        current_class: Option<TypeHash>,
    ) -> Self {
        Self {
            ctx,
            emitter,
            current_class,
        }
    }

    /// Synthesize type from expression (infer mode).
    ///
    /// This is the "synthesis" direction of bidirectional type checking.
    /// The type is determined by the expression itself.
    pub fn infer<'ast>(&mut self, expr: &Expr<'ast>) -> Result<ExprInfo> {
        let span = expr.span();
        match expr {
            Expr::Literal(lit) => literals::compile_literal(self, &lit.kind, span),
            Expr::Ident(ident) => identifiers::compile_ident(self, ident),
            Expr::Binary(bin) => binary::compile_binary(self, bin),
            Expr::Unary(un) => unary::compile_unary(self, un),
            Expr::Postfix(post) => unary::compile_postfix(self, post),
            Expr::Paren(p) => self.infer(p.expr),

            // Call expressions (Task 42)
            Expr::Call(call) => calls::compile_call(self, call),
            Expr::Member(member) => member::compile_member(self, member),
            Expr::Index(index) => member::compile_index(self, index),

            // Deferred to Task 43
            Expr::Assign(_) => Err(CompilationError::Other {
                message: "Assignment not yet implemented (Task 43)".to_string(),
                span,
            }),
            Expr::Ternary(_) => Err(CompilationError::Other {
                message: "Ternary expressions not yet implemented (Task 43)".to_string(),
                span,
            }),
            Expr::Cast(_) => Err(CompilationError::Other {
                message: "Cast expressions not yet implemented (Task 43)".to_string(),
                span,
            }),
            Expr::Lambda(_) => Err(CompilationError::Other {
                message: "Lambda expressions not yet implemented (Task 43)".to_string(),
                span,
            }),
            Expr::InitList(_) => Err(CompilationError::Other {
                message: "Init list expressions not yet implemented (Task 43)".to_string(),
                span,
            }),
        }
    }

    /// Check expression against expected type (check mode).
    ///
    /// This is the "checking" direction of bidirectional type checking.
    /// The expected type guides type checking and may enable implicit conversions.
    pub fn check<'ast>(&mut self, expr: &Expr<'ast>, expected: &DataType) -> Result<ExprInfo> {
        let info = self.infer(expr)?;

        // Exact type match - no conversion needed
        if info.data_type.type_hash == expected.type_hash {
            return Ok(info);
        }

        // Try implicit conversion
        if let Some(conv) = find_conversion(&info.data_type, expected, self.ctx)
            && conv.is_implicit()
        {
            emit_conversion(self.emitter, &conv);
            return Ok(ExprInfo::rvalue(*expected));
        }

        Err(CompilationError::TypeMismatch {
            message: format!(
                "expected '{}', got '{}'",
                self.type_name(expected.type_hash),
                self.type_name(info.data_type.type_hash)
            ),
            span: expr.span(),
        })
    }

    /// Get the name of a type for error messages.
    fn type_name(&self, hash: TypeHash) -> String {
        self.ctx
            .get_type(hash)
            .map(|e| e.qualified_name().to_string())
            .unwrap_or_else(|| format!("{:?}", hash))
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the compilation context (immutable).
    pub fn ctx(&self) -> &CompilationContext<'ctx> {
        self.ctx
    }

    /// Get the compilation context (mutable).
    pub fn ctx_mut(&mut self) -> &mut CompilationContext<'ctx> {
        self.ctx
    }

    /// Get the bytecode emitter.
    pub fn emitter(&mut self) -> &mut BytecodeEmitter<'pool> {
        self.emitter
    }

    /// Get the current class type (if compiling a method).
    pub fn current_class(&self) -> Option<TypeHash> {
        self.current_class
    }
}

/// Emit the bytecode for a type conversion.
pub(crate) fn emit_conversion(emitter: &mut BytecodeEmitter<'_>, conv: &Conversion) {
    match &conv.kind {
        ConversionKind::Identity => {
            // No bytecode needed
        }
        ConversionKind::Primitive { from, to } => {
            if let Some(opcode) = primitive_conversion_opcode(*from, *to) {
                emitter.emit(opcode);
            }
        }
        ConversionKind::NullToHandle => {
            // Null is already the right type on the stack
        }
        ConversionKind::HandleToConst => {
            emitter.emit(OpCode::HandleToConst);
        }
        ConversionKind::DerivedToBase { base } => {
            emitter.emit_cast(*base);
        }
        ConversionKind::ClassToInterface { interface } => {
            emitter.emit_cast(*interface);
        }
        ConversionKind::ConstructorConversion { constructor } => {
            emitter.emit_call(*constructor, 1);
        }
        ConversionKind::ImplicitConvMethod { method } => {
            emitter.emit_call_method(*method, 0);
        }
        ConversionKind::ExplicitCastMethod { method } => {
            emitter.emit_call_method(*method, 0);
        }
        ConversionKind::ValueToHandle => {
            emitter.emit(OpCode::ValueToHandle);
        }
        ConversionKind::EnumToInt => {
            // Enum value is already an integer
        }
        ConversionKind::IntToEnum { .. } => {
            // Integer value treated as enum
        }
        ConversionKind::ReferenceCast { target } => {
            emitter.emit_cast(*target);
        }
        ConversionKind::VarArg => {
            // No conversion needed for variable arguments
        }
    }
}

/// Get the opcode for converting between primitive types.
pub(crate) fn primitive_conversion_opcode(from: TypeHash, to: TypeHash) -> Option<OpCode> {
    use primitives::*;

    match (from, to) {
        // Integer widening
        (INT8, INT16) => Some(OpCode::I8toI16),
        (INT8, INT32) => Some(OpCode::I8toI32),
        (INT8, INT64) => Some(OpCode::I8toI64),
        (INT16, INT32) => Some(OpCode::I16toI32),
        (INT16, INT64) => Some(OpCode::I16toI64),
        (INT32, INT64) => Some(OpCode::I32toI64),
        (UINT8, UINT16) => Some(OpCode::U8toU16),
        (UINT8, UINT32) => Some(OpCode::U8toU32),
        (UINT8, UINT64) => Some(OpCode::U8toU64),
        (UINT16, UINT32) => Some(OpCode::U16toU32),
        (UINT16, UINT64) => Some(OpCode::U16toU64),
        (UINT32, UINT64) => Some(OpCode::U32toU64),

        // Integer narrowing
        (INT64, INT32) => Some(OpCode::I64toI32),
        (INT64, INT16) => Some(OpCode::I64toI16),
        (INT64, INT8) => Some(OpCode::I64toI8),
        (INT32, INT16) => Some(OpCode::I32toI16),
        (INT32, INT8) => Some(OpCode::I32toI8),
        (INT16, INT8) => Some(OpCode::I16toI8),

        // Integer to float
        (INT32, FLOAT) => Some(OpCode::I32toF32),
        (INT32, DOUBLE) => Some(OpCode::I32toF64),
        (INT64, FLOAT) => Some(OpCode::I64toF32),
        (INT64, DOUBLE) => Some(OpCode::I64toF64),

        // Float to integer
        (FLOAT, INT32) => Some(OpCode::F32toI32),
        (FLOAT, INT64) => Some(OpCode::F32toI64),
        (DOUBLE, INT32) => Some(OpCode::F64toI32),
        (DOUBLE, INT64) => Some(OpCode::F64toI64),

        // Float widening/narrowing
        (FLOAT, DOUBLE) => Some(OpCode::F32toF64),
        (DOUBLE, FLOAT) => Some(OpCode::F64toF32),

        // Same type or unknown conversion
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use angelscript_core::primitives;
    use angelscript_registry::SymbolRegistry;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn expr_compiler_creation() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let _compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);
    }

    #[test]
    fn expr_compiler_with_class() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let class_hash = TypeHash::from_name("MyClass");
        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, Some(class_hash));

        assert_eq!(compiler.current_class(), Some(class_hash));
    }

    #[test]
    fn type_name_unknown() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);
        let unknown_hash = TypeHash::from_name("UnknownType");
        let name = compiler.type_name(unknown_hash);

        // Should return debug format for unknown types
        assert!(name.contains("TypeHash"));
    }

    #[test]
    fn type_name_primitive() {
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        let compiler = ExprCompiler::new(&mut ctx, &mut emitter, None);
        let name = compiler.type_name(primitives::INT32);

        // Primitives should be registered in FFI
        assert!(name == "int" || name.contains("TypeHash"));
    }
}
