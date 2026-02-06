# Math Operators

## Overview
Math operators perform arithmetic on numeric types. AngelScript supports the standard binary arithmetic operators as well as unary positive and negative. Both operands of a binary operator are implicitly converted to a common type before the operation.

## Syntax
```angelscript
// Unary
+expr
-expr

// Binary
a + b    // addition
a - b    // subtraction
a * b    // multiplication
a / b    // division
a % b    // modulo (remainder)
a ** b   // exponentiation
```

## Semantics
- **Operand types:** All operands must be numeric types (`int8`, `int16`, `int`, `int64`, `uint8`, `uint16`, `uint`, `uint64`, `float`, `double`).
- **Implicit promotion:** Both operands of a binary operator are implicitly converted to the same type before the operation executes. The promotion follows the standard widening rules (e.g., `int` + `float` promotes the `int` to `float`).
- **Result type:** The result is always the same type as the (promoted) operands.
- **Unary positive (`+`):** No-op; returns the value unchanged. Available for all numeric types.
- **Unary negative (`-`):** Negates the value. Not available for unsigned types (`uint`, `uint8`, `uint16`, `uint64`).
- **Integer division:** Truncates toward zero.
- **Modulo (`%`):** Returns the remainder of integer division. For floating-point types, returns the IEEE remainder.
- **Exponentiation (`**`):** Raises the left operand to the power of the right operand. Has higher precedence than `*`, `/`, `%`.
- **Operator precedence:** `**` (level 3, right-to-left) > `*`, `/`, `%` (level 4, left-to-right) > `+`, `-` (level 5, left-to-right). Unary `+` and `-` have precedence level 2.

| Operator | Description      | Left | Right | Result |
|----------|-----------------|------|-------|--------|
| `+`      | unary positive  |      | NUM   | NUM    |
| `-`      | unary negative  |      | NUM   | NUM    |
| `+`      | addition        | NUM  | NUM   | NUM    |
| `-`      | subtraction     | NUM  | NUM   | NUM    |
| `*`      | multiplication  | NUM  | NUM   | NUM    |
| `/`      | division        | NUM  | NUM   | NUM    |
| `%`      | modulo          | NUM  | NUM   | NUM    |
| `**`     | exponentiation  | NUM  | NUM   | NUM    |

## Examples
```angelscript
int a = 10;
int b = 3;

int sum = a + b;        // 13
int diff = a - b;       // 7
int prod = a * b;       // 30
int quot = a / b;       // 3 (truncated)
int rem = a % b;        // 1
int power = 2 ** 10;    // 1024

float f = -3.14f;       // unary negation
int neg = -(a + b);     // -13

// Type promotion
float mixed = a + 1.5f; // a promoted to float, result is float
```

## Compilation Notes
- **Stack behavior:** For binary operators, the left operand is evaluated and pushed first, then the right operand. The arithmetic instruction pops both and pushes the result.
- **Type considerations:**
  - If operands differ in type, the compiler inserts implicit conversion bytecodes before the arithmetic instruction. For example, `int + float` requires an `i2f` conversion on the left operand before an `fadd` instruction.
  - The promotion target is determined by the "wider" type: `int` < `int64` < `float` < `double`; unsigned variants follow similar widening.
  - For integer types of different sizes, the smaller is widened to the larger.
- **Control flow:** No branching involved; these are straightforward stack operations.
- **Special cases:**
  - Division by zero for integer types causes a runtime exception. The bytecode generator may or may not emit a check depending on engine configuration.
  - Unary negation on unsigned types is a compile-time error.
  - Exponentiation (`**`) may be compiled as a function call (e.g., `pow`) rather than a single instruction, depending on the backend.
  - Object types may overload these operators via `opAdd`, `opSub`, `opMul`, `opDiv`, `opMod`, `opPow` and their reverse counterparts (`opAdd_r`, etc.).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::Add` | `+` addition | binding_power: (19, 20) |
| `BinaryOp::Sub` | `-` subtraction | binding_power: (19, 20) |
| `BinaryOp::Mul` | `*` multiplication | binding_power: (21, 22) |
| `BinaryOp::Div` | `/` division | binding_power: (21, 22) |
| `BinaryOp::Mod` | `%` modulo | binding_power: (21, 22) |
| `BinaryOp::Pow` | `**` exponentiation | binding_power: (24, 23) -- right-associative |
| `Expr::Unary` | Unary prefix expression variant | Wraps `&UnaryExpr` |
| `UnaryExpr` | Unary prefix operation | `op: UnaryOp`, `operand: &Expr`, `span: Span` |
| `UnaryOp::Neg` | `-` unary negation | binding_power: 25 |
| `UnaryOp::Plus` | `+` unary positive | binding_power: 25 |

**Notes:**
- All binary math operators are left-associative (right_bp = left_bp + 1) except `Pow` which is right-associative (right_bp < left_bp).
- Unary `Neg` and `Plus` share `UnaryOp::binding_power()` = 25, which is higher than all binary operators.
- The same `Expr::Binary` / `BinaryExpr` structure is used for arithmetic, bitwise, logic, and comparison operators; only the `BinaryOp` variant changes.

## Related Features
- [compound-assignments.md](compound-assignments.md) - `+=`, `-=`, `*=`, `/=`, `%=`, `**=`
- [bitwise-operators.md](bitwise-operators.md) - Bitwise operations on integer types
- [type-conversions.md](type-conversions.md) - Implicit numeric promotion rules
