# Task 42: Expression Compilation - Advanced

## Overview

Implement advanced expression compilation: cast expressions, lambda expressions, init lists, and ternary operator.

## Goals

1. Compile explicit cast expressions
2. Compile lambda expressions with captures
3. Compile init lists for arrays/containers
4. Compile ternary operator (?:)

## Dependencies

- Task 37: Local Scope (for lambda captures)
- Task 39: Expression Basics
- Task 40: Expression Calls

## Files to Create/Modify

```
crates/angelscript-compiler/src/expr/
├── cast.rs                # Cast expressions
├── lambda.rs              # Lambda expressions
├── init_list.rs           # Init list expressions
├── ternary.rs             # Ternary operator
└── mod.rs                 # Add modules
```

## Detailed Implementation

### Cast Expressions (expr/cast.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::{Expr, TypeExpr};

use crate::bytecode::OpCode;
use crate::conversion::{find_conversion, ConversionKind};
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use super::ExprCompiler;

/// Compile a cast expression: cast<Type>(expr)
pub fn compile_cast(
    compiler: &mut ExprCompiler,
    target_type: &TypeExpr,
    value: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Resolve target type
    let target = compiler.resolve_type(target_type, span)?;

    // Compile value expression
    let value_info = compiler.infer(value, span)?;

    // Check if cast is valid
    if let Some(conv) = find_conversion(&value_info.data_type, &target, compiler.ctx()) {
        emit_cast_conversion(compiler, &conv, &value_info.data_type, &target)?;
        return Ok(ExprInfo::rvalue(target));
    }

    // Special case: handle cast (downcasting)
    if value_info.data_type.is_handle && target.is_handle {
        return compile_handle_cast(compiler, &value_info.data_type, &target, span);
    }

    Err(CompileError::ConversionError {
        from: format!("{:?}", value_info.data_type.type_hash),
        to: format!("{:?}", target.type_hash),
        span,
    })
}

/// Emit the appropriate conversion instructions.
fn emit_cast_conversion(
    compiler: &mut ExprCompiler,
    conv: &crate::conversion::Conversion,
    from: &DataType,
    to: &DataType,
) -> Result<()> {
    match &conv.kind {
        ConversionKind::Identity => {
            // No conversion needed
        }

        ConversionKind::Primitive { from: f, to: t } => {
            if let Some(opcode) = crate::conversion::primitive::get_conversion_opcode(*f, *t) {
                compiler.emitter().emit(opcode);
            }
        }

        ConversionKind::HandleToConst => {
            compiler.emitter().emit(OpCode::HandleToConst);
        }

        ConversionKind::DerivedToBase { base } => {
            compiler.emitter().emit(OpCode::DerivedToBase);
        }

        ConversionKind::ClassToInterface { interface } => {
            compiler.emitter().emit(OpCode::ClassToInterface);
        }

        ConversionKind::ExplicitCastMethod { method } => {
            compiler.emitter().emit_call_method(*method, 0);
        }

        ConversionKind::ValueToHandle => {
            compiler.emitter().emit(OpCode::ValueToHandle);
        }

        _ => {
            // Other conversions
        }
    }

    Ok(())
}

