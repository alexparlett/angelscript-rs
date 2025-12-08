# Task 40: Expression Compilation - Basics

## Overview

Implement basic expression compilation: literals, identifiers, unary operators, and binary operators. This forms the foundation of the expression checker.

## Goals

1. Compile literals (int, float, string, bool, null)
2. Compile identifier access (variables, globals)
3. Compile unary operators (-, !, ~, ++, --)
4. Compile binary operators (+, -, *, /, etc.)
5. Implement bidirectional type checking (infer and check modes)

## Dependencies

- Task 32: Compilation Context
- Task 34: Conversion System
- Task 35: Overload Resolution (for operators)
- Task 37: Local Scope
- Task 38: Bytecode Emitter

## Files to Create

```
crates/angelscript-compiler/src/
├── expr/
│   ├── mod.rs             # ExprCompiler
│   ├── literals.rs        # Literal compilation
│   ├── identifiers.rs     # Variable/global access
│   ├── unary.rs           # Unary operators
│   └── binary.rs          # Binary operators
└── lib.rs
```

## Detailed Implementation

### ExprCompiler (expr/mod.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::Expr;

use crate::context::CompilationContext;
use crate::conversion::{can_implicitly_convert, find_conversion};
use crate::emit::BytecodeEmitter;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::scope::LocalScope;

mod literals;
mod identifiers;
mod unary;
mod binary;

/// Compiles expressions using bidirectional type checking.
pub struct ExprCompiler<'a, 'reg> {
    ctx: &'a CompilationContext<'reg>,
    scope: &'a mut LocalScope,
    emitter: &'a mut BytecodeEmitter,

    /// Current class type (for 'this' and method access)
    current_class: Option<TypeHash>,
}

impl<'a, 'reg> ExprCompiler<'a, 'reg> {
    pub fn new(
        ctx: &'a CompilationContext<'reg>,
        scope: &'a mut LocalScope,
        emitter: &'a mut BytecodeEmitter,
        current_class: Option<TypeHash>,
    ) -> Self {
        Self {
            ctx,
            scope,
            emitter,
            current_class,
        }
    }

    // ==========================================================================
    // Bidirectional Type Checking
    // ==========================================================================

    /// Synthesize type from expression (infer mode).
    ///
    /// Use when we don't know the expected type.
    pub fn infer(&mut self, expr: &Expr, span: Span) -> Result<ExprInfo> {
        match expr {
            // Literals
            Expr::IntLiteral(value) => literals::compile_int(self, *value, span),
            Expr::FloatLiteral(value) => literals::compile_float(self, *value, span),
            Expr::StringLiteral(value) => literals::compile_string(self, value, span),
            Expr::BoolLiteral(value) => literals::compile_bool(self, *value, span),
            Expr::NullLiteral => literals::compile_null(self, span),

            // Identifiers
            Expr::Identifier(name) => identifiers::compile_identifier(self, name, span),
            Expr::This => identifiers::compile_this(self, span),

            // Operators
            Expr::Unary { op, operand } => unary::compile_unary(self, *op, operand, span),
            Expr::Binary { left, op, right } => binary::compile_binary(self, left, *op, right, span),

            // Grouping
            Expr::Paren(inner) => self.infer(inner, span),

            // Others handled in later tasks
            Expr::Call { .. } => todo!("Task 40"),
            Expr::MethodCall { .. } => todo!("Task 40"),
            Expr::FieldAccess { .. } => todo!("Task 40"),
            Expr::Index { .. } => todo!("Task 40"),
            Expr::Cast { .. } => todo!("Task 41"),
            Expr::Lambda { .. } => todo!("Task 41"),
            Expr::InitList { .. } => todo!("Task 41"),
            Expr::Ternary { .. } => todo!("Task 41"),
            Expr::Assignment { .. } => self.compile_assignment(expr, span),
        }
    }

