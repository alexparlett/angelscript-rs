# Funcdefs

## Overview
Funcdefs (function definitions) define function signature types that can be used to create function pointers (function handles). They specify the return type and parameter types of a function without providing an implementation. Variables of a funcdef type can hold pointers to any function (global or class method via delegates) that matches the declared signature. Funcdefs are the foundation for callbacks, event systems, and dynamic dispatch patterns in AngelScript.

## Syntax

### Declaration

```angelscript
// Basic funcdef declaration
funcdef bool CALLBACK(int, int);

// Funcdef with named parameters (names are optional, for documentation)
funcdef void EventHandler(string eventName, int eventData);

// Funcdef returning void
funcdef void Action();

// Funcdef with reference parameters
funcdef void Processor(const string &in input, string &out output);
```

### Usage as Variable Type

```angelscript
// Declare a function handle variable
CALLBACK@ func = null;

// Assign a matching function
CALLBACK@ func = @myCompare;

// Assign using a lambda
CALLBACK@ func = function(int a, int b) { return a > b; };
```

### Delegate Creation

```angelscript
// Bind a class method to an object instance
CALLBACK@ func = CALLBACK(objectInstance.MethodName);
```

## Semantics

### Matching Rules

- A function matches a funcdef if its return type and all parameter types match exactly.
- Parameter names do not matter -- only the types must match.
- Reference qualifiers (`&in`, `&out`, `&inout`) and `const` qualifiers must match.
- The function can be a global function, an imported function, or a class method (via delegate).

### Function Handles

- Function handles are always accessed through the handle (`@`) syntax.
- A function handle can be `null`, which can be tested with the `is` operator: `if (func is null)`.
- Calling a null function handle causes a script exception.
- Function handles are reference-counted objects.

### Calling Through a Handle

- A function handle is called just like a regular function: `bool result = func(1, 2);`.
- The arguments are type-checked against the funcdef's parameter types.

### Delegates

- A delegate binds a class method to a specific object instance, creating a function handle that can be called as if it were a global function.
- Created via construct call syntax: `CALLBACK(objectInstance.Method)`.
- The delegate keeps a reference to the object, so the object stays alive as long as the delegate exists.
- When the delegate is called, the bound method is invoked on the bound object instance.

### Lambdas / Anonymous Functions

- Anonymous functions (lambdas) can be assigned to funcdef handles if their signatures match.
- Syntax: `function(int a, int b) { return a > b; }`.
- The lambda creates an anonymous function that is matched against the funcdef's signature.

## Examples

```angelscript
// Define a callback signature
funcdef bool CALLBACK(int, int);

// A function matching the signature
bool myCompare(int a, int b)
{
    return a > b;
}

void main()
{
    // Create a function handle pointing to myCompare
    CALLBACK@ func = @myCompare;

    // Check for null
    if (func is null)
    {
        print("Function handle is null\n");
        return;
    }

    // Call through the handle
    if (func(1, 2))
        print("true\n");
    else
        print("false\n");
}

// Delegate example
class Sorter
{
    bool Compare(int a, int b)
    {
        count++;
        return a > b;
    }
    int count = 0;
}

void SortExample()
{
    Sorter s;

    // Create a delegate binding Sorter::Compare to instance s
    CALLBACK@ func = CALLBACK(s.Compare);

    // Call the delegate -- invokes s.Compare(3, 4)
    func(3, 4);

    print("Comparisons: " + s.count + "\n");  // Prints 1
}

// Funcdef as function parameter (callback pattern)
funcdef void EventCallback(string);

void RegisterEvent(string eventName, EventCallback@ callback)
{
    // Store callback for later invocation
    callback(eventName + " triggered");
}

void OnEvent(string msg)
{
    print(msg + "\n");
}

void SetupEvents()
{
    RegisterEvent("click", @OnEvent);
}
```

## Compilation Notes

- **Module structure:** Each funcdef declaration creates a type entry in the engine's type system representing the function signature. The funcdef type has a unique type ID and stores the full signature (return type, parameter count, parameter types with qualifiers). Funcdef types are registered at the engine level, not per-module, because function handles can cross module boundaries.
- **Symbol resolution:** The funcdef name is registered in the global namespace (or the enclosing namespace) as a type name. When a variable is declared with a funcdef type (e.g. `CALLBACK@ func`), the compiler resolves the funcdef and uses its signature for type checking. When a function reference is assigned to a funcdef handle, the compiler compares the function's signature against the funcdef's signature.
- **Initialization:** Funcdef declarations are processed during the type registration phase (before function compilation). No runtime initialization is needed for the funcdef type itself. Function handle variables are initialized like any other handle -- to `null` or to a specific function/delegate.
- **Type system:** Funcdef types are reference types (always accessed via handles). They participate in handle assignment (`@`), `is`/`!is` null checks, and function call syntax. Two funcdefs with identical signatures are still distinct types -- you cannot assign a `CALLBACK_A@` to a `CALLBACK_B@` even if their parameter lists match. The delegate construct call (`FUNCDEF(obj.Method)`) creates a special delegate object that stores both the function pointer and the object reference.
- **Special cases:** Funcdefs can be declared as `shared` for cross-module sharing. Shared funcdefs ensure the same function signature type is used across modules, enabling function handles to be passed between modules. The `external shared funcdef` form references a shared funcdef without re-declaring it. Class-level funcdefs (declared inside a class) define method signatures that include an implicit `this` parameter.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Funcdef` | Top-level item variant for funcdef declarations | Wraps `FuncdefDecl` |
| `FuncdefDecl` | Funcdef (function signature type) declaration | `modifiers: DeclModifiers`, `return_type: ReturnType<'ast>`, `name: Ident<'ast>`, `template_params: &[Ident<'ast>]`, `params: &[FunctionParam<'ast>]`, `span: Span` |
| `ClassMember::Funcdef` | Funcdef nested inside a class | Wraps `FuncdefDecl` |

**Notes:**
- `FuncdefDecl` includes `template_params` for application-registered template funcdefs (e.g., `Callback<T>`).
- `FuncdefDecl` reuses `FunctionParam` for its parameter list, sharing the same structure as regular function parameters (including `ParamType` with `RefKind`).
- `DeclModifiers` on `FuncdefDecl` supports `shared` and `external` for cross-module funcdef sharing.
- Class-level funcdefs appear as `ClassMember::Funcdef` and define method signatures that include an implicit `this` parameter.

## Related Features

- [Global Functions](./global-functions.md) -- global functions can be referenced through funcdefs
- [Interfaces](./interfaces.md) -- interfaces provide an alternative polymorphism mechanism
- [Shared Entities](./shared-entities.md) -- funcdefs can be shared across modules
- [Namespaces](./namespaces.md) -- funcdefs can be declared inside namespaces
