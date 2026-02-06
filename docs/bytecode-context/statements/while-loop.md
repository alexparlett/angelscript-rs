# While Loop

## Overview
The `while` loop repeatedly executes a body of statements as long as a condition evaluates to `true`. The condition is checked **before** each iteration, so if the condition is initially `false`, the body is never executed.

## Syntax
```angelscript
while (condition)
{
    // body
}

// Single-statement body (no braces)
while (condition)
    statement;
```

## Semantics
- The condition expression must evaluate to `bool` (`true` or `false`).
- The condition is evaluated **before** each iteration of the loop body.
- If the condition evaluates to `true`, the body executes. After the body completes, control returns to the condition check.
- If the condition evaluates to `false`, the loop terminates and execution continues with the statement immediately following the loop.
- If the condition is `false` on the first check, the body is never executed (zero iterations).
- The body can be a single statement or a statement block (curly braces).
- `break` and `continue` can be used within the body to alter loop execution.

## Examples
```angelscript
// Count from 0 to 9
int i = 0;
while (i < 10)
{
    print("" + i);
    i++;
}

// Process until done
while (hasMoreItems())
{
    processNextItem();
}

// Infinite loop (must break out)
while (true)
{
    if (shouldStop())
        break;
    doWork();
}
```

## Compilation Notes
- **Control flow:** The while loop compiles to the following bytecode pattern:
  1. `loop_start` label: Evaluate the condition expression, leaving a bool on the stack.
  2. Emit a conditional jump-if-false to the `loop_end` label. This consumes the bool.
  3. Emit the loop body bytecode.
  4. Emit an unconditional jump back to `loop_start`.
  5. `loop_end` label.
- **Alternative layout (condition at end):** Some compilers optimize by placing the condition at the end and jumping to it initially:
  1. Emit an unconditional jump to `condition_label`.
  2. `body_label`: Emit the loop body bytecode.
  3. `condition_label`: Evaluate condition. Emit conditional jump-if-true to `body_label`.
  This avoids one unconditional jump per iteration at the cost of one extra jump before the first iteration.
- **Label generation:** Two labels are needed:
  - `loop_start` (or `condition_label`): target for the back-edge jump and for `continue` statements.
  - `loop_end`: target for the condition-false jump and for `break` statements.
- **Stack behavior:** The condition expression pushes one bool, consumed by the conditional jump. The body must leave the stack at the same depth as it started. The stack depth at loop_start must equal the depth at loop_end.
- **Break/continue targets:** The compiler must push the while loop's label pair onto a break/continue target stack so that `break` and `continue` statements inside the body know which labels to jump to. `break` jumps to `loop_end`. `continue` jumps to `loop_start` (condition re-evaluation).
- **Special cases:**
  - `while (true)` generates no condition check bytecode (or the compiler can detect the constant and emit only the body with an unconditional back-edge jump).
  - `while (false)` can be optimized away entirely (dead code).
  - Object variables declared inside the body must be destructed at the end of each iteration.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::While` | While loop statement variant | Wraps `&'ast WhileStmt<'ast>` (arena-allocated reference) |
| `WhileStmt` | While loop structure | `condition: &'ast Expr<'ast>`, `body: &'ast Stmt<'ast>`, `span: Span` |

**Notes:**
- The `body` is a generic `Stmt` reference, which can be either a single statement or a `Stmt::Block`.
- The `condition` is always present (not optional); `while (true)` is represented with a boolean literal expression.

## Related Features
- [Do-While Loop](./do-while-loop.md) - condition checked after body
- [For Loop](./for-loop.md) - compact loop with init/condition/increment
- [Break / Continue](./break-continue.md) - loop control statements
- [Statement Blocks](./statement-blocks.md) - scoping of variables within the loop body
