# AngelScript Operator Overloading Reference

This document provides a comprehensive overview of all operators that can be overloaded in AngelScript classes, their method signatures, and behavior.

## Overview

AngelScript allows classes to define custom behavior for operators through specially-named methods. When an operator is used on an object, the compiler rewrites the expression to call the corresponding method.

| Category | Operators | Method Pattern |
|----------|-----------|----------------|
| Prefixed Unary | `-`, `~`, `++`, `--` | `opFunc()` |
| Postfixed Unary | `++`, `--` | `opFunc()` |
| Comparison | `==`, `!=`, `<`, `>`, `<=`, `>=`, `is`, `!is` | `opEquals()`, `opCmp()` |
| Assignment | `=`, `+=`, `-=`, etc. | `opAssign()`, `opAddAssign()`, etc. |
| Binary | `+`, `-`, `*`, `/`, etc. | `opAdd()`, `opSub()`, etc. |
| Index | `[]` | `opIndex()` |
| Call | `()` | `opCall()` |
| Conversion | cast, implicit | `opConv()`, `opImplConv()` |
| Iteration | foreach | `opForBegin()`, `opForEnd()`, etc. |

---

## Prefixed Unary Operators

Prefixed unary operators appear before the operand: `op a`

The compiler rewrites these as: `a.opFunc()`

| Operator | Expression | Method | Signature |
|----------|------------|--------|-----------|
| Negation | `-a` | `opNeg` | `T opNeg()` |
| Bitwise NOT | `~a` | `opCom` | `T opCom()` |
| Pre-increment | `++a` | `opPreInc` | `T& opPreInc()` |
| Pre-decrement | `--a` | `opPreDec` | `T& opPreDec()` |

**Example:**
```angelscript
class Vector2 {
    float x, y;

    Vector2 opNeg() {
        return Vector2(-x, -y);
    }
}

Vector2 v(1, 2);
Vector2 negated = -v;  // Calls v.opNeg()
```

---

## Postfixed Unary Operators

Postfixed unary operators appear after the operand: `a op`

The compiler rewrites these as: `a.opFunc()`

| Operator | Expression | Method | Signature |
|----------|------------|--------|-----------|
| Post-increment | `a++` | `opPostInc` | `T opPostInc()` |
| Post-decrement | `a--` | `opPostDec` | `T opPostDec()` |

**Note:** Post-increment/decrement typically return the **old** value before modification.

**Example:**
```angelscript
class Counter {
    int value;

    Counter opPostInc() {
        Counter old = this;
        value++;
        return old;
    }
}
```

---

## Comparison Operators

### Equality (`==`, `!=`, `is`, `!is`)

The compiler rewrites `a == b` as `a.opEquals(b)` or `b.opEquals(a)`.

| Method | Signature | Return |
|--------|-----------|--------|
| `opEquals` | `bool opEquals(const T& in other) const` | `true` if equal |

**Behavior:**
- Returns `bool`
- `!=` is derived by negating the result of `opEquals`
- For `is` and `!is`, the method must accept a handle (`@`) to compare object identity

**Example:**
```angelscript
class Point {
    int x, y;

    bool opEquals(const Point& in other) const {
        return x == other.x && y == other.y;
    }
}

Point a, b;
if (a == b) { }  // Calls a.opEquals(b)
if (a != b) { }  // Calls !a.opEquals(b)
```

### Relational (`<`, `<=`, `>`, `>=`)

The compiler rewrites `a < b` as `a.opCmp(b) < 0` (or equivalent for `b.opCmp(a)`).

| Method | Signature | Return |
|--------|-----------|--------|
| `opCmp` | `int opCmp(const T& in other) const` | comparison result |

**Return Value Semantics:**
| Return | Meaning |
|--------|---------|
| `< 0` | `this` is less than `other` |
| `== 0` | `this` equals `other` |
| `> 0` | `this` is greater than `other` |

**Example:**
```angelscript
class Version {
    int major, minor;

    int opCmp(const Version& in other) const {
        if (major != other.major)
            return major - other.major;
        return minor - other.minor;
    }
}

Version a(1, 0), b(2, 0);
if (a < b) { }   // Calls a.opCmp(b) < 0 -> true
if (a >= b) { }  // Calls a.opCmp(b) >= 0 -> false
```

---

## Assignment Operators

Assignment operators modify the left operand and return a reference to `this`.

The compiler rewrites `a op= b` as `a.opFunc(b)`.

