# Task 43: Statement Compilation - Basics

## Overview

Implement basic statement compilation: blocks, variable declarations, return statements, and if/while control flow.

## Goals

1. Compile block statements with proper scoping
2. Compile variable declarations with initializers
3. Compile return statements with type checking
4. Compile if/else statements
5. Compile while loops

## Dependencies

- Task 37: Local Scope
- Task 38: Bytecode Emitter
- Task 39-41: Expression Compilation

## Files to Create/Modify

```
crates/angelscript-compiler/src/stmt/
├── mod.rs                # Statement compiler module
├── block.rs              # Block statements
├── var_decl.rs           # Variable declarations
├── return_stmt.rs        # Return statements
├── if_stmt.rs            # If/else statements
└── while_stmt.rs         # While loops
```

## Detailed Implementation

### Statement Compiler (stmt/mod.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::Stmt;

use crate::bytecode::BytecodeEmitter;
use crate::context::CompilationContext;
use crate::error::{CompileError, Result};
use crate::expr::ExprCompiler;
use crate::scope::LocalScope;

mod block;
mod var_decl;
mod return_stmt;
mod if_stmt;
mod while_stmt;

pub use block::*;
pub use var_decl::*;
pub use return_stmt::*;
pub use if_stmt::*;
pub use while_stmt::*;

/// Statement compiler - compiles statements to bytecode.
pub struct StmtCompiler<'ctx> {
    ctx: &'ctx CompilationContext<'ctx>,
    scope: LocalScope,
    emitter: BytecodeEmitter,
    /// Expected return type for the current function.
    return_type: DataType,
    /// Whether this function must return a value (non-void).
    must_return: bool,
}

impl<'ctx> StmtCompiler<'ctx> {
    pub fn new(
        ctx: &'ctx CompilationContext<'ctx>,
        return_type: DataType,
        params: Vec<(String, DataType, bool)>,  // (name, type, is_const)
    ) -> Self {
        let mut scope = LocalScope::new();

        // Declare parameters
        for (name, data_type, is_const) in params {
            scope.declare_local(name, data_type, is_const)
                .expect("Parameter declaration should not fail");
        }

        let must_return = !return_type.is_void();

        Self {
            ctx,
            scope,
            emitter: BytecodeEmitter::new(),
            return_type,
            must_return,
        }
    }

    /// Compile a statement.
    pub fn compile(&mut self, stmt: &Stmt, span: Span) -> Result<()> {
        match stmt {
            Stmt::Block(stmts) => self.compile_block(stmts, span),
            Stmt::VarDecl(decl) => self.compile_var_decl(decl, span),
            Stmt::Return(expr) => self.compile_return(expr.as_deref(), span),
            Stmt::If { condition, then_branch, else_branch } => {
                self.compile_if(condition, then_branch, else_branch.as_deref(), span)
            }
            Stmt::While { condition, body } => {
                self.compile_while(condition, body, span)
            }
            Stmt::ExprStmt(expr) => self.compile_expr_stmt(expr, span),
            _ => Err(CompileError::NotImplemented {
                feature: format!("statement: {:?}", stmt),
                span,
            }),
        }
    }

    /// Compile an expression statement (expression evaluated for side effects).
    fn compile_expr_stmt(&mut self, expr: &Expr, span: Span) -> Result<()> {
        let mut expr_compiler = self.expr_compiler();
        let info = expr_compiler.infer(expr, span)?;

        // Pop result if not void
        if !info.data_type.is_void() {
            self.emitter.emit(OpCode::Pop);
        }

        Ok(())
    }

    /// Create an expression compiler using current context.
    fn expr_compiler(&mut self) -> ExprCompiler<'_> {
        ExprCompiler::new(self.ctx, &mut self.scope, &mut self.emitter)
    }

    /// Get the compiled bytecode.
    pub fn finish(self) -> BytecodeChunk {
        self.emitter.finish()
    }

    /// Access the context.
    pub fn ctx(&self) -> &CompilationContext<'ctx> {
        self.ctx
    }

    /// Access the scope.
    pub fn scope(&self) -> &LocalScope {
        &self.scope
    }

    /// Mutable access to scope.
    pub fn scope_mut(&mut self) -> &mut LocalScope {
        &mut self.scope
    }

    /// Access the emitter.
    pub fn emitter(&mut self) -> &mut BytecodeEmitter {
        &mut self.emitter
    }
}
```

### Block Statements (stmt/block.rs)

```rust
use angelscript_core::Span;
use angelscript_parser::ast::Stmt;

