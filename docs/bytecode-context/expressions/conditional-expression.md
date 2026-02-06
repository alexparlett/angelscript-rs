# Conditional Expression (Ternary Operator)

## Overview
The conditional expression (ternary operator) selects between two values based on a boolean condition. It is the only ternary operator in AngelScript. Both branches must be of compatible types, and the expression can even be used as an lvalue under certain conditions.

## Syntax
```angelscript
condition ? exprIfTrue : exprIfFalse
```

## Semantics
- **Condition:** Must evaluate to `bool`. The condition is evaluated first.
- **Branch selection:** If the condition is `true`, only `exprIfTrue` is evaluated and returned. If `false`, only `exprIfFalse` is evaluated and returned.
- **Type compatibility:** Both branches must be of the same type or implicitly convertible to a common type. If they differ, the compiler applies the least-cost conversion principle:
  - The branch that costs less to convert is the one that gets converted.
  - If both conversions cost the same, or no valid conversion exists, it is a compile-time error.
- **Lvalue usage:** The conditional expression can be used as an lvalue (on the left side of an assignment) if both branches are lvalues of the same type.
- **Operator precedence:** Level 15, right-to-left associative. Lower than all binary operators except assignment.

## Examples
```angelscript
// Basic usage
int max = (a > b) ? a : b;

// Nested ternary
string grade = (score >= 90) ? "A" :
               (score >= 80) ? "B" :
               (score >= 70) ? "C" : "F";

// As lvalue
int x, y;
(useX ? x : y) = 42;   // assigns 42 to x or y based on condition

// Type conversion between branches
int a = 5;
float b = 3.0f;
auto result = true ? a : b;  // a converted to float, result is float
```

## Compilation Notes
- **Control flow:** The conditional expression requires branching:
  1. Evaluate the condition.
  2. If `false`, jump to the "else" branch (skip the "true" branch).
  3. Evaluate the "true" branch expression, then jump past the "false" branch.
  4. Label: evaluate the "false" branch expression.
  5. Label: continue with the result on the stack.

  This is structurally identical to an `if/else` but in expression context.

- **Stack behavior:**
  - Condition is evaluated and used for a conditional branch (consumed by the branch instruction).
  - Only one of the two branch expressions is evaluated and its value pushed onto the stack.
  - After the branch merge point, exactly one value is on the stack regardless of which path was taken.

- **Type considerations:**
  - If the two branches have different types, the compiler inserts conversion bytecodes in the appropriate branch before the merge point, so both paths produce the same type on the stack.
  - The conversion follows the least-cost principle used in overload resolution.

- **Special cases:**
  - **Lvalue conditional:** When used as an lvalue, the compiler must evaluate the condition, then compute the address of the selected branch's lvalue, and use that address for the subsequent store. Both branches must be lvalues of the exact same type (no conversion allowed in lvalue context).
  - **Nested ternary:** Right-to-left associativity means `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`. The bytecode generator handles this through recursive compilation of the "false" branch.
  - **Dead code in branches:** Since only one branch executes, side effects in the non-taken branch do not occur. The compiler must not optimize away either branch at compile time unless the condition is a constant.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Ternary` | Ternary conditional expression variant | Wraps `&TernaryExpr` |
| `TernaryExpr` | Ternary conditional (? :) | `condition: &Expr`, `then_expr: &Expr`, `else_expr: &Expr`, `span: Span` |

**Notes:**
- The ternary operator is the only expression type in the AST with three sub-expressions.
- Right-to-left associativity is implemented by the Pratt parser: nested ternary in the else branch (`a ? b : c ? d : e`) naturally parses as `a ? b : (c ? d : e)`.
- The ternary operator's binding power is not stored on a `BinaryOp` variant; it is handled as a special case in the Pratt parser, sitting between assignment `(2, 1)` and `LogicalOr`/`LogicalXor` `(3, 4)`.

## Related Features
- [logic-operators.md](logic-operators.md) - Boolean conditions and short-circuit evaluation
- [type-conversions.md](type-conversions.md) - Least-cost conversion between branch types
- [assignments.md](assignments.md) - Assigning to conditional lvalues
