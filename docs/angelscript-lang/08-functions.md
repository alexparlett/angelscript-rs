# Functions

## Declaration

```angelscript
int AFunction(int a, int b) {
    return a + b;
}
```

- Return type before function name
- Use `void` for functions that don't return a value
- Parameters listed between parentheses
- **No forward declarations needed** - functions are globally visible regardless of declaration order

## Parameter References

AngelScript requires explicit declaration of reference intent:

| Syntax | Purpose | Behavior |
|--------|---------|----------|
| `&in` | Input only | Usually receives a copy, original cannot be modified |
| `&out` | Output only | Receives uninitialized value, caller gets result after return |
| `&inout` or `&` | Both | Refers to actual value (reference types only) |

```angelscript
void Function(const int &in a, int &out b, Object &c) {
    b = a;           // Output parameter
    c.DoSomething(); // Modifies actual object
}
```

### Reference Constraints

- `&inout` (or plain `&`) only works with **reference types** (types that can have handles)
- This ensures the reference stays valid during function execution
- Value types on the heap can be passed, but stack-allocated values cannot

### const References

```angelscript
void Process(const Object &in obj) {
    // Cannot modify obj
}
```

Combining `const` with `&in` can improve performance for large objects.

## Default Arguments

```angelscript
void Function(int a, int b = 1, string c = "") {
    // b and c have defaults
}

Function(0);         // b=1, c=""
Function(0, 5);      // b=5, c=""
Function(0, 5, "x"); // b=5, c="x"
```

**Rules:**
- Once a parameter has a default, **all subsequent parameters must have defaults**
- Default expressions can reference **global** variables and functions only
- Special `void` expression for optional output parameters:

```angelscript
void func(int &out output = void) { output = 42; }
```

## Function Overloading

Multiple functions with the same name but different parameters:

```angelscript
void Function(int a, float b, string c) {}
void Function(string a, int b, float c) {}
void Function(float a, string b, int c) {}

Function(1, 2.5f, "a");   // First overload
Function("a", 1, 2.5f);   // Second overload
```

### Overload Resolution

The compiler matches argument types to parameters and selects the best match. Conversion cost determines priority (best to worst):

1. No conversion needed
2. Conversion to const
3. Enum to integer (same size)
4. Enum to integer (different size)
5. Primitive size increase
6. Primitive size decrease
7. Signed to unsigned
8. Unsigned to signed
9. Integer to float
10. Float to integer
11. Reference cast
12. Object to primitive conversion
13. Conversion to object
14. Variable argument type

**Cannot overload by return type only** - return type isn't part of selection criteria.

## Named Arguments

```angelscript
void func(int flagA = false, int flagB = false, int flagC = false) {}

func(flagC: true);              // Only set flagC
func(flagB: true, flagA: true); // Set B and A in any order
```

**Rule:** No positional arguments may follow named arguments.

## Argument Evaluation Order

Arguments are evaluated in **reverse order** (last to first).

```angelscript
func(a(), b(), c());  // c() called first, then b(), then a()
```

## Output Parameter with void

Use `void` as argument to ignore an output parameter:

```angelscript
void GetData(int &out a, int &out b) {}

int result;
GetData(result, void);  // Ignore second output
```

## Anonymous Functions (Lambdas)

```angelscript
funcdef void Callback(int);

void main() {
    Callback@ cb = function(int x) { print(x); };
    cb(42);
}
```

See also: Funcdefs for function pointer types.

## Method vs Function

| Feature | Global Function | Class Method |
|---------|-----------------|--------------|
| Declaration | Global scope | Inside class |
| `this` | Not available | Implicit reference to object |
| Virtual | N/A | Always virtual |
| Overloading | By parameter types | By parameter types |
