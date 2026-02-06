# Identity Comparison Operators

## Overview
The identity comparison operators `is` and `!is` compare the identity (address) of two object handles. Unlike `==`/`!=` which compare values, identity comparison checks whether two handles refer to the exact same object instance. These operators are only valid for reference types (handles).

## Syntax
```angelscript
a is b      // true if a and b reference the same object
a !is b     // true if a and b reference different objects

a is null   // true if a is a null handle
a !is null  // true if a is not a null handle
```

## Semantics
- **Operand types:** Both operands must be object handles (reference types). One operand may be `null`.
- **Result type:** Always `bool`.
- **Comparison mechanism:** Compares the memory addresses of the referenced objects. No methods are called, no value comparison occurs.
- **`is`:** Returns `true` if both handles point to the exact same object (same address), or both are `null`.
- **`!is`:** Returns `true` if the handles point to different objects, or exactly one is `null`.
- **Equivalence:** `@a == @b` has the same meaning as `a is b` (explicit handle-of comparison).
- **Null checking:** The idiomatic way to check whether a handle is null in AngelScript is `handle is null` or `handle !is null`.
- **Operator precedence:** Level 11, left-to-right associative. Shares precedence with `==`, `!=`, and `xor`/`^^`.

| Operator | Description     | Left     | Right    | Result |
|----------|----------------|----------|----------|--------|
| `is`     | same object    | handle   | handle   | `bool` |
| `!is`    | different object| handle  | handle   | `bool` |

## Examples
```angelscript
obj@ a = obj();
obj@ b = a;          // b references same object as a
obj@ c = obj();      // c references a different object

bool same = (a is b);     // true - same object
bool diff = (a is c);     // false - different objects
bool notSame = (a !is c); // true

// Null checking
obj@ handle;
if (handle is null) {
    // handle is not yet assigned
}

// Safe access pattern
if (handle !is null) {
    handle.doSomething();
}

// Contrast with value equality
obj@ x = obj();
obj@ y = obj();
// x is y    -> false (different objects)
// x == y    -> possibly true if opEquals returns true
```

## Compilation Notes
- **Stack behavior:** Left operand handle (address) is pushed, then right operand handle (address) is pushed. The identity comparison instruction pops both addresses and pushes a `bool` result.
- **Type considerations:**
  - No value dereferencing or method calls are needed. The comparison operates directly on pointer/address values.
  - If one operand is `null`, the compiler can emit a simple null-check instruction on the other operand's address.
  - Handle types with different base types may require checking whether a common base exists for the comparison to be valid.
- **Control flow:** When used in `if`/`while` conditions (especially null checks), the compiler may optimize by fusing the identity comparison into a conditional branch (e.g., "branch if null" / "branch if not null").
- **Special cases:**
  - `!is` can be compiled as `is` followed by logical NOT, or as a single "not identical" instruction.
  - Identity comparison should be very fast (single pointer comparison) and does not trigger any reference counting changes.
  - This operator cannot be overloaded; it always compares addresses.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::Is` | `is` identity comparison | binding_power: (13, 14) |
| `BinaryOp::NotIs` | `!is` non-identity comparison | binding_power: (13, 14) |

**Notes:**
- `Is` and `NotIs` share binding_power `(13, 14)` with `Equal` and `NotEqual`, all at the same precedence level and left-associative.
- `BinaryOp::is_comparison()` returns `true` for both `Is` and `NotIs`.
- Despite being semantically distinct (address comparison vs value comparison), identity and equality operators are at the same precedence level in the parser.

## Related Features
- [equality-comparison.md](equality-comparison.md) - `==` / `!=` for value comparison
- [handle-of.md](handle-of.md) - `@` operator and handle semantics
- [logic-operators.md](logic-operators.md) - Combining identity checks with boolean logic
