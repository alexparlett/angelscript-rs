# Namespaces

## Overview
Namespaces organize large projects into logical units, preventing name collisions between entities in different parts of the project. Entities declared within a namespace do not conflict with identically named entities in other namespaces. Namespaces can be nested to create hierarchical organization. All types of global entities (functions, variables, classes, interfaces, enums, funcdefs, typedefs, virtual properties) can be declared within namespaces.

## Syntax

### Basic Namespace Declaration

```angelscript
namespace A
{
    void function() { variable++; }
    int variable;
}

namespace B
{
    void function() { A::function(); }
}
```

### Nested Namespaces

```angelscript
namespace Parent
{
    int var;

    namespace Child
    {
        int var;

        void func()
        {
            // ...
        }
    }
}
```

### Scope Resolution Operator (`::`)

```angelscript
// Access entity in a specific namespace
A::function();

// Access nested namespace entity
Parent::Child::var;

// Access global scope explicitly
::globalVar;
```

## Semantics

### Visibility Rules

- Entities within the **same namespace** see each other normally, without needing qualification.
- Entities in **different namespaces** do not see each other directly. The scope resolution operator (`::`) must be used to access them.
- Entities in a **parent namespace** are visible to child namespaces, unless shadowed by an identically named entity in the child.
- When a child namespace shadows a parent entity, the parent entity must be accessed using explicit scope qualification.

### Global Scope Access

- The `::` operator without any preceding namespace name refers to the global (root) scope.
- This is needed when a namespace entity shadows a global entity of the same name.

```angelscript
int var;
namespace Parent
{
    int var;
    namespace Child
    {
        int var;
        void func()
        {
            var = Parent::var;       // Access parent namespace
            Parent::var = ::var;     // :: alone = global scope
        }
    }
}
```

### Fully Qualified Access

- From outside a namespace, entities are accessed with the full namespace path.

```angelscript
void func()
{
    int v = Parent::Child::var;  // Fully qualified access
}
```

### Namespace Reopening

- The same namespace can be declared in multiple places in the same file. Declarations are merged into the same namespace scope.

```angelscript
namespace Utils
{
    int Add(int a, int b) { return a + b; }
}

// Later in the same file or another section
namespace Utils
{
    int Multiply(int a, int b) { return a * b; }
}
// Both Add and Multiply are in Utils
```

### Using Namespace Directive

The `using namespace` directive allows access to entities from a specific namespace without requiring the scope resolution operator (`::`) on every reference.

**Syntax:**

```angelscript
using namespace <namespace_name>;
```

**Scope of Effect:**

- When declared at **global scope**, the directive makes the namespace entities visible throughout the entire script.
- When declared **inside a namespace**, the directive affects only that namespace scope.

**Example:**

```angelscript
namespace test
{
    void func() { }
    int value = 42;
}

// Using directive at global scope
using namespace test;

void main()
{
    func();           // Resolves to test::func() without explicit qualification
    int x = value;    // Resolves to test::value
}
```

**Multiple Using Directives:**

```angelscript
namespace Math
{
    float pi = 3.14159f;
}

namespace Physics
{
    float gravity = 9.81f;
}

using namespace Math;
using namespace Physics;

void main()
{
    float circumference = 2.0f * pi;   // Math::pi
    float force = gravity * mass;       // Physics::gravity
}
```

**Name Conflicts:**

If two imported namespaces declare the same entity name, the reference becomes ambiguous and requires explicit qualification:

```angelscript
namespace A { int value = 1; }
namespace B { int value = 2; }

using namespace A;
using namespace B;

void main()
{
    // int x = value;  // ERROR: Ambiguous reference
    int x = A::value;  // OK: Explicit qualification resolves ambiguity
}
```

**Using Within Namespaces:**

```angelscript
namespace Utils
{
    void helper() { }
}

namespace App
{
    using namespace Utils;  // Only visible within App namespace

    void process()
    {
        helper();  // Resolves to Utils::helper()
    }
}

void main()
{
    // helper();  // ERROR: Utils not visible here
    App::process();  // OK
}
```

### Entity Types in Namespaces

All global entity types can be declared within namespaces:

```angelscript
namespace Game
{
    // Variables
    int score;

    // Functions
    void AddScore(int points) { score += points; }

    // Classes
    class Player { /* ... */ }

    // Interfaces
    interface IUpdatable { void Update(float dt); }

    // Enums
    enum State { IDLE, RUNNING, JUMPING }

    // Funcdefs
    funcdef void Callback(int);

    // Typedefs
    typedef float real;

    // Virtual properties
    int health
    {
        get { return _hp; }
        set { _hp = value; }
    }
    private int _hp;
}
```

## Examples

