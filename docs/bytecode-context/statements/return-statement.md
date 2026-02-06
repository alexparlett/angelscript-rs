# Return Statement

## Overview
The `return` statement terminates execution of the current function and optionally passes a value back to the caller. Functions with a non-void return type must use `return` with an expression; `void` functions can use a bare `return` for early exit.

## Syntax
```angelscript
// Return with a value (required for non-void functions)
return expression;

// Bare return (only valid in void functions)
return;
```

## Semantics
- A function with a return type other than `void` **must** terminate with a `return` statement that includes an expression. The expression must evaluate to a type that is compatible with (assignable to) the function's declared return type.
- A `void` function can use `return;` (without an expression) to terminate early. Reaching the end of a void function without a return statement is also valid (implicit return).
- Using `return expression;` in a `void` function is a compile-time error.
- Using `return;` (without expression) in a non-void function is a compile-time error.
- The return expression is evaluated, and the result is passed to the caller through the function's return mechanism.
- `return` can appear anywhere within a function body, including inside loops, conditionals, and nested blocks. It immediately exits the function regardless of nesting depth.

## Examples
```angelscript
// Non-void function: must return a value
float valueOfPI()
{
    return 3.141592f;
}

// Void function: bare return for early exit
void doSomething()
{
    if (done)
        return;  // early exit
    // ... more work ...
}

// Return from inside a loop
int findFirst(int[] arr, int target)
{
    for (int i = 0; i < arr.length(); i++)
    {
        if (arr[i] == target)
            return i;  // exits function immediately
    }
    return -1;  // not found
}
```

## Compilation Notes
- **Control flow:** `return` compiles to:
  1. If a return expression is present: evaluate the expression and place the result in the return value location (register, stack slot, or designated return area depending on type).
  2. Emit cleanup bytecode for all local variables currently in scope (destructors, handle releases).
  3. Emit a function-return instruction that restores the caller's stack frame and transfers control back.
- **Stack cleanup:** The return statement may be nested inside multiple scopes (loops, blocks, conditionals). The compiler must emit cleanup code for **all** variables in all scopes from the current scope up to the function scope. This includes:
  - Calling destructors for local object variables.
  - Releasing (decrementing reference count of) local handles.
  - The cleanup order is innermost scope first, outermost last (reverse declaration order within each scope).
- **Return value passing:** The mechanism for returning values depends on the type:
  - **Primitive types:** Typically returned in a register or a fixed stack position.
  - **Object types (by value):** May require constructing the return value in a caller-provided memory location (return value optimization) or copying to a temporary.
  - **Handle types:** The reference count of the returned handle is managed to ensure it is not prematurely released.
- **Multiple return paths:** A non-void function may have multiple `return` statements (e.g., in different branches of an if-else). The compiler must verify that all code paths through the function end with a return statement. If any path can reach the end of a non-void function without returning, the compiler should emit an error.
- **Type considerations:** The return expression's type must match or be implicitly convertible to the function's return type. The compiler inserts conversion bytecode if needed (e.g., int to float).
- **Special cases:**
  - Returning from inside a try block must still execute cleanup for variables in the try scope.
  - Returning from inside a loop must clean up loop-scoped variables (including for-loop init variables).
  - The last statement in a void function does not need an explicit return; the compiler can emit an implicit return at the function epilogue.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Return` | Return statement variant | Wraps `ReturnStmt` (by value) |
| `ReturnStmt` | Return statement structure | `value: Option<&'ast Expr<'ast>>`, `span: Span` |

**Notes:**
- `value` is `None` for bare `return;` in void functions.
- `value` is `Some(expr)` for `return expr;` in non-void functions.
- Validation that void functions do not return values (and non-void functions always return values) is a compiler responsibility, not enforced by the AST structure.

## Related Features
- [Expression Statement](./expression-statement.md) - the return expression follows standard expression evaluation
- [Statement Blocks](./statement-blocks.md) - return must clean up all enclosing scopes
- [Try-Catch](./try-catch.md) - return from within try blocks
- [Variable Declarations](./variable-declarations.md) - variables must be cleaned up on return
