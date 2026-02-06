# Logic Operators

## Overview
Logic operators perform boolean logic on `bool` operands. AngelScript supports logical NOT, AND, OR, and XOR, each with both keyword and symbol syntax. The AND and OR operators use short-circuit evaluation, meaning the right operand is only evaluated when necessary.

## Syntax
```angelscript
// Keyword syntax
not expr
a and b
a or b
a xor b

// Symbol syntax (equivalent)
!expr
a && b
a || b
a ^^ b
```

## Semantics
- **Operand types:** All operands must be of type `bool`. Non-boolean types are not implicitly converted to bool.
- **Result type:** Always `bool`.
- **Logical NOT (`not` / `!`):** Unary operator. Returns `true` if the operand is `false`, and vice versa.
- **Logical AND (`and` / `&&`):** Returns `true` only if both operands are `true`. Uses short-circuit evaluation: if the left operand is `false`, the right operand is **not evaluated**.
- **Logical OR (`or` / `||`):** Returns `true` if at least one operand is `true`. Uses short-circuit evaluation: if the left operand is `true`, the right operand is **not evaluated**.
- **Logical XOR (`xor` / `^^`):** Returns `true` if exactly one operand is `true`. Does **not** short-circuit (both operands are always evaluated).

**Operator precedence (high to low):**
- `not` / `!` (unary, level 2, right-to-left)
- `and` / `&&` (level 12, left-to-right)
- `xor` / `^^` (level 11, grouped with `==`, `!=`, `is`, `!is`)
- `or` / `||` (level 14, left-to-right)

| Operator    | Alt   | Description         | Left   | Right  | Result |
|-------------|-------|---------------------|--------|--------|--------|
| `not`       | `!`   | logical NOT         |        | `bool` | `bool` |
| `and`       | `&&`  | logical AND         | `bool` | `bool` | `bool` |
| `or`        | `\|\|`| logical OR          | `bool` | `bool` | `bool` |
| `xor`       | `^^`  | logical exclusive OR| `bool` | `bool` | `bool` |

## Examples
```angelscript
bool a = true;
bool b = false;

bool r1 = !a;          // false
bool r2 = a && b;      // false
bool r3 = a || b;      // true
bool r4 = a ^^ b;      // true

// Short-circuit evaluation
if (obj !is null and obj.isValid()) {
    // obj.isValid() only called if obj is not null
}

if (a or b) {
    // b not evaluated if a is true
}

// Keyword and symbol forms are interchangeable
if (not a and b or c) { }
if (!a && b || c) { }    // equivalent
```

## Compilation Notes
- **Control flow:** Short-circuit evaluation requires conditional branching:
  - For `a && b`: Evaluate `a`. If `false`, jump to the end with result `false` (skip `b`). Otherwise, evaluate `b` and use its value as the result.
  - For `a || b`: Evaluate `a`. If `true`, jump to the end with result `true` (skip `b`). Otherwise, evaluate `b` and use its value as the result.
  - For `a ^^ b`: Both operands are always evaluated. No short-circuit branching needed. Compare the two values; result is `true` if they differ.
- **Stack behavior:**
  - NOT: Pop one bool, push the negated result.
  - AND (short-circuit): Evaluate LHS and push. Conditional jump if false (leave false on stack). Otherwise, pop LHS, evaluate RHS and push.
  - OR (short-circuit): Evaluate LHS and push. Conditional jump if true (leave true on stack). Otherwise, pop LHS, evaluate RHS and push.
  - XOR: Evaluate LHS and push. Evaluate RHS and push. Pop both, push XOR result.
- **Type considerations:** Operands must be `bool`. No implicit conversion from integer or handle types to `bool` is performed. If a non-bool expression is used, it is a compile-time error.
- **Special cases:**
  - Short-circuit evaluation means the right operand's side effects (function calls, increments, etc.) may or may not execute depending on the left operand's value. The bytecode must faithfully implement this.
  - When used in `if`/`while` conditions, the compiler may optimize by directly branching on the condition rather than materializing a bool value on the stack.
  - Nested short-circuit expressions (e.g., `a && b && c || d`) require careful nesting of conditional jumps with correct target labels.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::LogicalOr` | `\|\|` / `or` logical OR | binding_power: (3, 4) |
| `BinaryOp::LogicalXor` | `^^` / `xor` logical XOR | binding_power: (3, 4) |
| `BinaryOp::LogicalAnd` | `&&` / `and` logical AND | binding_power: (5, 6) |
| `Expr::Unary` | Unary prefix expression variant | Wraps `&UnaryExpr` |
| `UnaryExpr` | Unary prefix operation | `op: UnaryOp`, `operand: &Expr`, `span: Span` |
| `UnaryOp::LogicalNot` | `!` / `not` logical NOT | binding_power: 25 |

**Notes:**
- In the AST, `LogicalOr` and `LogicalXor` share the same binding_power `(3, 4)`. This means they are at the same precedence level and are left-associative.
- The doc states `xor`/`^^` is at a different level than `or`/`||` (level 11 vs level 14), but the AST groups them together at `(3, 4)`. The AST binding_power values are the authoritative source for the parser's behavior.
- `LogicalAnd` has higher precedence `(5, 6)` than `LogicalOr`/`LogicalXor` `(3, 4)`, matching the expected `&&` binds tighter than `||` behavior.
- Keyword forms (`and`, `or`, `not`, `xor`) and symbol forms (`&&`, `||`, `!`, `^^`) parse to the same AST variants.

## Related Features
- [equality-comparison.md](equality-comparison.md) - Value equality operators
- [identity-comparison.md](identity-comparison.md) - Handle identity operators
- [relational-comparison.md](relational-comparison.md) - Ordered comparison operators
- [conditional-expression.md](conditional-expression.md) - Ternary operator using bool condition
