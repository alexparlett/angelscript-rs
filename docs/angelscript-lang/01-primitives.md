# Primitive Data Types

## void

`void` is not a data type - it indicates the absence of a return value. Only used for function return types.

## bool

Boolean type with two possible values: `true` or `false`.

## Integer Types

| Type | Min Value | Max Value | Rust Equivalent |
|------|-----------|-----------|-----------------|
| `int8` | -128 | 127 | `i8` |
| `int16` | -32,768 | 32,767 | `i16` |
| `int` / `int32` | -2,147,483,648 | 2,147,483,647 | `i32` |
| `int64` | -9,223,372,036,854,775,808 | 9,223,372,036,854,775,807 | `i64` |
| `uint8` | 0 | 255 | `u8` |
| `uint16` | 0 | 65,535 | `u16` |
| `uint` / `uint32` | 0 | 4,294,967,295 | `u32` |
| `uint64` | 0 | 18,446,744,073,709,551,615 | `u64` |

**Note:** The engine is optimized for 32-bit types. Smaller variants are mainly for interfacing with application-defined variables.

**Aliases:**
- `int32` is an alias for `int`
- `uint32` is an alias for `uint`

## Real Number Types

| Type | Range | Smallest Positive | Max Digits | Rust Equivalent |
|------|-------|-------------------|------------|-----------------|
| `float` | ±3.402823466e+38 | 1.175494351e-38 | 6 | `f32` |
| `double` | ±1.79769313486231e+308 | 2.22507385850720e-308 | 15 | `f64` |

**Special Values:**
- Positive and negative zero
- Positive and negative infinity
- NaN (Not-a-Number) - for `float`, represented as 0x7fc00000

## Primitive Initialization

Variables of primitive types declared without an initial value will have **undefined/random values**. This differs from objects which get default-constructed.

```angelscript
int a;        // undefined value!
int b = 0;    // explicitly initialized
float f;      // undefined value!
float g = 0.0f; // explicitly initialized
```
