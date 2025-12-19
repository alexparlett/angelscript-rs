# Task 45b: Statement Compilation - Try/Catch

## Overview

Implement try/catch exception handling statements.

## Goals

1. Compile try blocks
2. Compile catch blocks with exception type matching
3. Handle exception propagation
4. Implement throw statements

## Dependencies

- Task 45: Statement Compilation - Loops (control flow foundation)
- Task 39: Bytecode Emitter (jump management)

## Files to Create/Modify

```
crates/angelscript-compiler/src/stmt/
├── try_catch_stmt.rs     # Try/catch statements
├── throw_stmt.rs         # Throw statements
└── mod.rs                # Add modules
```

## Detailed Implementation

### Try/Catch Statements (stmt/try_catch_stmt.rs)

```rust
use angelscript_core::{DataType, Span, TypeHash};
use angelscript_parser::ast::{TryStmt, CatchClause};

use crate::bytecode::OpCode;
use crate::error::{CompilationError, Result};
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a try/catch statement.
    ///
    /// Bytecode layout:
    /// ```text
    /// TryBegin -> catch_table_offset
    /// [try body]
    /// TryEnd
    /// Jump -> after_catches
    ///
    /// catch_1:
    /// [catch body 1]
    /// Jump -> after_catches
    ///
    /// catch_2:
    /// [catch body 2]
    /// Jump -> after_catches
    ///
    /// after_catches:
    /// ```
    pub fn compile_try(&mut self, try_stmt: &TryStmt) -> Result<()> {
        let span = try_stmt.span;

        // Emit try block header
        let try_begin = self.emitter.emit_try_begin();

        // Compile try body
        self.compile(try_stmt.body)?;

        // End try block
        self.emitter.emit_try_end();

        // Jump past all catch blocks on success
        let success_jump = self.emitter.emit_jump(OpCode::Jump);

        // Track catch block offsets for exception dispatch table
        let mut catch_entries = Vec::new();

        // Compile each catch clause
        for catch in &try_stmt.catches {
            let catch_offset = self.emitter.current_offset();

            // Create scope for catch variable
            self.ctx.push_local_scope();

            // Declare exception variable if named
            if let Some(ref var_name) = catch.var_name {
                let exception_type = self.resolve_type(&catch.exception_type)?;
                let slot = self.ctx.declare_local(
                    var_name.to_string(),
                    exception_type.clone(),
                    false,
                    catch.span,
                )?;
                self.ctx.mark_local_initialized(var_name);
                // Exception value is on stack, store it
                self.emitter.emit_set_local(slot);
            } else {
                // No variable, just pop the exception
                self.emitter.emit_pop();
            }

            // Compile catch body
            self.compile(catch.body)?;

            // Cleanup scope
            let exiting_vars = self.ctx.pop_local_scope();
            self.emit_scope_cleanup(&exiting_vars)?;

            // Jump to after all catches
            let catch_end_jump = self.emitter.emit_jump(OpCode::Jump);
            catch_entries.push((catch.exception_type.clone(), catch_offset, catch_end_jump));
        }

        // Patch all jumps to after catches
        let after_catches = self.emitter.current_offset();
        self.emitter.patch_jump(success_jump);
        for (_, _, end_jump) in &catch_entries {
            self.emitter.patch_jump(*end_jump);
        }

        // Patch try begin with catch table info
        self.emitter.patch_try_begin(try_begin, &catch_entries);

        Ok(())
    }
}
```

### Throw Statements (stmt/throw_stmt.rs)

```rust
use angelscript_core::Span;
use angelscript_parser::ast::ThrowStmt;

use crate::bytecode::OpCode;
use crate::error::Result;
use super::StmtCompiler;

impl<'ctx> StmtCompiler<'ctx> {
    /// Compile a throw statement.
    ///
    /// Bytecode:
    /// ```text
    /// [exception expression]
    /// Throw
    /// ```
    pub fn compile_throw(&mut self, throw_stmt: &ThrowStmt) -> Result<()> {
        // Compile exception expression
        let mut ec = self.expr_compiler();
        let _info = ec.infer(throw_stmt.expr)?;

        // Emit throw instruction
        self.emitter.emit(OpCode::Throw);

        Ok(())
    }

    /// Compile a rethrow statement (throw without expression in catch block).
    pub fn compile_rethrow(&mut self, span: Span) -> Result<()> {
        // Verify we're in a catch block
        if !self.in_catch_block() {
            return Err(CompilationError::Other {
                message: "rethrow is only valid inside a catch block".to_string(),
                span,
            });
        }

        self.emitter.emit(OpCode::Rethrow);
        Ok(())
    }
}
```

### Update Statement Compiler (stmt/mod.rs)

Add new statement types:

```rust
mod try_catch_stmt;
mod throw_stmt;

impl<'ctx> StmtCompiler<'ctx> {
    pub fn compile(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            // ... existing cases ...

            Stmt::Try(try_stmt) => self.compile_try(try_stmt),

            Stmt::Throw(throw_stmt) => self.compile_throw(throw_stmt),

            _ => Err(CompilationError::NotImplemented {
                feature: format!("statement: {:?}", stmt),
                span: stmt.span(),
            }),
        }
    }
}
```

## Bytecode Instructions Required

New opcodes needed in `bytecode/opcode.rs`:

```rust
/// Try block begin - pushes exception handler
TryBegin { catch_table: u32 },

/// Try block end - pops exception handler
TryEnd,

/// Throw exception
Throw,

/// Rethrow current exception (in catch block)
Rethrow,
```

## Exception Dispatch Table

The VM needs a way to match exceptions to catch blocks:

```rust
struct CatchEntry {
    /// Type of exception this catch handles (or None for catch-all)
    exception_type: Option<TypeHash>,
    /// Bytecode offset of catch block
    catch_offset: usize,
}

struct ExceptionTable {
    /// Ordered list of catch entries (checked in order)
    catches: Vec<CatchEntry>,
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_catch_basic() {
        // try { throw Exception(); } catch (Exception e) { }
    }

    #[test]
    fn try_catch_multiple() {
        // try { } catch (TypeA a) { } catch (TypeB b) { }
    }

    #[test]
    fn try_catch_all() {
        // try { } catch { } // catches everything
    }

    #[test]
    fn throw_statement() {
        // throw MyException("error");
    }

    #[test]
    fn rethrow_in_catch() {
        // try { } catch { throw; }
    }

    #[test]
    fn nested_try_catch() {
        // try { try { } catch { } } catch { }
    }

    #[test]
    fn exception_propagation() {
        // Unhandled exception propagates to caller
    }
}
```

## Acceptance Criteria

- [ ] Try blocks compile correctly
- [ ] Catch blocks match exception types
- [ ] Multiple catch blocks work (checked in order)
- [ ] Catch-all (no type) catches any exception
- [ ] Throw statements compile and emit exception
- [ ] Rethrow works inside catch blocks
- [ ] Nested try/catch works correctly
- [ ] Exception propagates if not caught
- [ ] Local cleanup on exception (destructors called)
- [ ] All tests pass

## Notes

- AngelScript exceptions are reference-counted objects
- Catch blocks are checked in declaration order
- First matching catch handles the exception
- Catch-all must be last if present
- Destructors must be called for locals when unwinding
