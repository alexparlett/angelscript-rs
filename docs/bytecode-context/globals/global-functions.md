# Global Functions

## Overview
Global functions are module-level routines that operate on input and produce results. They are the primary mechanism for implementing logic in AngelScript scripts. Global functions do not keep any persistent state themselves (unlike global variables), though they may read and modify global variables or memory passed by reference.

## Syntax

```angelscript
// Basic function declaration
int AFunction(int a, int b)
{
    return a + b;
}

// Void function
void DoSomething()
{
    // ...
}

// Function with reference parameters
void Swap(int &inout a, int &inout b)
{
    int temp = a;
    a = b;
    b = temp;
}

// Function with default arguments
void Configure(int mode, int flags = 0, string name = "default")
{
    // ...
}

// Overloaded functions
void Process(int value) { /* ... */ }
void Process(string value) { /* ... */ }
void Process(float a, float b) { /* ... */ }
```

## Semantics

### Declaration Rules

- The return type is specified before the function name. Use `void` if the function returns nothing.
- Parameters are listed between parentheses, each defined by its type and name.
- There is **no need for forward declarations** or function prototypes. A function is globally visible regardless of where it is declared in the source. A function at the bottom of the file can be called by a function at the top.
- The function is always declared together with its body (no separate declaration and definition).

### Visibility

- All global functions are visible to all other code within the same module.
- Functions in different modules are not directly accessible to each other (use `import` for cross-module function calls).
- Functions registered by the host application (native functions) are visible to all modules.
- Functions share the global namespace with variables, classes, enums, and other declarations. Name conflicts with identical signatures produce a compiler error.

### Overloading

- Multiple functions with the same name but different parameter lists are allowed.
- The compiler selects the best match based on argument types and conversion costs.
- Cannot overload by return type alone -- the return type is not considered during overload resolution.
- Overload resolution priority (best to worst): exact match, const conversion, enum-to-int same size, enum-to-int different size, primitive widening, primitive narrowing, sign conversion, int-to-float, float-to-int, reference cast, object-to-primitive, conversion-to-object, variable argument type.

### Parameter References

- `&in` -- input reference; usually receives a copy; the original cannot be modified.
- `&out` -- output reference; receives an uninitialized value; the caller gets the result after return.
- `&inout` (or plain `&`) -- bidirectional reference; refers to the actual value. Only works with reference types.
- `const` can be combined with references for read-only access.

### Default Arguments

- Parameters with defaults must come after all parameters without defaults.
- Default expressions can reference global variables and functions only.
- The special `void` expression can be used as a default for output parameters: `void func(int &out output = void)`.

### Named Arguments

```angelscript
void func(int flagA = false, int flagB = false, int flagC = false) {}
func(flagC: true);              // Only set flagC
func(flagB: true, flagA: true); // Set B and A in any order
```

No positional arguments may follow named arguments.

### Argument Evaluation Order

Arguments are evaluated in **reverse order** (last parameter to first).

## Examples

```angelscript
// A utility function accessible from anywhere in the module
float Clamp(float value, float min, float max)
{
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

// Modifying global state
int score = 0;

void AddScore(int points)
{
    score += points;
}

// Using output parameters
bool TryParse(string input, int &out result)
{
    // Parse logic
    result = parseInt(input);
    return true;
}

void main()
{
    int value;
    if (TryParse("42", value))
    {
        print("Parsed: " + value + "\n");
    }
}
```

## Compilation Notes

- **Module structure:** Each global function becomes an entry in the module's function table. The entry stores the function's name, return type, parameter types, and the bytecode for the function body. Functions are identified by their index in the module's function list, which is used for CALL instructions.
- **Symbol resolution:** Function names are resolved in the module's symbol table, considering namespace scope. When a call is encountered, the compiler searches for matching function signatures by name and parameter types. Functions from registered (native) application interfaces are checked if no module-level match is found. Namespace-qualified calls (e.g. `A::function()`) resolve the namespace first, then the function within it.
- **Initialization:** Global functions themselves require no initialization. However, they are compiled as part of the module build and their bytecode is available immediately. Functions referenced in global variable initializers must be compiled before the initialization pass runs.
- **Type system:** Function signatures participate in the type system through funcdefs. A global function can be assigned to a funcdef handle if the signatures match. The compiler verifies return types and parameter types (including reference qualifiers and const-ness) when checking signature compatibility.
- **Special cases:** Imported functions from other modules occupy separate slots in the module's import table rather than the main function table. They are resolved at bind-time, not compile-time. If an imported function is called before being bound, the script aborts with an exception.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Function` | Top-level item variant for functions | Wraps `FunctionDecl` |
| `FunctionDecl` | Function declaration (used for global functions, class methods, constructors, destructors) | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType<'ast>>`, `name: Ident<'ast>`, `template_params: &[Ident<'ast>]`, `params: &[FunctionParam<'ast>]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block<'ast>>`, `is_destructor: bool`, `span: Span` |
| `FunctionParam` | Function parameter | `ty: ParamType<'ast>`, `name: Option<Ident<'ast>>`, `default: Option<&Expr<'ast>>`, `is_variadic: bool`, `span: Span` |
| `DeclModifiers` | Top-level declaration modifiers | `shared: bool`, `external: bool`, `abstract_: bool`, `final_: bool` |
| `FuncAttr` | Function-specific attributes | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |
| `ReturnType` | Return type with optional reference | `ty: TypeExpr<'ast>`, `is_ref: bool`, `span: Span` |
| `ParamType` | Parameter type with reference qualifier | `ty: TypeExpr<'ast>`, `ref_kind: RefKind`, `span: Span` |
| `RefKind` | Reference qualifier for parameters | Enum: `None`, `Ref`, `RefIn`, `RefOut`, `RefInOut` |

**Notes:**
- The same `FunctionDecl` struct is used for both global functions and class methods; context (whether it appears as `Item::Function` or `ClassMember::Method`) determines whether it is a global function.
- `FunctionParam.name` is `Option` because interface method parameters may omit names.
- Default arguments are represented as `FunctionParam.default: Option<&Expr>`.
- The `property` flag in `FuncAttr` marks explicit property accessor functions (e.g., `int get_prop() property`).

## Related Features

- [Global Variables](./global-variables.md) -- functions and variables share the global namespace
- [Funcdefs](./funcdefs.md) -- function signature types for function handles
- [Namespaces](./namespaces.md) -- functions can be declared in namespaces
- [Imports](./imports.md) -- importing functions from other modules
- [Shared Entities](./shared-entities.md) -- functions can be declared as shared
