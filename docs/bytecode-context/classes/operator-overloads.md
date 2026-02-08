# Operator Overloads

## Overview
Operator overloading allows script classes to define custom behavior for standard operators (+, -, *, ==, [], etc.). This is accomplished by implementing specially named class methods that the compiler recognizes and rewrites expressions to call. Operator overloading improves code readability by enabling natural expression syntax for user-defined types. AngelScript supports prefix unary, postfix unary, comparison, assignment, binary, index, functor (call), and type conversion operators.

## Syntax

### Prefixed Unary Operators
```angelscript
class MyClass
{
    MyClass opNeg()     { /* return negated */ }    // -obj
    MyClass opCom()     { /* return complement */ } // ~obj
    MyClass opPreInc()  { /* increment, return */ } // ++obj
    MyClass opPreDec()  { /* decrement, return */ } // --obj
}
```

### Postfixed Unary Operators
```angelscript
class MyClass
{
    MyClass opPostInc() { /* return, then increment */ }  // obj++
    MyClass opPostDec() { /* return, then decrement */ }  // obj--
}
```

### Comparison Operators
```angelscript
class MyClass
{
    // Equality: must return bool
    bool opEquals(const MyClass &in other) const { return value == other.value; }

    // Comparison: must return int (-1, 0, or 1)
    int opCmp(const MyClass &in other) const
    {
        if (value < other.value) return -1;
        if (value > other.value) return 1;
        return 0;
    }

    int value;
}
```

### Assignment Operators
```angelscript
class MyClass
{
    MyClass@ opAssign(const MyClass &in other)      { /* copy */  return this; }  // =
    MyClass@ opAddAssign(const MyClass &in other)   { /* add */   return this; }  // +=
    MyClass@ opSubAssign(const MyClass &in other)   { /* sub */   return this; }  // -=
    MyClass@ opMulAssign(const MyClass &in other)   { /* mul */   return this; }  // *=
    MyClass@ opDivAssign(const MyClass &in other)   { /* div */   return this; }  // /=
    MyClass@ opModAssign(const MyClass &in other)   { /* mod */   return this; }  // %=
    MyClass@ opPowAssign(const MyClass &in other)   { /* pow */   return this; }  // **=
    MyClass@ opAndAssign(const MyClass &in other)   { /* and */   return this; }  // &=
    MyClass@ opOrAssign(const MyClass &in other)    { /* or */    return this; }  // |=
    MyClass@ opXorAssign(const MyClass &in other)   { /* xor */   return this; }  // ^=
    MyClass@ opShlAssign(const MyClass &in other)   { /* shl */   return this; }  // <<=
    MyClass@ opShrAssign(const MyClass &in other)   { /* shr */   return this; }  // >>=
    MyClass@ opUShrAssign(const MyClass &in other)  { /* ushr */  return this; }  // >>>=
}
```

### Binary Operators
```angelscript
class MyClass
{
    // Left-side operator: a + b  ->  a.opAdd(b)
    MyClass opAdd(const MyClass &in other) const { /* ... */ }

    // Right-side (reverse) operator: b + a  ->  a.opAdd_r(b)
    MyClass opAdd_r(const MyClass &in other) const { /* ... */ }
}
```

### Index Operator
```angelscript
class MyClass
{
    // Single method form
    int opIndex(int idx) { return data[idx]; }

    // Property accessor form (separate get/set)
    float get_opIndex(int idx) const       { return values[idx]; }
    void  set_opIndex(int idx, float val)  { values[idx] = val; }
}
```

### Functor (Call) Operator
```angelscript
class Callback
{
    void opCall(int arg)
    {
        print("Called with " + arg);
    }
}
```

### Foreach Iterator Operators
```angelscript
class Container
{
    // Initialize iteration: returns iterator (int or custom type)
    int opForBegin() const { return 0; }

    // Check termination: returns true when iterator should stop
    bool opForEnd(int iterator) const { return iterator >= size; }

    // Advance iterator: returns updated iterator
    int opForNext(int iterator) const { return iterator + 1; }

    // Retrieve value(s): opForValue for single value
    ElementType@ opForValue(int iterator) { return elements[iterator]; }

    // For multiple iteration variables: opForValue0, opForValue1, etc.
    KeyType opForValue0(int iterator) const { return keys[iterator]; }
    ValType@ opForValue1(int iterator) { return values[iterator]; }

    private ElementType@[] elements;
    private KeyType[] keys;
    private ValType@[] values;
    private int size;
}
```

