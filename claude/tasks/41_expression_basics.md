# Task 41: Expression Compilation - Basics

## Overview

Implement basic expression compilation: literals, identifiers, unary operators, and binary operators. This forms the foundation of the expression compiler.

## Goals

1. Compile literals (int, float, string, bool, null)
2. Compile identifier access (variables, globals)
3. Compile unary operators (-, +, !, ~, ++, --, @)
4. Compile binary operators (+, -, *, /, %, etc.)
5. Implement bidirectional type checking (infer and check modes)
6. Create operator resolution module for primitive and user-defined operators

## Dependencies

- Task 32: String Factory Configuration (for string literals) - uses `Context::string_factory()`
- Task 33: Compilation Context
- Task 35: Conversion System
- Task 36: Overload Resolution (for operator method selection)
- Task 38: Local Scope
- Task 40: Bytecode Emitter

## Files to Create

```
crates/angelscript-compiler/src/
├── expr/
│   ├── mod.rs             # ExprCompiler struct
│   ├── literals.rs        # Literal compilation
│   ├── identifiers.rs     # Variable/global access
│   ├── unary.rs           # Unary operators
│   └── binary.rs          # Binary operators
├── operators/
│   ├── mod.rs             # Public API: resolve_binary(), resolve_unary()
│   ├── primitive.rs       # Primitive type operator tables
│   ├── binary.rs          # Binary operator resolution logic
│   └── unary.rs           # Unary operator resolution logic
└── lib.rs                 # Export new modules
```

## Important Design Notes

### String Literals

String literals use `Context::string_factory()` to determine:
1. What type the literal produces (e.g., `string`)
2. What factory function to call with the raw string data

The `StringFactory` trait is defined in `angelscript_core::string_factory` and accessed via `Context::string_factory()` in `src/context.rs`. The `CompilationContext` will need access to this info (passed during construction or via a reference to `Context`).

### AST Types

Use the actual parser AST types from `angelscript_parser::ast::expr`:
- `Expr::Literal(LiteralExpr { kind: LiteralKind::Int(i64), span })`
- `Expr::Binary(&BinaryExpr { left, op, right, span })`
- `Expr::Ident(IdentExpr { scope, ident, type_args, span })`
- etc.

### Error Types

Use `CompilationError` from `angelscript_core`, not a local error type.

---

## Operator Resolution Design

### Overview

The `operators/` module determines how to compile an operator given operand types:
1. **Primitive operations** - direct opcodes (AddI32, MulF64, etc.)
2. **User-defined operators** - method calls (opAdd, opEquals, etc.)

### Key Types

```rust
/// Result of operator resolution
pub enum OperatorResolution {
    /// Use a primitive opcode
    Primitive {
        opcode: OpCode,
        /// Conversion for left operand (if needed)
        left_conv: Option<OpCode>,
        /// Conversion for right operand (if needed)
        right_conv: Option<OpCode>,
        /// Result type
        result_type: DataType,
    },
    /// Call a method on the left operand (e.g., left.opAdd(right))
    MethodOnLeft {
        method_hash: TypeHash,
        arg_conversion: Option<Conversion>,
        result_type: DataType,
    },
    /// Call a reverse method on right operand (e.g., right.opAdd_r(left))
    MethodOnRight {
        method_hash: TypeHash,
        arg_conversion: Option<Conversion>,
        result_type: DataType,
    },
    /// Pointer/handle comparison (for is/!is default)
    HandleComparison {
        negate: bool,  // true for !is
    },
}
```

### Binary Operator Resolution Algorithm

For arithmetic/bitwise operators (`+`, `-`, `*`, `/`, `%`, `**`, `&`, `|`, `^`, `<<`, `>>`, `>>>`):

1. **Check if both types are primitive numeric**
   - Same type → direct opcode
   - Different types → promote to common type, then direct opcode
2. **Check left type for operator behavior** (e.g., `OpAdd`)
3. **Check right type for reverse operator** (e.g., `OpAddR`)
4. **Error if no resolution found**

For comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`):

1. **Check primitives** → direct comparison opcodes
2. **For `==`/`!=`**: Try `opEquals` on left, then right
3. **Fall back to `opCmp`**: Try on left, then right; compare result to 0
4. **Fall back**: Try implicit conversion to numeric, then primitive compare
5. **Error if no resolution found**

For identity operators (`is`, `!is`):

1. **Try `opEquals` with handle parameter** on left, then right
2. **Fall back to pointer comparison** (`EqHandle` opcode) - this is the default
3. **Error if operands are not handle types**

### Logical Operators (`&&`, `||`, `^^`)

**NOT overloadable** - handled directly in `expr/binary.rs` with short-circuit evaluation:

```rust
// && short-circuit:
// 1. Compile left
// 2. JumpIfFalse to end
// 3. Pop (left was true)
// 4. Compile right
// 5. end: (result on stack)

// || short-circuit:
// 1. Compile left
// 2. JumpIfTrue to end
// 3. Pop (left was false)
// 4. Compile right
// 5. end: (result on stack)
```

### Mapping BinaryOp to OperatorBehavior

| BinaryOp | Primary Behavior | Reverse Behavior |
|----------|------------------|------------------|
| `Add` | `OpAdd` | `OpAddR` |
| `Sub` | `OpSub` | `OpSubR` |
| `Mul` | `OpMul` | `OpMulR` |
| `Div` | `OpDiv` | `OpDivR` |
| `Mod` | `OpMod` | `OpModR` |
| `Pow` | `OpPow` | `OpPowR` |
| `BitwiseAnd` | `OpAnd` | `OpAndR` |
| `BitwiseOr` | `OpOr` | `OpOrR` |
| `BitwiseXor` | `OpXor` | `OpXorR` |
| `ShiftLeft` | `OpShl` | `OpShlR` |
| `ShiftRight` | `OpShr` | `OpShrR` |
| `ShiftRightUnsigned` | `OpUShr` | `OpUShrR` |
| `Equal`, `NotEqual` | `OpEquals` | - |
| `Less`, `LessEqual`, `Greater`, `GreaterEqual` | `OpCmp` | - |
| `Is`, `NotIs` | `OpEquals` (handle param) | - |

### Unary Operator Resolution

| UnaryOp | Primitive Opcode | Behavior |
|---------|------------------|----------|
| `Neg` | `NegI32`/`NegI64`/`NegF32`/`NegF64` | `OpNeg` |
| `Plus` | (no-op for primitives) | - |
| `LogicalNot` | `Not` (bool only) | - |
| `BitwiseNot` | `BitNot` (integers only) | `OpCom` |
| `PreInc` | (special handling) | `OpPreInc` |
| `PreDec` | (special handling) | `OpPreDec` |
| `HandleOf` | (special handling) | - |

For postfix operators (`PostfixOp::PostInc`, `PostfixOp::PostDec`):
- Behaviors: `OpPostInc`, `OpPostDec`

---

## Detailed Implementation

### ExprCompiler (expr/mod.rs)

```rust
use angelscript_core::{CompilationError, DataType, Span, TypeHash};
use angelscript_parser::ast::{Expr, LiteralKind};

use crate::context::CompilationContext;
use crate::conversion::find_conversion;
use crate::emit::BytecodeEmitter;
use crate::expr_info::ExprInfo;
use crate::scope::LocalScope;

mod binary;
mod identifiers;
mod literals;
mod unary;

type Result<T> = std::result::Result<T, CompilationError>;

/// Compiles expressions using bidirectional type checking.
pub struct ExprCompiler<'a, 'ctx, 'pool> {
    ctx: &'a CompilationContext<'ctx>,
    scope: &'a mut LocalScope,
    emitter: &'a mut BytecodeEmitter<'pool>,
    /// Current class type (for 'this' and method access)
    current_class: Option<TypeHash>,
}

impl<'a, 'ctx, 'pool> ExprCompiler<'a, 'ctx, 'pool> {
    pub fn new(
        ctx: &'a CompilationContext<'ctx>,
        scope: &'a mut LocalScope,
        emitter: &'a mut BytecodeEmitter<'pool>,
        current_class: Option<TypeHash>,
    ) -> Self {
        Self { ctx, scope, emitter, current_class }
    }

