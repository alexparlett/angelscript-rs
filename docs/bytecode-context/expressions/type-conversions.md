# Type Conversions

## Overview
Type conversions change a value from one type to another. AngelScript supports both implicit conversions (automatic, performed by the compiler) and explicit conversions (requested by the programmer). Conversions are categorized as value casts (creating a new value) and reference casts (reinterpreting a handle through a different type interface).

## Syntax
```angelscript
// Implicit conversion (automatic)
float f = 3;                    // int to float
Base@ b = @derived;             // derived handle to base handle

// Explicit value cast (constructor-style)
int i = int(3.7f);             // float to int
float f = float(a) / 2;       // int to float

// Explicit reference cast
Derived@ d = cast<Derived>(baseHandle);

// Object conversion operators
double d = doubleConvertibleObj;           // uses opImplConv
int i = int(explicitConvertibleObj);       // uses opConv
```

## Semantics

### Implicit Conversions
Implicit conversions are inserted automatically by the compiler when types do not match exactly. They follow these rules:

**Primitive widening (always safe):**

| From | To |
|------|----|
| `int8` | `int16`, `int`, `int64` |
| `int16` | `int`, `int64` |
| `int` | `int64` |
| `uint8` | `uint16`, `uint`, `uint64` |
| `uint16` | `uint`, `uint64` |
| `uint` | `uint64` |
| Any integer | `float`, `double` (precision may be lost) |
| `float` | `double` |
| Enum | Integer of same or larger size |

**Handle conversions (always safe):**

| From | To |
|------|----|
| `Derived@` | `Base@` (upcast) |
| `Class@` | `Interface@` (if class implements interface) |
| `T@` | `const T@` (add const) |

**Not implicitly convertible:**
- Larger to smaller primitives (narrowing)
- `float`/`double` to integer
- `Base@` to `Derived@` (requires explicit cast)
- Signed to unsigned and vice versa

### Explicit Value Casts
Constructor-style syntax `Type(expr)` creates a new value of the target type. For primitives, this performs the conversion directly (e.g., truncation for float-to-int). For objects, this calls a conversion constructor.

### Explicit Reference Casts
`cast<Type>(handle)` attempts to reinterpret a handle through a different type interface. The handle still refers to the same object; only the view changes. Returns `null` if the actual object type is not compatible with the target type (e.g., a downcast to the wrong derived class).

### Conversion Cost (Overload Resolution)
When resolving overloaded functions or conditional expression branch compatibility, conversions are ranked by cost:

1. No conversion (exact match)
2. Const conversion (`T` to `const T`)
3. Enum to integer (same size)
4. Enum to integer (different size)
5. Primitive widening
6. Primitive narrowing
7. Signed/unsigned change
8. Integer to float
9. Float to integer
10. Reference cast
11. Object to primitive (`opConv` / `opImplConv`)
12. Conversion to object (conversion constructor)
13. Variable argument type

Lower cost equals better match. If two overloads tie in cost, it is a compile-time ambiguity error.

### Class Conversion Operators

| Mechanism | Direction | Usage |
|-----------|-----------|-------|
| Constructor | Other -> Class | Implicit or explicit |
| `explicit` constructor | Other -> Class | Explicit only |
| `opConv` | Class -> Other | Explicit only |
| `opImplConv` | Class -> Other | Implicit or explicit |
| `opCast` | Class@ -> Other@ | Explicit reference cast |
| `opImplCast` | Class@ -> Other@ | Implicit reference cast |
| Inheritance | Derived@ -> Base@ | Implicit upcast |
| `cast<T>` | Base@ -> Derived@ | Explicit downcast |

