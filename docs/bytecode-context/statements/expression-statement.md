# Expression Statement

## Overview
An expression statement is a standalone expression terminated by a semicolon. It allows any valid expression to be used as a statement, though in practice this is most commonly used for assignments, function calls, and increment/decrement operations. The result value of the expression (if any) is discarded.

## Syntax
```angelscript
// Assignment
a = b;

// Function call
func();

// Method call
obj.method();

// Compound assignment
x += 10;

// Increment / decrement
i++;
--j;

// Any valid expression (result discarded)
a + b;  // legal but pointless
```

## Semantics
- Any valid expression can appear as a statement.
- The expression must be terminated with a semicolon (`;`).
- The result value of the expression is discarded after evaluation. If the expression produces a value, it is simply popped from the evaluation stack.
- Side effects of the expression (assignments, function calls, I/O) are the purpose of expression statements.
- Common forms include variable assignments, function/method calls, and pre/post increment/decrement operations.

## Examples
```angelscript
void example()
{
    int a = 0;
    int b = 5;

    a = b;          // assignment expression as statement
    a = b + 10;     // compound expression assigned
    doWork();       // void function call
    int c = getVal(); // this is a variable declaration, not an expression statement

    a++;            // post-increment
    ++a;            // pre-increment
    a += b;         // compound assignment
}
```

## Compilation Notes
- **Result discarding:** After evaluating the expression, if a value remains on the evaluation stack, the compiler must emit a pop instruction to discard it. For void-returning function calls, no pop is needed.
- **Stack behavior:** The expression is evaluated using the standard expression evaluation mechanics (pushing operands, calling operators, etc.). After the statement completes, the stack must be at the same depth as before the statement began. The compiler should verify this invariant.
- **Side-effect-only expressions:** Assignments modify a variable slot directly (or through a reference). Function calls push a return value (if non-void) which must be popped. Increment/decrement operations modify the variable and may or may not leave a value on the stack depending on pre vs. post form.
- **Optimization opportunity:** The compiler can detect expressions with no side effects (e.g., `a + b;` with no assignment) and either warn or optimize them away, though AngelScript typically still evaluates them.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Expr` | Expression statement variant | Wraps `ExprStmt` (by value) |
| `ExprStmt` | Expression statement structure | `expr: Option<&'ast Expr<'ast>>`, `span: Span` |

**Notes:**
- The `expr` field is `Option` to represent empty statements (a bare `;`), where `expr` is `None`.
- For non-empty expression statements, `expr` contains the expression whose result is discarded after evaluation.

## Related Features
- [Variable Declarations](./variable-declarations.md) - declaration statements vs. expression statements
- [Return Statement](./return-statement.md) - return uses an expression but is not an expression statement
