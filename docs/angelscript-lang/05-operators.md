# Operators

## Operator Precedence (Highest to Lowest)

| Precedence | Operators | Associativity |
|------------|-----------|---------------|
| 1 | `()` `[]` `.` `::` | Left to right |
| 2 | `!` `not` `~` `++` `--` `+` `-` (unary) `@` | Right to left |
| 3 | `**` | Right to left |
| 4 | `*` `/` `%` | Left to right |
| 5 | `+` `-` | Left to right |
| 6 | `<<` `>>` `>>>` | Left to right |
| 7 | `&` | Left to right |
| 8 | `^` | Left to right |
| 9 | `\|` | Left to right |
| 10 | `<` `<=` `>` `>=` | Left to right |
| 11 | `==` `!=` `is` `!is` | Left to right |
| 12 | `and` `&&` | Left to right |
| 13 | `xor` `^^` | Left to right |
| 14 | `or` `\|\|` | Left to right |
| 15 | `?:` | Right to left |
| 16 | `=` `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `^=` `<<=` `>>=` `>>>=` | Right to left |

## Arithmetic Operators

| Operator | Description | Types |
|----------|-------------|-------|
| `+` | Addition | Numeric |
| `-` | Subtraction | Numeric |
| `*` | Multiplication | Numeric |
| `/` | Division | Numeric |
| `%` | Modulo (remainder) | Numeric |
| `**` | Exponentiation | Numeric |

**Type Rules:**
- Both operands converted to the same type
- Result is the converted type
- Integer division truncates toward zero

## Unary Operators

| Operator | Description | Types |
|----------|-------------|-------|
| `+` | Positive (no-op) | Numeric |
| `-` | Negation | Numeric (not uint) |
| `~` | Bitwise complement | Integer |
| `!` / `not` | Logical NOT | Boolean |

## Bitwise Operators

| Operator | Description |
|----------|-------------|
| `~` | Complement (NOT) |
| `&` | AND |
| `\|` | OR |
| `^` | XOR |
| `<<` | Left shift |
| `>>` | Right shift (sign-extended) |
| `>>>` | Arithmetic right shift (zero-filled) |

**Type Rules:**
- Operands converted to integers (preserving sign)
- Result type matches left operand

## Comparison Operators

### Equality

| Operator | Description |
|----------|-------------|
| `==` | Equal to |
| `!=` | Not equal to |

### Relational

| Operator | Description |
|----------|-------------|
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |

### Identity (Handles Only)

| Operator | Description |
|----------|-------------|
| `is` | Same object reference |
| `!is` | Different object references |

```angelscript
if (handle is null) {}     // Check for null
if (a is b) {}             // Same object?
```

**Note:** `==` compares values (calls `opEquals`), `is` compares addresses.

## Logical Operators

| Operator | Alternative | Description |
|----------|-------------|-------------|
| `!` | `not` | Logical NOT |
| `&&` | `and` | Logical AND |
| `\|\|` | `or` | Logical OR |
| `^^` | `xor` | Logical XOR |

**Short-circuit evaluation:**
- `a and b`: `b` only evaluated if `a` is true
- `a or b`: `b` only evaluated if `a` is false

## Increment/Decrement

| Operator | Description |
|----------|-------------|
| `++x` | Pre-increment (increment then use) |
| `x++` | Post-increment (use then increment) |
| `--x` | Pre-decrement (decrement then use) |
| `x--` | Post-decrement (use then decrement) |

## Assignment Operators

| Operator | Equivalent |
|----------|------------|
| `=` | Simple assignment |
| `+=` | `a = a + b` |
| `-=` | `a = a - b` |
| `*=` | `a = a * b` |
| `/=` | `a = a / b` |
| `%=` | `a = a % b` |
| `**=` | `a = a ** b` |
| `&=` | `a = a & b` |
| `\|=` | `a = a \| b` |
| `^=` | `a = a ^ b` |
| `<<=` | `a = a << b` |
| `>>=` | `a = a >> b` |
| `>>>=` | `a = a >>> b` |

**Advantage of compound assignment:** Left-hand side evaluated only once.

## Special Operators

### Conditional (Ternary)

```angelscript
result = condition ? valueIfTrue : valueIfFalse;
```

Can be lvalue if both branches are lvalues of same type:
```angelscript
(condition ? a : b) = 10;  // Assigns to a or b
```

### Handle-of

```angelscript
@handle = @object;   // Assign handle
@handle = null;      // Clear handle
```

### Scope Resolution

```angelscript
::globalVar            // Access global scope
Namespace::item        // Access namespace member
Base::method()         // Call base class method
```

### Member Access

```angelscript
object.property
object.method()
```

### Indexing

```angelscript
array[index]
dictionary[key]
```

Type of index depends on the object type (calls `opIndex`).

### Function Call

```angelscript
func(arg1, arg2)
object.method(arg1)
```
