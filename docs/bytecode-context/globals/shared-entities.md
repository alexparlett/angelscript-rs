# Shared Entities

## Overview
Shared entities allow script code to be reused across multiple script modules within the same engine. When an entity is declared as `shared`, all modules that declare it share a single implementation rather than each having their own copy. This reduces memory consumption and ensures type identity across modules, so objects of a shared type can be freely passed between modules without type incompatibility issues. The `external` keyword provides a shorthand for referencing shared entities that have already been compiled in another module.

## Syntax

### Declaring Shared Entities

```angelscript
// Shared class
shared class Foo
{
    void MethodInFoo(int b) { bar = b; }
    int bar;
}

// Shared function
shared void GlobalFunc() {}

// Shared interface
shared interface IShared
{
    void Method();
}

// Shared enum
shared enum SharedEnum
{
    Value1,
    Value2
}

// Shared funcdef
shared funcdef bool SharedCallback(int);
```

### External Shared Entities

```angelscript
// Reference a shared class already compiled in another module
external shared class Foo;

// Reference a shared function
external shared void GlobalFunc();

// Reference a shared interface
external shared interface IShared;

// Reference a shared enum
external shared enum SharedEnum;

// Reference a shared funcdef
external shared funcdef bool SharedCallback(int);
```

## Semantics

### What Can Be Shared

The following entity types can be declared as `shared`:
- Classes
- Interfaces
- Functions
- Enums
- Funcdefs

**Global variables cannot be shared** (this may be supported in future versions).

### The `shared` Keyword

- Place the `shared` keyword before the entity's declaration keyword (`class`, `interface`, `void`, `enum`, `funcdef`).
- The shared entity must be self-contained: it **cannot access non-shared entities**. This is because non-shared entities are exclusive to each module and may not exist in other modules.
- If a shared entity tries to reference a non-shared entity, the compiler produces an error.

### Implementation Consistency

- All modules that declare the same shared entity must implement it **identically**.
- If implementations differ, the compiler will produce an error in the module compiled after the first one that defined the entity.
- The easiest way to ensure consistency is to use the same source file for shared entities, but this is not strictly required.
- The engine compares the entity's structure (member layout, method signatures, enum values, etc.) to verify consistency.

### The `external` Keyword

- The `external` keyword declares that a shared entity has already been compiled in another module, so the current module just references it rather than re-declaring the full implementation.
- `external` must appear **before** `shared` in the declaration.
- External declarations end with a semicolon (no body, no implementation).
- If the referenced entity has not been compiled in any prior module, the compiler produces an error.

### Benefits of `external`

- Shorter source code -- no need to duplicate the full implementation.
- Faster compilation -- the compiler looks up the existing entity instead of re-compiling it.
- No consistency errors -- cannot accidentally introduce mismatched implementations.

### Restrictions on Shared Entities

- Shared entities cannot reference non-shared entities (variables, functions, classes, etc.).
- Shared classes can only inherit from other shared classes.
- Shared classes can only implement shared interfaces.
- Shared functions can only call other shared functions or registered (native) application functions.
- Shared functions can only use shared types in their parameters and return types.

## Examples

### Cross-Module Communication

```angelscript
// shared_types.as - Core module compiled first
shared class Message
{
    string content;
    int priority;
}

// Module A can create Message objects
// Module B can receive and process them
// The type is compatible across modules because it is shared
```

### Plugin System

```angelscript
// core.as - Compiled first
shared interface IPlugin
{
    void Initialize();
    void Update();
    string GetName();
}

// plugin_a.as - References the shared interface
external shared interface IPlugin;

class MyPlugin : IPlugin
{
    void Initialize() { /* ... */ }
    void Update() { /* ... */ }
    string GetName() { return "MyPlugin"; }
}
```

### Shared Utility Functions

```angelscript
// utils.as - Compiled first
shared int Clamp(int value, int min, int max)
{
    if (value < min) return min;
    if (value > max) return max;
    return value;
}

// game.as - Uses the shared function
external shared int Clamp(int value, int min, int max);

void main()
{
    int health = Clamp(rawHealth, 0, 100);
}
```

### Full Shared vs External Pattern

