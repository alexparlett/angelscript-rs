# Try-Catch Blocks

## Overview
Try-catch blocks provide structured exception handling in AngelScript. Code that might throw an exception is placed in the `try` block, and the recovery logic is placed in the `catch` block. AngelScript uses **untyped catch** -- the catch block handles all exceptions regardless of their type or origin.

## Syntax
```angelscript
try
{
    // code that might throw an exception
}
catch
{
    // executed if an exception was thrown in the try block
}
```

## Semantics
- The `try` block is entered and its statements execute normally.
- If no exception occurs during execution of the try block, the catch block is skipped entirely and execution continues after the catch block.
- If an exception occurs during execution of the try block:
  - Execution of the try block is immediately interrupted.
  - Any remaining statements in the try block are **not** executed.
  - Control transfers to the `catch` block.
  - After the catch block finishes, execution continues with the statement following the entire try-catch construct.
- **Untyped catch:** AngelScript does not support typed catch clauses (no `catch (ExceptionType e)`). The single `catch` block handles all exceptions.
- **Exception sources include:**
  - **Null pointer access:** Dereferencing an uninitialized handle (null handle).
  - **Division by zero:** Integer division or modulo by zero.
  - **Application-raised exceptions:** Exceptions raised by host application-registered functions.
  - **Script-raised exceptions:** Exceptions explicitly thrown by script code via the standard library exception functions.
- The exception information (message, etc.) can be retrieved within the catch block using the standard library exception handling functions.
- Try-catch blocks can be nested. An exception in an inner try is caught by the inner catch; if the inner catch re-raises or a different exception occurs, it propagates to the outer try-catch.

## Examples
```angelscript
// Basic exception handling
try
{
    DoSomethingThatMightThrowException();
    // This line is not executed if the above throws
    print("success");
}
catch
{
    print("an exception occurred");
}

// Handling null handle access
MyClass@ obj = null;
try
{
    obj.doSomething();  // throws: null handle access
}
catch
{
    print("caught null handle exception");
}

// Nested try-catch
try
{
    try
    {
        riskyOperation();
    }
    catch
    {
        // handle inner exception
        fallbackOperation();  // this might also throw
    }
}
catch
{
    // handle exception from fallbackOperation or any unhandled inner exception
    print("outer catch");
}
```

## Compilation Notes
- **Control flow:** The try-catch compiles to:
  1. Push an exception handler entry onto the runtime exception handler stack. This entry records the `catch_label` as the handler target.
  2. Emit the try-body bytecode.
  3. Pop the exception handler entry (no exception occurred).
  4. Emit an unconditional jump to `end_label` (skip the catch block).
  5. `catch_label`: Emit the catch-body bytecode.
  6. `end_label`: continue with subsequent code.
- **Exception handler registration:** At runtime, entering a try block must register an exception handler with the VM. This is typically done via a special bytecode instruction (e.g., `TRY catch_label`) that pushes a handler frame onto the VM's exception handler stack. The handler frame stores the catch target address and the stack state at the try entry point.
- **Exception dispatch:** When an exception occurs (at any nesting depth within the try block):
  1. The VM searches the exception handler stack for the nearest handler.
  2. The stack is unwound to the state recorded in the handler frame. This includes destroying local variables that were created inside the try block.
  3. The instruction pointer is set to the catch label.
  4. Execution resumes in the catch block.
- **Stack unwinding:** When an exception is caught, the VM must:
  - Destruct all object-type local variables that were constructed within the try block but are now going out of scope.
  - Release all handles acquired within the try block.
  - Restore the evaluation stack to the depth it had at the try block entry.
  - This may involve walking the variable/scope table to find which variables need cleanup.
- **Label generation:** Two labels are needed:
  - `catch_label`: entry point of the catch block.
  - `end_label`: the instruction following the entire try-catch construct.
- **Handler cleanup:** At the end of the try block (normal exit), the exception handler entry must be popped from the handler stack (e.g., via an `END_TRY` instruction). This prevents the handler from catching exceptions that occur after the try block.
- **Nested try-catch:** Multiple handler entries can be stacked. The innermost handler catches first. If the catch block itself throws (and is not in its own try), the exception propagates to the next outer handler.
- **Special cases:**
  - `return` from within a try block must pop the exception handler before returning. The compiler must emit handler-pop bytecode before the return cleanup sequence.
  - `break` or `continue` that exits a try block (when the try is inside a loop) must also pop the exception handler.
  - The catch block itself is not protected by the try handler. Exceptions in the catch block propagate to the next enclosing try-catch or terminate the script.
  - If an exception occurs during stack unwinding (e.g., a destructor throws), behavior depends on the VM implementation (typically the original exception is lost and the new exception propagates).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::TryCatch` | Try-catch statement variant | Wraps `&'ast TryCatchStmt<'ast>` (arena-allocated reference) |
| `TryCatchStmt` | Try-catch structure | `try_block: Block<'ast>`, `catch_block: Block<'ast>`, `span: Span` |

**Notes:**
- Both `try_block` and `catch_block` are `Block` values (not references), embedded directly in the struct.
- AngelScript uses untyped catch -- there is no exception variable or type in the `catch_block`, so the AST has no catch parameter field.
- `Block` (not `Stmt`) is used for the bodies, meaning the try and catch sections are always brace-delimited blocks, not arbitrary single statements.

## Related Features
- [Statement Blocks](./statement-blocks.md) - try and catch bodies are statement blocks with their own scopes
- [Return Statement](./return-statement.md) - return from within try must pop exception handlers
- [Break / Continue](./break-continue.md) - break/continue crossing try boundaries must pop handlers