## Examples
```angelscript
// Implicit primitive widening
int a = 5;
float b = 2.0f;
float c = a + b;            // a promoted to float

// Implicit handle upcast
class Animal {}
class Dog : Animal {}
Dog dog;
Animal@ animal = @dog;      // implicit upcast

// Explicit value cast
float f = 3.7f;
int i = int(f);              // i = 3 (truncated toward zero)

// Explicit reference cast (downcast)
Animal@ animalRef = Dog();
Dog@ dogRef = cast<Dog>(animalRef);  // succeeds if actually a Dog
if (dogRef !is null) {
    // safe to use dogRef
}

// Failed cast returns null
class Cat : Animal {}
Cat@ catRef = cast<Cat>(animalRef);  // null (it's a Dog, not a Cat)

// opImplConv
class Temperature {
    double celsius;
    double opImplConv() const { return celsius; }
}
Temperature t;
t.celsius = 100.0;
double d = t;                // calls opImplConv, d = 100.0

// Conversion constructor
class Wrapper {
    int val;
    Wrapper(int v) { val = v; }
}
Wrapper w = 42;              // implicit conversion via constructor
```

## Compilation Notes
- **Stack behavior:**
  - **Primitive conversions:** The source value is on the stack. A conversion instruction (e.g., `i2f`, `f2i`, `i8_to_i32`) replaces it with the converted value. One pop, one push (or in-place transformation).
  - **Value casts (constructor):** The target type's constructor is called with the source value as an argument. This may involve allocating space for the new object, pushing the source value, and calling the constructor.
  - **Reference casts:** The source handle is on the stack. A runtime type check is performed. If the check succeeds, the handle remains on the stack (possibly with an adjusted vtable pointer). If it fails, `null` is pushed instead.
- **Type considerations:**
  - The compiler must determine the conversion path at compile time: which conversion mechanism to use and whether it is valid.
  - For implicit conversions, the compiler inserts the conversion bytecodes transparently.
  - For explicit conversions, the programmer's intent is clear, so the compiler may allow conversions that would not be permitted implicitly (e.g., narrowing).
  - In arithmetic expressions, the "common type" is determined by finding the widest type among the operands. All narrower operands are promoted.
- **Control flow:**
  - Primitive and value conversions involve no branching.
  - Reference casts (`cast<T>`) require a runtime type check, which involves a conditional: if the type check fails, the result is `null` instead of the handle. This compiles to a type-check instruction followed by a conditional null-push.
- **Special cases:**
  - **Narrowing conversions:** Explicit narrowing (e.g., `int(3.7f)`) truncates toward zero for float-to-int. The bytecode must use the truncating variant of the conversion instruction.
  - **Precision loss:** Integer-to-float conversion may lose precision for large integers (e.g., `int64` values exceeding the float mantissa width). No runtime error is raised.
  - **Conversion chains:** The compiler may need to chain multiple conversions (e.g., `int8` -> `int` -> `float`) if no direct conversion path exists.
  - **opConv/opImplConv:** These compile to method calls on the source object. The method returns the converted value which replaces the object on the stack.
  - **opCast/opImplCast:** These compile to method calls returning a handle. A null check on the result may be needed depending on the usage context.
  - **Boolean context:** `opImplConv` returning `bool` is NOT used in boolean conditions for reference types (to avoid ambiguity between handle-null-check and value-conversion). The compiler must enforce this restriction.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::Cast` | Cast expression variant | Wraps `&CastExpr` |
| `CastExpr` | Explicit type cast | `target_type: TypeExpr`, `expr: &Expr`, `span: Span` |

**Notes:**
- `CastExpr` covers explicit reference casts (`cast<Type>(expr)`) in the AST.
- Constructor-style value casts (`int(3.7f)`) are parsed as `Expr::Call` with the type name as the callee, not as `Expr::Cast`. The distinction between a value cast and an anonymous object constructor call is resolved during semantic analysis.
- Implicit conversions inserted by the compiler do not appear in the AST; they are added during type checking / bytecode generation.
- `Expr::Lambda` / `LambdaExpr` is a separate expression type (documented in `functions/anonymous-functions.md`, not in scope here) that is unrelated to type conversions.

## Related Features
- [math-operators.md](math-operators.md) - Implicit promotion in arithmetic
- [equality-comparison.md](equality-comparison.md) - Type coercion in comparisons
- [conditional-expression.md](conditional-expression.md) - Least-cost conversion for branch types
- [handle-of.md](handle-of.md) - Handle type conversions
- [anonymous-objects.md](anonymous-objects.md) - Constructor-style casts vs anonymous objects
- [function-calls.md](function-calls.md) - Conversion cost in overload resolution