use crate::error::Result;
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a block statement.
    pub fn compile_block(&mut self, stmts: &[Stmt], span: Span) -> Result<()> {
        // Push new scope
        self.scope.push();

        // Compile each statement
        for stmt in stmts {
            self.compile(stmt, span)?;
        }

        // Pop scope - emit cleanup for locals
        let locals_to_destroy = self.scope.pop();
        self.emit_local_cleanup(&locals_to_destroy)?;

        Ok(())
    }

    /// Emit cleanup code for locals going out of scope.
    fn emit_local_cleanup(&mut self, locals: &[LocalInfo]) -> Result<()> {
        // Destroy locals in reverse declaration order
        for local in locals.iter().rev() {
            if local.needs_destructor {
                self.emitter.emit_destroy_local(local.slot);
            }
        }
        Ok(())
    }
}
```

### Variable Declarations (stmt/var_decl.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::{VarDecl, TypeExpr, Expr};

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a variable declaration.
    pub fn compile_var_decl(&mut self, decl: &VarDecl, span: Span) -> Result<()> {
        // Resolve type (explicit or inferred)
        let var_type = self.resolve_var_type(decl, span)?;

        // Declare the variable in scope
        let slot = self.scope.declare_local(
            decl.name.clone(),
            var_type,
            decl.is_const,
        )?;

        // Compile initializer or default construct
        if let Some(init) = &decl.initializer {
            self.compile_initializer(init, &var_type, slot, span)?;
        } else {
            self.compile_default_init(&var_type, slot, span)?;
        }

        Ok(())
    }

    /// Resolve variable type from declaration.
    fn resolve_var_type(&self, decl: &VarDecl, span: Span) -> Result<DataType> {
        if let Some(type_expr) = &decl.type_expr {
            // Explicit type
            self.ctx.resolve_type(type_expr, span)
        } else if let Some(init) = &decl.initializer {
            // Infer from initializer (auto)
            let mut expr_compiler = ExprCompiler::new(
                self.ctx,
                &mut self.scope.clone(),  // Temp scope for inference
                &mut BytecodeEmitter::new(),  // Discard bytecode
            );
            let info = expr_compiler.infer(init, span)?;
            Ok(info.data_type)
        } else {
            Err(CompileError::TypeMismatch {
                expected: "explicit type or initializer".to_string(),
                got: "neither".to_string(),
                span,
            })
        }
    }

    /// Compile variable initializer.
    fn compile_initializer(
        &mut self,
        init: &Expr,
        var_type: &DataType,
        slot: u16,
        span: Span,
    ) -> Result<()> {
        let mut expr_compiler = self.expr_compiler();

        // Compile initializer with expected type
        expr_compiler.check(init, var_type, span)?;

        // Store to local slot
        self.emitter.emit_store_local(slot);

        Ok(())
    }

    /// Compile default initialization.
    fn compile_default_init(
        &mut self,
        var_type: &DataType,
        slot: u16,
        span: Span,
    ) -> Result<()> {
        // Check type has default constructor
        if var_type.is_primitive() {
            // Primitives get zero-initialized
            self.emitter.emit_push_zero(var_type);
            self.emitter.emit_store_local(slot);
        } else if var_type.is_handle {
            // Handles start as null
            self.emitter.emit(OpCode::PushNull);
            self.emitter.emit_store_local(slot);
        } else {
            // Object types need default constructor
            let class = self.ctx.get_type(var_type.type_hash)
                .and_then(|t| t.as_class())
                .ok_or_else(|| CompileError::TypeNotFound {
                    name: format!("{:?}", var_type.type_hash),
                    span,
                })?;

            // Find default constructor
            let default_ctor = class.behaviors.default_constructor
                .ok_or_else(|| CompileError::NoDefaultConstructor {
                    type_name: class.name.clone(),
                    span,
                })?;

            // Allocate and construct
            self.emitter.emit_alloc_local(slot, var_type.type_hash);
            self.emitter.emit_load_local_addr(slot);
            self.emitter.emit_call(default_ctor, 1);  // this pointer
            self.emitter.emit(OpCode::Pop);  // Discard void return
        }

        Ok(())
    }
}
```

### Return Statements (stmt/return_stmt.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::Expr;

