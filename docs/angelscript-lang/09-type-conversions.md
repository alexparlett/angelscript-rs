# Type Conversions

## Implicit Conversions

Implicit conversions happen automatically when types don't match exactly.

### Primitive Type Conversions

| From | To | Notes |
|------|-----|-------|
| `int8` | Larger signed int | Widening |
| `int16` | Larger signed int | Widening |
| `int` | `int64` | Widening |
| `uint8` | Larger unsigned int | Widening |
| `uint16` | Larger unsigned int | Widening |
| `uint` | `uint64` | Widening |
| Any int | `float` | Precision may be lost |
| Any int | `double` | Precision may be lost |
| `float` | `double` | Widening |
| Enum | Integer of same/larger size | |

### Handle Conversions

| From | To | Notes |
|------|-----|-------|
| Derived@ | Base@ | Upcast always safe |
| Class@ | Interface@ | If class implements interface |
| T@ | const T@ | Add const |

### Cannot Implicitly Convert

- Larger to smaller primitives (narrowing)
- `float`/`double` to integer
- Base@ to Derived@ (needs explicit cast)
- Signed to unsigned and vice versa (can lose data)

## Explicit Conversions

### Value Cast (Constructor-style)

```angelscript
float f = 3.7f;
int i = int(f);      // i = 3 (truncated)

MyClass obj = MyClass(value);  // Conversion constructor
```

### Reference Cast

```angelscript
Derived@ d = cast<Derived>(baseHandle);
```

- Returns `null` if cast is invalid (object isn't actually the target type)
- Does NOT create a copy - same object, different handle type

## Conversion Cost (Overload Resolution)

When resolving overloaded functions, conversions are ranked:

1. **No conversion** (exact match)
2. **Const conversion** (`T` to `const T`)
3. **Enum to integer** (same size)
4. **Enum to integer** (different size)
5. **Primitive widening** (size increases)
6. **Primitive narrowing** (size decreases)
7. **Signed/unsigned change**
8. **Integer to float**
9. **Float to integer**
10. **Reference cast**
11. **Object to primitive** (`opConv`/`opImplConv`)
12. **Conversion to object** (conversion constructor)
13. **Variable argument type**

Lower cost = better match.

## Class Conversion Operators

### opImplConv - Implicit Conversion

```angelscript
class MyValue {
    double value;

    // Implicit conversion TO double
    double opImplConv() const {
        return value;
    }
}

MyValue v;
double d = v;  // Calls opImplConv
```

### opConv - Explicit Conversion Only

```angelscript
class MyValue {
    double value;

    // Explicit conversion TO int
    int opConv() const {
        return int(value);
    }
}

MyValue v;
int i = int(v);  // OK - explicit
int j = v;       // Error - no implicit conversion
```

### Conversion Constructors

```angelscript
class MyClass {
    // Implicit conversion FROM int
    MyClass(int value) {
        // ...
    }

    // Explicit conversion FROM string
    MyClass(string s) explicit {
        // ...
    }
}

MyClass a = 42;           // OK - implicit
MyClass b = "hello";      // Error - explicit required
MyClass c = MyClass("hello");  // OK - explicit
```

## Reference Casts (opCast/opImplCast)

For returning a **handle to a different type** (same object):

```angelscript
class MyClass {
    OtherClass@ other;

    // Explicit reference cast
    OtherClass@ opCast() {
        return other;
    }

    const OtherClass@ opCast() const {
        return other;
    }
}

MyClass@ m = MyClass();
OtherClass@ o = cast<OtherClass>(m);  // Uses opCast
```

### opImplCast

Same as `opCast` but allows implicit reference casting:

```angelscript
class MyClass {
    OtherClass@ opImplCast() { return other; }
}

OtherClass@ o = myClassHandle;  // Implicit cast
```

## Boolean Context

For reference types, `opImplConv` returning `bool` is **NOT** used in boolean conditions (ambiguous whether checking handle or value).

Instead, check explicitly:
```angelscript
if (handle !is null) {}    // Check handle
if (handle.IsValid()) {}   // Check value via method
```

## Conversion in Expressions

### Arithmetic Operations

Both operands converted to common type:
```angelscript
int a = 5;
float b = 2.0f;
auto c = a + b;  // a converted to float, result is float
```

### Comparison

```angelscript
int a = 5;
float b = 5.0f;
if (a == b) {}  // a converted to float for comparison
```

### Assignment

Right-hand side converted to left-hand type:
```angelscript
float f = 3;      // 3 converted to float
int i = 3.7f;     // Error - no implicit float to int
int j = int(3.7f); // OK - explicit
```

## Summary Table

| Mechanism | Direction | When Used |
|-----------|-----------|-----------|
| Constructor | Other → Class | Explicit or implicit |
| `explicit` constructor | Other → Class | Explicit only |
| `opConv` | Class → Other | Explicit only |
| `opImplConv` | Class → Other | Implicit or explicit |
| `opCast` | Class@ → Other@ | Explicit reference cast |
| `opImplCast` | Class@ → Other@ | Implicit reference cast |
| Inheritance | Derived@ → Base@ | Implicit upcast |
| `cast<T>` | Base@ → Derived@ | Explicit downcast |
