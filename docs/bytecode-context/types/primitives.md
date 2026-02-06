# Primitive Types

## Overview

AngelScript provides built-in primitive types for boolean logic, integer arithmetic, and floating-point arithmetic. The special `void` pseudo-type indicates the absence of a value (used only as a function return type). Primitives are always value types: they live on the stack, are copied by value, and have no reference-counting or garbage-collection overhead.

## Syntax

```angelscript
// void - only valid as a function return type
void doSomething() { }

// bool
bool flag = true;
bool other = false;

// Integer declarations
int8   a = -100;
int16  b = -30000;
int    c = 42;          // int is 32-bit signed
int32  d = 42;          // int32 is an alias for int
int64  e = 9000000000;
uint8  f = 200;
uint16 g = 60000;
uint   h = 3000000000;  // uint is 32-bit unsigned
uint32 i = 3000000000;  // uint32 is an alias for uint
uint64 j = 18000000000000000000;

// Integer literal forms
int dec = 42;
int hex = 0xFF;
int oct = 0o77;         // if supported by engine configuration
int bin = 0b10101010;   // if supported by engine configuration
int64 big = 100000L;    // L suffix for int64

// Float and double declarations
float  x = 3.14f;
double y = 3.14159265358979;

// Float literal forms
float  f1 = 1.0f;
float  f2 = 1.0F;
float  f3 = .5f;
float  f4 = 1e10f;
double d1 = 1.0;
double d2 = 1e10;
double d3 = 1.0d;
```

## Semantics

### void

- Not a real data type; represents "no value."
- Can only appear as a function return type.
- Cannot declare variables of type `void`.

### bool

- Exactly two values: `true` and `false`.
- Keywords `true` and `false` are built-in constants.
- Used as the result type of comparison and logical operators.

### Integer types

| Type | Size (bytes) | Signed | Min | Max |
|------|:---:|:---:|-----|-----|
| `int8` | 1 | yes | -128 | 127 |
| `int16` | 2 | yes | -32,768 | 32,767 |
| `int` / `int32` | 4 | yes | -2,147,483,648 | 2,147,483,647 |
| `int64` | 8 | yes | -9,223,372,036,854,775,808 | 9,223,372,036,854,775,807 |
| `uint8` | 1 | no | 0 | 255 |
| `uint16` | 2 | no | 0 | 65,535 |
| `uint` / `uint32` | 4 | no | 0 | 4,294,967,295 |
| `uint64` | 8 | no | 0 | 18,446,744,073,709,551,615 |

- `int32` is an alias for `int`; `uint32` is an alias for `uint`.
- The engine is optimized for 32-bit types. Smaller variants are mainly for accessing application-registered variables; for local variables, prefer 32-bit.

### Real number types

| Type | Size (bytes) | Range | Smallest Positive | Max Significant Digits |
|------|:---:|-------|------|:---:|
| `float` | 4 | +/- 3.402823466e+38 | 1.175494351e-38 | 6 |
| `double` | 8 | +/- 1.79769313486231e+308 | 2.22507385850720e-308 | 15 |

- Assumes IEEE 754 representation.
- Special values: positive zero, negative zero, positive infinity, negative infinity, NaN.
- For `float`, NaN is represented by the 32-bit word `0x7fc00000`.
- Rounding errors occur if more digits than the maximum are used.

### Default values

Primitive variables declared without an initializer have **undefined/uninitialized values**. This is unlike objects, which are default-constructed.

```angelscript
int a;          // undefined value -- use is dangerous
int b = 0;      // explicitly initialized
float f;        // undefined value
float g = 0.0f; // explicitly initialized
```

### Type aliases

| Alias | Resolves to |
|-------|------------|
| `int32` | `int` |
| `uint32` | `uint` |

### Type promotion rules

When binary operators mix primitive types, implicit promotion occurs:

1. `int8`, `int16` promote to `int` (32-bit) for arithmetic.
2. `uint8`, `uint16` promote to `uint` (32-bit) for arithmetic.
3. When mixing `int` and `uint` of the same size, the signed operand is converted to unsigned.
4. When mixing `int`/`uint` with `float`, the integer is converted to `float`.
5. When mixing `float` with `double`, the `float` is promoted to `double`.
6. When mixing any integer type with `double`, the integer is converted to `double`.
7. `int64` / `uint64` operations stay 64-bit; mixing 32-bit and 64-bit promotes to 64-bit.
8. `bool` does not implicitly convert to/from integers (explicit cast required).

