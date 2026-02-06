# Operator Precedence

## Overview

AngelScript operator precedence determines the order in which operators are evaluated in expressions. Operators with higher precedence bind more tightly than those with lower precedence. When operators have equal precedence, associativity (left-to-right or right-to-left) determines the evaluation order. The precedence rules are similar to C/C++ with some AngelScript-specific additions (exponent operator `**`, identity operators `is`/`!is`, and keyword alternatives for logical operators).

## Syntax

Operators are used in expressions to combine values:

```angelscript
// Precedence determines evaluation order
int result = 2 + 3 * 4;       // 14, not 20 (multiplication before addition)
int result2 = (2 + 3) * 4;    // 20 (parentheses override precedence)

// Associativity determines direction
int a = 2 ** 3 ** 2;          // Right-to-left: 2 ** (3 ** 2) = 2 ** 9 = 512
int b = 10 - 5 - 2;           // Left-to-right: (10 - 5) - 2 = 3
```

## Semantics

### Unary operators (highest precedence)

Unary operators have higher precedence than all binary operators. Among unary operators, post-operators bind more tightly than pre-operators, and operators closer to the operand bind more tightly.

Listed from highest to lowest precedence:

| Precedence | Operator | Description | Associativity |
|:---:|----------|-------------|:---:|
| 1 | `::` | Scope resolution | Left-to-right |
| 2 | `[]` | Indexing | Left-to-right |
| 3 | `++ --` (postfix) | Post-increment, post-decrement | Left-to-right |
| 4 | `.` | Member access | Left-to-right |
| 5 | `++ --` (prefix) | Pre-increment, pre-decrement | Right-to-left |
| 6 | `not` `!` | Logical NOT | Right-to-left |
| 7 | `+` `-` (unary) | Unary positive, unary negative | Right-to-left |
| 8 | `~` | Bitwise complement | Right-to-left |
| 9 | `@` | Handle-of | Right-to-left |

### Binary and ternary operators

Listed from highest to lowest precedence:

| Precedence | Operator(s) | Description | Associativity |
|:---:|-------------|-------------|:---:|
| 10 | `**` | Exponent | Right-to-left |
| 11 | `*` `/` `%` | Multiply, divide, modulo | Left-to-right |
| 12 | `+` `-` | Add, subtract | Left-to-right |
| 13 | `<<` `>>` `>>>` | Left shift, right shift, arithmetic right shift | Left-to-right |
| 14 | `&` | Bitwise AND | Left-to-right |
| 15 | `^` | Bitwise XOR | Left-to-right |
| 16 | `\|` | Bitwise OR | Left-to-right |
| 17 | `<=` `<` `>=` `>` | Comparison (relational) | Left-to-right |
| 18 | `==` `!=` `is` `!is` `xor` `^^` | Equality, identity, logical XOR | Left-to-right |
| 19 | `and` `&&` | Logical AND | Left-to-right |
| 20 | `or` `\|\|` | Logical OR | Left-to-right |
| 21 | `?:` | Ternary conditional | Right-to-left |
| 22 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` | Assignment and compound assignment | Right-to-left |

### Keyword alternatives

AngelScript provides keyword alternatives for some operators:

| Keyword | Equivalent symbol |
|---------|------------------|
| `and` | `&&` |
| `or` | `\|\|` |
| `not` | `!` |
| `xor` | `^^` |
| `is` | (no symbol equivalent -- identity comparison) |
| `!is` | (no symbol equivalent -- non-identity comparison) |

### Shift operators

| Operator | Description |
|----------|-------------|
| `<<` | Left shift: shifts bits left, filling with zeros |
| `>>` | Right shift: shifts bits right, filling with the sign bit (arithmetic shift) |
| `>>>` | Unsigned right shift: shifts bits right, filling with zeros (logical shift) |

### Identity vs equality

The `is` and `!is` operators are at the same precedence level as `==` and `!=`:

```angelscript
// is/!is compare addresses (identity)
if (a is b) { }      // Same object?
if (a !is null) { }   // Not null?