    /// Synthesize type from expression (infer mode).
    pub fn infer<'ast>(&mut self, expr: &Expr<'ast>) -> Result<ExprInfo> {
        let span = expr.span();
        match expr {
            Expr::Literal(lit) => literals::compile_literal(self, &lit.kind, span),
            Expr::Ident(ident) => identifiers::compile_ident(self, ident, span),
            Expr::Binary(bin) => binary::compile_binary(self, bin),
            Expr::Unary(un) => unary::compile_unary(self, un),
            Expr::Postfix(post) => unary::compile_postfix(self, post),
            Expr::Paren(p) => self.infer(p.expr),

            // Handled in later tasks
            Expr::Call(_) => todo!("Task 42: Call expressions"),
            Expr::Member(_) => todo!("Task 42: Member access"),
            Expr::Index(_) => todo!("Task 42: Index expressions"),
            Expr::Assign(_) => todo!("Task 42: Assignment"),
            Expr::Ternary(_) => todo!("Task 42: Ternary"),
            Expr::Cast(_) => todo!("Task 42: Cast"),
            Expr::Lambda(_) => todo!("Task 43: Lambda"),
            Expr::InitList(_) => todo!("Task 43: Init list"),
        }
    }

    /// Check expression against expected type (check mode).
    pub fn check<'ast>(&mut self, expr: &Expr<'ast>, expected: &DataType) -> Result<ExprInfo> {
        let info = self.infer(expr)?;

        if info.data_type.type_hash == expected.type_hash {
            return Ok(info);
        }

        // Try implicit conversion
        if let Some(conv) = find_conversion(&info.data_type, expected, self.ctx) {
            if conv.is_implicit {
                self.emit_conversion(&info.data_type, expected)?;
                return Ok(ExprInfo::rvalue(*expected));
            }
        }

        Err(CompilationError::TypeMismatch {
            expected: self.type_name(expected.type_hash),
            actual: self.type_name(info.data_type.type_hash),
            span: expr.span(),
        })
    }

    fn emit_conversion(&mut self, from: &DataType, to: &DataType) -> Result<()> {
        // Use conversion module to get opcode
        if let Some(opcode) = crate::conversion::get_conversion_opcode(from.type_hash, to.type_hash) {
            self.emitter.emit(opcode);
        }
        Ok(())
    }

    fn type_name(&self, hash: TypeHash) -> String {
        self.ctx.get_type(hash)
            .map(|e| e.qualified_name().to_string())
            .unwrap_or_else(|| format!("{:?}", hash))
    }

    // Accessors
    pub fn ctx(&self) -> &CompilationContext<'ctx> { self.ctx }
    pub fn scope(&self) -> &LocalScope { self.scope }
    pub fn scope_mut(&mut self) -> &mut LocalScope { self.scope }
    pub fn emitter(&mut self) -> &mut BytecodeEmitter<'pool> { self.emitter }
    pub fn current_class(&self) -> Option<TypeHash> { self.current_class }
}
```

### Literals (expr/literals.rs)

```rust
use angelscript_core::{primitives, CompilationError, DataType, Span};
use angelscript_parser::ast::LiteralKind;

use crate::expr_info::ExprInfo;
use super::{ExprCompiler, Result};

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
    // Determine smallest type that fits
    let (type_hash, emit_fn) = if value >= i32::MIN as i64 && value <= i32::MAX as i64 {
        (primitives::INT32, |e: &mut _, v| e.emit_i32(v as i32))
    } else {
        (primitives::INT64, |e: &mut _, v| e.emit_i64(v))
    };

    emit_fn(compiler.emitter(), value);
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
    if value {
        compiler.emitter().emit_true();
    } else {
        compiler.emitter().emit_false();
    }
    Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
}

fn compile_string(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    bytes: &[u8],
    span: Span,
) -> Result<ExprInfo> {
    // Get string factory from context
    // Note: CompilationContext needs access to string factory info
    let factory = compiler.ctx().string_factory()
        .ok_or_else(|| CompilationError::NoStringFactory { span })?;

    // Add string constant to pool
    let const_idx = compiler.emitter().add_string_constant(bytes);
    compiler.emitter().emit_constant(const_idx);

    // Call factory function
    compiler.emitter().emit_call(factory.factory_func, 1);

    Ok(ExprInfo::rvalue(DataType::simple(factory.type_hash)))
}

fn compile_null(compiler: &mut ExprCompiler<'_, '_, '_>) -> Result<ExprInfo> {
    compiler.emitter().emit_null();
    Ok(ExprInfo::rvalue(DataType::null_handle()))
}
```

### Binary Operators (expr/binary.rs)

```rust
use angelscript_core::{primitives, CompilationError, DataType};
use angelscript_parser::ast::{BinaryExpr, BinaryOp};