| Operator | Expression | Method | Signature |
|----------|------------|--------|-----------|
| Assign | `a = b` | `opAssign` | `T& opAssign(const T& in)` |
| Add-assign | `a += b` | `opAddAssign` | `T& opAddAssign(const T& in)` |
| Sub-assign | `a -= b` | `opSubAssign` | `T& opSubAssign(const T& in)` |
| Mul-assign | `a *= b` | `opMulAssign` | `T& opMulAssign(const T& in)` |
| Div-assign | `a /= b` | `opDivAssign` | `T& opDivAssign(const T& in)` |
| Mod-assign | `a %= b` | `opModAssign` | `T& opModAssign(const T& in)` |
| Pow-assign | `a **= b` | `opPowAssign` | `T& opPowAssign(const T& in)` |
| And-assign | `a &= b` | `opAndAssign` | `T& opAndAssign(const T& in)` |
| Or-assign | `a \|= b` | `opOrAssign` | `T& opOrAssign(const T& in)` |
| Xor-assign | `a ^= b` | `opXorAssign` | `T& opXorAssign(const T& in)` |
| Shl-assign | `a <<= b` | `opShlAssign` | `T& opShlAssign(int)` |
| Shr-assign | `a >>= b` | `opShrAssign` | `T& opShrAssign(int)` |
| UShr-assign | `a >>>= b` | `opUShrAssign` | `T& opUShrAssign(int)` |

**Auto-generated opAssign:**
The compiler automatically generates a default `opAssign` that performs member-wise copy. This can be explicitly deleted if assignment should not be allowed.

**Example:**
```angelscript
class Vector2 {
    float x, y;

    Vector2& opAddAssign(const Vector2& in other) {
        x += other.x;
        y += other.y;
        return this;
    }
}

Vector2 a(1, 2), b(3, 4);
a += b;  // Calls a.opAddAssign(b), a is now (4, 6)
```

---

## Binary Operators

Binary operators take two operands: `a op b`

The compiler rewrites as: `a.opFunc(b)` or `b.opFunc_r(a)` (for reverse)

| Operator | Expression | Method | Reverse Method |
|----------|------------|--------|----------------|
| Add | `a + b` | `opAdd` | `opAdd_r` |
| Subtract | `a - b` | `opSub` | `opSub_r` |
| Multiply | `a * b` | `opMul` | `opMul_r` |
| Divide | `a / b` | `opDiv` | `opDiv_r` |
| Modulo | `a % b` | `opMod` | `opMod_r` |
| Power | `a ** b` | `opPow` | `opPow_r` |
| Bitwise AND | `a & b` | `opAnd` | `opAnd_r` |
| Bitwise OR | `a \| b` | `opOr` | `opOr_r` |
| Bitwise XOR | `a ^ b` | `opXor` | `opXor_r` |
| Shift left | `a << b` | `opShl` | `opShl_r` |
| Shift right | `a >> b` | `opShr` | `opShr_r` |
| Unsigned shift right | `a >>> b` | `opUShr` | `opUShr_r` |

**Typical Signatures:**
```angelscript
T opAdd(const T& in other) const
T opAdd_r(const U& in other) const  // For U + T when U doesn't have opAdd
```

**Reverse Methods (`_r` suffix):**
When `a.opFunc(b)` is not available, the compiler tries `b.opFunc_r(a)`. This allows mixed-type operations where only one type defines the operator.

**Example:**
```angelscript
class Vector2 {
    float x, y;

    Vector2 opAdd(const Vector2& in other) const {
        return Vector2(x + other.x, y + other.y);
    }

    // Allow: float * Vector2
    Vector2 opMul_r(float scalar) const {
        return Vector2(x * scalar, y * scalar);
    }
}

Vector2 a(1, 2), b(3, 4);
Vector2 c = a + b;     // Calls a.opAdd(b)
Vector2 d = 2.0f * a;  // Calls a.opMul_r(2.0f)
```

---

## Index Operator

The index operator `a[i]` allows array-like access.

The compiler rewrites `a[i]` as `a.opIndex(i)`.

| Method | Signature | Usage |
|--------|-----------|-------|
| `opIndex` | `T& opIndex(int index)` | Read/write access |
| `opIndex` (const) | `const T& opIndex(int index) const` | Read-only access |

**Alternative Property Syntax:**
For more control, use property accessors:

```angelscript
class Array {
    // Getter
    int get_opIndex(int idx) const property {
        return data[idx];
    }

    // Setter
    void set_opIndex(int idx, int value) property {
        data[idx] = value;
    }
}
```

**Example:**
```angelscript
class IntArray {
    int[] data;

    int& opIndex(int index) {
        return data[index];
    }
}

IntArray arr;
arr[0] = 42;      // Calls arr.opIndex(0) for assignment
int x = arr[0];   // Calls arr.opIndex(0) for read
```

---

## Functor Operator (Call Operator)