### Type Conversion Operators
```angelscript
class MyClass
{
    // Explicit value conversion: int(myObj)
    int opConv() const { return intValue; }

    // Implicit value conversion: automatically used by compiler
    double opImplConv() const { return doubleValue; }

    // Explicit reference cast: cast<OtherType>(myObj)
    OtherType@ opCast() { return otherRef; }
    const OtherType@ opCast() const { return otherRef; }

    // Implicit reference cast: automatically used by compiler
    AnotherType@ opImplCast() { return anotherRef; }
    const AnotherType@ opImplCast() const { return anotherRef; }
}
```

## Semantics

### Operator-to-Method Mapping

| Category | Operator | Method(s) |
|----------|----------|-----------|
| Prefix unary | `-` | `opNeg` |
| Prefix unary | `~` | `opCom` |
| Prefix unary | `++` | `opPreInc` |
| Prefix unary | `--` | `opPreDec` |
| Postfix unary | `++` | `opPostInc` |
| Postfix unary | `--` | `opPostDec` |
| Comparison | `==`, `!=` | `opEquals` (must return `bool`) |
| Comparison | `<`, `<=`, `>`, `>=` | `opCmp` (must return `int`) |
| Comparison | `is`, `!is` | `opEquals` (takes handle parameter) |
| Assignment | `=` | `opAssign` |
| Assignment | `+=` | `opAddAssign` |
| Assignment | `-=` | `opSubAssign` |
| Assignment | `*=` | `opMulAssign` |
| Assignment | `/=` | `opDivAssign` |
| Assignment | `%=` | `opModAssign` |
| Assignment | `**=` | `opPowAssign` |
| Assignment | `&=` | `opAndAssign` |
| Assignment | `\|=` | `opOrAssign` |
| Assignment | `^=` | `opXorAssign` |
| Assignment | `<<=` | `opShlAssign` |
| Assignment | `>>=` | `opShrAssign` |
| Assignment | `>>>=` | `opUShrAssign` |
| Binary | `+` | `opAdd` / `opAdd_r` |
| Binary | `-` | `opSub` / `opSub_r` |
| Binary | `*` | `opMul` / `opMul_r` |
| Binary | `/` | `opDiv` / `opDiv_r` |
| Binary | `%` | `opMod` / `opMod_r` |
| Binary | `**` | `opPow` / `opPow_r` |
| Binary | `&` | `opAnd` / `opAnd_r` |
| Binary | `\|` | `opOr` / `opOr_r` |
| Binary | `^` | `opXor` / `opXor_r` |
| Binary | `<<` | `opShl` / `opShl_r` |
| Binary | `>>` | `opShr` / `opShr_r` |
| Binary | `>>>` | `opUShr` / `opUShr_r` |
| Index | `[]` | `opIndex` or `get_opIndex`/`set_opIndex` |
| Functor | `()` | `opCall` |
| Foreach | `foreach(var : container)` | `opForBegin`, `opForEnd`, `opForNext`, `opForValue` / `opForValue0`, `opForValue1`, ... |
| Conversion | `type(expr)` | constructor, `opConv`, `opImplConv` |
| Conversion | `cast<type>(expr)` | `opCast`, `opImplCast` |

### Resolution Rules

**Unary operators:** `op a` is rewritten as `a.opfunc()`.

**Binary operators:** `a op b` is rewritten as both `a.opfunc(b)` and `b.opfunc_r(a)`. The compiler selects the best match based on implicit conversion cost. The `_r` (reverse) suffix is used when the class instance is on the right side of the operator.

**Comparison operators:**
- `a == b` tries `a.opEquals(b)` and `b.opEquals(a)`, choosing the best match. `!=` negates the result.
- `a < b` tries `a.opCmp(b) < 0` and `0 < b.opCmp(a)`, choosing the best match. Other relational operators use the same pattern with appropriate comparison against 0.
- If `opEquals` is not found, the compiler falls back to `opCmp` for equality checks.
- The `is` operator expects `opEquals` to take a **handle** parameter for address comparison.

**Assignment operators:** `a op= b` is rewritten as `a.opfunc(b)`. Assignment operators should return a handle to `this` to support chaining (`a = b = c`).

**Default assignment:** All script classes have a default `opAssign` that performs a bitwise copy of all members. This can be overridden.

**Index operator:** `a[i]` is rewritten as `a.opIndex(i)`. If using property accessor form, reads become `a.get_opIndex(i)` and writes become `a.set_opIndex(i, val)`. Multiple index arguments are supported.

**Functor operator:** `expr(args)` is rewritten as `expr.opCall(args)` when `expr` evaluates to an object.

