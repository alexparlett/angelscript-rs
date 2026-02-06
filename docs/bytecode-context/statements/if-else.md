# If / If-Else

## Overview
The `if` statement conditionally executes a block of code based on a boolean expression. It can be extended with `else` to provide an alternative path, and multiple `if-else` blocks can be chained to test a sequence of conditions.

## Syntax
```angelscript
// Simple if
if (condition)
{
    // executed if condition is true
}

// if-else
if (condition)
{
    // executed if condition is true
}
else
{
    // executed if condition is false
}

// if-else-if chain
if (condition1)
{
    // executed if condition1 is true
}
else if (condition2)
{
    // executed if condition1 is false and condition2 is true
}
else
{
    // executed if all conditions are false
}

// Single-statement body (no braces)
if (condition)
    doSomething();
else
    doSomethingElse();
```

## Semantics
- The conditional expression must evaluate to `bool` (`true` or `false`). Non-boolean types are not implicitly converted.
- The `if` body is executed only when the condition is `true`.
- The `else` clause is optional. If present, its body is executed when the condition is `false`.
- In an `if-else-if` chain, each condition is evaluated sequentially. The first condition that evaluates to `true` causes its corresponding body to execute; all subsequent conditions and bodies are skipped. If no condition is true and a final `else` is present, that else body executes.
- The body can be a single statement or a statement block (curly braces). A statement block creates a new scope.

## Examples
```angelscript
void classify(int value)
{
    if (value < 0)
    {
        print("negative");
    }
    else if (value == 0)
    {
        print("zero");
    }
    else
    {
        print("positive");
    }
}

// Nested if
void check(int x, int y)
{
    if (x > 0)
    {
        if (y > 0)
            print("both positive");
        else
            print("x positive, y non-positive");
    }
}
```

## Compilation Notes
- **Control flow:** The compiler generates conditional jump bytecode. The typical pattern is:
  1. Evaluate the condition expression, leaving a bool on the stack.
  2. Emit a conditional jump-if-false to a label (the else branch or the end of the if).
  3. Emit the if-body bytecode.
  4. If an `else` clause exists, emit an unconditional jump over the else-body to the end label, then place the else label, then emit the else-body bytecode.
  5. Place the end label.
- **If-else-if chains:** These are compiled as nested if-else structures. Each `else if` introduces a new condition test at the else label of the previous if. The compiler can flatten this into a linear sequence of test-and-jump instructions.
- **Label generation:** Each `if` statement requires one or two labels:
  - `else_label` (or `end_label` if no else): target for the false branch of the condition.
  - `end_label`: target for the unconditional jump that skips the else body (only needed when an else clause exists).
- **Stack behavior:** The condition expression pushes one bool value onto the stack. The conditional jump consumes it. After the jump, the stack is back to its pre-condition depth. Each branch body must leave the stack at the same depth.
- **Type considerations:** The condition must be of type `bool`. The compiler must reject non-boolean conditions or insert an implicit conversion if the language rules allow it (AngelScript requires explicit bool).
- **Special cases:** An empty if-body or else-body is legal but generates no bytecode for that branch (just the jump targets). The compiler may optimize away the entire construct if the condition is a compile-time constant.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::If` | If statement variant | Wraps `&'ast IfStmt<'ast>` (arena-allocated reference) |
| `IfStmt` | If/if-else structure | `condition: &'ast Expr<'ast>`, `then_stmt: &'ast Stmt<'ast>`, `else_stmt: Option<&'ast Stmt<'ast>>`, `span: Span` |

**Notes:**
- `else if` chains are represented as nested `IfStmt` nodes: the outer `else_stmt` contains a `Stmt::If` wrapping the next `IfStmt`.
- Both `then_stmt` and `else_stmt` are generic `Stmt` references, allowing either a single statement or a `Stmt::Block`.
- `else_stmt` is `None` when there is no else clause.

## Related Features
- [Switch-Case](./switch-case.md) - alternative branching for integer values
- [Statement Blocks](./statement-blocks.md) - scoping within if/else bodies
- [Break / Continue](./break-continue.md) - not applicable to if (only loops and switch)