    /// Check expression against expected type (check mode).
    ///
    /// Use when we know what type the expression should produce.
    /// Can enable better error messages and type inference.
    pub fn check(&mut self, expr: &Expr, expected: &DataType, span: Span) -> Result<ExprInfo> {
        // First infer the type
        let info = self.infer(expr, span)?;

        // Check if it matches or can be converted
        if info.data_type.type_hash == expected.type_hash {
            return Ok(info);
        }

        // Try implicit conversion
        if can_implicitly_convert(&info.data_type, expected, self.ctx) {
            self.emit_conversion(&info.data_type, expected)?;
            return Ok(ExprInfo::rvalue(*expected));
        }

        Err(CompileError::TypeMismatch {
            expected: format!("{:?}", expected.type_hash),
            got: format!("{:?}", info.data_type.type_hash),
            span,
        })
    }

    /// Coerce expression to target type, emitting conversion if needed.
    pub fn coerce(&mut self, info: &ExprInfo, target: &DataType, span: Span) -> Result<()> {
        if info.data_type.type_hash == target.type_hash {
            return Ok(());
        }

        if let Some(conv) = find_conversion(&info.data_type, target, self.ctx) {
            if conv.is_implicit {
                self.emit_conversion(&info.data_type, target)?;
                return Ok(());
            }
        }

        Err(CompileError::ConversionError {
            from: format!("{:?}", info.data_type.type_hash),
            to: format!("{:?}", target.type_hash),
            span,
        })
    }

    /// Emit conversion bytecode.
    fn emit_conversion(&mut self, from: &DataType, to: &DataType) -> Result<()> {
        use crate::conversion::primitive::get_conversion_opcode;

        if let Some(opcode) = get_conversion_opcode(from.type_hash, to.type_hash) {
            self.emitter.emit_conversion(opcode);
        }
        // Other conversions (user-defined, etc.) handled differently

        Ok(())
    }

    // ==========================================================================
    // Assignment
    // ==========================================================================

    fn compile_assignment(&mut self, expr: &Expr, span: Span) -> Result<ExprInfo> {
        if let Expr::Assignment { target, op, value } = expr {
            // Compile target as lvalue
            let target_info = self.infer(target, span)?;

            if !target_info.is_assignable() {
                return Err(CompileError::NotAssignable {
                    reason: if target_info.is_lvalue && !target_info.is_mutable {
                        "const variable".to_string()
                    } else {
                        "not an lvalue".to_string()
                    },
                    span,
                });
            }

            // Compile value and coerce to target type
            let value_info = self.check(value, &target_info.data_type, span)?;

            // Handle compound assignment (+=, -=, etc.)
            if let Some(binary_op) = op.to_binary_op() {
                // Load current value, perform op, store result
                // This is simplified - full impl needs proper lvalue handling
            }

            // Emit store instruction (depends on target kind)
            // This needs info from target compilation about where to store

            Ok(ExprInfo::rvalue(target_info.data_type))
        } else {
            unreachable!()
        }
    }

    // ==========================================================================
    // Accessors
    // ==========================================================================

    pub fn emitter(&mut self) -> &mut BytecodeEmitter {
        self.emitter
    }

    pub fn ctx(&self) -> &CompilationContext<'reg> {
        self.ctx
    }

    pub fn scope(&self) -> &LocalScope {
        self.scope
    }

    pub fn scope_mut(&mut self) -> &mut LocalScope {
        self.scope
    }

    pub fn current_class(&self) -> Option<TypeHash> {
        self.current_class
    }
}
```

### Literals (expr/literals.rs)

```rust
use angelscript_core::{primitives, DataType, Span};

use crate::error::Result;
use crate::expr_info::ExprInfo;
use super::ExprCompiler;

pub fn compile_int(compiler: &mut ExprCompiler, value: i64, _span: Span) -> Result<ExprInfo> {
    compiler.emitter().emit_int(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::INT32)))
}

