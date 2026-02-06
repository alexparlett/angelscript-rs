# Function Declarations

## Overview
Functions in AngelScript are declared globally and consist of a signature (defining parameter types and return type) plus a body containing the implementation. Functions do not maintain their own state between calls, though they can modify global variables or memory passed by reference. There is no need for forward declarations -- functions are globally visible regardless of where they appear in the script.

## Syntax

### Basic declaration
```angelscript
returnType functionName(paramType1 param1, paramType2 param2, ...) {
    // function body
}
```

### Void function (no return value)
```angelscript
void DoWork(int input) {
    // no return value
}
```

### Parameter reference qualifiers
```angelscript
void Function(const int &in a, int &out b, Object &inout c) {
    b = a;
    c.DoSomething();
}
```

The `&` reference modifier requires a direction qualifier:

| Syntax       | Direction   | Description |
|-------------|-------------|-------------|
| `&in`       | Input only  | The function receives what is typically a copy of the original value. The original cannot be modified through this reference. |
| `&out`      | Output only | The reference points to an uninitialized value on entry. After the function returns, the assigned value is copied back to the caller's destination. |
| `&inout` / `&` | Both    | The reference points directly to the actual value. Only reference types (types that can have handles) are permitted as inout references, because the value must live on the heap to guarantee validity throughout the function's execution. |

### Const references
```angelscript
void Process(const Object &in obj) {
    // obj cannot be modified
}
```

Combining `const` with `&in` can improve performance for large objects by allowing the compiler to pass the actual value rather than making a copy when it can prove immutability.

## Semantics

- **Global visibility:** A function is visible from any point in the script, even before its textual declaration. No prototypes are needed.
- **No persistent state:** Functions do not retain memory across calls. Side effects must go through global variables, output parameters, or references to heap objects.
- **Return type:** Specified before the function name. Must be `void` if the function does not return a value.
- **Parameters:** Each parameter is defined by its type and name. Parameters without a reference qualifier are passed by value (a copy is made).

### Parameter passing by value vs. by reference

- **By value (no `&`):** A copy of the argument is made. Modifications to the parameter inside the function do not affect the caller's value. This is the default for primitive types and value types.
- **By `&in` reference:** Semantically input-only. The compiler typically passes a copy, but may optimize to pass by pointer for `const &in` parameters on large objects.
- **By `&out` reference:** The parameter starts uninitialized. On function return, the value is copied to the caller's target. Useful for returning multiple values.
- **By `&inout` reference:** The parameter aliases the caller's actual object. Restricted to reference types (heap-allocated objects that support handles), ensuring the reference cannot be invalidated during function execution.

### Named arguments
```angelscript
void func(int flagA = false, int flagB = false, int flagC = false) {}

func(flagC: true);              // Only set flagC
func(flagB: true, flagA: true); // Set B and A in any order
```

No positional arguments may follow named arguments in a call.

### Argument evaluation order
Arguments are evaluated in reverse order (last argument first, first argument last):
```angelscript
func(a(), b(), c());  // c() called first, then b(), then a()
```

### Output parameter with void
The special `void` expression can be used as an argument to ignore an output parameter:
```angelscript
void GetData(int &out a, int &out b) {}

int result;
GetData(result, void);  // Ignore second output
```

## Examples

### Simple function
```angelscript
int Add(int a, int b) {
    return a + b;
}
```

### Mixed parameter modes
```angelscript
void Transform(const string &in input, string &out output, int multiplier) {
    output = "";
    for (int i = 0; i < multiplier; i++)
        output += input;
}
```

### Method vs global function
```angelscript
class Foo {
    int value;
    int GetValue() { return value; }   // Method: has implicit 'this'
}

int GetGlobal() { return 42; }         // Global: no 'this'
```

## Compilation Notes
- **Calling convention:** Arguments are pushed onto the virtual stack. Primitive types occupy fixed-width stack slots. Object types may be passed as pointers/handles depending on reference mode.
- **Stack behavior:** Arguments are evaluated and pushed in reverse order (right to left). The callee pops its own parameters. The return value is placed in a designated register or stack position.
- **`&in` parameters:** The compiler generates code to copy the argument value into a temporary before passing a reference to that temporary. For `const &in` on large types, the compiler may optimize by passing the original address directly when it can prove no aliasing hazard.
- **`&out` parameters:** The compiler allocates a temporary for the parameter. After the function returns, a copy-back instruction moves the result to the caller's target variable.
- **`&inout` parameters:** The compiler passes the actual address of the caller's object. A runtime check or type constraint ensures the object is heap-allocated.
- **void return:** No return value slot is reserved on the stack; the function epilogue simply restores the frame pointer.
- **Named arguments:** Resolved entirely at compile time. The compiler reorders the argument evaluation to match the parameter positions in the function signature.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionDecl` | Function declaration | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `is_destructor: bool`, `span: Span` |
| `FunctionParam` | Parameter | `ty: ParamType`, `name: Option<Ident>`, `default: Option<&Expr>`, `is_variadic: bool`, `span: Span` |
| `ParamType` | Parameter type with reference qualifier | `ty: TypeExpr`, `ref_kind: RefKind`, `span: Span` |
| `ReturnType` | Return type with optional reference | `ty: TypeExpr`, `is_ref: bool`, `span: Span` |
| `RefKind` | Reference direction qualifier | Variants: `None`, `Ref`, `RefIn`, `RefOut`, `RefInOut` |
| `DeclModifiers` | Top-level modifiers | `shared: bool`, `external: bool`, `abstract_: bool`, `final_: bool` |
| `FuncAttr` | Function-specific attributes | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |

**Notes:**
- The `FunctionParam.is_variadic` field tracks variadic parameters (`...`), which is not explicitly covered in this doc's syntax section but is supported by the parser.
- `FunctionDecl.is_const` maps to the `const` keyword after the parameter list (for class methods).
- `FunctionDecl.template_params` is for application-registered template functions and is not part of script-level AngelScript syntax.
- The `FuncAttr.delete` field represents deleted functions (`delete` decorator); this is not mentioned in this doc. See [Methods](./methods.md) for `override` and `final` attributes.

## Related Features
- [Function Overloading](./function-overloading.md)
- [Default Arguments](./default-arguments.md)
- [Return References](./return-references.md)
- [Anonymous Functions](./anonymous-functions.md)
- [Function References](./function-references.md)
- [Variable Declarations](../statements/variable-declarations.md)