The call operator allows objects to be called like functions: `obj(args)`

The compiler rewrites `expr(arglist)` as `expr.opCall(arglist)`.

| Method | Signature |
|--------|-----------|
| `opCall` | `ReturnType opCall(params...)` |

**Example:**
```angelscript
class Adder {
    int base;

    int opCall(int value) {
        return base + value;
    }
}

Adder add5;
add5.base = 5;
int result = add5(10);  // Calls add5.opCall(10) -> 15
```

---

## Type Conversion Operators

Type conversion operators allow objects to be converted to other types.

### Explicit Conversion (`opConv`, `opCast`)

Used with explicit cast syntax: `T(obj)` or `cast<T>(obj)`

| Method | Usage | Return |
|--------|-------|--------|
| `opConv` | Value conversion | `T opConv() const` |
| `opCast` | Handle conversion | `T@ opCast()` |

### Implicit Conversion (`opImplConv`, `opImplCast`)

Used automatically when the compiler needs a type conversion.

| Method | Usage | Return |
|--------|-------|--------|
| `opImplConv` | Implicit value conversion | `T opImplConv() const` |
| `opImplCast` | Implicit handle conversion | `T@ opImplCast()` |

**Important Note:**
The compiler will **not** use `bool opImplConv()` on reference types to avoid ambiguity with null checks.

**Example:**
```angelscript
class Wrapper {
    int value;

    // Explicit: int(wrapper) or cast<int>(wrapper)
    int opConv() const {
        return value;
    }

    // Implicit: automatically converts where int is expected
    int opImplConv() const {
        return value;
    }
}

Wrapper w;
w.value = 42;
int explicit_val = int(w);  // Explicit conversion
int implicit_val = w;        // Implicit conversion (if opImplConv defined)
```

---

## Foreach Loop Operators

These operators enable iteration over custom containers with `foreach`.

| Method | Signature | Purpose |
|--------|-----------|---------|
| `opForBegin` | `IteratorType opForBegin()` | Initialize iterator |
| `opForEnd` | `bool opForEnd(IteratorType iter)` | Check if iteration complete |
| `opForNext` | `IteratorType opForNext(IteratorType iter)` | Advance iterator |
| `opForValue` | `ValueType opForValue(IteratorType iter)` | Get current value |

**Multiple Values:**
For containers that return multiple values per iteration (like maps), use numbered variants:
- `opForValue0(iter)` - First value (e.g., key)
- `opForValue1(iter)` - Second value (e.g., value)

**Example:**
```angelscript
class IntList {
    int[] items;

    int opForBegin() { return 0; }

    bool opForEnd(int iter) {
        return iter >= items.length();
    }

    int opForNext(int iter) {
        return iter + 1;
    }

    int opForValue(int iter) {
        return items[iter];
    }
}

IntList list;
foreach (int item : list) {
    // Iterates over all items
}
```

---

## Operator Resolution Order

When the compiler encounters an operator expression, it attempts resolution in this order:

1. **Direct method**: `a.opFunc(b)`
2. **Reverse method**: `b.opFunc_r(a)`
3. **Type conversion + operator**: Convert operand, then apply operator

For comparison operators:
1. Try `a.opEquals(b)` or `a.opCmp(b)`
2. Try `b.opEquals(a)` or `b.opCmp(a)` (with result adjustment)

---

## Summary Table

| Category | Operators | Methods |
|----------|-----------|---------|
| **Unary Prefix** | `-`, `~`, `++`, `--` | `opNeg`, `opCom`, `opPreInc`, `opPreDec` |
| **Unary Postfix** | `++`, `--` | `opPostInc`, `opPostDec` |
| **Comparison** | `==`, `!=` | `opEquals` |
| **Relational** | `<`, `<=`, `>`, `>=` | `opCmp` |
| **Identity** | `is`, `!is` | `opEquals` (with handle) |
| **Assignment** | `=`, `+=`, `-=`, etc. | `opAssign`, `opAddAssign`, etc. |
| **Arithmetic** | `+`, `-`, `*`, `/`, `%`, `**` | `opAdd`, `opSub`, `opMul`, `opDiv`, `opMod`, `opPow` |
| **Bitwise** | `&`, `\|`, `^`, `<<`, `>>`, `>>>` | `opAnd`, `opOr`, `opXor`, `opShl`, `opShr`, `opUShr` |
| **Index** | `[]` | `opIndex` |
| **Call** | `()` | `opCall` |
| **Conversion** | cast | `opConv`, `opCast`, `opImplConv`, `opImplCast` |
| **Iteration** | foreach | `opForBegin`, `opForEnd`, `opForNext`, `opForValue` |