**Foreach operators:** `foreach(var : container)` is rewritten as:
```angelscript
for (auto @iterator = container.opForBegin();
     !container.opForEnd(iterator);
     iterator = container.opForNext(iterator))
{
    auto var = container.opForValue(iterator);
    // ... loop body
}
```
For multiple iteration variables `foreach(k, v : container)`, the compiler uses `opForValue0` and `opForValue1`:
```angelscript
for (auto @iterator = container.opForBegin();
     !container.opForEnd(iterator);
     iterator = container.opForNext(iterator))
{
    auto k = container.opForValue0(iterator);
    auto v = container.opForValue1(iterator);
    // ... loop body
}
```
- `opForBegin()` must return an iterator value (often `int`, but can be a custom type or handle).
- `opForEnd(iterator)` must return `bool`, indicating termination (returns `true` when done).
- `opForNext(iterator)` must return the next iterator value (same type as `opForBegin`).
- `opForValue(iterator)` returns the current element. For multiple variables, use `opForValue0`, `opForValue1`, etc. (one for each iteration variable).

**Type conversion operators:**
- `type(expr)`: first checks for a conversion constructor on the target type, then tries `expr.opConv()` returning the target type.
- For implicit conversions: checks for non-explicit conversion constructors, then tries `expr.opImplConv()`.
- `cast<type>(expr)`: tries `expr.opCast()` returning a handle of the target type.
- For implicit reference casts: tries `expr.opImplCast()`.
- `opConv`/`opImplConv` are for **value conversions** (creating new instances).
- `opCast`/`opImplCast` are for **reference casts** (returning handles to existing objects).
- `bool opImplConv` is **not used** in boolean conditions for reference types (ambiguous whether checking handle or object).

## Examples
```angelscript
class Vector2
{
    float x, y;

    Vector2() { x = 0; y = 0; }
    Vector2(float _x, float _y) { x = _x; y = _y; }

    // Binary operators
    Vector2 opAdd(const Vector2 &in other) const
    {
        return Vector2(x + other.x, y + other.y);
    }

    Vector2 opSub(const Vector2 &in other) const
    {
        return Vector2(x - other.x, y - other.y);
    }

    // Scalar multiplication: vec * scalar
    Vector2 opMul(float scalar) const
    {
        return Vector2(x * scalar, y * scalar);
    }

    // Reverse scalar multiplication: scalar * vec
    Vector2 opMul_r(float scalar) const
    {
        return Vector2(x * scalar, y * scalar);
    }

    // Unary negation
    Vector2 opNeg() const
    {
        return Vector2(-x, -y);
    }

    // Comparison
    bool opEquals(const Vector2 &in other) const
    {
        return x == other.x && y == other.y;
    }

    // Assignment
    Vector2@ opAssign(const Vector2 &in other)
    {
        x = other.x;
        y = other.y;
        return this;
    }

    Vector2@ opAddAssign(const Vector2 &in other)
    {
        x += other.x;
        y += other.y;
        return this;
    }
}

void example()
{
    Vector2 a(1, 2);
    Vector2 b(3, 4);

    Vector2 c = a + b;        // a.opAdd(b)
    Vector2 d = -a;           // a.opNeg()
    Vector2 e = a * 2.0f;     // a.opMul(2.0f)
    Vector2 f = 2.0f * a;     // a.opMul_r(2.0f)
    a += b;                   // a.opAddAssign(b)

    if (a == b) {}            // a.opEquals(b)
}
```

```angelscript
// Functor example
class Multiplier
{
    float factor;
    Multiplier(float f) { factor = f; }

    float opCall(float input)
    {
        return input * factor;
    }
}

void example()
{
    Multiplier doubler(2.0f);
    float result = doubler(5.0f);  // doubler.opCall(5.0f) -> 10.0f
}
```

```angelscript
// Type conversion example
class Celsius
{
    float degrees;
    Celsius(float d) { degrees = d; }

    // Explicit conversion to Fahrenheit
    Fahrenheit opConv() const
    {
        return Fahrenheit(degrees * 9.0f / 5.0f + 32.0f);
    }
}
```

```angelscript
// Foreach iterator example - simple array-like container
class IntList
{
    private int[] data;

    void add(int value) { data.insertLast(value); }

    // Foreach protocol: use integer index as iterator
    int opForBegin() const
    {
        return 0;
    }

    bool opForEnd(int iterator) const
    {
        return iterator >= data.length();
    }

    int opForNext(int iterator) const
    {
        return iterator + 1;
    }

    int opForValue(int iterator) const
    {
        return data[iterator];
    }
}

void example()
{
    IntList list;
    list.add(10);
    list.add(20);
    list.add(30);

    // Compiler rewrites this into opForBegin/opForEnd/opForNext/opForValue calls
    foreach(int val : list)
    {
        print("Value: " + val);
    }
}
```

