# Function Overloading

## Overview
Function overloading allows multiple functions with the same name to coexist as long as they have different parameter lists. The compiler selects the correct overload at each call site by matching argument types to parameter types and choosing the candidate with the lowest total conversion cost.

## Syntax
```angelscript
void Function(int a, float b, string c) {}
void Function(string a, int b, float c) {}
void Function(float a, string b, int c) {}

void main() {
    Function(1, 2.5f, "a");   // Calls first overload
    Function("a", 1, 2.5f);   // Calls second overload
    Function(2.5f, "a", 1);   // Calls third overload
}
```

## Semantics

### Overload resolution algorithm
The compiler resolves overloads by processing arguments left to right:

1. For each candidate function, the compiler examines every argument/parameter pair.
2. For each pair, it determines the type of implicit conversion required (if any).
3. Candidates that require an impossible conversion are eliminated.
4. Among remaining candidates, the one with the best (lowest-cost) conversions is selected.
5. If no single best candidate exists, the compiler reports an ambiguity error.

### Conversion cost ordering (best to worst)
The following list defines the preference order. A match earlier in the list is strictly preferred over one later:

1. **No conversion needed** -- exact type match
2. **Conversion to const** -- adding const qualifier
3. **Enum to integer of same size** -- e.g., `enum : int` to `int`
4. **Enum to integer of different size** -- e.g., `enum : int8` to `int`
5. **Primitive size increase** -- e.g., `int8` to `int`, `float` to `double`
6. **Primitive size decrease** -- e.g., `int` to `int8`, `double` to `float`
7. **Signed to unsigned integer** -- e.g., `int` to `uint`
8. **Unsigned to signed integer** -- e.g., `uint` to `int`
9. **Integer to float** -- e.g., `int` to `float`
10. **Float to integer** -- e.g., `float` to `int`
11. **Reference cast** -- e.g., derived handle to base handle
12. **Object to primitive conversion** -- via opImplConv or similar
13. **Conversion to object** -- via constructor or opConv
14. **Variable argument type** -- matching a variadic `?` parameter (worst match)

### Return type is not a distinguishing factor
Overloads cannot differ solely by return type. The return type is the result of calling the function, not a criterion used to select which function to call. Declaring two functions with the same name and parameter types but different return types is a compile error.

### Const overloading for methods
Class methods can be overloaded on `const` qualification of `this`. A `const` method is preferred when the object is accessed through a const handle or const reference; the non-const version is preferred otherwise.

```angelscript
class Container {
    int &opIndex(int idx)       { /* mutable access */ }
    const int &opIndex(int idx) const { /* read-only access */ }
}
```

### Ambiguity errors
When two or more candidates have equal conversion cost for all arguments, the compiler cannot choose and reports an error. This commonly happens when:
- Two overloads differ only in closely-ranked conversions (e.g., one needs signed-to-unsigned, another needs size increase).
- An argument literal could equally match multiple parameter types.

Resolution strategies:
- Use explicit casts on arguments to force an exact match.
- Add or remove overloads to eliminate the ambiguity.

## Examples

### Numeric type overloading
```angelscript
void Print(int value)    { /* integer version */ }
void Print(float value)  { /* float version */ }
void Print(string value) { /* string version */ }

void main() {
    Print(42);       // Exact match: int
    Print(3.14f);    // Exact match: float
    Print("hello");  // Exact match: string
    Print(3.14);     // double -> float (size decrease) preferred over double -> int
}
```

### Interaction with default arguments
```angelscript
void Process(int a, int b = 0) {}
void Process(int a) {}

// Ambiguous! Process(1) matches both overloads with equal cost.
// The compiler will raise an error.
```

## Compilation Notes
- **Compile-time resolution:** Overload resolution is performed entirely at compile time. The emitted bytecode contains a direct call to the specific resolved function; there is no runtime dispatch table for overloads.
- **Conversion cost calculation:** The compiler assigns a numeric cost to each argument conversion. The total cost for a candidate is the sum (or ordered comparison) of per-argument costs. The candidate with the strictly lowest cost wins.
- **No runtime overhead:** Because the correct overload is selected at compile time, overloaded functions have identical call performance to non-overloaded functions.
- **Interaction with default arguments:** When default arguments allow a function to be called with fewer arguments, the compiler treats the defaulted version as a separate candidate during overload resolution. This can create ambiguities if another overload matches with the same argument count.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionDecl` | Each overloaded variant is a separate `FunctionDecl` | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `is_destructor: bool`, `span: Span` |
| `FunctionParam` | Parameter (distinguishes overloads) | `ty: ParamType`, `name: Option<Ident>`, `default: Option<&Expr>`, `is_variadic: bool`, `span: Span` |

**Notes:**
- Function overloading is a **semantic** concept, not an AST-level construct. The parser produces independent `FunctionDecl` nodes that happen to share the same `name`. Overload resolution is performed by the compiler during type checking, not during parsing.
- Const overloading for methods is distinguished by `FunctionDecl.is_const`.

## Related Features
- [Function Declarations](./function-declarations.md)
- [Default Arguments](./default-arguments.md)
- [Anonymous Functions](./anonymous-functions.md)