use crate::bytecode::OpCode;
use crate::error::{CompileError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a return statement.
    pub fn compile_return(&mut self, expr: Option<&Expr>, span: Span) -> Result<()> {
        match (expr, self.return_type.is_void()) {
            // return expr; in non-void function
            (Some(e), false) => {
                let mut expr_compiler = self.expr_compiler();
                expr_compiler.check(e, &self.return_type, span)?;

                // Cleanup locals before return
                self.emit_return_cleanup()?;

                self.emitter.emit(OpCode::Return);
            }

            // return; in void function
            (None, true) => {
                self.emit_return_cleanup()?;
                self.emitter.emit(OpCode::ReturnVoid);
            }

            // return expr; in void function - error
            (Some(_), true) => {
                return Err(CompileError::TypeMismatch {
                    expected: "void (no return value)".to_string(),
                    got: "expression".to_string(),
                    span,
                });
            }

            // return; in non-void function - error
            (None, false) => {
                return Err(CompileError::TypeMismatch {
                    expected: format!("{:?}", self.return_type),
                    got: "void (no return value)".to_string(),
                    span,
                });
            }
        }

        Ok(())
    }

    /// Emit cleanup for all locals before returning.
    fn emit_return_cleanup(&mut self) -> Result<()> {
        // Get all locals in all scopes
        let all_locals = self.scope.all_locals();

        // Destroy in reverse order
        for local in all_locals.iter().rev() {
            if local.needs_destructor {
                self.emitter.emit_destroy_local(local.slot);
            }
        }

        Ok(())
    }
}
```

### If Statements (stmt/if_stmt.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::{Expr, Stmt};

use crate::bytecode::OpCode;
use crate::error::Result;
use crate::primitives;
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile an if/else statement.
    pub fn compile_if(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
        span: Span,
    ) -> Result<()> {
        // Compile condition - must be bool
        let mut expr_compiler = self.expr_compiler();
        expr_compiler.check(
            condition,
            &DataType::simple(primitives::BOOL),
            span,
        )?;

        if let Some(else_stmt) = else_branch {
            // if-else form
            self.compile_if_else(then_branch, else_stmt, span)
        } else {
            // if-only form
            self.compile_if_only(then_branch, span)
        }
    }

    /// Compile if without else.
    fn compile_if_only(&mut self, then_branch: &Stmt, span: Span) -> Result<()> {
        // Jump to end if false
        let end_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // Pop condition
        self.emitter.emit(OpCode::Pop);

        // Compile then branch
        self.compile(then_branch, span)?;

        // Patch jump
        self.emitter.patch_jump(end_jump);

        // Pop condition (for false path - jumped here)
        self.emitter.emit(OpCode::Pop);

        Ok(())
    }

    /// Compile if with else.
    fn compile_if_else(
        &mut self,
        then_branch: &Stmt,
        else_branch: &Stmt,
        span: Span,
    ) -> Result<()> {
        // Jump to else if false
        let else_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // Pop condition (true path)
        self.emitter.emit(OpCode::Pop);

        // Compile then branch
        self.compile(then_branch, span)?;

        // Jump over else
        let end_jump = self.emitter.emit_jump(OpCode::Jump);

        // Else target
        self.emitter.patch_jump(else_jump);

        // Pop condition (false path)
        self.emitter.emit(OpCode::Pop);

        // Compile else branch
        self.compile(else_branch, span)?;

        // End target
        self.emitter.patch_jump(end_jump);

        Ok(())
    }
}
```

### While Loops (stmt/while_stmt.rs)

```rust
use angelscript_core::{DataType, Span};
use angelscript_parser::ast::{Expr, Stmt};

use crate::bytecode::OpCode;
use crate::error::Result;
use crate::primitives;
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a while loop.
    pub fn compile_while(
        &mut self,
        condition: &Expr,
        body: &Stmt,
        span: Span,
    ) -> Result<()> {
        // Mark loop start for break/continue
        let loop_start = self.emitter.current_offset();
        self.emitter.push_loop(loop_start);

        // Compile condition
        let mut expr_compiler = self.expr_compiler();
        expr_compiler.check(
            condition,
            &DataType::simple(primitives::BOOL),
            span,
        )?;

        // Exit if false
        let exit_jump = self.emitter.emit_jump(OpCode::JumpIfFalse);

        // Pop condition (true path)
        self.emitter.emit(OpCode::Pop);

        // Compile body
        self.compile(body, span)?;

        // Jump back to condition
        self.emitter.emit_loop(loop_start);

        // Exit target
        self.emitter.patch_jump(exit_jump);

        // Pop condition (false path)
        self.emitter.emit(OpCode::Pop);

        // Patch break jumps and pop loop context
        self.emitter.pop_loop();

        Ok(())
    }
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_scoping() {
        // { int x = 1; { int x = 2; } }
        // Inner x shadows outer, both properly destroyed
    }

    #[test]
    fn var_decl_explicit_type() {
        // int x = 42;
    }

    #[test]
    fn var_decl_auto() {
        // auto x = 42;  // Infer int
    }

    #[test]
    fn var_decl_default_init() {
        // int x;  // Zero-initialized
    }

    #[test]
    fn return_value() {
        // return 42;
    }

    #[test]
    fn return_void() {
        // return;
    }

    #[test]
    fn if_only() {
        // if (x > 0) { y = 1; }
    }

    #[test]
    fn if_else() {
        // if (x > 0) { y = 1; } else { y = 0; }
    }

    #[test]
    fn while_loop() {
        // while (x > 0) { x -= 1; }
    }
}
```

## Acceptance Criteria

- [ ] Block statements create/destroy scopes correctly
- [ ] Variables declared at block scope
- [ ] Variable initializers type-checked
- [ ] Auto type inference works
- [ ] Default initialization for primitives and objects
- [ ] Return type checking (value vs void)
- [ ] If/else control flow correct
- [ ] While loop with proper jump targets
- [ ] Local cleanup on scope exit
- [ ] All tests pass

## Next Phase

Task 44: Statement Compilation - Loops (for, foreach, switch, break/continue)