pub fn compile_float(compiler: &mut ExprCompiler, value: f64, _span: Span) -> Result<ExprInfo> {
    compiler.emitter().emit_float(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::DOUBLE)))
}

pub fn compile_string(compiler: &mut ExprCompiler, value: &str, _span: Span) -> Result<ExprInfo> {
    compiler.emitter().emit_string(value.to_string());
    Ok(ExprInfo::rvalue(DataType::simple(primitives::STRING)))
}

pub fn compile_bool(compiler: &mut ExprCompiler, value: bool, _span: Span) -> Result<ExprInfo> {
    compiler.emitter().emit_bool(value);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}

pub fn compile_null(compiler: &mut ExprCompiler, _span: Span) -> Result<ExprInfo> {
    compiler.emitter().emit_null();
    Ok(ExprInfo::rvalue(DataType::null_handle()))
}
```

### Identifiers (expr/identifiers.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};

use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::scope::VarLookup;
use super::ExprCompiler;

pub fn compile_identifier(compiler: &mut ExprCompiler, name: &str, span: Span) -> Result<ExprInfo> {
    // Check local scope first
    if let Some(lookup) = compiler.scope_mut().get_or_capture(name) {
        return match lookup {
            VarLookup::Local(var) => {
                compiler.emitter().emit_get_local(var.slot);
                Ok(if var.is_const {
                    ExprInfo::const_lvalue(var.data_type)
                } else {
                    ExprInfo::lvalue(var.data_type)
                })
            }
            VarLookup::Captured(cap) => {
                // Emit capture access
                // This would be a special opcode for closure captures
                Ok(if cap.is_const {
                    ExprInfo::const_lvalue(cap.data_type)
                } else {
                    ExprInfo::lvalue(cap.data_type)
                })
            }
        };
    }

    // Check globals
    if let Some(global_hash) = compiler.ctx().resolve_global(name) {
        let global = compiler.ctx().get_global(global_hash)
            .ok_or_else(|| CompileError::Internal {
                message: format!("Global {} not found", name),
            })?;

        compiler.emitter().emit_constant(crate::bytecode::Constant::TypeHash(global_hash));
        compiler.emitter().emit(crate::bytecode::OpCode::GetGlobal);

        return Ok(if global.is_const {
            ExprInfo::const_lvalue(global.data_type)
        } else {
            ExprInfo::lvalue(global.data_type)
        });
    }

    // Check if it's a function name (for function pointers)
    let funcs = compiler.ctx().find_functions(name);
    if funcs.len() == 1 {
        // Single function - create function pointer
        let func_hash = funcs[0];
        compiler.emitter().emit_constant(crate::bytecode::Constant::TypeHash(func_hash));
        compiler.emitter().emit(crate::bytecode::OpCode::FuncPtr);

        // Get function signature for type
        if let Some(func) = compiler.ctx().get_function(func_hash) {
            let funcdef_type = DataType::simple(func.def().funcdef_type());
            return Ok(ExprInfo::rvalue(funcdef_type));
        }
    }

    Err(CompileError::UndefinedVariable {
        name: name.to_string(),
        span,
    })
}

pub fn compile_this(compiler: &mut ExprCompiler, span: Span) -> Result<ExprInfo> {
    let class_hash = compiler.current_class().ok_or_else(|| {
        CompileError::Internal {
            message: "'this' used outside class method".to_string(),
        }
    })?;

    compiler.emitter().emit(crate::bytecode::OpCode::GetThis);
    Ok(ExprInfo::rvalue(DataType::simple(class_hash).as_handle()))
}
```

### Binary Operators (expr/binary.rs)