// ==/!= compare values (equality)
if (a == b) { }       // Equal values? (calls opEquals)
```

### Short-circuit evaluation

- `&&` (`and`): If the left operand is false, the right operand is **not evaluated**.
- `||` (`or`): If the left operand is true, the right operand is **not evaluated**.

```angelscript
// Safe null check with short-circuit
if (obj !is null && obj.value > 0) {
    // obj.value is only accessed if obj is not null
}
```

## Examples

```angelscript
// Precedence examples
int a = 2 + 3 * 4;            // 14 (multiply first)
int b = (2 + 3) * 4;          // 20 (parentheses override)
int c = 2 ** 3 + 1;           // 9 (exponent first: 8 + 1)
int d = 1 + 2 << 3;           // 24 ((1+2) << 3, add before shift)

// Bitwise operator precedence
int e = 5 | 3 & 6;            // 7 (& before |: 5 | (3 & 6) = 5 | 2 = 7)
int f = (5 | 3) & 6;          // 6 (parentheses override)

// Comparison and logical
bool g = 1 < 2 && 3 > 2;     // true (comparisons before &&)
bool h = true || false && false; // true (&&  before ||: true || (false && false))

// Ternary operator
int i = (a > b) ? a : b;      // Max of a and b

// Assignment is right-to-left
int x, y, z;
x = y = z = 10;               // z=10, y=10, x=10

// Compound assignment
int val = 10;
val += 5;     // 15
val *= 2;     // 30
val <<= 1;    // 60
val **= 2;    // 3600

// Handle-of operator precedence
obj o;
obj@ h = @o;                  // @ has lower precedence than . but higher than binary ops

// Scope resolution (highest precedence)
int v = SomeNamespace::someValue;
```

## Compilation Notes

- **Memory layout:** Operators themselves have no memory representation. They are compiled into bytecode instructions that operate on stack values and registers.
- **Stack behavior:** Binary operators typically pop two values from the stack, perform the operation, and push the result. Unary operators pop one value and push the result. The ternary operator uses conditional branching bytecode.
- **Type considerations:**
  - The compiler must apply type promotion rules before emitting operator bytecodes. For example, `int + float` requires promoting the int to float before the add instruction.
  - Comparison operators (`<`, `>`, `<=`, `>=`, `==`, `!=`) produce a `bool` result regardless of operand types.
  - Identity operators (`is`, `!is`) compare pointer values directly -- no value comparison or method calls involved.
  - Bitwise operators (`&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`) operate on integer types only.
  - The exponent operator (`**`) may be implemented as a built-in or via a function call depending on the operand types.
  - Compound assignments (`+=`, `-=`, etc.) are semantically equivalent to `a = a op b` but may be optimized to modify in place.
- **Lifecycle:** No special lifecycle concerns. All operator results are temporaries with the same lifetime rules as any expression result.
- **Special cases:**
  - **Short-circuit evaluation** for `&&` and `||` requires conditional branch instructions. The compiler must emit a branch after the left operand to skip the right operand when appropriate.
  - **Ternary operator** requires branching: evaluate condition, branch to true-path or false-path, merge results.
  - **Operator overloads**: For object types, operators may resolve to method calls (`opAdd`, `opEquals`, `opCmp`, etc.). The compiler must check for registered operator overloads and emit method calls instead of primitive instructions.
  - **`@` (handle-of)**: This is a unary prefix operator that extracts the handle from a variable. It does not produce a new instruction per se, but changes how the compiler treats the expression (as a handle rather than a value).
  - **`>>>` (arithmetic right shift)**: Distinct from `>>` in that it always fills with zeros, regardless of the sign bit. The compiler must emit a different shift instruction for this.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/ops.rs`, `crates/angelscript-parser/src/ast/expr.rs`

The precedence table above maps to binding_power values in the parser as follows:

