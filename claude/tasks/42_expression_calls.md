# Task 42: Expression Compilation - Calls

## Overview

Implement function calls, method calls, and constructor calls with overload resolution.

## Goals

1. Compile function calls with overload resolution
2. Compile method calls on objects
3. Compile constructor calls (new expressions)
4. Handle default parameter values
5. Handle member access (field/property)

## Dependencies

- Task 36: Overload Resolution
- Task 40: Expression Basics
- Task 41d: Const-Correctness (provides `DataType::is_effectively_const()`)

## Const-Correctness Requirements

Task 41d established the infrastructure for const-correctness. This task must enforce it at:

1. **Method calls on const objects**: Non-const methods cannot be called on effectively const objects
   ```rust
   if obj_info.data_type.is_effectively_const() && !func.def().is_const() {
       return Err(CompileError::CannotModifyConst { span });
   }
   ```

2. **Property writes on const objects**: Cannot call setter on const object
   ```rust
   if obj_info.data_type.is_effectively_const() {
       return Err(CompileError::CannotModifyConst { span });
   }
   ```

3. **Assignment to const lvalues**: Use `ExprInfo::is_mutable` (already tracked)
   ```rust
   if !target_info.is_mutable {
       return Err(CompileError::CannotModifyConst { span });
   }
   ```

