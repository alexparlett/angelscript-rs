# For Loop

## Overview
The `for` loop is a compact form of a counted or iterating loop that combines initialization, condition checking, and iteration advancement into a single statement. Variables declared in the initialization section are scoped to the loop.

## Syntax
```angelscript
// Standard for loop
for (init; condition; increment)
{
    // body
}

// Typical usage
for (int i = 0; i < 10; i++)
{
    // body
}

// Multiple variable declarations in init
for (int a = 0, b = 10; a < b; a++, b--)
{
    // body
}

// Empty sections (infinite loop)
for (;;)
{
    // body (must use break to exit)
}

// Empty condition (always true)
for (int i = 0; ; i++)
{
    if (i >= 10) break;
}

// Single-statement body
for (int i = 0; i < 10; i++)
    doSomething(i);
```

## Semantics
- The for loop has three sections separated by semicolons:
  1. **Init** (before first `;`): Executed exactly once before the loop begins. Can contain variable declarations or expressions. Variables declared here are scoped to the entire for statement (visible in the condition, increment, and body). Multiple variable declarations are separated by commas, and all must have the same type.
  2. **Condition** (between the two `;`s): Evaluated before each iteration. Must evaluate to `bool`. If `true`, the body executes. If `false`, the loop terminates. An **empty condition** is treated as `true` (infinite loop).
  3. **Increment** (after second `;`): Executed after each iteration of the body, before the condition is re-evaluated. Multiple increment expressions are separated by commas.
- The execution order is: init -> [condition -> body -> increment] -> [condition -> body -> increment] -> ... until condition is false.
- `break` and `continue` can be used within the body. `continue` jumps to the increment section (not directly to the condition).

## Examples
```angelscript
// Basic counting loop
for (int i = 0; i < 10; i++)
{
    print("i = " + i);
}

// Multiple variables and increments
for (int lo = 0, hi = 100; lo < hi; lo++, hi--)
{
    print("lo=" + lo + " hi=" + hi);
}

// Infinite loop with break
for (;;)
{
    if (isDone())
        break;
    processNext();
}

// Iterating with step
for (int i = 0; i < 100; i += 10)
{
    print("" + i);
}
```

## Compilation Notes
- **Control flow:** The for loop is semantically equivalent to:
  ```
  {
      init;
      while (condition) {
          body;
          increment;
      }
  }
  ```
  The compiler generates bytecode following this pattern:
  1. Enter a new scope (for init variables).
  2. Emit init bytecode (variable declarations and/or expressions).
  3. `condition_label`: Evaluate condition (if not empty), leaving bool on stack.
  4. Emit conditional jump-if-false to `loop_end`.
  5. Emit body bytecode.
  6. `continue_label`: Emit increment bytecode.
  7. Emit unconditional jump to `condition_label`.
  8. `loop_end` label.
  9. Exit scope (destruct/release init variables).
- **Alternative layout (condition at end):** As with while loops, the compiler may place the condition at the end:
  1. Emit init bytecode.
  2. Emit unconditional jump to `condition_label`.
  3. `body_label`: Emit body bytecode.
  4. `continue_label`: Emit increment bytecode.
  5. `condition_label`: Evaluate condition. Emit conditional jump-if-true to `body_label`.
  6. `loop_end` label.
- **Label generation:** Three labels are needed:
  - `condition_label`: target for the back-edge jump (re-evaluating the condition).
  - `continue_label`: target for `continue` statements. This is the **increment section**, not the condition. This is critical -- continue in a for loop must execute the increment before re-checking the condition.
  - `loop_end`: target for the condition-false jump and for `break` statements.
- **Variable scoping:** Variables declared in the init section are scoped to the for loop. The compiler creates a scope that encompasses the entire for statement. These variables must be destructed/released when the loop ends (at `loop_end`), whether by normal termination or `break`.
- **Stack behavior:** The init section may push variable allocations. The condition pushes one bool, consumed by the conditional jump. The increment section evaluates expressions whose results are discarded. All three sections must leave the stack balanced per iteration.
- **Multiple declarations:** Multiple declarations in the init section (`int a = 0, b = 10`) are processed sequentially, each allocating a separate variable slot. All must share the same type.
- **Multiple increment expressions:** Multiple increment expressions (`a++, b--`) are evaluated left to right. Each expression's result is discarded.
- **Empty sections:**
  - Empty init: no bytecode emitted for init.
  - Empty condition: no condition bytecode; the back-edge is an unconditional jump (equivalent to `while (true)`).
  - Empty increment: no increment bytecode; `continue` jumps directly to the condition.
- **Special cases:**
  - `for (;;)` compiles to a simple unconditional loop (body + unconditional jump back).
  - Object variables declared in the init or body must be properly destructed when the loop exits.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::For` | For loop statement variant | Wraps `&'ast ForStmt<'ast>` (arena-allocated reference) |
| `ForStmt` | For loop structure | `init: Option<ForInit<'ast>>`, `condition: Option<&'ast Expr<'ast>>`, `update: &'ast [&'ast Expr<'ast>]`, `body: &'ast Stmt<'ast>`, `span: Span` |
| `ForInit` | For loop initializer (enum) | `VarDecl(VarDeclStmt<'ast>)` or `Expr(&'ast Expr<'ast>)` |

**Notes:**
- The `init` field is `Option<ForInit>` to handle `for (;;)` where init is omitted.
- `ForInit` is an enum distinguishing variable declarations (`int i = 0`) from expressions (`i = 0`) in the initializer.
- The `condition` is `Option` to represent empty conditions (treated as always true).
- The `update` field is a slice of expression references, supporting multiple comma-separated update expressions (`i++, j--`).
- `ForInit::VarDecl` reuses the same `VarDeclStmt` type used by standalone variable declarations.

## Related Features
- [While Loop](./while-loop.md) - equivalent to for with only a condition
- [Do-While Loop](./do-while-loop.md) - body-first loop
- [Break / Continue](./break-continue.md) - continue jumps to increment, break exits the loop
- [Variable Declarations](./variable-declarations.md) - variables declared in init are scoped to the loop
- [Statement Blocks](./statement-blocks.md) - the for loop creates an implicit scope