```angelscript
// Two modules with same-named functions in different namespaces
namespace Physics
{
    void Update(float dt)
    {
        // Physics simulation step
    }

    float gravity = 9.81f;
}

namespace Graphics
{
    void Update(float dt)
    {
        // Rendering step
    }

    int screenWidth = 1920;
}

void main()
{
    float dt = 0.016f;
    Physics::Update(dt);
    Graphics::Update(dt);

    // Access namespaced variables
    Physics::gravity = 10.0f;
    int w = Graphics::screenWidth;
}

// Nested namespaces for deep organization
namespace Engine
{
    namespace Core
    {
        class Logger
        {
            void Log(string msg) { /* ... */ }
        }
    }

    namespace Render
    {
        void DrawFrame()
        {
            Engine::Core::Logger log;
            log.Log("Drawing frame");
        }
    }
}
```

## Compilation Notes

- **Module structure:** Namespaces are not separate module-level entities with their own storage. Instead, they are a scoping mechanism applied to the symbol table. Each entity (function, variable, class, etc.) is stored in its normal location (function table, variable table, type table) but with a namespace-qualified name. The compiler tracks the current namespace context during parsing and prefixes entity registrations accordingly.
- **Symbol resolution:** Name lookup proceeds from the innermost scope outward: first the current namespace, then parent namespaces, then the global scope. If a match is found at any level, lookup stops. The `::` prefix forces lookup to start at the global scope. Fully qualified names (e.g. `A::B::name`) are resolved by walking the namespace hierarchy left to right. When the scope resolution operator is used, only the specified namespace is searched -- no fallback to parent or global scope occurs for the qualified portion.
- **Using namespace resolution:** The `using namespace` directive adds the specified namespace to the compiler's symbol search path for unqualified name lookups. When an unqualified name is encountered, the compiler searches: (1) the current local scope, (2) the current namespace, (3) parent namespaces, (4) namespaces added via `using namespace` (in declaration order), (5) the global scope. If multiple `using` directives introduce the same name, the reference is ambiguous and requires explicit qualification. The directive affects only subsequent name lookups in the scope where it's declared (global or namespace-level).
- **Initialization:** Namespaces have no runtime representation. They affect only compile-time name resolution. Global variables inside namespaces follow the same initialization rules as other global variables (primitives first, then non-primitives). The initialization order is not affected by namespace boundaries.
- **Type system:** Namespace-qualified type names (e.g. `Game::Player`) are resolved to the same type entries as their unqualified counterparts within the namespace. The namespace is part of the type's qualified name for disambiguation purposes but does not create a separate type system. Two types with the same name in different namespaces are distinct types with distinct type IDs.
- **Special cases:** When using cross-module features like `import`, the imported function's namespace context from the source module is preserved. Registered (native) application entities can also be placed in namespaces by the host. Namespace-scoped entities can be declared as `shared`, following the same rules as global-scope shared entities. The namespace hierarchy supports an arbitrary depth of nesting.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Namespace` | Top-level item variant for namespace declarations | Wraps `NamespaceDecl` |
| `NamespaceDecl` | Namespace declaration | `path: &[Ident<'ast>]`, `items: &[Item<'ast>]`, `span: Span` |
| `Item::UsingNamespace` | Top-level item variant for using-namespace directives | Wraps `UsingNamespaceDecl` |
| `UsingNamespaceDecl` | Using namespace directive | `path: &[Ident<'ast>]`, `span: Span` |
| `Scope` | Scope path for namespace-qualified type references | `is_absolute: bool`, `segments: &[Ident<'ast>]`, `span: Span` |

**Notes:**
- `NamespaceDecl.path` is `&[Ident]`, supporting nested namespace paths (e.g., `namespace A::B::C` produces a path of `["A", "B", "C"]`).
- `NamespaceDecl.items` is `&[Item]`, meaning namespaces can contain any top-level item type (functions, classes, variables, enums, nested namespaces, etc.).
- `UsingNamespaceDecl` represents `using namespace Game::Utils;` directives. It is a separate `Item` variant (`Item::UsingNamespace`), not part of `NamespaceDecl`. The `path` field specifies the namespace to import.
- The `Scope` type (from `node.rs`) is used for namespace-qualified references in type expressions (e.g., `Game::Player`), with `is_absolute: true` for `::global` scope access.
- The `using namespace` directive adds the target namespace to the symbol search path but does not create any new entities or modify the namespace structure itself. It's purely a name resolution directive processed during semantic analysis.

## Related Features

- [Global Variables](./global-variables.md) -- variables can be scoped to namespaces
- [Global Functions](./global-functions.md) -- functions can be scoped to namespaces
- [Enums](./enums.md) -- enums can be declared within namespaces
- [Interfaces](./interfaces.md) -- interfaces can be declared within namespaces
- [Shared Entities](./shared-entities.md) -- namespaced entities can be shared
