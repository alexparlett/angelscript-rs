# Assignments

## Overview
Assignment expressions store a value into a memory location (lvalue). In AngelScript, assignments are expressions that evaluate to the stored value, enabling chaining. The right-hand side is always evaluated before the left-hand side.

## Syntax
```angelscript
lvalue = rvalue;

// Chained assignment
a = b = c;
```

## Semantics
- `lvalue` must be an expression that evaluates to a writable memory location: a variable, a property, an indexed element, or one branch of a conditional lvalue.
- The assignment expression evaluates to the same value and type as the data stored in the lvalue.
- The right-hand expression is always computed before the left-hand expression. This matters when both sides have side effects.
- If the right-hand type differs from the left-hand type, an implicit conversion is attempted. If no valid implicit conversion exists, a compile-time error is raised.
- For object types, assignment invokes the `opAssign` method if defined. For handles, assignment without `@` copies the value; assignment with `@` rebinds the handle.
- Assignment has the lowest precedence of all operators (precedence level 16), and is right-to-left associative.

## Examples
```angelscript
int a;
int b;
a = 10;          // simple assignment
a = b = 5;       // chained: b gets 5, then a gets 5

float f = 3;     // implicit int-to-float conversion on RHS

// Handle assignment
obj@ h;
obj o;
@h = @o;         // rebind handle
h = o;           // copy value into referenced object (requires non-null h)
```

## Compilation Notes
- **Evaluation order:** The compiler must emit bytecode for the right-hand side first, leaving the result on the stack. Then the left-hand side address is computed. Finally the store instruction executes.
- **Stack behavior:** RHS is evaluated and pushed. LHS address is resolved. A store/copy instruction consumes the top-of-stack value and writes it to the LHS address. The result of the assignment expression remains on the stack for use in chained assignments or enclosing expressions.
- **Type considerations:** If implicit conversion is needed, the compiler inserts conversion bytecodes between RHS evaluation and the store instruction (e.g., `i2f` for int-to-float).
- **Special cases:**
  - Chained assignments (`a = b = c`) are handled naturally by right-to-left associativity: `c` is evaluated, stored into `b`, the result remains on the stack, and is then stored into `a`.
  - Object assignment may require calling `opAssign` or a copy constructor rather than a simple store.
  - Handle assignment (`@h = @o`) compiles to a reference-count increment on the new target and decrement on the old target.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Assign` | Assignment expression variant | Wraps `&AssignExpr` |
| `AssignExpr` | Assignment operation | `target: &Expr`, `op: AssignOp`, `value: &Expr`, `span: Span` |
| `AssignOp::Assign` | `=` simple assignment | — |

**Notes:**
- `AssignOp::binding_power()` returns `(2, 1)` -- lowest precedence, right-associative (right_bp < left_bp).
- Only the simple `=` operator is covered here. Compound assignment operators (`+=`, `-=`, etc.) are separate `AssignOp` variants documented in [compound-assignments.md](compound-assignments.md).
- The `target` field accepts any `Expr` (validated as lvalue during semantic analysis, not at the AST level).

## Related Features
- [compound-assignments.md](compound-assignments.md) - Combined operator-and-assign forms
- [type-conversions.md](type-conversions.md) - Implicit and explicit conversion rules
- [handle-of.md](handle-of.md) - Handle assignment semantics
- [conditional-expression.md](conditional-expression.md) - Conditional lvalue assignment
