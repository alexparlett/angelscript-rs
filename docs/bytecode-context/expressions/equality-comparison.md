# Equality Comparison Operators

## Overview
The equality comparison operators `==` and `!=` compare two values to determine if they are equal or not equal. These perform value comparison (not identity comparison) and always produce a `bool` result. For object types, equality comparison invokes the `opEquals` method.

## Syntax
```angelscript
a == b    // equal to
a != b    // not equal to
```

## Semantics
- **Result type:** Always `bool`.
- **Primitive types:** Direct value comparison. If operands differ in type, implicit promotion is applied (same rules as arithmetic operators) before comparison.
- **Object types:** The `opEquals` method is called on the object. If no `opEquals` is defined, the comparison is a compile-time error for that type.
- **Handle types:** When comparing handles with `==` / `!=`, value equality is checked (calls `opEquals` on the referenced objects). To check identity (same object), use `is` / `!is` instead. However, `@a == @b` is equivalent to `a is b` (comparing handle addresses).
- **Null comparison:** Handles can be compared to `null` using `==` or `!=`, but `is` / `!is` is the idiomatic way to check for null handles.
- **Type coercion:** When operands are of different types, the compiler attempts implicit conversion following the least-cost principle. If no valid conversion exists, it is a compile-time error.
- **Operator precedence:** Level 11, left-to-right associative. Shares precedence with `is`, `!is`, and `xor`/`^^`.

| Operator | Description | Left | Right | Result |
|----------|-------------|------|-------|--------|
| `==`     | equal       | any  | any   | `bool` |
| `!=`     | not equal   | any  | any   | `bool` |

## Examples
```angelscript
int a = 5;
int b = 5;
bool eq = (a == b);     // true
bool neq = (a != b);    // false

// Type promotion
float f = 5.0f;
if (a == f) { }         // a promoted to float, then compared

// Object equality (calls opEquals)
string s1 = "hello";
string s2 = "hello";
if (s1 == s2) { }       // true - value equality

// Handle equality vs identity
obj@ h1 = obj();
obj@ h2 = h1;
if (h1 == h2) { }       // value equality via opEquals
if (h1 is h2) { }       // identity: true (same object)
```

## Compilation Notes
- **Stack behavior:** Left operand is evaluated and pushed, then right operand is evaluated and pushed. The comparison instruction pops both and pushes a `bool` result.
- **Type considerations:**
  - For primitive types with differing widths, implicit conversion bytecodes are inserted before the comparison instruction.
  - For object types, the comparison compiles to a method call to `opEquals` rather than a primitive comparison instruction.
  - `!=` can be compiled as `==` followed by a logical NOT, or as a distinct instruction if the backend supports it.
- **Control flow:** When used directly in an `if`/`while` condition, the compiler may optimize by fusing the comparison and branch into a single conditional jump instruction rather than materializing the bool.
- **Special cases:**
  - Comparing floating-point values: NaN is not equal to itself (`NaN != NaN` is true, `NaN == NaN` is false) per IEEE 754 rules.
  - Null handle comparison: When one operand is `null`, the comparison reduces to a null-check on the other operand's address.
  - Object types without `opEquals` cannot use `==`/`!=`; this is enforced at compile time.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::Equal` | `==` equal | binding_power: (13, 14) |
| `BinaryOp::NotEqual` | `!=` not equal | binding_power: (13, 14) |

**Notes:**
- `Equal` and `NotEqual` share binding_power `(13, 14)`, left-associative.
- They also share the same precedence level with `BinaryOp::Is` and `BinaryOp::NotIs` (identity comparison), all at `(13, 14)`.
- `BinaryOp::is_comparison()` returns `true` for both `Equal` and `NotEqual`.

## Related Features
- [identity-comparison.md](identity-comparison.md) - `is` / `!is` for handle identity
- [relational-comparison.md](relational-comparison.md) - `<`, `>`, `<=`, `>=` for ordering
- [type-conversions.md](type-conversions.md) - Implicit promotion rules for mixed types
- [logic-operators.md](logic-operators.md) - Combining comparison results with boolean logic