4. **Reference parameter passing**: Cannot pass const to non-const `&inout`/`&out` parameter
   - Handled by conversion system (const objects won't match non-const ref params)

## Files to Create/Modify

```
crates/angelscript-compiler/src/expr/
├── calls.rs               # Function/method calls
├── member.rs              # Member access
└── mod.rs                 # Add modules
```

## Detailed Implementation

### Function/Method Calls (expr/calls.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use crate::overload::{resolve_overload, OverloadMatch};
use super::ExprCompiler;

/// Compile a function call expression.
pub fn compile_call(
    compiler: &mut ExprCompiler,
    callee: &Expr,
    args: &[Expr],
    span: Span,
) -> Result<ExprInfo> {
    match callee {
        // Direct function name
        Expr::Identifier(name) => compile_function_call(compiler, name, args, span),

        // Method call: obj.method(args)
        Expr::FieldAccess { object, field } => {
            compile_method_call(compiler, object, field, args, span)
        }

        // Call through expression (funcdef, lambda)
        _ => compile_indirect_call(compiler, callee, args, span),
    }
}

/// Compile a direct function call by name.
fn compile_function_call(
    compiler: &mut ExprCompiler,
    name: &str,
    args: &[Expr],
    span: Span,
) -> Result<ExprInfo> {
    // Compile arguments first (left to right)
    let arg_types = compile_arguments(compiler, args, span)?;

    // Find function candidates
    let candidates = compiler.ctx().find_functions(name);
    if candidates.is_empty() {
        return Err(CompileError::FunctionNotFound {
            name: name.to_string(),
            span,
        });
    }

    // Resolve overload
    let resolution = resolve_overload(&candidates, &arg_types, compiler.ctx())
        .map_err(|e| e.with_span(span))?;

    // Apply argument conversions
    emit_arg_conversions(compiler, &resolution, &arg_types)?;

    // Emit call
    let func = compiler.ctx().get_function(resolution.func_hash)
        .ok_or_else(|| CompileError::Internal {
            message: "Resolved function not found".to_string(),
        })?;

    compiler.emitter().emit_call(resolution.func_hash, args.len() as u8);

    Ok(ExprInfo::rvalue(func.def().return_type))
}

/// Compile a method call on an object.
fn compile_method_call(
    compiler: &mut ExprCompiler,
    object: &Expr,
    method_name: &str,
    args: &[Expr],
    span: Span,
) -> Result<ExprInfo> {
    // Compile object expression
    let obj_info = compiler.infer(object, span)?;

    // Compile arguments
    let arg_types = compile_arguments(compiler, args, span)?;

    // Find method candidates on object type
    let candidates = compiler.ctx().find_methods(obj_info.data_type.type_hash, method_name);
    if candidates.is_empty() {
        // Try property accessor: get_property()
        if let Some(result) = try_property_getter(compiler, &obj_info, method_name, span)? {
            return Ok(result);
        }

        return Err(CompileError::MemberNotFound {
            type_name: format!("{:?}", obj_info.data_type.type_hash),
            member: method_name.to_string(),
            span,
        });
    }

    // Resolve overload
    let resolution = resolve_overload(&candidates, &arg_types, compiler.ctx())
        .map_err(|e| e.with_span(span))?;

    // Check const correctness: non-const methods cannot be called on const objects
    let func = compiler.ctx().get_function(resolution.func_hash).unwrap();
    if obj_info.data_type.is_effectively_const() && !func.def().is_const() {
        return Err(CompileError::CannotModifyConst { span });
    }

    // Apply argument conversions
    emit_arg_conversions(compiler, &resolution, &arg_types)?;

    // Emit method call
    let is_interface = compiler.ctx().get_type(obj_info.data_type.type_hash)
        .map(|t| t.is_interface())
        .unwrap_or(false);

    if is_interface {
        compiler.emitter().emit_call_virtual(resolution.func_hash, args.len() as u8);
    } else {
        compiler.emitter().emit_call_method(resolution.func_hash, args.len() as u8);
    }

    Ok(ExprInfo::rvalue(func.def().return_type))
}

/// Compile an indirect call through a funcdef/lambda.
fn compile_indirect_call(
    compiler: &mut ExprCompiler,
    callee: &Expr,
    args: &[Expr],
    span: Span,
) -> Result<ExprInfo> {
    // Compile callee expression (should produce a funcdef type)
    let callee_info = compiler.infer(callee, span)?;

    // Get funcdef signature
    let funcdef = compiler.ctx().get_type(callee_info.data_type.type_hash)
        .and_then(|t| t.as_funcdef())
        .ok_or_else(|| CompileError::TypeMismatch {
            expected: "function".to_string(),
            got: format!("{:?}", callee_info.data_type.type_hash),
            span,
        })?;

    // Check argument count
    if args.len() != funcdef.params.len() {
        return Err(CompileError::WrongArgCount {
            expected: funcdef.params.len(),
            got: args.len(),
            span,
        });
    }

    // Compile and check arguments
    for (arg, param_type) in args.iter().zip(funcdef.params.iter()) {
        compiler.check(arg, param_type, span)?;
    }

    // Emit indirect call
    compiler.emitter().emit(OpCode::CallFuncPtr);
    compiler.emitter().emit_byte(OpCode::Pop, args.len() as u8);  // arg count

    Ok(ExprInfo::rvalue(funcdef.return_type))
}

/// Compile a constructor call (new expression).
pub fn compile_new(
    compiler: &mut ExprCompiler,
    type_expr: &TypeExpr,
    args: &[Expr],
    span: Span,
) -> Result<ExprInfo> {
    // Resolve type
    let data_type = compiler.resolve_type(type_expr, span)?;

    let class = compiler.ctx().get_type(data_type.type_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeMismatch {
            expected: "class type".to_string(),
            got: format!("{:?}", data_type.type_hash),
            span,
        })?;

    // Compile arguments
    let arg_types = compile_arguments(compiler, args, span)?;

    // Find constructor
    let constructors = &class.behaviors.constructors;
    if constructors.is_empty() && !args.is_empty() {
        return Err(CompileError::NoMatchingOverload {
            name: format!("{}::constructor", class.name),
            args: format_arg_types(&arg_types),
            span,
        });
    }

    let ctor_hash = if constructors.is_empty() {
        // Default constructor
        TypeHash::from_constructor(data_type.type_hash, &[])
    } else {
        let resolution = resolve_overload(constructors, &arg_types, compiler.ctx())
            .map_err(|e| e.with_span(span))?;
        emit_arg_conversions(compiler, &resolution, &arg_types)?;
        resolution.func_hash
    };

    // Emit new instruction
    compiler.emitter().emit_new(data_type.type_hash, ctor_hash, args.len() as u8);

    Ok(ExprInfo::rvalue(data_type.as_handle()))
}

// ==========================================================================
// Helpers
// ==========================================================================

/// Compile arguments and return their types.
fn compile_arguments(
    compiler: &mut ExprCompiler,
    args: &[Expr],
    span: Span,
) -> Result<Vec<DataType>> {
    let mut arg_types = Vec::with_capacity(args.len());

    for arg in args {
        let info = compiler.infer(arg, span)?;
        arg_types.push(info.data_type);
    }

    Ok(arg_types)
}

/// Emit conversions for arguments based on resolution.
fn emit_arg_conversions(
    compiler: &mut ExprCompiler,
    resolution: &OverloadMatch,
    arg_types: &[DataType],
) -> Result<()> {
    // Arguments are already on stack, we need to convert them in place
    // This is tricky - may need to restructure

    for (i, conv) in resolution.arg_conversions.iter().enumerate() {
        if let Some(conv) = conv {
            if !conv.is_exact() {
                // Emit conversion for argument i
                // This requires knowing the stack layout
            }
        }
    }

    Ok(())
}

fn try_property_getter(
    compiler: &mut ExprCompiler,
    obj_info: &ExprInfo,
    name: &str,
    span: Span,
) -> Result<Option<ExprInfo>> {
    // Look for get_name() method
    let getter_name = format!("get_{}", name);
    let methods = compiler.ctx().find_methods(obj_info.data_type.type_hash, &getter_name);

    if let Some(method_hash) = methods.first() {
        let func = compiler.ctx().get_function(*method_hash).unwrap();

        // Check const correctness: non-const methods cannot be called on const objects
        if obj_info.data_type.is_effectively_const() && !func.def().is_const() {
            return Err(CompileError::CannotModifyConst { span });
        }

        compiler.emitter().emit_call_method(*method_hash, 0);
        return Ok(Some(ExprInfo::rvalue(func.def().return_type)));
    }

    Ok(None)
}

fn format_arg_types(types: &[DataType]) -> String {
    types.iter()
        .map(|t| format!("{:?}", t.type_hash))
        .collect::<Vec<_>>()
        .join(", ")
}
```

### Member Access (expr/member.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use crate::expr_info::ExprInfo;
use super::ExprCompiler;

/// Compile field access expression.
pub fn compile_field_access(
    compiler: &mut ExprCompiler,
    object: &Expr,
    field: &str,
    span: Span,
) -> Result<ExprInfo> {
    // Compile object
    let obj_info = compiler.infer(object, span)?;

    let class = compiler.ctx().get_type(obj_info.data_type.type_hash)
        .and_then(|t| t.as_class())
        .ok_or_else(|| CompileError::TypeMismatch {
            expected: "class type".to_string(),
            got: format!("{:?}", obj_info.data_type.type_hash),
            span,
        })?;

    // Find field/property
    if let Some(prop) = class.find_property(field) {
        // Check visibility
        // ...

        if let Some(getter) = prop.getter {
            // Virtual property - call getter
            let func = compiler.ctx().get_function(getter).unwrap();

            // Check const correctness: non-const methods cannot be called on const objects
            if obj_info.data_type.is_effectively_const() && !func.def().is_const() {
                return Err(CompileError::CannotModifyConst { span });
            }

            compiler.emitter().emit_call_method(getter, 0);
            return Ok(ExprInfo::rvalue(prop.data_type));
        } else {
            // Direct field - compute offset
            let field_index = class.properties.iter()
                .position(|p| p.name == field)
                .unwrap() as u16;

            compiler.emitter().emit_get_field(field_index);

            let is_mutable = !obj_info.data_type.is_effectively_const() && !prop.data_type.is_const;
            return Ok(if is_mutable {
                ExprInfo::lvalue(prop.data_type)
            } else {
                ExprInfo::const_lvalue(prop.data_type)
            });
        }
    }

    Err(CompileError::MemberNotFound {
        type_name: class.name.clone(),
        member: field.to_string(),
        span,
    })
}

/// Compile index access expression (array[i]).
pub fn compile_index(
    compiler: &mut ExprCompiler,
    object: &Expr,
    index: &Expr,
    span: Span,
) -> Result<ExprInfo> {
    // Compile object
    let obj_info = compiler.infer(object, span)?;

    // Compile index
    let index_info = compiler.infer(index, span)?;

    // Find opIndex method
    let methods = compiler.ctx().find_methods(obj_info.data_type.type_hash, "opIndex");
    if methods.is_empty() {
        return Err(CompileError::NoOperator {
            op: "[]".to_string(),
            left: format!("{:?}", obj_info.data_type.type_hash),
            right: format!("{:?}", index_info.data_type.type_hash),
            span,
        });
    }

    // Find matching overload
    let resolution = crate::overload::resolve_overload(
        &methods,
        &[index_info.data_type],
        compiler.ctx(),
    ).map_err(|e| e.with_span(span))?;

    let func = compiler.ctx().get_function(resolution.func_hash).unwrap();

    // Check const correctness: non-const methods cannot be called on const objects
    if obj_info.data_type.is_effectively_const() && !func.def().is_const() {
        return Err(CompileError::CannotModifyConst { span });
    }

    compiler.emitter().emit_call_method(resolution.func_hash, 1);

    // opIndex typically returns a reference, making this an lvalue
    let return_type = func.def().return_type;
    let is_lvalue = return_type.ref_modifier != RefModifier::None;

    Ok(if is_lvalue {
        ExprInfo::lvalue(return_type)
    } else {
        ExprInfo::rvalue(return_type)
    })
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_simple_function() {
        // void foo(); foo();
    }

    #[test]
    fn call_with_overload() {
        // foo(int), foo(float); foo(1) should pick int version
    }

    #[test]
    fn call_method() {
        // player.update()
    }

    #[test]
    fn call_constructor() {
        // Player@ p = Player();
    }

    #[test]
    fn property_getter() {
        // player.health (calls get_health)
    }

    #[test]
    fn index_access() {
        // arr[0]
    }
}
```

## Acceptance Criteria

- [ ] Direct function calls resolve and compile
- [ ] Overloaded functions select correct overload
- [ ] Method calls work on objects
- [ ] Interface method calls use virtual dispatch
- [ ] Constructor calls (new) work
- [ ] Property getters called for property access
- [ ] Index operator (opIndex) works
- [ ] Const correctness checked for method calls
- [ ] Funcdef/lambda indirect calls work
- [ ] All tests pass

## Next Phase

Task 42: Expression Compilation - Advanced (cast, lambda, init list, ternary)