use crate::bytecode::OpCode;
use crate::expr_info::ExprInfo;
use crate::operators::{resolve_binary, OperatorResolution};
use super::{ExprCompiler, Result};

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

    // Compile operands
    let left_info = compiler.infer(expr.left)?;
    let right_info = compiler.infer(expr.right)?;

    // Resolve operator
    let resolution = resolve_binary(
        &left_info.data_type,
        &right_info.data_type,
        expr.op,
        compiler.ctx(),
        expr.span,
    )?;

    emit_resolution(compiler, resolution, expr.op)
}

fn emit_resolution(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    resolution: OperatorResolution,
    op: BinaryOp,
) -> Result<ExprInfo> {
    match resolution {
        OperatorResolution::Primitive { opcode, left_conv, right_conv, result_type } => {
            // Note: operands already on stack from infer() calls
            // Conversions would need to be emitted before the operands...
            // This needs refinement - may need to track stack state
            compiler.emitter().emit(opcode);
            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::MethodOnLeft { method_hash, arg_conversion, result_type } => {
            if let Some(conv) = arg_conversion {
                // Emit conversion for right operand
            }
            compiler.emitter().emit_call_method(method_hash, 1);

            // Negate for != and !is
            if matches!(op, BinaryOp::NotEqual | BinaryOp::NotIs) {
                compiler.emitter().emit(OpCode::Not);
            }

            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::MethodOnRight { method_hash, arg_conversion, result_type } => {
            // Swap operands for reverse operator
            compiler.emitter().emit(OpCode::Swap);
            if let Some(conv) = arg_conversion {
                // Emit conversion for left operand (now on top)
            }
            compiler.emitter().emit_call_method(method_hash, 1);

            if matches!(op, BinaryOp::NotEqual | BinaryOp::NotIs) {
                compiler.emitter().emit(OpCode::Not);
            }

            Ok(ExprInfo::rvalue(result_type))
        }
        OperatorResolution::HandleComparison { negate } => {
            compiler.emitter().emit(OpCode::EqHandle);
            if negate {
                compiler.emitter().emit(OpCode::Not);
            }
            Ok(ExprInfo::rvalue(DataType::simple(primitives::BOOL)))
        }
    }
}

/// Compile && with short-circuit evaluation
fn compile_logical_and<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    // Compile left, coerce to bool
    compiler.check(expr.left, &bool_type)?;

    // If false, skip right (result is false)
    let jump_to_end = compiler.emitter().emit_jump_placeholder(OpCode::JumpIfFalse);

    // Pop left result (was true, continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right, coerce to bool
    compiler.check(expr.right, &bool_type)?;

    // Patch jump to here
    compiler.emitter().patch_jump(jump_to_end);

    Ok(ExprInfo::rvalue(bool_type))
}

/// Compile || with short-circuit evaluation
fn compile_logical_or<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    // Compile left, coerce to bool
    compiler.check(expr.left, &bool_type)?;

    // If true, skip right (result is true)
    let jump_to_end = compiler.emitter().emit_jump_placeholder(OpCode::JumpIfTrue);

    // Pop left result (was false, continue)
    compiler.emitter().emit(OpCode::Pop);

    // Compile right, coerce to bool
    compiler.check(expr.right, &bool_type)?;

    // Patch jump
    compiler.emitter().patch_jump(jump_to_end);

    Ok(ExprInfo::rvalue(bool_type))
}

/// Compile ^^ (logical XOR) - no short circuit, both sides always evaluated
fn compile_logical_xor<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    expr: &BinaryExpr<'ast>,
) -> Result<ExprInfo> {
    let bool_type = DataType::simple(primitives::BOOL);

    compiler.check(expr.left, &bool_type)?;
    compiler.check(expr.right, &bool_type)?;

    // XOR is: (a || b) && !(a && b)
    // But simpler: a != b for booleans
    compiler.emitter().emit(OpCode::EqBool);
    compiler.emitter().emit(OpCode::Not);

    Ok(ExprInfo::rvalue(bool_type))
}
```

### Operator Resolution (operators/mod.rs)

```rust
//! Operator resolution for expression compilation.

mod binary;
mod primitive;
mod unary;

pub use binary::resolve_binary;
pub use unary::resolve_unary;

use angelscript_core::{DataType, TypeHash};
use crate::bytecode::OpCode;
use crate::conversion::Conversion;

