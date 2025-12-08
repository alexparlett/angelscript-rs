# Task 44: Statement Compilation - Loops

## Overview

Implement advanced loop statements: for loops, foreach loops, switch/case statements, and break/continue handling.

## Goals

1. Compile for loops (C-style)
2. Compile foreach loops with iterators
3. Compile switch/case statements
4. Implement break and continue statements

## Dependencies

- Task 42: Statement Basics (while loop foundation)
- Task 38: Bytecode Emitter (jump management)
- Task 40: Expression Calls (iterator method calls)

## Files to Create/Modify

```
crates/angelscript-compiler/src/stmt/
├── for_stmt.rs           # For loops
├── foreach.rs            # Foreach loops
├── switch_stmt.rs        # Switch/case statements
├── break_continue.rs     # Break and continue
└── mod.rs                # Add modules
```

## Detailed Implementation

### For Loops (stmt/for_stmt.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::{Expr, Stmt, VarDecl};

use crate::bytecode::OpCode;
use crate::error::Result;
use crate::primitives;
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a for loop: for (init; cond; update) body
    pub fn compile_for(
        &mut self,
        init: Option<&ForInit>,
        condition: Option<&Expr>,
        update: Option<&Expr>,
        body: &Stmt,
        span: Span,
    ) -> Result<()> {
        // For loop has its own scope (for init variable)
        self.scope.push();

        // Compile initializer
        if let Some(init) = init {
            match init {
                ForInit::VarDecl(decl) => self.compile_var_decl(decl, span)?,
                ForInit::Expr(expr) => {
                    let mut ec = self.expr_compiler();
                    let info = ec.infer(expr, span)?;
                    if !info.data_type.is_void() {
                        self.emitter.emit(OpCode::Pop);
                    }
                }
            }
        }

        // Loop start (after init, at condition)
        let loop_start = self.emitter.current_offset();

        // Continue target is at update, not condition
        let continue_target = if update.is_some() {
            // Will be patched after body
            None
        } else {
            Some(loop_start)
        };

        self.emitter.push_loop_ex(loop_start, continue_target);

        // Compile condition (if present)
        let exit_jump = if let Some(cond) = condition {
            let mut ec = self.expr_compiler();
            ec.check(cond, &DataType::simple(primitives::BOOL), span)?;
            Some(self.emitter.emit_jump(OpCode::JumpIfFalse))
        } else {
            None  // Infinite loop
        };

        // Pop condition if present
        if condition.is_some() {
            self.emitter.emit(OpCode::Pop);
        }

        // Compile body
        self.compile(body, span)?;

        // Continue target (before update)
        let continue_offset = self.emitter.current_offset();
        self.emitter.set_continue_target(continue_offset);

        // Compile update expression
        if let Some(upd) = update {
            let mut ec = self.expr_compiler();
            let info = ec.infer(upd, span)?;
            if !info.data_type.is_void() {
                self.emitter.emit(OpCode::Pop);
            }
        }

        // Jump back to condition
        self.emitter.emit_loop(loop_start);

        // Exit target
        if let Some(jump) = exit_jump {
            self.emitter.patch_jump(jump);
            self.emitter.emit(OpCode::Pop);  // Pop condition
        }

        // Patch break jumps
        self.emitter.pop_loop();

        // Pop for scope
        let locals = self.scope.pop();
        self.emit_local_cleanup(&locals)?;

        Ok(())
    }
}