| Doc Precedence | Operator(s) | AST Type | binding_power | Associativity |
|:-:|-------------|----------|:---:|:---:|
| 22 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&=`, `\|=`, `^=`, `<<=`, `>>=`, `>>>=` | `AssignOp::*` | (2, 1) | Right-to-left |
| 21 | `?:` | `Expr::Ternary` / `TernaryExpr` | special-cased in parser | Right-to-left |
| 20 | `\|\|` / `or` | `BinaryOp::LogicalOr` | (3, 4) | Left-to-right |
| 20 | `^^` / `xor` | `BinaryOp::LogicalXor` | (3, 4) | Left-to-right |
| 19 | `&&` / `and` | `BinaryOp::LogicalAnd` | (5, 6) | Left-to-right |
| 16 | `\|` | `BinaryOp::BitwiseOr` | (7, 8) | Left-to-right |
| 15 | `^` | `BinaryOp::BitwiseXor` | (9, 10) | Left-to-right |
| 14 | `&` | `BinaryOp::BitwiseAnd` | (11, 12) | Left-to-right |
| 18 | `==`, `!=`, `is`, `!is` | `BinaryOp::Equal`, `NotEqual`, `Is`, `NotIs` | (13, 14) | Left-to-right |
| 17 | `<`, `<=`, `>`, `>=` | `BinaryOp::Less`, `LessEqual`, `Greater`, `GreaterEqual` | (15, 16) | Left-to-right |
| 13 | `<<`, `>>`, `>>>` | `BinaryOp::ShiftLeft`, `ShiftRight`, `ShiftRightUnsigned` | (17, 18) | Left-to-right |
| 12 | `+`, `-` | `BinaryOp::Add`, `Sub` | (19, 20) | Left-to-right |
| 11 | `*`, `/`, `%` | `BinaryOp::Mul`, `Div`, `Mod` | (21, 22) | Left-to-right |
| 10 | `**` | `BinaryOp::Pow` | (24, 23) | Right-to-left |
| 5-9 | `++`, `--`, `!`/`not`, `+`/`-` (unary), `~`, `@` | `UnaryOp::*` | 25 | Right-to-left |
| 2-4 | `.`, `[]`, `++`/`--` (postfix) | `Expr::Member`, `Expr::Index`, `Expr::Call`, `PostfixOp::*` | 27 | Left-to-right |

**Notes:**
- **Discrepancy -- LogicalXor grouping:** The doc table (precedence 18) groups `xor`/`^^` with equality operators (`==`, `!=`, `is`, `!is`). However, in the AST `BinaryOp::LogicalXor` has binding_power `(3, 4)`, the same as `LogicalOr`, not `(13, 14)` like equality. The AST binding_power values are the authoritative source for parser behavior.
- **Discrepancy -- Bitwise vs relational ordering:** The doc table places bitwise operators (`&` at 14, `^` at 15, `|` at 16) between shifts (13) and relational (17). The AST binding_power values confirm this ordering: shifts (17,18) > relational (15,16) > equality (13,14) > bitwise AND (11,12) > bitwise XOR (9,10) > bitwise OR (7,8). The doc's numbering convention inverts the "higher number = higher precedence" pattern used in binding_power.
- **Unary operators:** All `UnaryOp` variants share a single `binding_power()` of 25. The fine-grained sub-ordering among unary prefix operators (doc levels 5-9) is not distinguished by binding_power; they all bind equally tightly.
- **Postfix operators:** `PostfixOp::binding_power()` returns 27, the highest value, matching the doc's expectation that postfix binds tightest. Member access, indexing, and calls share this postfix tier.
- The `Expr::Literal` / `LiteralExpr` node (for numeric/string/bool/null literals) has no dedicated documentation file; literal representation is covered across `types/primitives.md` and `types/strings.md`.

## Related Features

- [Primitive types (operand types and promotion)](./primitives.md)
- [Object handles (@ operator, is/!is)](./handles.md)
- [Objects (operator overloads)](./objects.md)