/// Result of operator resolution
#[derive(Debug, Clone)]
pub enum OperatorResolution {
    /// Use a primitive opcode
    Primitive {
        opcode: OpCode,
        left_conv: Option<OpCode>,
        right_conv: Option<OpCode>,
        result_type: DataType,
    },
    /// Call method on left operand
    MethodOnLeft {
        method_hash: TypeHash,
        arg_conversion: Option<Conversion>,
        result_type: DataType,
    },
    /// Call reverse method on right operand
    MethodOnRight {
        method_hash: TypeHash,
        arg_conversion: Option<Conversion>,
        result_type: DataType,
    },
    /// Handle/pointer comparison (default for is/!is)
    HandleComparison {
        negate: bool,
    },
}
```

### Operator Resolution - Binary (operators/binary.rs)

```rust
use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, primitives};
use angelscript_parser::ast::BinaryOp;

use crate::context::CompilationContext;
use super::{OperatorResolution, primitive};

pub fn resolve_binary(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try primitive operation first
    if let Some(resolution) = primitive::try_primitive_binary(left, right, op) {
        return Ok(resolution);
    }

    // Handle identity operators specially
    if matches!(op, BinaryOp::Is | BinaryOp::NotIs) {
        return resolve_identity(left, right, op, ctx, span);
    }

    // Handle comparison operators
    if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
        return resolve_equality(left, right, op, ctx, span);
    }

    if matches!(op, BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual) {
        return resolve_comparison(left, right, op, ctx, span);
    }

    // Try operator method on left type
    let (primary, reverse) = op_to_behavior(op);

    if let Some(behavior) = primary {
        if let Some(resolution) = try_operator_method(left, right, behavior, true, ctx) {
            return Ok(resolution);
        }
    }

    // Try reverse operator on right type
    if let Some(behavior) = reverse {
        if let Some(resolution) = try_operator_method(right, left, behavior, false, ctx) {
            return Ok(resolution);
        }
    }

    Err(CompilationError::NoMatchingOperator {
        op: format!("{:?}", op),
        left_type: ctx.type_name(left.type_hash),
        right_type: ctx.type_name(right.type_hash),
        span,
    })
}

fn resolve_identity(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Must be handle types
    if !left.is_handle() && !left.is_null_handle() {
        return Err(CompilationError::OperatorRequiresHandle {
            op: "is".to_string(),
            span,
        });
    }

    // Try opEquals with handle parameter
    if let Some(resolution) = try_opequals_handle(left, right, ctx) {
        return Ok(resolution);
    }
    if let Some(resolution) = try_opequals_handle(right, left, ctx) {
        // Reverse - swap args back
        return Ok(resolution);
    }

    // Default: pointer comparison
    Ok(OperatorResolution::HandleComparison {
        negate: op == BinaryOp::NotIs,
    })
}

fn resolve_equality(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try opEquals on left
    if let Some(resolution) = try_operator_method(left, right, OperatorBehavior::OpEquals, true, ctx) {
        return Ok(resolution);
    }
    // Try opEquals on right
    if let Some(resolution) = try_operator_method(right, left, OperatorBehavior::OpEquals, false, ctx) {
        return Ok(resolution);
    }

    // Fall back to opCmp
    if let Some(resolution) = try_opcmp(left, right, op, ctx) {
        return Ok(resolution);
    }

    // Fall back: try implicit conversion to numeric
    if let Some(resolution) = try_numeric_fallback(left, right, op, ctx) {
        return Ok(resolution);
    }

    Err(CompilationError::NoMatchingOperator {
        op: format!("{:?}", op),
        left_type: ctx.type_name(left.type_hash),
        right_type: ctx.type_name(right.type_hash),
        span,
    })
}

