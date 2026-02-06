# Anonymous Functions

## Overview
Anonymous functions (also called lambdas) are functions declared inline at the point of use, typically to be assigned to a function handle (funcdef). They provide a concise way to pass behavior as an argument without declaring a separate named function. In AngelScript, anonymous functions take on the signature of the funcdef they are assigned to, so parameter types and return type can often be inferred.

## Syntax

### Inferred parameter types
```angelscript
funcdef bool CMP(int first, int second);

void main() {
    int valueA = 1, valueB = 2;

    bool result1 = func(valueA, valueB, function(a, b) { return a == b; });
    bool result2 = func(valueA, valueB, function(a, b) { return a != b; });
}

bool func(int a, int b, CMP @f) {
    return f(a, b);
}
```

When the target funcdef is unambiguous, parameter types and return type are inferred from the funcdef signature. The parameter names are chosen by the programmer but the types come from the funcdef.

### Explicit parameter types
```angelscript
funcdef void A(int);
funcdef void B(float);

void func(A @) {}
void func(B @) {}

void main() {
    // Must specify types to resolve ambiguity between A and B
    func(function(int a) {});
}
```

When multiple funcdefs could match (e.g., overloaded functions taking different funcdef handle types), the parameter types must be stated explicitly to disambiguate.

### Assigning to a handle variable
```angelscript
funcdef void Callback(int);

void main() {
    Callback @cb = function(x) { print(x); };
    cb(42);
}
```

## Semantics

### Signature inference
An anonymous function derives its full signature (parameter types and return type) from the funcdef it is being assigned to or passed as. The programmer only needs to provide parameter names. If the target is ambiguous, explicit types on the parameters resolve the ambiguity.

### No closure captures
Anonymous functions in AngelScript **cannot access variables from the enclosing scope**. They are not closures. The body of an anonymous function can only use:
- Its own parameters
- Global variables and functions
- Literals and constants

Local variables from the scope where the lambda is declared are **not** captured. This is a fundamental limitation of the current AngelScript implementation.

### Funcdef compatibility
An anonymous function is compatible with a funcdef if:
- The number of parameters matches
- Each parameter type matches (or can be inferred)
- The return type matches (or can be inferred)

The anonymous function is effectively compiled as a hidden named function and a handle to it is created.

### Multi-statement bodies
The body can contain multiple statements:
```angelscript
funcdef int Transform(int);

void main() {
    Transform @t = function(x) {
        int result = x * 2;
        result += 1;
        return result;
    };
}
```

## Examples

### Sorting comparator
```angelscript
funcdef bool LessThan(int, int);

void Sort(int[] &arr, LessThan @cmp) {
    // Sorting implementation using cmp
}

void main() {
    int[] data = {5, 3, 1, 4, 2};
    Sort(data, function(a, b) { return a < b; });   // Ascending
    Sort(data, function(a, b) { return a > b; });   // Descending
}
```

### Disambiguation with explicit types
```angelscript
funcdef void IntHandler(int);
funcdef void FloatHandler(float);

void Register(IntHandler @h) {}
void Register(FloatHandler @h) {}

void main() {
    Register(function(int val) {});     // Selects IntHandler overload
    Register(function(float val) {});   // Selects FloatHandler overload
}
```

### Callback pattern
```angelscript
funcdef void OnComplete(bool success);

void DoAsyncWork(OnComplete @callback) {
    bool ok = true;
    // ... do work ...
    callback(ok);
}

void main() {
    DoAsyncWork(function(success) {
        if (success)
            print("Done!");
    });
}
```

## Compilation Notes
- **Internal named function:** The compiler transforms each anonymous function into a regular (hidden) global function with a compiler-generated unique name. The lambda syntax is purely syntactic sugar; at the bytecode level, it is an ordinary function.
- **Handle creation:** After compiling the anonymous function body, the compiler generates bytecode to create a function handle (delegate) pointing to the generated function. This handle is then passed or assigned like any other funcdef handle.
- **No capture overhead:** Because anonymous functions do not capture local variables, there is no closure object, no heap allocation for captured state, and no reference counting overhead beyond the function handle itself. The generated function is stateless.
- **Type inference at compile time:** The compiler determines the anonymous function's full signature by examining the target funcdef before compiling the function body. If inference fails (ambiguity), a compile error is raised. No runtime type matching occurs.
- **Signature matching with overloads:** When an anonymous function is passed to an overloaded function, the compiler uses the explicit parameter types (if provided) to narrow down the matching funcdef. The overload resolution and lambda signature resolution are interleaved.
- **Stack frame:** The anonymous function's stack frame is identical to that of any other function with the same signature. Parameters are pushed by the caller and popped by the callee in the standard calling convention.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `LambdaExpr` | Anonymous function expression | `params: &[LambdaParam]`, `return_type: Option<ReturnType>`, `body: &Block`, `span: Span` |
| `LambdaParam` | Lambda parameter (types may be inferred) | `ty: Option<ParamType>`, `name: Option<Ident>`, `span: Span` |

**Notes:**
- Anonymous functions are parsed as `LambdaExpr` nodes within the expression AST (`expr.rs`), not as `FunctionDecl` nodes in `decl.rs`. They are expressions, not declarations.
- `LambdaParam.ty` is `Option<ParamType>` because parameter types can be omitted when they are inferred from the target funcdef. When types are specified explicitly (for disambiguation), `ty` is `Some(ParamType)`.
- `LambdaExpr.return_type` is `Option<ReturnType>` because the return type is typically inferred from the target funcdef.
- The `function` keyword in the source syntax produces a `LambdaExpr` in the AST. The compiler later transforms it into a hidden named function.

## Related Features
- [Function Declarations](./function-declarations.md)
- [Function References](./function-references.md)
- [Function Overloading](./function-overloading.md)
