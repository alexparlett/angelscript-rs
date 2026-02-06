# Break and Continue

## Overview
`break` and `continue` are loop control statements that alter the normal flow of loop execution. `break` terminates the enclosing loop or switch statement entirely, while `continue` skips the rest of the current iteration and proceeds to the next one. Both operate on the **innermost** enclosing construct.

## Syntax
```angelscript
break;
continue;
```

## Semantics

### break
- Terminates the **smallest enclosing loop statement** (`while`, `do-while`, `for`) **or switch statement**.
- Execution continues with the statement immediately following the terminated loop or switch.
- `break` can only appear inside a loop or switch body. Using it outside of these constructs is a compile-time error.
- When nested inside multiple loops/switches, `break` only affects the innermost one.
- AngelScript does not have labeled break (cannot break out of multiple levels at once).

### continue
- Jumps to the **next iteration** of the smallest enclosing loop statement (`while`, `do-while`, `for`).
- `continue` is **not applicable** to switch statements. A `continue` inside a switch that is inside a loop will jump to the next iteration of the enclosing loop, not affect the switch.
- For each loop type, `continue` jumps to a different point:
  - **while:** jumps to the condition evaluation.
  - **do-while:** jumps to the condition evaluation (at the bottom).
  - **for:** jumps to the **increment section**, which then proceeds to the condition evaluation.
- `continue` can only appear inside a loop body. Using it outside of a loop is a compile-time error.

## Examples
```angelscript
// break exits the loop
for (;;)
{
    if (condition)
        break;  // exits the infinite loop
    doWork();
}

// continue skips the current iteration
for (int n = 0; n < 10; n++)
{
    if (n == 5)
        continue;  // skip processing when n is 5
    process(n);     // executed for n = 0,1,2,3,4,6,7,8,9
}

// break in a switch inside a loop
for (int i = 0; i < 10; i++)
{
    switch (i)
    {
    case 5:
        break;   // exits the switch, NOT the for loop
    default:
        process(i);
    }
    // execution continues here after the switch break
}

// continue in a switch inside a loop
for (int i = 0; i < 10; i++)
{
    switch (getAction(i))
    {
    case SKIP:
        continue;  // jumps to i++ of the for loop (skips rest of switch AND loop body)
    case PROCESS:
        process(i);
        break;      // exits the switch only
    }
    postProcess(i);  // skipped by continue, reached after switch break
}
```

## Compilation Notes
- **Control flow:** Both `break` and `continue` compile to unconditional jump instructions targeting labels maintained by the enclosing loop or switch.
- **Break target resolution:** The compiler maintains a stack (or chain) of break targets. Each loop and switch statement pushes its `end` label onto this stack upon entry and pops it upon exit. When `break` is encountered, the compiler emits a jump to the top of the break target stack.
- **Continue target resolution:** The compiler maintains a separate stack of continue targets. Only loop statements push continue targets (switches do not). The continue target differs by loop type:
  - **while:** `loop_start` label (condition evaluation).
  - **do-while:** `condition_label` (condition evaluation at the bottom).
  - **for:** `continue_label` (increment section, which precedes the condition).
- **Stack cleanup before jump:** If `break` or `continue` exits one or more nested scopes, the compiler must emit cleanup bytecode **before** the jump. This includes:
  - Calling destructors for object variables that are going out of scope.
  - Releasing handles that are going out of scope.
  - Adjusting the stack pointer if local variables were allocated on the evaluation stack.
  The compiler must walk the scope chain from the current scope outward to the target loop/switch scope and emit cleanup for each scope being exited.
- **Nested constructs:** When loops and switches are nested, the break and continue target stacks have multiple entries. The compiler always resolves to the innermost target. For example, `break` inside a switch inside a for loop breaks the switch (not the loop). `continue` inside a switch inside a for loop continues the for loop (since switch does not have a continue target).
- **Validation:** The compiler must verify at compile time that `break` appears only inside a loop or switch, and `continue` appears only inside a loop. Otherwise, a compile error is emitted.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Break` | Break statement variant | Wraps `BreakStmt` (by value) |
| `BreakStmt` | Break statement structure | `span: Span` |
| `Stmt::Continue` | Continue statement variant | Wraps `ContinueStmt` (by value) |
| `ContinueStmt` | Continue statement structure | `span: Span` |

**Notes:**
- Both `BreakStmt` and `ContinueStmt` contain only a `span` field -- they carry no additional data in the AST.
- The target resolution (which loop or switch to break/continue from) is handled during compilation, not in the AST.
- AngelScript does not support labeled break/continue, so there is no label field.

## Related Features
- [While Loop](./while-loop.md) - break exits the loop, continue re-evaluates condition
- [Do-While Loop](./do-while-loop.md) - break exits the loop, continue jumps to condition
- [For Loop](./for-loop.md) - break exits the loop, continue jumps to increment
- [Switch-Case](./switch-case.md) - break exits the switch, continue not applicable to switch itself
