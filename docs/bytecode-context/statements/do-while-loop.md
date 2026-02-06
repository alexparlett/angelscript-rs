# Do-While Loop

## Overview
The `do-while` loop executes a body of statements and then checks a condition to determine whether to repeat. Because the condition is checked **after** the body, the body is guaranteed to execute at least once.

## Syntax
```angelscript
do
{
    // body
} while (condition);

// Single-statement body (no braces)
do
    statement;
while (condition);
```

## Semantics
- The body is executed **before** the condition is evaluated. This guarantees at least one execution of the body.
- The condition expression must evaluate to `bool` (`true` or `false`).
- After the body executes, the condition is evaluated. If `true`, execution jumps back to the start of the body. If `false`, the loop terminates and execution continues with the statement following the `while(condition);`.
- The entire statement is terminated by a semicolon after the closing parenthesis of the condition.
- `break` and `continue` can be used within the body. `continue` jumps to the condition check (not back to the top of the body).

## Examples
```angelscript
// Execute at least once, continue while condition holds
int j = 0;
do
{
    print("" + j);
    j++;
} while (j < 10);

// Read input at least once
string input;
do
{
    input = readLine();
    process(input);
} while (input != "quit");
```

## Compilation Notes
- **Control flow:** The do-while loop compiles to a simpler bytecode pattern than the while loop because the condition is at the end:
  1. `loop_start` label: Emit the loop body bytecode.
  2. `condition_label`: Evaluate the condition expression, leaving a bool on the stack.
  3. Emit a conditional jump-if-true back to `loop_start`. This consumes the bool.
  This pattern requires only one conditional jump per iteration (no unconditional jump needed), making it slightly more efficient than a while loop.
- **Label generation:** Two or three labels are needed:
  - `loop_start`: target for the conditional back-edge jump. Also the target for `break` resolution scope start.
  - `condition_label`: target for `continue` statements (they skip to the condition, not back to the body start).
  - `loop_end` (implicit, the instruction after the conditional jump): target for `break` statements.
- **Stack behavior:** The condition pushes one bool, consumed by the conditional jump. The body must leave the stack at the same depth each iteration.
- **Break/continue targets:**
  - `break` jumps to `loop_end` (the instruction after the conditional jump-if-true).
  - `continue` jumps to `condition_label` (the condition evaluation), not to `loop_start`. This is a key difference from the while loop where continue goes to the condition as well, but here the condition is at the bottom.
- **Special cases:**
  - `do { ... } while (true);` is an infinite loop. The compiler can emit the conditional jump-if-true as an unconditional jump.
  - `do { ... } while (false);` executes the body exactly once. The compiler can optimize away the condition check and back-edge jump entirely.
  - Object variables declared inside the body must be destructed at the end of each iteration before the condition is evaluated.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::DoWhile` | Do-while loop statement variant | Wraps `&'ast DoWhileStmt<'ast>` (arena-allocated reference) |
| `DoWhileStmt` | Do-while loop structure | `body: &'ast Stmt<'ast>`, `condition: &'ast Expr<'ast>`, `span: Span` |

**Notes:**
- The field order in `DoWhileStmt` mirrors the execution order: `body` first, then `condition`.
- Like `WhileStmt`, `body` is a generic `Stmt` reference and `condition` is always present (not optional).

## Related Features
- [While Loop](./while-loop.md) - condition checked before body
- [For Loop](./for-loop.md) - compact loop with init/condition/increment
- [Break / Continue](./break-continue.md) - loop control statements
- [Statement Blocks](./statement-blocks.md) - scoping of variables within the loop body