## Examples

```angelscript
// Basic arithmetic
int a = 10;
int b = 3;
int c = a / b;      // 3 (integer division)
float d = float(a) / float(b);  // 3.333...

// Type promotion
float f = 1 + 2.5f;   // 1 promoted to float => 3.5f
double g = 1.0f + 2.0; // float promoted to double => 3.0

// Boolean logic
bool x = true;
bool y = !x;          // false
bool z = x && y;      // false

// Overflow wraps (unsigned)
uint8 u = 255;
u++;                   // u is now 0

// Hex literal
int mask = 0xFF00;
```

## Compilation Notes

- **Memory layout:** All primitives have fixed sizes. `bool` is 1 byte. `int8`/`uint8` are 1 byte. `int16`/`uint16` are 2 bytes. `int`/`uint` and `float` are 4 bytes. `int64`/`uint64` and `double` are 8 bytes.
- **Stack behavior:** Primitives are always stack-allocated. They occupy stack slots directly (typically one DWORD for 32-bit types, two DWORDs or one QWORD for 64-bit types). The bytecode VM may use a uniform slot size and widen smaller types during operations.
- **Type considerations:** Sub-32-bit types (`int8`, `int16`, `uint8`, `uint16`) may be widened to 32-bit for arithmetic operations and then truncated on store. The compiler should insert implicit conversion instructions when mixing types (e.g., `i32TOi64`, `i32TOf`, `fTOd`).
- **Lifecycle:** No construction or destruction needed. Stack space is reserved on function entry and released on function exit. No reference counting.
- **Special cases:**
  - Division by zero for integer types should raise a script exception.
  - Float NaN comparisons: `NaN != NaN` is true; `NaN == NaN` is false.
  - The `void` type produces no bytecode value; functions returning void simply omit the return-value push.
  - Type aliases (`int32` -> `int`, `uint32` -> `uint`) are resolved at parse time and produce identical bytecode.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Primitive(PrimitiveType)` | Wraps a `PrimitiveType` variant to form the base of a `TypeExpr` | Holds one `PrimitiveType` value |
| `PrimitiveType::Void` | `void` pseudo-type | Size: 0 bytes |
| `PrimitiveType::Bool` | `bool` type | Size: 1 byte |
| `PrimitiveType::Int` | `int` (32-bit signed) | Size: 4 bytes |
| `PrimitiveType::Int8` | `int8` (8-bit signed) | Size: 1 byte |
| `PrimitiveType::Int16` | `int16` (16-bit signed) | Size: 2 bytes |
| `PrimitiveType::Int64` | `int64` (64-bit signed) | Size: 8 bytes |
| `PrimitiveType::UInt` | `uint` (32-bit unsigned) | Size: 4 bytes |
| `PrimitiveType::UInt8` | `uint8` (8-bit unsigned) | Size: 1 byte |
| `PrimitiveType::UInt16` | `uint16` (16-bit unsigned) | Size: 2 bytes |
| `PrimitiveType::UInt64` | `uint64` (64-bit unsigned) | Size: 8 bytes |
| `PrimitiveType::Float` | `float` (32-bit IEEE 754) | Size: 4 bytes |
| `PrimitiveType::Double` | `double` (64-bit IEEE 754) | Size: 8 bytes |

**Notes:**
- `PrimitiveType` has exactly 12 variants, matching the 12 primitive types described above (including `void`).
- The type aliases `int32` and `uint32` are resolved at parse time; they do not have separate `PrimitiveType` variants. They map to `PrimitiveType::Int` and `PrimitiveType::UInt` respectively.
- Helper methods on `PrimitiveType`: `size_bytes()`, `is_integer()`, `is_float()`, `is_signed()`, `is_unsigned()`.
- A primitive type is represented in the AST as `TypeExpr { is_const: bool, scope: None, base: TypeBase::Primitive(_), template_args: &[], suffixes: &[], span }`.
- `TypeExpr::primitive(prim, span)` is a convenience constructor that creates a non-const, unscoped, unsuffixed primitive type expression.

## Related Features

- [Objects and value types](./objects.md)
- [Auto declarations (type inference)](./auto-declarations.md)
- [Operator precedence](./operator-precedence.md)