```angelscript
// Module 1 - Full declaration (compiled first)
shared class Config
{
    int width = 800;
    int height = 600;
    string title = "Game";
}

shared enum Difficulty
{
    EASY,
    NORMAL,
    HARD
}

// Module 2 - External references (compiled second)
external shared class Config;
external shared enum Difficulty;

void ApplyConfig(Config@ cfg, Difficulty diff)
{
    // Can use Config and Difficulty because they are shared
}
```

## Compilation Notes

- **Module structure:** Shared entities are stored at the **engine level**, not per-module. When the first module declares a shared entity, the engine creates the type/function entry in a shared registry. Subsequent modules that declare the same shared entity (either full or external) reference the same engine-level entry. This ensures a single type ID and single vtable for shared classes/interfaces across all modules.
- **Symbol resolution:** When the compiler encounters a `shared` declaration, it first checks whether the engine already has a shared entity with the same name. If yes and the declaration is `external`, it uses the existing entry. If yes and the declaration is a full redeclaration, it validates that the new declaration is identical to the existing one. If no existing entry is found and the declaration is `external`, a compiler error is produced. For name lookup within shared code, only other shared entities and registered application entities are visible.
- **Initialization:** Shared classes have their constructors and destructors compiled once (in the first module that declares them). Subsequent modules reuse the same compiled bytecode. Shared functions are similarly compiled once. Shared enums have their values set once during the first module's compilation. No re-initialization occurs when later modules reference the shared entity.
- **Type system:** Shared types have a single type ID across all modules, which is the key property that enables cross-module object passing. Without `shared`, each module generates its own type ID for a class, making objects from one module incompatible with the same-named class in another module. The type checker enforces that shared entities only depend on other shared entities or registered application types. This constraint ensures that the shared entity's behavior is deterministic regardless of which module context it executes in.
- **Special cases:** When a module is discarded, shared entities it declared are not removed from the engine as long as other modules still reference them. The engine tracks reference counts on shared entities. Only when the last module referencing a shared entity is discarded does the shared entity get removed. This reference counting is important for hot-reloading scenarios where modules are rebuilt independently. Shared entities in namespaces follow the same rules, with the namespace forming part of the qualified name used for matching across modules.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `DeclModifiers` | Carries `shared` and `external` flags for declarations | `shared: bool`, `external: bool`, `abstract_: bool`, `final_: bool` |

The `shared` and `external` flags are carried by `DeclModifiers`, which appears on the following declaration types:

| Declaration Type | Has `DeclModifiers` | Supports `shared` | Supports `external` |
|-----------------|--------------------|--------------------|---------------------|
| `FunctionDecl` | Yes | Yes | Yes |
| `ClassDecl` | Yes | Yes | Yes |
| `InterfaceDecl` | Yes | Yes | Yes |
| `EnumDecl` | Yes | Yes | Yes |
| `FuncdefDecl` | Yes | Yes | Yes |
| `GlobalVarDecl` | No | No | No |
| `TypedefDecl` | No | No | No |
| `ImportDecl` | No | No | No |
| `NamespaceDecl` | No | No | No |
| `MixinDecl` | No (inner `ClassDecl` has it) | Via inner `ClassDecl` | Via inner `ClassDecl` |

**Notes:**
- `DeclModifiers` also carries `abstract_` and `final_` flags, which are relevant to classes (not to the `shared`/`external` mechanism itself).
- `GlobalVarDecl` has no `DeclModifiers`, confirming the documentation that global variables cannot be shared.
- The `external` keyword is modeled as a boolean on `DeclModifiers`, not as a separate declaration type. An `external shared class Foo;` is represented as a `ClassDecl` with `modifiers.external = true`, `modifiers.shared = true`, and empty `members`.

## Related Features

- [Global Functions](./global-functions.md) -- functions can be declared as shared
- [Interfaces](./interfaces.md) -- interfaces can be shared for cross-module polymorphism
- [Enums](./enums.md) -- enums can be shared for cross-module constant sharing
- [Funcdefs](./funcdefs.md) -- funcdefs can be shared for cross-module callback types
- [Imports](./imports.md) -- imports are an alternative cross-module mechanism (function-only, bind-time)
- [Namespaces](./namespaces.md) -- shared entities can be declared within namespaces