/// Compile a handle downcast.
fn compile_handle_cast(
    compiler: &mut ExprCompiler,
    from: &DataType,
    to: &DataType,
    span: Span,
) -> Result<ExprInfo> {
    // Runtime type check and cast
    compiler.emitter().emit_cast(to.type_hash);
    Ok(ExprInfo::rvalue(*to))
}
```

### Lambda Expressions (expr/lambda.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::{Expr, LambdaExpr, ParamDecl};

use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::scope::LocalScope;
use super::ExprCompiler;

/// Compile a lambda expression.
pub fn compile_lambda(
    compiler: &mut ExprCompiler,
    lambda: &LambdaExpr,
    expected_type: Option<&DataType>,
    span: Span,
) -> Result<ExprInfo> {
    // Get expected funcdef type (for parameter type inference)
    let funcdef = expected_type
        .and_then(|t| compiler.ctx().get_type(t.type_hash))
        .and_then(|t| t.as_funcdef());

    // Resolve/infer parameter types
    let params = resolve_lambda_params(compiler, &lambda.params, funcdef, span)?;

    // Resolve return type (if specified, or infer from body)
    let return_type = if let Some(ret) = &lambda.return_type {
        compiler.resolve_type(ret, span)?
    } else if let Some(fd) = funcdef {
        fd.return_type
    } else {
        // Infer from body - requires type inference
        DataType::void()  // Placeholder
    };

    // Create nested scope for lambda body
    let parent_scope = std::mem::replace(compiler.scope_mut(), LocalScope::new());
    let mut lambda_scope = LocalScope::nested(parent_scope);

    // Declare parameters in lambda scope
    for (i, param) in params.iter().enumerate() {
        lambda_scope.declare_param(
            param.name.clone(),
            param.data_type,
            param.is_const,
            span,
        )?;
    }

    // Compile lambda body
    // This would create a separate function and capture variables
    let captures = lambda_scope.captures().to_vec();

    // Restore parent scope
    let parent = lambda_scope.take_parent().unwrap();
    *compiler.scope_mut() = parent;

    // Emit lambda creation
    // This involves:
    // 1. Creating a function entry for the lambda
    // 2. Emitting capture loads
    // 3. Creating closure object

    // Determine funcdef type for the lambda
    let lambda_type = create_lambda_funcdef_type(compiler, &params, &return_type)?;

    Ok(ExprInfo::rvalue(DataType::simple(lambda_type)))
}

/// Resolve lambda parameters, using expected types where available.
fn resolve_lambda_params(
    compiler: &mut ExprCompiler,
    params: &[ParamDecl],
    funcdef: Option<&FuncdefEntry>,
    span: Span,
) -> Result<Vec<LambdaParam>> {
    let mut result = Vec::new();

    for (i, param) in params.iter().enumerate() {
        let data_type = if let Some(type_expr) = &param.type_expr {
            // Explicit type
            compiler.resolve_type(type_expr, span)?
        } else if let Some(fd) = funcdef {
            // Infer from expected funcdef
            if i < fd.params.len() {
                fd.params[i]
            } else {
                return Err(CompileError::TypeMismatch {
                    expected: "typed parameter".to_string(),
                    got: "untyped parameter without inference context".to_string(),
                    span,
                });
            }
        } else {
            return Err(CompileError::TypeMismatch {
                expected: "typed parameter".to_string(),
                got: "cannot infer lambda parameter type".to_string(),
                span,
            });
        };

        result.push(LambdaParam {
            name: param.name.to_string(),
            data_type,
            is_const: param.is_const,
        });
    }

    Ok(result)
}

struct LambdaParam {
    name: String,
    data_type: DataType,
    is_const: bool,
}

fn create_lambda_funcdef_type(
    compiler: &mut ExprCompiler,
    params: &[LambdaParam],
    return_type: &DataType,
) -> Result<TypeHash> {
    // Create or find matching funcdef type
    // This may need to register a new anonymous funcdef
    todo!("Create lambda funcdef type")
}
```

### Init Lists (expr/init_list.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use super::ExprCompiler;

/// Compile an init list expression: {1, 2, 3}
pub fn compile_init_list(
    compiler: &mut ExprCompiler,
    elements: &[Expr],
    expected_type: Option<&DataType>,
    span: Span,
) -> Result<ExprInfo> {
    // Need expected type to know what container to create
    let target_type = expected_type.ok_or_else(|| CompileError::TypeMismatch {
        expected: "container type".to_string(),
        got: "cannot infer init list type".to_string(),
        span,
    })?;

    let class = compiler.ctx().get_type(target_type.type_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeMismatch {
            expected: "container class".to_string(),
            got: format!("{:?}", target_type.type_hash),
            span,
        })?;

    // Find list factory or constructor
    // AngelScript uses list factory behavior for init lists
    if let Some(list_factory) = class.behaviors.list_factory {
        return compile_list_factory(compiler, list_factory, elements, target_type, span);
    }

    // Fallback: create empty container and use opIndex or add methods
    compile_init_list_manual(compiler, elements, target_type, span)
}

/// Compile using list factory (opListConstruct).
fn compile_list_factory(
    compiler: &mut ExprCompiler,
    factory: TypeHash,
    elements: &[Expr],
    target_type: &DataType,
    span: Span,
) -> Result<ExprInfo> {
    let func = compiler.ctx().get_function(factory).unwrap();

    // Emit init list begin marker
    compiler.emitter().emit_byte(OpCode::InitListBegin, elements.len() as u8);

    // Compile each element
    // Get element type from template args or list factory signature
    let element_type = get_element_type(compiler, target_type)?;

    for element in elements {
        compiler.check(element, &element_type, span)?;
    }

    // Emit init list end and call factory
    compiler.emitter().emit(OpCode::InitListEnd);
    compiler.emitter().emit_call(factory, 1);  // Pass list as argument

    Ok(ExprInfo::rvalue(*target_type))
}

