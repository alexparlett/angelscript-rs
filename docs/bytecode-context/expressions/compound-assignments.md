# Compound Assignments

## Overview
Compound assignment operators combine an arithmetic, bitwise, or shift operation with an assignment in a single expression. The key advantage is that the lvalue is evaluated only once, which is both more efficient and semantically significant when the lvalue has side effects.

## Syntax
```angelscript
lvalue += rvalue;
lvalue -= rvalue;
lvalue *= rvalue;
lvalue /= rvalue;
lvalue %= rvalue;
lvalue **= rvalue;
lvalue &= rvalue;
lvalue |= rvalue;
lvalue ^= rvalue;
lvalue <<= rvalue;
lvalue >>= rvalue;
lvalue >>>= rvalue;
```

## Semantics
- A compound assignment `lvalue op= rvalue` is semantically equivalent to `lvalue = lvalue op rvalue`, except the lvalue is evaluated only once.
- The single-evaluation guarantee matters when the lvalue is a complex expression with side effects (e.g., `arr[i++] += 5` increments `i` only once).
- The same type promotion and conversion rules apply as for the corresponding binary operator.
- The result is stored back into the lvalue. The expression evaluates to the stored value.
- Precedence: All compound assignments share precedence level 16 with simple assignment (`=`), right-to-left associative.

| Operator | Equivalent Operation |
|----------|---------------------|
| `+=`     | addition            |
| `-=`     | subtraction         |
| `*=`     | multiplication      |
| `/=`     | division            |
| `%=`     | modulo              |
| `**=`    | exponentiation      |
| `&=`     | bitwise AND         |
| `\|=`    | bitwise OR          |
| `^=`     | bitwise XOR         |
| `<<=`    | left shift          |
| `>>=`    | right shift         |
| `>>>=`   | arithmetic right shift |

## Examples
```angelscript
int a = 10;
a += 5;           // a is now 15
a -= 3;           // a is now 12
a *= 2;           // a is now 24
a /= 4;           // a is now 6
a %= 4;           // a is now 2
a **= 3;          // a is now 8

int flags = 0;
flags |= 0x01;    // set bit 0
flags |= 0x04;    // set bit 2
flags &= ~0x01;   // clear bit 0

int x = 1;
x <<= 4;          // x is now 16
x >>= 2;          // x is now 4

// Lvalue evaluated only once
array<int> arr = {1, 2, 3};
int i = 0;
arr[i++] += 10;   // arr[0] becomes 11, i becomes 1 (i++ evaluated once)
```

## Compilation Notes
- **Stack behavior:** The lvalue address is computed once and saved. The current value at that address is loaded, the rvalue is evaluated, the binary operation is performed, and the result is stored back to the saved address.
- **Evaluation order:** The lvalue address is computed first, then the rvalue expression. This differs from simple assignment where RHS is evaluated before LHS.
- **Type considerations:** The same implicit promotion rules apply as for the corresponding binary operator. If the promoted type differs from the lvalue type, a conversion back to the lvalue type is inserted after the operation.
- **Special cases:**
  - For complex lvalue expressions (e.g., `obj.arr[func()]`), the address must be computed once and cached, not recomputed for the load and store separately. This typically means storing the address in a temporary.
  - Object types may define compound assignment operators directly (e.g., `opAddAssign`) which may be more efficient than separate load-operate-store.
  - The bytecode generator should check for type-specific compound assignment overloads before falling back to the decomposed form.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Assign` | Assignment expression variant | Wraps `&AssignExpr` |
| `AssignExpr` | Assignment operation | `target: &Expr`, `op: AssignOp`, `value: &Expr`, `span: Span` |
| `AssignOp::AddAssign` | `+=` add-assign | — |
| `AssignOp::SubAssign` | `-=` subtract-assign | — |
| `AssignOp::MulAssign` | `*=` multiply-assign | — |
| `AssignOp::DivAssign` | `/=` divide-assign | — |
| `AssignOp::ModAssign` | `%=` modulo-assign | — |
| `AssignOp::PowAssign` | `**=` power-assign | — |
| `AssignOp::AndAssign` | `&=` bitwise-and-assign | — |
| `AssignOp::OrAssign` | `\|=` bitwise-or-assign | — |
| `AssignOp::XorAssign` | `^=` bitwise-xor-assign | — |
| `AssignOp::ShlAssign` | `<<=` shift-left-assign | — |
| `AssignOp::ShrAssign` | `>>=` shift-right-assign | — |
| `AssignOp::UshrAssign` | `>>>=` unsigned-shift-right-assign | — |

**Notes:**
- All compound assignments share the same AST node (`Expr::Assign` / `AssignExpr`) as simple assignment; only the `op: AssignOp` variant differs.
- `AssignOp::binding_power()` returns `(2, 1)` for all variants -- lowest precedence, right-associative.
- `AssignOp::is_simple()` returns `false` for all compound variants (only `AssignOp::Assign` returns `true`).

## Related Features
- [assignments.md](assignments.md) - Simple assignment operator
- [math-operators.md](math-operators.md) - Arithmetic operators (`+`, `-`, `*`, `/`, `%`, `**`)
- [bitwise-operators.md](bitwise-operators.md) - Bitwise operators (`&`, `|`, `^`, `<<`, `>>`, `>>>`)