/// For loop initializer.
pub enum ForInit {
    VarDecl(VarDecl),
    Expr(Expr),
}
```

### Foreach Loops (stmt/foreach.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::{Expr, Stmt, TypeExpr};

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a foreach loop: foreach (elem : container) body
    pub fn compile_foreach(
        &mut self,
        elem_name: &str,
        elem_type: Option<&TypeExpr>,
        container: &Expr,
        body: &Stmt,
        span: Span,
    ) -> Result<()> {
        // Create scope for iterator and element
        self.scope.push();

        // Compile container expression
        let mut ec = self.expr_compiler();
        let container_info = ec.infer(container, span)?;

        // Find iterator method on container
        let (iter_method, elem_data_type) = self.find_iterator_info(
            &container_info.data_type,
            elem_type,
            span,
        )?;

        // Create iterator: container.begin() or container.getIterator()
        self.emitter.emit(OpCode::Dup);  // Keep container for end check
        self.emitter.emit_call_method(iter_method, 0);

        // Store iterator in hidden local
        let iter_slot = self.scope.declare_hidden_local(
            "__iter".to_string(),
            DataType::iterator(),  // Iterator type
        )?;
        self.emitter.emit_store_local(iter_slot);

        // Store container end iterator or null
        // (Implementation depends on iterator pattern used)

        // Declare element variable
        let elem_slot = self.scope.declare_local(
            elem_name.to_string(),
            elem_data_type,
            false,  // Not const (can modify element)
        )?;

        // Loop start
        let loop_start = self.emitter.current_offset();
        self.emitter.push_loop(loop_start);

        // Check iterator validity: iter.hasNext() or iter != end
        self.emitter.emit_load_local(iter_slot);
        let has_next = self.find_has_next_method(&container_info.data_type, span)?;
        self.emitter.emit_call_method(has_next, 0);

        // Exit if no more elements
        let exit_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);
        self.emitter.emit(OpCode::Pop);

        // Get current element: iter.next() or *iter++
        self.emitter.emit_load_local(iter_slot);
        let next_method = self.find_next_method(&container_info.data_type, span)?;
        self.emitter.emit_call_method(next_method, 0);
        self.emitter.emit_store_local(elem_slot);

        // Compile body
        self.compile(body, span)?;

        // Jump back to condition
        self.emitter.emit_loop(loop_start);

        // Exit target
        self.emitter.patch_jump(exit_jump);
        self.emitter.emit(OpCode::Pop);

        // Pop loop context
        self.emitter.pop_loop();

        // Cleanup
        let locals = self.scope.pop();
        self.emit_local_cleanup(&locals)?;

        Ok(())
    }

    /// Find iterator creation method and element type.
    fn find_iterator_info(
        &self,
        container_type: &DataType,
        expected_elem: Option<&TypeExpr>,
        span: Span,
    ) -> Result<(TypeHash, DataType)> {
        let class = self.ctx.get_type(container_type.type_hash)
            .and_then(|t| t.as_class())
            .ok_or_else(|| CompileError::NotIterable {
                type_name: format!("{:?}", container_type.type_hash),
                span,
            })?;

        // Look for opForBegin behavior
        if let Some(for_begin) = class.behaviors.for_begin {
            let func = self.ctx.get_function(for_begin)
                .ok_or_else(|| CompileError::Internal {
                    message: "opForBegin not found".to_string(),
                })?;

            // Element type is first param of opForValue
            let elem_type = if let Some(for_value) = class.behaviors.for_value {
                let value_func = self.ctx.get_function(for_value)?;
                // Return type of opForValue is element type
                value_func.return_type
            } else {
                return Err(CompileError::NotIterable {
                    type_name: class.name.clone(),
                    span,
                });
            };

            return Ok((for_begin, elem_type));
        }

        // Alternative: look for getIterator() method
        // ...

        Err(CompileError::NotIterable {
            type_name: class.name.clone(),
            span,
        })
    }

    fn find_has_next_method(&self, container_type: &DataType, span: Span) -> Result<TypeHash> {
        let class = self.ctx.get_type(container_type.type_hash)
            .and_then(|t| t.as_class())?;

        class.behaviors.for_condition
            .ok_or_else(|| CompileError::NotIterable {
                type_name: class.name.clone(),
                span,
            })
    }

    fn find_next_method(&self, container_type: &DataType, span: Span) -> Result<TypeHash> {
        let class = self.ctx.get_type(container_type.type_hash)
            .and_then(|t| t.as_class())?;

        class.behaviors.for_next
            .ok_or_else(|| CompileError::NotIterable {
                type_name: class.name.clone(),
                span,
            })
    }
}
```