/// Compile init list by creating container and adding elements.
fn compile_init_list_manual(
    compiler: &mut ExprCompiler,
    elements: &[Expr],
    target_type: &DataType,
    span: Span,
) -> Result<ExprInfo> {
    // Create empty container
    let default_ctor = TypeHash::from_constructor(target_type.type_hash, &[]);
    compiler.emitter().emit_new(target_type.type_hash, default_ctor, 0);

    // For each element, call add or use index
    // This is a simplified approach
    for (i, element) in elements.iter().enumerate() {
        compiler.emitter().emit(OpCode::Dup);  // Dup container ref
        let _ = compiler.infer(element, span)?;
        // Call insertLast, add, or opIndex
    }

    Ok(ExprInfo::rvalue(*target_type))
}

fn get_element_type(compiler: &ExprCompiler, container_type: &DataType) -> Result<DataType> {
    let class = compiler.ctx().get_type(container_type.type_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::Internal {
            message: "Container type not found".to_string(),
        })?;

    // If template instance, get first type arg
    if let Some(first_arg) = class.type_args.first() {
        return Ok(*first_arg);
    }

    Err(CompileError::Internal {
        message: "Cannot determine element type".to_string(),
    })
}
```

### Ternary Operator (expr/ternary.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::conversion::can_implicitly_convert;
use super::ExprCompiler;

/// Compile ternary expression: cond ? then : else
pub fn compile_ternary(
    compiler: &mut ExprCompiler,
    condition: &Expr,
    then_expr: &Expr,
    else_expr: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Compile and check condition is bool
    let cond_info = compiler.check(
        condition,
        &DataType::simple(angelscript_core::primitives::BOOL),
        span,
    )?;

    // Jump to else if false
    let else_jump = compiler.emitter().emit_jump(OpCode::JumpIfFalse);

    // Pop condition result
    compiler.emitter().emit(OpCode::Pop);

    // Compile then branch
    let then_info = compiler.infer(then_expr, span)?;

    // Jump over else branch
    let end_jump = compiler.emitter().emit_jump(OpCode::Jump);

    // Patch else jump
    compiler.emitter().patch_jump(else_jump);

    // Pop condition result (for else branch)
    compiler.emitter().emit(OpCode::Pop);

    // Compile else branch
    let else_info = compiler.infer(else_expr, span)?;

    // Patch end jump
    compiler.emitter().patch_jump(end_jump);

    // Determine result type - must be compatible
    let result_type = unify_types(compiler, &then_info.data_type, &else_info.data_type, span)?;

    // May need to emit conversions
    // This is tricky because they're in different branches

    Ok(ExprInfo::rvalue(result_type))
}

/// Find common type for ternary branches.
fn unify_types(
    compiler: &ExprCompiler,
    a: &DataType,
    b: &DataType,
    span: Span,
) -> Result<DataType> {
    // Same type - easy
    if a.type_hash == b.type_hash {
        return Ok(*a);
    }

    // Can a convert to b?
    if can_implicitly_convert(a, b, compiler.ctx()) {
        return Ok(*b);
    }

    // Can b convert to a?
    if can_implicitly_convert(b, a, compiler.ctx()) {
        return Ok(*a);
    }

    Err(CompileError::TypeMismatch {
        expected: format!("{:?}", a.type_hash),
        got: format!("{:?}", b.type_hash),
        span,
    })
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_primitive() {
        // cast<float>(42)
    }

    #[test]
    fn cast_handle_downcast() {
        // cast<Player@>(entity)
    }

    #[test]
    fn lambda_simple() {
        // function(int x) { return x * 2; }
    }

    #[test]
    fn lambda_with_capture() {
        // int y = 10; auto f = function(int x) { return x + y; }
    }

    #[test]
    fn init_list_array() {
        // array<int> arr = {1, 2, 3};
    }

    #[test]
    fn ternary_basic() {
        // x > 0 ? 1 : 0
    }

    #[test]
    fn ternary_different_types() {
        // x > 0 ? 1 : 1.5  (should unify to float)
    }
}
```

## Acceptance Criteria

- [ ] Cast expressions work for primitives
- [ ] Cast expressions work for handle up/downcasting
- [ ] Lambda expressions compile with explicit types
- [ ] Lambda parameter types inferred from expected funcdef
- [ ] Lambda captures work correctly
- [ ] Init lists work with list factory
- [ ] Init lists infer element type from expected type
- [ ] Ternary operator handles bool condition
- [ ] Ternary branches must have compatible types
- [ ] All tests pass

## Next Phase

Task 43: Statement Compilation - Basics (blocks, variables, control flow)