```rust
use angelscript_core::{primitives, DataType, Span};
use angelscript_parser::ast::BinaryOp;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::overload::operators::{resolve_binary_operator, OperatorResolution};
use super::ExprCompiler;

pub fn compile_binary(
    compiler: &mut ExprCompiler,
    left: &Expr,
    op: BinaryOp,
    right: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Handle short-circuit operators specially
    match op {
        BinaryOp::And => return compile_and(compiler, left, right, span),
        BinaryOp::Or => return compile_or(compiler, left, right, span),
        _ => {}
    }

    // Compile operands
    let left_info = compiler.infer(left, span)?;
    let right_info = compiler.infer(right, span)?;

    // Resolve operator
    let resolution = resolve_binary_operator(
        &left_info.data_type,
        &right_info.data_type,
        op,
        compiler.ctx(),
    )?;

    match resolution {
        OperatorResolution::Primitive { opcode, result_type } => {
            compiler.emitter().emit(opcode);
            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::Method { method_hash, on_left, arg_conversion, result_type } => {
            // Emit method call
            if on_left {
                // left.op(right)
                if let Some(conv) = arg_conversion {
                    // Convert right operand if needed
                }
                compiler.emitter().emit_call_method(method_hash, 1);
            } else {
                // right.op_r(left) - swap operands
                compiler.emitter().emit(OpCode::Swap);
                compiler.emitter().emit_call_method(method_hash, 1);
            }
            Ok(ExprInfo::rvalue(result_type))
        }
    }
}

/// Compile && with short-circuit evaluation.
fn compile_and(
    compiler: &mut ExprCompiler,
    left: &Expr,
    right: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Compile left
    let left_info = compiler.check(left, &DataType::simple(primitives::BOOL), span)?;

    // If false, skip right and push false
    let skip_right = compiler.emitter().emit_jump(OpCode::JumpIfFalse);

    // Pop left result (it was true, so continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right
    let right_info = compiler.check(right, &DataType::simple(primitives::BOOL), span)?;

    // Patch jump to here (result is already on stack - either false from left or result from right)
    compiler.emitter().patch_jump(skip_right);

    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}

/// Compile || with short-circuit evaluation.
fn compile_or(
    compiler: &mut ExprCompiler,
    left: &Expr,
    right: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Compile left
    let left_info = compiler.check(left, &DataType::simple(primitives::BOOL), span)?;

    // If true, skip right (result is true)
    let skip_right = compiler.emitter().emit_jump(OpCode::JumpIfTrue);

    // Pop left result (it was false, so continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right
    let right_info = compiler.check(right, &DataType::simple(primitives::BOOL), span)?;

    // Patch jump
    compiler.emitter().patch_jump(skip_right);

    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}
```

### Unary Operators (expr/unary.rs)

