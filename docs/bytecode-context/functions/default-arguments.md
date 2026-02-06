# Default Arguments

## Overview
Default arguments allow function parameters to be omitted at the call site. When an argument is not provided, the compiler automatically substitutes the default expression defined in the function declaration. This reduces the need for multiple overloads that differ only in providing preset values.

## Syntax

### Basic default arguments
```angelscript
void Function(int a, int b = 1, string c = "") {
    // b and c have default values
}

void main() {
    Function(0);           // b=1, c=""
    Function(0, 5);        // b=5, c=""
    Function(0, 5, "x");   // b=5, c="x"
}
```

### Default expression referencing globals
```angelscript
int myvar = 42;
void Function(int a, int b = myvar) {}

void main() {
    int myvar = 1;
    Function(1);    // Uses the GLOBAL myvar (42), not the local one
}
```

### Optional output parameter with void
```angelscript
void func(int &out output = void) {
    output = 42;
}

void main() {
    func();         // Output is discarded (void default)
    int val;
    func(val);      // val receives 42
}
```

## Semantics

### Ordering rule
Once a parameter has a default value, **all subsequent parameters must also have default values**. It is a compile error to have a non-defaulted parameter after a defaulted one.

```angelscript
// Valid:
void F(int a, int b = 0, int c = 0) {}

// Invalid -- compile error:
void G(int a, int b = 0, int c) {}
```

### Default expression scope
Default argument expressions are evaluated in the **global scope** at the call site. They may reference:
- Global variables
- Global functions
- Constant literals

They may **not** reference:
- Local variables at the call site (even if a local variable has the same name as a global, the global is used)
- Parameters of the function being called

### The void default for output parameters
The special expression `void` can be used as the default for an `&out` parameter. This makes the output parameter optional -- if the caller does not provide an argument, the output is silently discarded. The function body still writes to the parameter normally; the write simply has no effect when the void default is active.

### Interaction with overloading
Default arguments can create effective overloads at the call site. A function `void F(int a, int b = 0)` can be called as `F(1)` or `F(1, 2)`. If another overload `void F(int a)` exists, calling `F(1)` becomes ambiguous because both candidates match with the same number of arguments and identical conversion cost.

### Interaction with named arguments
Default arguments work naturally with named arguments:
```angelscript
void Setup(int width = 800, int height = 600, bool fullscreen = false) {}

Setup(fullscreen: true);  // width=800, height=600, fullscreen=true
```

## Examples

### Progressive detail function
```angelscript
void DrawRect(float x, float y, float w, float h,
              uint color = 0xFFFFFFFF,
              float rotation = 0.0f) {
    // Draw with optional color and rotation
}

DrawRect(10, 20, 100, 50);                    // White, no rotation
DrawRect(10, 20, 100, 50, 0xFF0000FF);        // Red, no rotation
DrawRect(10, 20, 100, 50, 0xFF0000FF, 45.0f); // Red, rotated
```

### Global function as default
```angelscript
int GetDefaultSize() { return 64; }

void CreateBuffer(int size = GetDefaultSize()) {
    // Uses the result of GetDefaultSize() when size not provided
}
```

## Compilation Notes
- **Caller-side insertion:** Default arguments are not stored in the callee's bytecode. Instead, the compiler inserts the default expression evaluation code at each call site where the argument is omitted. This means the callee always receives the full set of arguments.
- **Evaluation timing:** Default expressions are evaluated at call time, not at function declaration time. If a default references a global variable, the current value of that variable at the time of the call is used.
- **Stack layout:** From the callee's perspective, the stack frame is identical whether the caller provided explicit arguments or the compiler inserted defaults. All parameter slots are filled.
- **void default for &out:** When `void` is used as the default for an output parameter, the compiler allocates a temporary (throwaway) variable for the output. After the function returns, the copy-back step writes to this temporary, which is then discarded. No actual output propagation occurs.
- **Overload interaction:** The compiler considers each valid argument count as a separate candidate signature during overload resolution. A function with N parameters and M defaults generates effective signatures for argument counts (N-M) through N.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionParam` | Parameter with optional default | `ty: ParamType`, `name: Option<Ident>`, `default: Option<&Expr>`, `is_variadic: bool`, `span: Span` |

**Notes:**
- Default arguments are represented by the `FunctionParam.default` field. When `default` is `Some(&Expr)`, the parameter has a default value expression. When `None`, no default is provided.
- The default expression is an arbitrary `Expr` node, which can represent literals, global variable references, function calls, or any other expression. The parser does not restrict what expressions may appear as defaults; scope validation happens during compilation.
- The `void` default for `&out` parameters is represented as a `void` expression in the `default` field.

## Related Features
- [Function Declarations](./function-declarations.md)
- [Function Overloading](./function-overloading.md)