fn resolve_comparison(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OperatorResolution, CompilationError> {
    // Try opCmp
    if let Some(resolution) = try_opcmp(left, right, op, ctx) {
        return Ok(resolution);
    }

    // Fall back: try implicit conversion to numeric
    if let Some(resolution) = try_numeric_fallback(left, right, op, ctx) {
        return Ok(resolution);
    }

    Err(CompilationError::NoMatchingOperator {
        op: format!("{:?}", op),
        left_type: ctx.type_name(left.type_hash),
        right_type: ctx.type_name(right.type_hash),
        span,
    })
}

/// Map BinaryOp to (primary behavior, reverse behavior)
fn op_to_behavior(op: BinaryOp) -> (Option<OperatorBehavior>, Option<OperatorBehavior>) {
    use OperatorBehavior::*;
    match op {
        BinaryOp::Add => (Some(OpAdd), Some(OpAddR)),
        BinaryOp::Sub => (Some(OpSub), Some(OpSubR)),
        BinaryOp::Mul => (Some(OpMul), Some(OpMulR)),
        BinaryOp::Div => (Some(OpDiv), Some(OpDivR)),
        BinaryOp::Mod => (Some(OpMod), Some(OpModR)),
        BinaryOp::Pow => (Some(OpPow), Some(OpPowR)),
        BinaryOp::BitwiseAnd => (Some(OpAnd), Some(OpAndR)),
        BinaryOp::BitwiseOr => (Some(OpOr), Some(OpOrR)),
        BinaryOp::BitwiseXor => (Some(OpXor), Some(OpXorR)),
        BinaryOp::ShiftLeft => (Some(OpShl), Some(OpShlR)),
        BinaryOp::ShiftRight => (Some(OpShr), Some(OpShrR)),
        BinaryOp::ShiftRightUnsigned => (Some(OpUShr), Some(OpUShrR)),
        _ => (None, None),
    }
}

fn try_operator_method(
    obj_type: &DataType,
    arg_type: &DataType,
    behavior: OperatorBehavior,
    on_left: bool,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    let type_entry = ctx.get_type(obj_type.type_hash)?;
    let class = type_entry.as_class()?;
    let methods = class.behaviors.get_operator(behavior)?;

    // Use overload resolution to find best match
    // (simplified - should use full overload resolution)
    for &method_hash in methods {
        let func = ctx.get_function(method_hash)?;
        // Check if arg_type matches parameter
        // ... overload resolution logic ...

        return Some(if on_left {
            OperatorResolution::MethodOnLeft {
                method_hash,
                arg_conversion: None,
                result_type: func.def.return_type,
            }
        } else {
            OperatorResolution::MethodOnRight {
                method_hash,
                arg_conversion: None,
                result_type: func.def.return_type,
            }
        });
    }

    None
}

fn try_opequals_handle(
    obj_type: &DataType,
    arg_type: &DataType,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    // Look for opEquals that takes a handle parameter
    // ... implementation ...
    None
}

fn try_opcmp(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    // Look for opCmp, then emit comparison to 0
    // ... implementation ...
    None
}

fn try_numeric_fallback(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    // Try opImplConv to numeric type, then use primitive comparison
    // ... implementation ...
    None
}
```

---

## Sub-Tasks

### Sub-Task 1: Operators Module (Pending)
Create `operators/` module with resolution logic.

**Files:**
- `crates/angelscript-compiler/src/operators/mod.rs`
- `crates/angelscript-compiler/src/operators/primitive.rs`
- `crates/angelscript-compiler/src/operators/binary.rs`
- `crates/angelscript-compiler/src/operators/unary.rs`

### Sub-Task 2: Expression Compiler Core (Pending)
Create `expr/mod.rs` with `ExprCompiler` struct.

**Files:**
- `crates/angelscript-compiler/src/expr/mod.rs`

### Sub-Task 3: Literals (Pending)
Implement literal compilation.

**Files:**
- `crates/angelscript-compiler/src/expr/literals.rs`

### Sub-Task 4: Identifiers (Pending)
Implement identifier/variable access.

**Files:**
- `crates/angelscript-compiler/src/expr/identifiers.rs`

### Sub-Task 5: Unary Operators (Pending)
Implement unary operator compilation.

**Files:**
- `crates/angelscript-compiler/src/expr/unary.rs`

### Sub-Task 6: Binary Operators (Pending)
Implement binary operator compilation.

**Files:**
- `crates/angelscript-compiler/src/expr/binary.rs`

### Sub-Task 7: Integration & Testing (Pending)
Wire up modules, add to lib.rs, write tests.

---

## Acceptance Criteria

- [ ] Integer/float/string/bool/null literals compile correctly
- [ ] Variable access emits GetLocal with correct slot
- [ ] Global access works
- [ ] 'this' access works in methods
- [ ] Primitive arithmetic operators emit correct opcodes
- [ ] Primitive comparison operators produce bool result
- [ ] Logical `&&` and `||` have short-circuit evaluation
- [ ] Logical `^^` evaluates both sides
- [ ] `is`/`!is` falls back to pointer comparison
- [ ] `==`/`!=` falls back to `opCmp` then numeric conversion
- [ ] Unary operators (-, +, !, ~) work
- [ ] Pre/post increment/decrement work on lvalues
- [ ] User-defined operators call correct methods
- [ ] All tests pass

## Next Task

Task 42: Expression Compilation - Calls (function calls, method calls, constructors, member access, indexing)