```rust
use angelscript_core::{primitives, DataType, Span};
use angelscript_parser::ast::UnaryOp;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use super::ExprCompiler;

pub fn compile_unary(
    compiler: &mut ExprCompiler,
    op: UnaryOp,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    match op {
        UnaryOp::Neg => compile_negation(compiler, operand, span),
        UnaryOp::Not => compile_logical_not(compiler, operand, span),
        UnaryOp::BitNot => compile_bitwise_not(compiler, operand, span),
        UnaryOp::PreInc | UnaryOp::PreDec => compile_pre_inc_dec(compiler, op, operand, span),
        UnaryOp::PostInc | UnaryOp::PostDec => compile_post_inc_dec(compiler, op, operand, span),
    }
}

fn compile_negation(
    compiler: &mut ExprCompiler,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    let info = compiler.infer(operand, span)?;

    let opcode = match info.data_type.type_hash {
        h if h == primitives::INT32 => OpCode::NegI32,
        h if h == primitives::INT64 => OpCode::NegI64,
        h if h == primitives::FLOAT => OpCode::NegF32,
        h if h == primitives::DOUBLE => OpCode::NegF64,
        _ => {
            // Try opNeg method
            return compile_unary_method(compiler, "opNeg", &info, span);
        }
    };

    compiler.emitter().emit(opcode);
    Ok(ExprInfo::rvalue(info.data_type))
}

fn compile_logical_not(
    compiler: &mut ExprCompiler,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    let info = compiler.check(operand, &DataType::simple(primitives::BOOL), span)?;
    compiler.emitter().emit(OpCode::Not);
    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}

fn compile_bitwise_not(
    compiler: &mut ExprCompiler,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    let info = compiler.infer(operand, span)?;

    // Must be integer type
    if !is_integer_type(info.data_type.type_hash) {
        return Err(CompileError::NoOperator {
            op: "~".to_string(),
            left: format!("{:?}", info.data_type.type_hash),
            right: "".to_string(),
            span,
        });
    }

    compiler.emitter().emit(OpCode::BitNot);
    Ok(ExprInfo::rvalue(info.data_type))
}

fn compile_pre_inc_dec(
    compiler: &mut ExprCompiler,
    op: UnaryOp,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    let info = compiler.infer(operand, span)?;

    if !info.is_assignable() {
        return Err(CompileError::NotAssignable {
            reason: "increment/decrement requires lvalue".to_string(),
            span,
        });
    }

    // Load, increment/decrement, store, result is new value
    // Implementation depends on operand type (local, field, etc.)

    Ok(ExprInfo::rvalue(info.data_type))
}

fn compile_post_inc_dec(
    compiler: &mut ExprCompiler,
    op: UnaryOp,
    operand: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    let info = compiler.infer(operand, span)?;

    if !info.is_assignable() {
        return Err(CompileError::NotAssignable {
            reason: "increment/decrement requires lvalue".to_string(),
            span,
        });
    }

    // Load, dup, increment/decrement, store, result is old value (from dup)

    Ok(ExprInfo::rvalue(info.data_type))
}

fn compile_unary_method(
    compiler: &mut ExprCompiler,
    method_name: &str,
    operand_info: &ExprInfo,
    span: Span,
) -> Result<ExprInfo> {
    let methods = compiler.ctx().find_methods(operand_info.data_type.type_hash, method_name);

    if methods.is_empty() {
        return Err(CompileError::NoOperator {
            op: method_name.to_string(),
            left: format!("{:?}", operand_info.data_type.type_hash),
            right: "".to_string(),
            span,
        });
    }

    let method_hash = methods[0];  // Should do proper overload resolution
    let func = compiler.ctx().get_function(method_hash).unwrap();
    let return_type = func.def().return_type;

    compiler.emitter().emit_call_method(method_hash, 0);
    Ok(ExprInfo::rvalue(return_type))
}

fn is_integer_type(hash: TypeHash) -> bool {
    use primitives::*;
    matches!(hash, INT8 | INT16 | INT32 | INT64 | UINT8 | UINT16 | UINT32 | UINT64)
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_int_literal() {
        let mut emitter = BytecodeEmitter::new();
        let mut scope = LocalScope::new();
        let mut compiler = ExprCompiler::new(&ctx, &mut scope, &mut emitter, None);

        let info = compiler.infer(&Expr::IntLiteral(42), Span::default());
        assert!(info.is_ok());
        assert_eq!(info.unwrap().data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn compile_variable_access() {
        // Declare variable x, then access it
    }

    #[test]
    fn compile_binary_add() {
        // 1 + 2 should produce int
    }

    #[test]
    fn compile_short_circuit_and() {
        // false && x should not evaluate x
    }
}
```

## Acceptance Criteria

- [ ] Integer/float/string/bool/null literals compile correctly
- [ ] Variable access emits GetLocal with correct slot
- [ ] Global access works
- [ ] 'this' access works in methods
- [ ] Arithmetic operators resolve and emit correct opcodes
- [ ] Comparison operators produce bool result
- [ ] Logical && and || have short-circuit evaluation
- [ ] Unary operators (-, !, ~) work
- [ ] Pre/post increment/decrement work on lvalues
- [ ] User-defined operators call methods
- [ ] All tests pass

## Next Phase

Task 40: Expression Compilation - Calls (function calls, method calls, constructors)
