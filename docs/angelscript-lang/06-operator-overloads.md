# Operator Overloads

Operator overloading is done by implementing class methods with specific names. The compiler recognizes these methods and rewrites expressions to call them.

## Prefixed Unary Operators

| Operator | Method Name |
|----------|-------------|
| `-` | `opNeg` |
| `~` | `opCom` |
| `++` | `opPreInc` |
| `--` | `opPreDec` |

Expression `op a` is rewritten as `a.opfunc()`.

## Postfixed Unary Operators

| Operator | Method Name |
|----------|-------------|
| `++` | `opPostInc` |
| `--` | `opPostDec` |

Expression `a op` is rewritten as `a.opfunc()`.

## Comparison Operators

| Operator | Method Name |
|----------|-------------|
| `==` | `opEquals` |
| `!=` | `opEquals` (result negated) |
| `<` | `opCmp` |
| `<=` | `opCmp` |
| `>` | `opCmp` |
| `>=` | `opCmp` |
| `is` | `opEquals` (handle comparison) |
| `!is` | `opEquals` (handle comparison, negated) |

### opEquals

- `a == b` tries both `a.opEquals(b)` and `b.opEquals(a)`, uses best match
- Must return `bool`

### opCmp

- `a < b` is rewritten as `a.opCmp(b) < 0` or `0 < b.opCmp(a)`
- Must return `int`
- Return negative if `this` < argument
- Return 0 if equal
- Return positive if `this` > argument

**Fallback:** If `opEquals` not found, compiler looks for `opCmp`.

### Identity (is/!is)

`opEquals` should take a **handle** parameter to compare addresses:

```angelscript
bool opEquals(const MyClass@ other) const {
    return this is other;  // compare addresses
}
```

## Assignment Operators

| Operator | Method Name |
|----------|-------------|
| `=` | `opAssign` |
| `+=` | `opAddAssign` |
| `-=` | `opSubAssign` |
| `*=` | `opMulAssign` |
| `/=` | `opDivAssign` |
| `%=` | `opModAssign` |
| `**=` | `opPowAssign` |
| `&=` | `opAndAssign` |
| `\|=` | `opOrAssign` |
| `^=` | `opXorAssign` |
| `<<=` | `opShlAssign` |
| `>>=` | `opShrAssign` |
| `>>>=` | `opUShrAssign` |

Expression `a op= b` is rewritten as `a.opfunc(b)`.

**Example:**
```angelscript
obj@ opAssign(const obj &in other) {
    // copy data from other
    return this;  // return handle for chaining
}
```

**Default:** All script classes have a default `opAssign` that does bitwise copy.

## Binary Operators

| Operator | Method | Reverse Method |
|----------|--------|----------------|
| `+` | `opAdd` | `opAdd_r` |
| `-` | `opSub` | `opSub_r` |
| `*` | `opMul` | `opMul_r` |
| `/` | `opDiv` | `opDiv_r` |
| `%` | `opMod` | `opMod_r` |
| `**` | `opPow` | `opPow_r` |
| `&` | `opAnd` | `opAnd_r` |
| `\|` | `opOr` | `opOr_r` |
| `^` | `opXor` | `opXor_r` |
| `<<` | `opShl` | `opShl_r` |
| `>>` | `opShr` | `opShr_r` |
| `>>>` | `opUShr` | `opUShr_r` |

Expression `a op b` tries both `a.opfunc(b)` and `b.opfunc_r(a)`, uses best match.

**The `_r` suffix** indicates "reverse" - used when the operand order is swapped. This enables expressions like `5 + myObject` where the class is on the right.

## Index Operator

| Operator | Method Name |
|----------|-------------|
| `[]` | `opIndex` |

Expression `a[i]` is rewritten as `a.opIndex(i)`.

**Property accessor form:**
```angelscript
class MyObj {
    float get_opIndex(int idx) const { return 0; }
    void set_opIndex(int idx, float value) { }
}
```

- Read: `a[i]` → `a.get_opIndex(i)`
- Write: `a[i] = x` → `a.set_opIndex(i, x)`

## Functor Operator (Call)

| Operator | Method Name |
|----------|-------------|
| `()` | `opCall` |

Expression `expr(args)` is rewritten as `expr.opCall(args)` when `expr` evaluates to an object.

## Type Conversion Operators

| Expression | Methods Tried |
|------------|---------------|
| `type(expr)` | constructor, `opConv`, `opImplConv` |
| `cast<type>(expr)` | `opCast`, `opImplCast` |

### Value Conversions (opConv / opImplConv)

```angelscript
class MyObj {
    double myValue;

    // Implicit conversion FROM double (conversion constructor)
    MyObj(double v) { myValue = v; }

    // Implicit conversion TO double
    double opImplConv() const { return myValue; }

    // Explicit-only conversion FROM int
    MyObj(int v) explicit { myValue = v; }

    // Explicit-only conversion TO int
    int opConv() const { return int(myValue); }
}
```

- `opImplConv` - implicit conversions allowed
- `opConv` - explicit conversion only (requires cast syntax)
- `explicit` keyword on constructors - prevents implicit use

### Reference Casts (opCast / opImplCast)

For returning a **different handle type to the same object**:

```angelscript
class MyObjA {
    MyObjB@ objB;
    MyObjC@ objC;

    // Explicit cast to MyObjB
    MyObjB@ opCast() { return objB; }
    const MyObjB@ opCast() const { return objB; }

    // Implicit cast to MyObjC
    MyObjC@ opImplCast() { return objC; }
    const MyObjC@ opImplCast() const { return objC; }
}
```

**Important:** For reference types, `bool opImplConv` is NOT used in boolean conditions (ambiguous whether checking handle or object).

## Summary: Method Resolution

For `a op b`:
1. Try `a.opfunc(b)`
2. Try `b.opfunc_r(a)`
3. Choose best match based on implicit conversion cost

For comparison `a == b`:
1. Try `a.opEquals(b)`
2. Try `b.opEquals(a)`
3. Fall back to `opCmp` if `opEquals` not found
