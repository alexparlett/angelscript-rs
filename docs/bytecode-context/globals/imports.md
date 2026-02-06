# Imports

## Overview
The import directive allows a script module to declare that it needs a function from another module, without that function being available at compile time. This enables dynamic loading of script modules that can still interact with each other. The imported function is compiled against its declared signature, and the actual binding to a concrete function in another module happens later at runtime through the host application.

## Syntax

```angelscript
// Import a function from another module
import void MyFunction(int a, int b) from "Another module";

// Import with return type
import int Calculate(float x, float y) from "MathModule";

// Import with reference parameters
import void ProcessData(const string &in input, string &out output) from "DataModule";
```

The general form is:

```angelscript
import <return_type> <function_name>(<parameter_list>) from "<module_name>";
```

## Semantics

### Declaration Rules

- The `import` keyword is followed by a complete function signature (return type, name, parameters).
- The `from` clause specifies the name of the source module as a string literal.
- Only functions can be imported -- not variables, classes, interfaces, enums, or other entities.
- The imported function signature is used for type checking at compile time.

### Binding Process

1. The script is compiled with the import declaration. The compiler accepts calls to the imported function based on its declared signature, even though no implementation exists yet.
2. After compilation, the host application is responsible for binding the imported function to an actual function in another module using the engine's API (e.g. `BindImportedFunction`).
3. The binding can be done at any time after compilation, and can even be changed or unbound later.
4. If a script calls an imported function that has **not yet been bound**, the script is aborted with a script exception.

### Module Name

- The module name in the `from` clause is a string that identifies the source module.
- The interpretation of this string depends entirely on the host application. It could be a file path, a module ID, or any other identifier the application recognizes.
- The engine does not automatically load or compile the named module -- the host application manages module lifecycle.

### Scope and Visibility

- Imported functions are visible within the importing module, just like locally declared functions.
- They participate in overload resolution alongside local functions and registered application functions.
- Imported functions are in the same namespace as other global entities.

## Examples

```angelscript
// Import functions from a math library module
import float Sin(float angle) from "MathLib";
import float Cos(float angle) from "MathLib";
import float Sqrt(float value) from "MathLib";

// Import from a utility module
import void Log(const string &in message) from "Utilities";

// Use imported functions normally
void main()
{
    float angle = 3.14159f / 4.0f;
    float s = Sin(angle);
    float c = Cos(angle);

    Log("sin=" + s + ", cos=" + c);

    float hyp = Sqrt(s * s + c * c);
    Log("hypotenuse=" + hyp);
}

// Imported functions in expressions
import int Max(int a, int b) from "MathLib";

void ProcessValues(int x, int y)
{
    int biggest = Max(x, Max(y, 0));
}
```

## Compilation Notes

- **Module structure:** Each import declaration creates an entry in the module's **import table**, which is separate from the regular function table. The import entry stores the function signature (name, return type, parameter types) and the source module name string. At compile time, calls to imported functions generate bytecode that references the import table index rather than a direct function index.
- **Symbol resolution:** Imported function names are resolved in the global namespace alongside local functions and registered application functions. During overload resolution, imported functions are candidates just like local functions. If both a local function and an imported function match a call, the local function takes precedence. The compiler validates that calls to imported functions match the declared signature.
- **Initialization:** Import declarations are processed during the module's compilation phase. No initialization bytecode is generated for imports themselves. The binding of imported functions to actual implementations is a post-compilation step managed by the host application. Until binding occurs, the import table entry contains a null function reference.
- **Type system:** Imported functions are type-checked against their declared signature at compile time. The compiler has no access to the actual implementation, so it relies entirely on the declared types. If the bound function's actual signature differs from the declared import signature, behavior is undefined (the host application is responsible for ensuring compatibility).
- **Special cases:** The CALL bytecode instruction for imported functions uses a different addressing mode than calls to local functions, referencing the import table instead of the function table. If a module is rebuilt or discarded, all bindings for its imports are invalidated. The host application must re-bind imports after recompilation. Imported functions cannot be passed as function handles (funcdef references) because they are not regular function entries -- they exist only as import stubs until bound.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Import` | Top-level item variant for import declarations | Wraps `ImportDecl` |
| `ImportDecl` | Import function declaration | `return_type: ReturnType<'ast>`, `name: Ident<'ast>`, `params: &[FunctionParam<'ast>]`, `attrs: FuncAttr`, `module: String`, `span: Span` |

**Notes:**
- `ImportDecl.module` is a `String` (not `&str`), representing the module name from the `from "module"` clause. This is the only AST declaration field that uses an owned `String` rather than an arena-allocated reference, which is why `ImportDecl` derives `Clone` but not `Copy` (unlike most other declaration types).
- `ImportDecl` does not have `DeclModifiers` -- imports are inherently cross-module references and do not support `shared`/`external` qualifiers.
- `ImportDecl` includes `attrs: FuncAttr`, which could carry the `property` flag for imported property accessor functions.
- `ImportDecl` reuses `FunctionParam` for its parameter list, sharing the same structure as regular function declarations.

## Related Features

- [Global Functions](./global-functions.md) -- imported functions behave like global functions once bound
- [Shared Entities](./shared-entities.md) -- an alternative mechanism for cross-module entity sharing
- [Namespaces](./namespaces.md) -- imported functions respect namespace scoping
