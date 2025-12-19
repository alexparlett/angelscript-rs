# Expressions

## Assignments

```angelscript
lvalue = rvalue;
```

- `lvalue` must be a memory location (variable, property, indexed element)
- Assignment evaluates to the assigned value (enables chaining: `a = b = c`)
- **Right-hand side is evaluated before left-hand side**

## Function Calls

```angelscript
func();
func(arg);
func(arg1, arg2);
lvalue = func();
```

**Argument evaluation order:** Arguments are evaluated in **reverse order** (last argument first).

### Output Parameters

```angelscript
void func(int &out outputValue) {
    outputValue = 42;
}

int value;
func(value);    // value receives 42
func(void);     // ignore output with 'void' keyword
```

### Named Arguments

```angelscript
void func(int flagA = false, int flagB = false, int flagC = false) {}

func(flagC: true);              // only set flagC
func(flagB: true, flagA: true); // set B and A
```

No positional arguments may follow named arguments.

## Math Operators

| Operator | Description | Operands | Result |
|----------|-------------|----------|--------|
| `+` (unary) | positive | NUM | NUM |
| `-` (unary) | negative | NUM | NUM |
| `+` | addition | NUM, NUM | NUM |
| `-` | subtraction | NUM, NUM | NUM |
| `*` | multiplication | NUM, NUM | NUM |
| `/` | division | NUM, NUM | NUM |
| `%` | modulo | NUM, NUM | NUM |
| `**` | exponent | NUM, NUM | NUM |

**Notes:**
- Both operands implicitly converted to same type
- Result is same type as (converted) operands
- Unary `-` not available for `uint` types

## Bitwise Operators

| Operator | Description | Operands | Result |
|----------|-------------|----------|--------|
| `~` | complement | NUM | NUM |
| `&` | AND | NUM, NUM | NUM |
| `\|` | OR | NUM, NUM | NUM |
| `^` | XOR | NUM, NUM | NUM |
| `<<` | left shift | NUM, NUM | NUM |
| `>>` | right shift | NUM, NUM | NUM |
| `>>>` | arithmetic right shift | NUM, NUM | NUM |

**Notes:**
- Operands converted to integers (keeping sign)
- Result type matches left operand

## Compound Assignments

```angelscript
lvalue += rvalue;   // equivalent to: lvalue = lvalue + rvalue
```

Available: `+=  -=  *=  /=  %=  **=  &=  |=  ^=  <<=  >>=  >>>=`

**Advantage:** lvalue evaluated only once (important for complex expressions).

## Logic Operators

| Operator | Alt | Description |
|----------|-----|-------------|
| `not` | `!` | logical NOT |
| `and` | `&&` | logical AND |
| `or` | `\|\|` | logical OR |
| `xor` | `^^` | logical XOR |

**Short-circuit evaluation:** In `a and b`, `b` is only evaluated if `a` is `true`. In `a or b`, `b` is only evaluated if `a` is `false`.

## Comparison Operators

### Equality

| Operator | Description |
|----------|-------------|
| `==` | equal |
| `!=` | not equal |

### Relational

| Operator | Description |
|----------|-------------|
| `<` | less than |
| `>` | greater than |
| `<=` | less or equal |
| `>=` | greater or equal |

### Identity (handles only)

| Operator | Description |
|----------|-------------|
| `is` | same object |
| `!is` | different object |

```angelscript
if (a is null) { }      // check for null handle
if (a is b) { }         // check if same object instance
```

Identity compares **addresses**, not values.

## Increment/Decrement

```angelscript
a = i++;   // post-increment: a = i; i = i + 1;
b = --i;   // pre-decrement:  i = i - 1; b = i;
```

| Form | When increment happens |
|------|----------------------|
| `++i` / `--i` | before value is used |
| `i++` / `i--` | after value is used |

## Indexing Operator

```angelscript
arr[i] = 1;
value = arr[i];
```

Type of index expression depends on the object type.

## Conditional (Ternary) Expression

```angelscript
result = condition ? valueIfTrue : valueIfFalse;
```

**Rules:**
- Both branches must be same type (or implicitly convertible)
- If conversion needed, least-cost conversion wins
- Can be used as lvalue if both branches are lvalues of same type:

```angelscript
int a, b;
(expr ? a : b) = 42;  // assigns to a or b
```

## Member Access

```angelscript
object.property = 1;
object.method();
```

## Handle-of Operator

```angelscript
@handle = @object;   // make handle reference object
@handle = null;      // clear handle
```

See [02-objects-handles.md](02-objects-handles.md) for details.

## Parentheses

```angelscript
a = c * (a + b);         // override precedence
if ((a or b) and c) { }  // group logic
```

## Scope Resolution

```angelscript
int value;
void function() {
    int value;           // shadows global
    ::value = value;     // '::' accesses global scope
}

namespace Foo {
    void bar() {}
}
Foo::bar();              // access namespaced item
```

## Type Conversions

### Implicit (automatic)

```angelscript
int a = 1.0f;           // float to int
intf @a = @clss();      // derived to base handle
```

### Explicit value cast

```angelscript
float b = float(a) / 2;  // constructor-style cast
```

### Explicit reference cast

```angelscript
clss @b = cast<clss>(a);  // returns null if invalid
```

Reference cast returns handle to **same object** through different interface. Returns `null` if cast is invalid.

## Anonymous Objects

```angelscript
func(MyClass(1, 2, 3));                              // construct in-place
func(dictionary = {{'a', 1}, {'b', 2}});             // with init list
funcExpectsArray({1, 2, 3, 4});                      // implicit type from context
```