### Switch Statements (stmt/switch_stmt.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::{Expr, Stmt, SwitchCase};

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a switch statement.
    pub fn compile_switch(
        &mut self,
        expr: &Expr,
        cases: &[SwitchCase],
        span: Span,
    ) -> Result<()> {
        // Compile switch expression
        let mut ec = self.expr_compiler();
        let switch_info = ec.infer(expr, span)?;

        // Validate switch type (must be integral or have opEquals)
        self.validate_switch_type(&switch_info.data_type, span)?;

        // Track jump targets
        let mut case_jumps: Vec<(JumpHandle, usize)> = Vec::new();  // (jump, case_index)
        let mut default_index: Option<usize> = None;
        let mut end_jumps: Vec<JumpHandle> = Vec::new();

        // Push switch context for break handling
        self.emitter.push_switch();

        // Emit comparison jumps for each case
        for (i, case) in cases.iter().enumerate() {
            match &case.value {
                Some(value) => {
                    // case VALUE:
                    // Dup switch value, compare, jump if equal
                    self.emitter.emit(OpCode::Dup);
                    let mut ec = self.expr_compiler();
                    ec.check(value, &switch_info.data_type, span)?;
                    self.emit_equality(&switch_info.data_type)?;
                    let jump = self.emitter.emit_jump(OpCode::JumpIfTrue);
                    self.emitter.emit(OpCode::Pop);  // Pop comparison result
                    case_jumps.push((jump, i));
                }
                None => {
                    // default:
                    if default_index.is_some() {
                        return Err(CompileError::DuplicateDefault { span });
                    }
                    default_index = Some(i);
                }
            }
        }

        // Jump to default or end
        let default_jump = if default_index.is_some() {
            Some(self.emitter.emit_jump(OpCode::Jump))
        } else {
            end_jumps.push(self.emitter.emit_jump(OpCode::Jump));
            None
        };

        // Pop switch value before case bodies
        self.emitter.emit(OpCode::Pop);

        // Emit case bodies
        for (i, case) in cases.iter().enumerate() {
            // Patch jumps that target this case
            for (jump, target_i) in &case_jumps {
                if *target_i == i {
                    self.emitter.patch_jump(*jump);
                    self.emitter.emit(OpCode::Pop);  // Pop comparison result (true)
                }
            }

            // Patch default jump
            if Some(i) == default_index {
                if let Some(jump) = default_jump {
                    self.emitter.patch_jump(jump);
                }
            }

            // Compile case body
            for stmt in &case.body {
                self.compile(stmt, span)?;
            }

            // Fall through to next case (no implicit break)
            // User must use explicit break
        }

        // Patch all end jumps (break targets)
        for jump in end_jumps {
            self.emitter.patch_jump(jump);
        }
        self.emitter.pop_switch();

        Ok(())
    }

    /// Validate type is valid for switch.
    fn validate_switch_type(&self, data_type: &DataType, span: Span) -> Result<()> {
        // Primitives are always valid
        if data_type.is_primitive() {
            return Ok(());
        }

        // Objects need opEquals
        let class = self.ctx.get_type(data_type.type_hash)
            .and_then(|t| t.as_class());

        if let Some(c) = class {
            if c.behaviors.equals.is_some() {
                return Ok(());
            }
        }

        Err(CompileError::InvalidSwitchType {
            type_name: format!("{:?}", data_type.type_hash),
            span,
        })
    }

    /// Emit equality comparison.
    fn emit_equality(&mut self, data_type: &DataType) -> Result<()> {
        if data_type.is_primitive() {
            self.emitter.emit(OpCode::Equal);
        } else {
            // Call opEquals method
            let class = self.ctx.get_type(data_type.type_hash)
                .and_then(|t| t.as_class())
                .unwrap();
            let equals = class.behaviors.equals.unwrap();
            self.emitter.emit_call_method(equals, 1);
        }
        Ok(())
    }
}
```

### Break and Continue (stmt/break_continue.rs)

```rust
use angelscript_core::Span;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a break statement.
    pub fn compile_break(&mut self, span: Span) -> Result<()> {
        // Must be inside loop or switch
        if !self.emitter.in_loop_or_switch() {
            return Err(CompileError::BreakOutsideLoop { span });
        }

        // Emit cleanup for locals in current scope down to loop scope
        let locals_to_cleanup = self.scope.locals_since_loop_start();
        for local in locals_to_cleanup.iter().rev() {
            if local.needs_destructor {
                self.emitter.emit_destroy_local(local.slot);
            }
        }

        // Emit break jump (will be patched when loop ends)
        self.emitter.emit_break();

        Ok(())
    }

    /// Compile a continue statement.
    pub fn compile_continue(&mut self, span: Span) -> Result<()> {
        // Must be inside loop (not switch)
        if !self.emitter.in_loop() {
            return Err(CompileError::ContinueOutsideLoop { span });
        }

        // Emit cleanup for locals in current scope down to loop scope
        let locals_to_cleanup = self.scope.locals_since_loop_start();
        for local in locals_to_cleanup.iter().rev() {
            if local.needs_destructor {
                self.emitter.emit_destroy_local(local.slot);
            }
        }

        // Emit continue jump to loop's continue target
        self.emitter.emit_continue();

        Ok(())
    }
}
```

### Update Statement Compiler (stmt/mod.rs)

Add new statement types to the main compile method:

```rust
impl<'ctx> StmtCompiler<'ctx> {
    pub fn compile(&mut self, stmt: &Stmt, span: Span) -> Result<()> {
        match stmt {
            // ... existing cases ...

            Stmt::For { init, condition, update, body } => {
                self.compile_for(init.as_ref(), condition.as_deref(),
                    update.as_deref(), body, span)
            }

            Stmt::ForEach { elem_name, elem_type, container, body } => {
                self.compile_foreach(elem_name, elem_type.as_ref(),
                    container, body, span)
            }

            Stmt::Switch { expr, cases } => {
                self.compile_switch(expr, cases, span)
            }

            Stmt::Break => self.compile_break(span),

            Stmt::Continue => self.compile_continue(span),

            _ => Err(CompileError::NotImplemented {
                feature: format!("statement: {:?}", stmt),
                span,
            }),
        }
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_loop_basic() {
        // for (int i = 0; i < 10; i++) { sum += i; }
    }

    #[test]
    fn for_loop_no_init() {
        // for (; i < 10; i++) { }
    }

    #[test]
    fn for_loop_infinite() {
        // for (;;) { if (done) break; }
    }

    #[test]
    fn foreach_array() {
        // foreach (int x : arr) { sum += x; }
    }

    #[test]
    fn foreach_inferred_type() {
        // foreach (auto x : arr) { }
    }

    #[test]
    fn switch_int() {
        // switch (x) { case 1: ...; case 2: ...; default: ...; }
    }

    #[test]
    fn switch_string() {
        // switch (s) { case "a": ...; case "b": ...; }
    }

    #[test]
    fn break_in_loop() {
        // while (true) { if (x) break; }
    }

    #[test]
    fn break_in_switch() {
        // switch (x) { case 1: break; }
    }

    #[test]
    fn continue_in_for() {
        // for (int i = 0; i < 10; i++) { if (skip) continue; }
    }

    #[test]
    fn nested_loops_break() {
        // Outer: while (true) { while (true) { break; } }
        // Inner break doesn't affect outer
    }
}
```

## Acceptance Criteria

- [ ] For loops compile correctly (init, condition, update)
- [ ] For loop variables scoped to loop
- [ ] Foreach loops work with arrays
- [ ] Foreach loops work with custom iterators (opFor* behaviors)
- [ ] Switch statements with int cases
- [ ] Switch statements with string/object cases (opEquals)
- [ ] Switch default case works
- [ ] Switch fall-through behavior correct
- [ ] Break exits innermost loop/switch
- [ ] Continue jumps to loop update/condition
- [ ] Nested loops handle break/continue correctly
- [ ] Local cleanup on break/continue
- [ ] All tests pass

## Next Phase

Task 45: Function Compilation Pass (orchestrating full function compilation)
