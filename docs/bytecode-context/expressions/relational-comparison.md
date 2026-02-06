# Relational Comparison Operators

## Overview
The relational comparison operators `<`, `>`, `<=`, and `>=` compare two values to determine their ordering relationship. These operators always produce a `bool` result and are used with ordered types (numeric types and objects that define `opCmp`).

## Syntax
```angelscript
a < b     // less than
a > b     // greater than
a <= b    // less than or equal
a >= b    // greater than or equal
```

## Semantics
- **Result type:** Always `bool`.
- **Primitive types:** Direct numeric comparison. If operands differ in type, implicit promotion is applied (same rules as arithmetic operators) before comparison.
- **Object types:** The `opCmp` method is called on the object. It returns a negative value if `this < other`, zero if equal, and a positive value if `this > other`. If no `opCmp` is defined, relational comparison is a compile-time error for that type.
- **Type coercion:** When operands differ in type, the compiler attempts implicit conversion following the least-cost principle.
- **Operator precedence:** Level 10, left-to-right associative. Higher precedence than equality operators.

| Operator | Description          | Left | Right | Result |
|----------|---------------------|------|-------|--------|
| `<`      | less than           | any  | any   | `bool` |
| `>`      | greater than        | any  | any   | `bool` |
| `<=`     | less than or equal  | any  | any   | `bool` |
| `>=`     | greater than or equal | any | any   | `bool` |

## Examples
```angelscript
int a = 3;
int b = 5;

bool r1 = (a < b);     // true
bool r2 = (a > b);     // false
bool r3 = (a <= 3);    // true
bool r4 = (a >= 5);    // false

// Type promotion
float f = 2.5f;
if (a > f) { }         // a promoted to float, then compared

// Range check
if (x >= 0 && x < 100) {
    // x is in range [0, 100)
}
```

## Compilation Notes
- **Stack behavior:** Left operand is evaluated and pushed, then right operand is evaluated and pushed. The comparison instruction pops both and pushes a `bool` result.
- **Type considerations:**
  - For primitive types with differing widths, implicit conversion bytecodes are inserted before the comparison instruction.
  - For object types, the comparison compiles to a call to `opCmp`, followed by a comparison of the return value against zero.
  - Separate instructions or instruction variants may exist for each comparison direction (less-than, greater-than, etc.), or they may all reduce to `opCmp` + zero comparison.
- **Control flow:** When used directly in an `if`/`while` condition, the compiler may optimize by fusing the comparison and branch into a single conditional jump instruction.
- **Special cases:**
  - Floating-point NaN comparisons: All relational comparisons involving NaN return `false` (NaN is unordered).
  - Unsigned integer comparison requires different instructions than signed comparison on most backends.
  - Objects without `opCmp` cannot use relational operators; this is enforced at compile time.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::Less` | `<` less than | binding_power: (15, 16) |
| `BinaryOp::LessEqual` | `<=` less than or equal | binding_power: (15, 16) |
| `BinaryOp::Greater` | `>` greater than | binding_power: (15, 16) |
| `BinaryOp::GreaterEqual` | `>=` greater than or equal | binding_power: (15, 16) |

**Notes:**
- All four relational operators share binding_power `(15, 16)`, left-associative.
- They have higher precedence than equality/identity operators at `(13, 14)`, matching the standard expectation that `a < b == c < d` parses as `(a < b) == (c < d)`.
- `BinaryOp::is_comparison()` returns `true` for all four relational variants.

## Related Features
- [equality-comparison.md](equality-comparison.md) - `==` / `!=` for value equality
- [identity-comparison.md](identity-comparison.md) - `is` / `!is` for handle identity
- [logic-operators.md](logic-operators.md) - Combining comparisons with `&&`, `||`
- [type-conversions.md](type-conversions.md) - Implicit promotion for mixed types
