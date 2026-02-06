# Increment and Decrement Operators

## Overview
The increment (`++`) and decrement (`--`) operators add or subtract 1 from an lvalue. They come in prefix and postfix forms which differ in whether the original or modified value is returned as the expression result.

## Syntax
```angelscript
// Prefix (modify, then return new value)
++lvalue
--lvalue

// Postfix (return old value, then modify)
lvalue++
lvalue--
```

## Semantics
- **Operand:** Must be a numeric lvalue (variable, array element, property, etc.).
- **Increment amount:** Always 1.
- **Prefix (`++i` / `--i`):** The variable is modified first, then the new value is used in the expression.
- **Postfix (`i++` / `i--`):** The current value is captured for use in the expression, then the variable is modified.
- **Result type:** Same as the operand type.
- **Operator precedence:**
  - Postfix `++`/`--` have higher precedence than prefix (level 1 vs level 2 in the unary hierarchy).
  - Post-operators bind tighter than pre-operators since they are closer to the operand.

| Form   | Operation                          | Return Value |
|--------|------------------------------------|-------------|
| `++i`  | `i = i + 1`, then use `i`        | new value   |
| `--i`  | `i = i - 1`, then use `i`        | new value   |
| `i++`  | use `i`, then `i = i + 1`        | old value   |
| `i--`  | use `i`, then `i = i - 1`        | old value   |

## Examples
```angelscript
int i = 5;

// Prefix
int a = ++i;   // i becomes 6, a = 6
int b = --i;   // i becomes 5, b = 5

// Postfix
int c = i++;   // c = 5 (old value), i becomes 6
int d = i--;   // d = 6 (old value), i becomes 5

// In loops
for (int j = 0; j < 10; j++) {
    // j incremented after each iteration body
}

// In array indexing
array<int> arr = {10, 20, 30};
int idx = 0;
int val = arr[idx++];  // val = arr[0] = 10, idx becomes 1
```

## Compilation Notes
- **Stack behavior:**
  - **Prefix:** Load the lvalue address. Increment/decrement the value at that address. Push the new value onto the stack.
  - **Postfix:** Load the lvalue address. Push the current value onto the stack (for use by the surrounding expression). Then increment/decrement the value at the address. The old value remains on the stack as the expression result.
- **Type considerations:** The operand must be a numeric type. The increment/decrement amount is always 1 of the same type. No implicit conversion is needed.
- **Control flow:** No branching involved; these are straightforward load-modify-store operations.
- **Special cases:**
  - Postfix requires saving the original value before modification. This may require a temporary variable or stack duplication in the bytecode.
  - When the result of a postfix expression is not used (e.g., standalone `i++;`), the compiler can optimize away the temporary save and treat it like prefix.
  - Object types may overload increment/decrement via `opPreInc`, `opPreDec`, `opPostInc`, `opPostDec`.
  - Multiple increments on the same variable within a single expression (e.g., `i++ + ++i`) produce implementation-defined behavior and should be avoided.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Unary` | Unary prefix expression variant | Wraps `&UnaryExpr` |
| `UnaryExpr` | Unary prefix operation | `op: UnaryOp`, `operand: &Expr`, `span: Span` |
| `UnaryOp::PreInc` | `++` prefix increment | binding_power: 25 |
| `UnaryOp::PreDec` | `--` prefix decrement | binding_power: 25 |
| `Expr::Postfix` | Postfix expression variant | Wraps `&PostfixExpr` |
| `PostfixExpr` | Postfix operation | `operand: &Expr`, `op: PostfixOp`, `span: Span` |
| `PostfixOp::PostInc` | `++` postfix increment | binding_power: 27 |
| `PostfixOp::PostDec` | `--` postfix decrement | binding_power: 27 |

**Notes:**
- Prefix and postfix increment/decrement use different AST nodes: `Expr::Unary`/`UnaryExpr` for prefix vs `Expr::Postfix`/`PostfixExpr` for postfix.
- `PostfixOp::binding_power()` returns 27 (highest precedence), while `UnaryOp::binding_power()` returns 25. This ensures postfix operators bind tighter than prefix operators.
- The `++`/`--` token is disambiguated by the parser based on position: prefix when appearing before an expression, postfix when appearing after.

## Related Features
- [math-operators.md](math-operators.md) - Addition and subtraction operators
- [compound-assignments.md](compound-assignments.md) - `+=` and `-=` as alternatives
- [indexing-operator.md](indexing-operator.md) - Common use in index expressions
