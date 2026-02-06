# Bitwise Operators

## Overview
Bitwise operators manipulate individual bits of integer values. AngelScript provides complement, AND, OR, XOR, left shift, right shift, and arithmetic right shift operators. Both operands are converted to integers before the operation.

## Syntax
```angelscript
// Unary
~expr           // bitwise complement (NOT)

// Binary
a & b           // bitwise AND
a | b           // bitwise OR
a ^ b           // bitwise XOR
a << b          // left shift
a >> b          // right shift (sign-extended)
a >>> b         // arithmetic right shift (zero-filled)
```

## Semantics
- **Operand types:** All operands must be numeric. Operands are converted to integer types while preserving the sign of the original type before the operation executes.
- **Result type:** The result type matches the left-hand operand type (for binary operators) or the operand type (for unary complement).
- **Complement (`~`):** Inverts all bits. Unary operator.
- **AND (`&`):** Sets each bit to 1 only if both corresponding input bits are 1.
- **OR (`|`):** Sets each bit to 1 if at least one corresponding input bit is 1.
- **XOR (`^`):** Sets each bit to 1 if exactly one corresponding input bit is 1.
- **Left shift (`<<`):** Shifts bits left, filling with zeros on the right. Equivalent to multiplication by powers of 2.
- **Right shift (`>>`):** Shifts bits right with sign extension (preserves the sign bit for signed types). For unsigned types, fills with zeros.
- **Arithmetic right shift (`>>>`):** Shifts bits right, always filling with zeros regardless of sign. This is the logical right shift.

**Operator precedence (high to low):**
- `~` (unary, level 2)
- `<<`, `>>`, `>>>` (level 6, left-to-right)
- `&` (level 7, left-to-right)
- `^` (level 8, left-to-right)
- `|` (level 9, left-to-right)

| Operator | Description             | Left | Right | Result |
|----------|------------------------|------|-------|--------|
| `~`      | bitwise complement     |      | NUM   | NUM    |
| `&`      | bitwise AND            | NUM  | NUM   | NUM    |
| `\|`     | bitwise OR             | NUM  | NUM   | NUM    |
| `^`      | bitwise XOR            | NUM  | NUM   | NUM    |
| `<<`     | left shift             | NUM  | NUM   | NUM    |
| `>>`     | right shift            | NUM  | NUM   | NUM    |
| `>>>`    | arithmetic right shift | NUM  | NUM   | NUM    |

## Examples
```angelscript
int a = 0xFF00;
int b = 0x0F0F;

int complement = ~a;         // inverts all bits of a
int andResult = a & b;       // 0x0F00
int orResult = a | b;        // 0xFF0F
int xorResult = a ^ b;      // 0xF00F

int shifted = 1 << 4;       // 16 (0x10)
int rshift = 0x80 >> 2;     // 0x20

// Sign extension difference
int neg = -128;
int signExt = neg >> 2;     // sign-extended: still negative
int zeroFill = neg >>> 2;   // zero-filled: large positive
```

## Compilation Notes
- **Stack behavior:** For binary operators, the left operand is evaluated and pushed first, then the right operand. The bitwise instruction pops both and pushes the result. For unary complement, one operand is popped and the result is pushed.
- **Type considerations:**
  - Both operands are converted to integers before the operation. If a floating-point value is used, it is first converted to integer (truncated). This conversion must be emitted before the bitwise instruction.
  - The sign of the original type is preserved during conversion. A signed float becomes a signed int; an unsigned type stays unsigned.
  - The result type matches the left operand's type (after integer conversion).
- **Control flow:** No branching involved; these are straightforward stack operations.
- **Special cases:**
  - The `>>` vs `>>>` distinction is critical: `>>` is arithmetic (sign-extending) for signed types and logical (zero-filling) for unsigned types. `>>>` is always logical (zero-filling). The bytecode generator must emit different shift instructions based on this distinction.
  - Shift amounts larger than the bit width of the type produce undefined/implementation-defined behavior. The compiler may or may not mask the shift amount.
  - Object types may overload these operators via `opAnd`, `opOr`, `opXor`, `opShl`, `opShr`, `opUShr` and their reverse counterparts.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`, `crates/angelscript-parser/src/ast/ops.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Binary` | Binary expression variant | Wraps `&BinaryExpr` |
| `BinaryExpr` | Binary operation | `left: &Expr`, `op: BinaryOp`, `right: &Expr`, `span: Span` |
| `BinaryOp::BitwiseAnd` | `&` bitwise AND | binding_power: (11, 12) |
| `BinaryOp::BitwiseOr` | `\|` bitwise OR | binding_power: (7, 8) |
| `BinaryOp::BitwiseXor` | `^` bitwise XOR | binding_power: (9, 10) |
| `BinaryOp::ShiftLeft` | `<<` left shift | binding_power: (17, 18) |
| `BinaryOp::ShiftRight` | `>>` right shift (sign-extended) | binding_power: (17, 18) |
| `BinaryOp::ShiftRightUnsigned` | `>>>` arithmetic right shift (zero-filled) | binding_power: (17, 18) |
| `Expr::Unary` | Unary prefix expression variant | Wraps `&UnaryExpr` |
| `UnaryExpr` | Unary prefix operation | `op: UnaryOp`, `operand: &Expr`, `span: Span` |
| `UnaryOp::BitwiseNot` | `~` bitwise complement | binding_power: 25 |

**Notes:**
- All binary bitwise operators are left-associative (right_bp = left_bp + 1).
- Precedence ordering from the AST: shifts (17,18) > bitwise AND (11,12) > bitwise XOR (9,10) > bitwise OR (7,8). This matches the doc's stated precedence hierarchy.
- The unary `BitwiseNot` has binding_power 25 (shared by all `UnaryOp` variants), higher than all binary operators.

## Related Features
- [compound-assignments.md](compound-assignments.md) - `&=`, `|=`, `^=`, `<<=`, `>>=`, `>>>=`
- [math-operators.md](math-operators.md) - Arithmetic operators on numeric types
- [logic-operators.md](logic-operators.md) - Logical (boolean) operators (different from bitwise)
