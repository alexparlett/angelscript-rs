# Task 43b: Assignment Expressions

## Overview

Implement assignment expression compilation. Currently stubbed out in `expr/mod.rs:97-100`.

## Goals

1. Compile simple assignment (`a = b`)
2. Compile compound assignments (`+=`, `-=`, `*=`, etc.)
3. Compile member assignment (`obj.field = value`)
4. Compile index assignment (`arr[i] = value`)

## Dependencies

- Task 41: Expression Compilation - Basics
- Task 42: Expression Compilation - Calls (for member access)

## Current State

```rust
// In expr/mod.rs:97-100
Expr::Assign(_) => Err(CompilationError::Other {
    message: "Assignment not yet implemented (Task 43)".to_string(),
    span,
}),
```

## Files to Create/Modify

```
crates/angelscript-compiler/src/expr/
├── assignment.rs     # NEW: Assignment expression compilation
└── mod.rs            # Modify: dispatch to assignment module
```

## Detailed Implementation

### Assignment Module (expr/assignment.rs)

```rust
//! Assignment expression compilation.

use angelscript_core::{CompilationError, Span};
use angelscript_parser::ast::{AssignExpr, AssignOp, Expr};

use super::{ExprCompiler, Result};
use crate::expr_info::ExprInfo;

/// Compile an assignment expression.
pub fn compile_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    assign: &AssignExpr<'ast>,
) -> Result<ExprInfo> {
    let span = assign.span;

    match assign.op {
        AssignOp::Assign => compile_simple_assign(compiler, assign),
        _ => compile_compound_assign(compiler, assign),
    }
}

fn compile_simple_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    assign: &AssignExpr<'ast>,
) -> Result<ExprInfo> {
    // Analyze target without emitting code yet
    let target_info = analyze_assign_target(compiler, assign.target, assign.span)?;

    // Compile the value expression with type checking
    let value_info = compiler.check(assign.value, &target_info.data_type)?;

    // Emit store instruction based on target kind
    emit_store(compiler, &target_info)?;

    // Assignment expression result is the assigned value
    Ok(ExprInfo::rvalue(target_info.data_type))
}

fn compile_compound_assign<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    assign: &AssignExpr<'ast>,
) -> Result<ExprInfo> {
    // For compound assignment (a += b):
    // 1. Load current value of target
    // 2. Compile value expression
    // 3. Apply operation
    // 4. Store result back

    let target_info = analyze_assign_target(compiler, assign.target, assign.span)?;

    // Load current value
    emit_load(compiler, &target_info)?;

    // Compile value expression
    let _value_info = compiler.check(assign.value, &target_info.data_type)?;

    // Apply binary operation
    let binary_op = compound_to_binary_op(assign.op)?;
    emit_binary_op(compiler, binary_op, &target_info.data_type)?;

    // Store result
    emit_store(compiler, &target_info)?;

    Ok(ExprInfo::rvalue(target_info.data_type))
}

/// Information about an assignment target.
struct AssignTarget {
    data_type: DataType,
    kind: AssignTargetKind,
}

enum AssignTargetKind {
    Local { slot: u16 },
    Global { hash: TypeHash },
    Field { field_index: u16 },
    Index { opindex_method: TypeHash },
}

fn analyze_assign_target<'ast>(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    target: &Expr<'ast>,
    span: Span,
) -> Result<AssignTarget> {
    match target {
        Expr::Ident(ident) => {
            // Local or global variable
            // ...
        }
        Expr::Member(member) => {
            // Object field
            // ...
        }
        Expr::Index(index) => {
            // Array/container index
            // ...
        }
        _ => Err(CompilationError::Other {
            message: "invalid assignment target".to_string(),
            span,
        }),
    }
}

fn emit_store(
    compiler: &mut ExprCompiler<'_, '_, '_>,
    target: &AssignTarget,
) -> Result<()> {
    match &target.kind {
        AssignTargetKind::Local { slot } => {
            compiler.emitter().emit_set_local(*slot);
        }
        AssignTargetKind::Global { hash } => {
            compiler.emitter().emit_set_global(*hash);
        }
        AssignTargetKind::Field { field_index } => {
            compiler.emitter().emit_set_field(*field_index);
        }
        AssignTargetKind::Index { opindex_method } => {
            // Call opIndex setter variant
            compiler.emitter().emit_call_method(*opindex_method, 2);
        }
    }
    Ok(())
}
```

### Update Expression Compiler (expr/mod.rs)

```rust
mod assignment;

// In infer() method:
Expr::Assign(assign) => assignment::compile_assign(self, assign),
```

## Bytecode Instructions

Already available:
- `SetLocal` - store to local variable slot
- `SetGlobal` - store to global variable by hash
- `SetField` - store to object field by index

May need for index assignment:
- Call opIndex setter via `CallMethod`

## Edge Cases

1. **Const assignment error**: `const int x = 5; x = 10;`
2. **Rvalue assignment error**: `5 = x;`
3. **Type mismatch**: `int x; x = "hello";`
4. **Handle assignment**: `Foo@ f; f = null;`
5. **Reference assignment**: `int& r = x; r = 5;`

## Testing

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn simple_assignment_local() {
        // int x; x = 5;
    }

    #[test]
    fn simple_assignment_global() {
        // global: int g = 0;
        // g = 10;
    }

    #[test]
    fn compound_assignment_add() {
        // int x = 5; x += 3;
    }

    #[test]
    fn member_assignment() {
        // obj.field = value;
    }

    #[test]
    fn index_assignment() {
        // arr[0] = value;
    }

    #[test]
    fn assign_to_const_error() {
        // const int x = 5; x = 10;
    }

    #[test]
    fn assign_to_rvalue_error() {
        // 5 = 10;
    }

    #[test]
    fn assign_type_mismatch_error() {
        // int x; x = "hello";
    }
}
```

## Acceptance Criteria

- [ ] Simple assignment to local variables works
- [ ] Simple assignment to global variables works
- [ ] Compound assignments (+= -= *= etc.) work
- [ ] Member assignment works
- [ ] Index assignment works (via opIndex)
- [ ] Const assignment rejected with error
- [ ] Rvalue assignment rejected with error
- [ ] Type mismatches rejected with error
- [ ] All tests pass

## Notes

- Assignment expressions return the assigned value (enabling `a = b = c`)
- Compound assignment must load, operate, then store
- Index assignment may need special handling for opIndex setter
