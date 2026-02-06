# Global Variables

## Overview
Global variables are module-level variable declarations that are shared between all contexts accessing the script module. They persist across function calls and are accessible from any function within the module. Global variables provide module-wide state that lives for the entire lifetime of the module.

## Syntax

```angelscript
// Simple declaration with initialization
int MyValue = 0;

// Constant global
const uint Flag1 = 0x01;

// Multiple declarations
float gravity = 9.81f;
string moduleName = "physics";
const float PI = 3.14159f;

// Complex type globals
MyClass@ globalObj = MyClass();
array<int> globalArray = {1, 2, 3};
```

## Semantics

- Global variables are accessible from all functions within the module.
- Values are initialized at compile time and changes are maintained between calls.
- If a global variable holds a memory resource (e.g. a string), its memory is released when the module is discarded or the script engine is reset.
- All global declarations share the same namespace, so their names must not conflict with other global entities (functions, classes, enums, etc.) or with built-in types and functions registered by the host application.
- All declarations are visible to all code, regardless of declaration order. A function can reference a global variable declared after itself in source order.

### Constant Globals

- The `const` qualifier makes the variable read-only after initialization.
- Constant globals must be initialized at declaration.
- They can use compile-time constant expressions or function calls in their initializer.

### Initialization Order

- Variables of **primitive types** are initialized before variables of **non-primitive types**. This allows class constructors to access other global variables that are already initialized with their correct values.
- Among non-primitive types, there is **no guaranteed initialization order**. If a non-primitive global variable's constructor accesses another non-primitive global variable, the accessed variable may not yet be initialized, potentially leading to null-pointer exceptions.
- Be careful with calling functions from within initialization expressions of global variables. While the compiler tries to initialize globals in the order they are needed, it does not always succeed. If a function accesses a global variable that has not yet been initialized, the result is unpredictable behavior or a null-pointer exception.

### Cross-Module Access

- Global variables are scoped to their module and cannot be directly accessed from other modules.
- To share data between modules, use `shared` types or the `import` mechanism for functions.
- Global variables cannot currently be declared as `shared` (this may be supported in future versions).

## Examples

```angelscript
// Basic global variable usage
int counter = 0;

void IncrementCounter()
{
    counter++;
}

int GetCounter()
{
    return counter;
}

// Constant flags
const uint FLAG_ACTIVE   = 0x01;
const uint FLAG_VISIBLE  = 0x02;
const uint FLAG_ENABLED  = 0x04;

// Primitive initialized before non-primitive
int maxItems = 100;                    // Initialized first (primitive)
array<int> items(maxItems);            // Initialized second (non-primitive), can use maxItems

// Dangerous: two non-primitive globals referencing each other
MyClass@ objA = MyClass(objB);         // objB may not be initialized yet!
MyClass@ objB = MyClass(objA);         // objA may not be initialized yet!
```

## Compilation Notes

- **Module structure:** Each global variable becomes an entry in the module's global variable table. The entry stores the variable's type, name, and initialization bytecode. Global variables are stored in a contiguous memory region within the module, indexed by their position.
- **Symbol resolution:** Global variable names are resolved within the module's symbol table. They share the namespace with all other global entities (functions, classes, enums, etc.). Name conflicts produce a compiler error. Namespace-qualified variables (e.g. `MyNamespace::myVar`) are resolved by first looking up the namespace, then the variable within it.
- **Initialization:** The compiler generates initialization bytecode that runs when the module is built. Primitive-type globals are initialized first in a separate pass, then non-primitive globals. Within each category, the compiler attempts dependency-based ordering but does not guarantee a topological sort across all non-primitive globals. Each global's initializer runs as a small function body that assigns the result to the global's storage slot.
- **Type system:** Global variables can be of any type: primitives, enums, objects (value or reference types), handles, and arrays. The `const` qualifier prevents mutation after initialization and is enforced at compile time.
- **Special cases:** Global variables that hold reference-counted objects keep a reference alive for the module's lifetime. When the module is discarded, all global variables are released in reverse order of their storage index, decrementing reference counts as needed. This is important for avoiding circular references between modules.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::GlobalVar` | Top-level item variant for global variables | Wraps `GlobalVarDecl` |
| `GlobalVarDecl` | Global variable declaration | `visibility: Visibility`, `ty: TypeExpr<'ast>`, `name: Ident<'ast>`, `init: Option<&Expr<'ast>>`, `span: Span` |
| `Visibility` | Access modifier for the declaration | Enum: `Public`, `Private`, `Protected` |
| `Ident` | Identifier with source location | `name: &str`, `span: Span` |

**Notes:**
- `GlobalVarDecl` has a `visibility` field (`Visibility` enum), which supports `Private` for backing variables of virtual properties (e.g., `private int _health`). The default is `Public`.
- The `const` qualifier is represented through `TypeExpr.is_const` on the `ty` field, not as a separate field on `GlobalVarDecl`.
- `GlobalVarDecl` does not have a `DeclModifiers` field, consistent with the documentation noting that global variables cannot currently be declared as `shared` or `external`.

## Related Features

- [Namespaces](./namespaces.md) -- global variables can be declared within namespaces
- [Global Functions](./global-functions.md) -- functions share the same global namespace
- [Virtual Properties](./virtual-properties.md) -- alternative to raw global variables with accessor logic
- [Shared Entities](./shared-entities.md) -- global variables cannot currently be shared
