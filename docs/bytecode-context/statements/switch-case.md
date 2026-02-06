# Switch-Case

## Overview
The `switch` statement provides multi-way branching based on an integer expression. It is more efficient than chained `if-else` statements when comparing a single value against many compile-time constant alternatives, especially when the case values are numerically close together.

## Syntax
```angelscript
switch (expression)
{
case constant1:
    // executed if expression == constant1
    break;

case constant2:
case constant3:
    // executed if expression == constant2 or constant3 (fall-through)
    break;

default:
    // executed if no case matched
}
```

## Semantics
- The switch expression must evaluate to an **integer type** (signed or unsigned). Floating-point, string, and other types are not permitted.
- Each `case` label must be a **compile-time constant**: either an integer literal or a `const` variable that was initialized with a constant expression. If the const variable's initializer cannot be determined at compile time, it cannot be used as a case value.
- Case values must be unique within the same switch statement.
- **Fall-through behavior:** Execution flows sequentially through case labels unless interrupted by a `break` statement. If a case does not end with `break`, execution continues into the next case's body. This is intentional and allows grouping multiple case values to share a single body.
- The `default` label is optional. If present, its body executes when no `case` matches the switch expression. The `default` label is conventionally placed last, though the language does not strictly require it.
- Each `case` body can contain any statements, including nested blocks, variable declarations, and other control flow.

## Examples
```angelscript
void handleEvent(int eventType)
{
    switch (eventType)
    {
    case 0:
        print("none");
        break;

    case 1:
        print("click");
        break;

    case 2:
    case 3:
        // both cases 2 and 3 handled here
        print("drag or drop");
        break;

    default:
        print("unknown event: " + eventType);
    }
}

// Using const variables as case values
const int EVENT_CLICK = 1;
const int EVENT_DRAG  = 2;

void handleEvent2(int eventType)
{
    switch (eventType)
    {
    case EVENT_CLICK:
        onClick();
        break;
    case EVENT_DRAG:
        onDrag();
        break;
    }
}
```

## Compilation Notes
- **Jump table vs. chained comparisons:** When case values are dense (close together numerically), the compiler can generate a **jump table** (indexed array of jump targets) for O(1) dispatch. When case values are sparse, the compiler falls back to a series of compare-and-jump instructions (similar to chained if-else). The reference notes that switch is "much faster than a series of ifs" when cases are close in value, implying jump table generation.
- **Control flow structure:**
  1. Evaluate the switch expression, leaving an integer on the stack.
  2. Depending on the dispatch strategy:
     - **Jump table:** Compute `expression - min_case_value` as an index. Bounds-check against the table size. If out of range, jump to `default` (or end). Otherwise, use the index to look up a jump target in the table.
     - **Chained comparisons:** Compare the expression against each case value in sequence. For each match, jump to the corresponding case body.
  3. Emit case body bytecode with labels at each case entry point.
  4. `break` statements compile to unconditional jumps to the end-of-switch label.
  5. Fall-through is the natural behavior (no jump emitted between consecutive case bodies).
- **Label generation:**
  - One label per `case` (including `default`).
  - One `end_switch` label for `break` targets.
- **Stack behavior:** The switch expression is evaluated once and consumed by the dispatch logic. Each case body must leave the stack at the same depth as the entry point. Break does not affect the stack (it is a simple jump).
- **Fall-through:** No special bytecode is needed for fall-through. The compiler simply does not emit a jump between consecutive case bodies. Only explicit `break` statements generate jumps.
- **Special cases:**
  - A switch with no cases is legal but generates no meaningful dispatch.
  - A switch with only a `default` label unconditionally executes the default body.
  - The compiler must validate that all case values are unique and are compile-time constants.
  - `break` within a switch terminates the switch, not an enclosing loop. The compiler must track the correct break target when switch statements are nested inside loops.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/stmt.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Stmt::Switch` | Switch statement variant | Wraps `&'ast SwitchStmt<'ast>` (arena-allocated reference) |
| `SwitchStmt` | Switch structure | `expr: &'ast Expr<'ast>`, `cases: &'ast [SwitchCase<'ast>]`, `span: Span` |
| `SwitchCase` | Single case clause | `values: &'ast [&'ast Expr<'ast>]`, `stmts: &'ast [Stmt<'ast>]`, `span: Span` |

**Notes:**
- The `default` case is represented as a `SwitchCase` with an empty `values` slice. The helper method `SwitchCase::is_default()` checks `self.values.is_empty()`.
- Multiple case labels sharing the same body (fall-through grouping like `case 1: case 2:`) are represented as a single `SwitchCase` with multiple entries in `values`.
- Each `SwitchCase` contains its own `stmts` slice rather than a single `Stmt`, allowing multiple statements without requiring an explicit block.

## Related Features
- [If-Else](./if-else.md) - alternative conditional branching
- [Break / Continue](./break-continue.md) - break terminates the switch; continue is not applicable to switch
- [Statement Blocks](./statement-blocks.md) - scoping within case bodies