```angelscript
// Foreach iterator example - dictionary-like container with multiple iteration variables
class StringMap
{
    private string[] keys;
    private int[] values;

    void set(const string &in key, int val)
    {
        keys.insertLast(key);
        values.insertLast(val);
    }

    // Foreach protocol with multiple iteration variables
    int opForBegin() const
    {
        return 0;
    }

    bool opForEnd(int iterator) const
    {
        return iterator >= keys.length();
    }

    int opForNext(int iterator) const
    {
        return iterator + 1;
    }

    // Return key via opForValue0
    string opForValue0(int iterator) const
    {
        return keys[iterator];
    }

    // Return value via opForValue1
    int opForValue1(int iterator) const
    {
        return values[iterator];
    }
}

void example()
{
    StringMap map;
    map.set("health", 100);
    map.set("mana", 50);

    // Compiler rewrites this into opForValue0 and opForValue1 calls
    foreach(string key, int value : map)
    {
        print(key + " = " + value);
    }
}
```

## Compilation Notes
- **Expression rewriting:** The compiler rewrites operator expressions into method calls during the parsing/semantic analysis phase. For example, `a + b` is transformed into candidate expressions `a.opAdd(b)` and `b.opAdd_r(a)`, and then overload resolution selects the best match. The resulting method call is then compiled like any other method call.
- **Method dispatch:** Operator methods are regular virtual methods. They are stored in the vtable and dispatched through the standard virtual call mechanism. The `this` pointer is pushed first, followed by the operand(s).
- **Binary operator resolution:** For `a op b`, the compiler generates two candidates: `a.opfunc(b)` and `b.opfunc_r(a)`. It scores each based on the cost of implicit conversions needed for the arguments. The lowest-cost match wins. If both are equally good, a compiler error (ambiguity) is raised.
- **Comparison operator compilation:**
  - `a == b`: compiled as `a.opEquals(b)` (or the reverse). The result is a `bool`.
  - `a != b`: compiled as `!a.opEquals(b)`.
  - `a < b`: compiled as `a.opCmp(b) < 0` (or `0 < b.opCmp(a)`). The `opCmp` call returns an `int`, which is then compared against 0 with the original relational operator.
  - Fallback: if `opEquals` is missing but `opCmp` exists, `a == b` compiles as `a.opCmp(b) == 0`.
- **Assignment operator return:** Assignment operators should return `this` (as a handle) to enable chaining. The compiler treats the return value of the assignment as the result of the assignment expression, making `a = b = c` possible.
- **Default opAssign:** The compiler-generated default `opAssign` performs a member-by-member copy (bitwise for primitives, `opAssign` for objects, handle assignment with ref counting for handles).
- **Index operator forms:** When both `opIndex` (single method) and `get_opIndex`/`set_opIndex` (accessor pair) exist, the compiler uses the accessor pair for contexts where it needs to distinguish reads from writes. The single-method `opIndex` returns a reference that can be used for both reading and writing.
- **Conversion operator dispatch:** For `type(expr)`:
  1. Check if `type` has a constructor taking `expr`'s type.
  2. If not, check if `expr`'s type has an `opConv()` returning `type`.
  3. For implicit conversions, check non-explicit constructors first, then `opImplConv()`.
  The conversion call is compiled as a regular method call on the source object (for `opConv`/`opImplConv`) or a constructor/factory call on the target type.
- **Stack behavior:** Operator methods follow the same calling convention as regular methods. The `this` pointer is the first implicit argument. Return values are placed on the stack for the caller to consume. For binary operators, the result is a new object (typically constructed inside the operator method and returned).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `ClassMember::Method` | Operator method within a class | Contains a `FunctionDecl` |
| `FunctionDecl` | Operator method declaration | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `span: Span`, plus other fields |
| `FuncAttr` | Includes `property` flag for accessor-style operators | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |

**Notes:**
- Operator overloads are regular `FunctionDecl` nodes where `name` is the operator method name (e.g., `opAdd`, `opEquals`, `opIndex`, `opCall`, `opConv`). There is no special AST node for operator methods.
- The `FuncAttr.property` flag is used for property-style accessor operators (`get_opIndex`/`set_opIndex` and `get_prop`/`set_prop` methods).
- The `is_const` flag is relevant for operators like `opEquals` and `opCmp`, which should be const methods.
- Expression rewriting (e.g., `a + b` into `a.opAdd(b)`) is a semantic transformation performed by the compiler, not represented in the AST.

## Related Features
- [Methods](./methods.md) - operator overloads are implemented as regular methods
- [Properties](./properties.md) - `get_opIndex`/`set_opIndex` uses property accessor syntax
- [Constructors](./constructors.md) - conversion constructors as an alternative to `opConv`
- [Inheritance](./inheritance.md) - operator methods are virtual and can be overridden
- [Class Declarations](./class-declarations.md) - class body where operators are declared
