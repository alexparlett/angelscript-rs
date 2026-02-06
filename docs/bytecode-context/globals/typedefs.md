# Typedefs

## Overview
Typedefs create aliases for existing types, allowing scripts to use a more descriptive or domain-specific name for a type. In the current version of AngelScript, typedefs are restricted to **primitive types only**. Future versions may support aliasing all kinds of types.

## Syntax

```angelscript
typedef float  real32;
typedef double real64;
typedef int    EntityId;
typedef uint8  byte;
typedef int16  short;
```

The general form is:

```angelscript
typedef <existing_primitive_type> <new_name>;
```

## Semantics

### Restrictions

- Typedefs can **only** alias primitive types: `int`, `int8`, `int16`, `int64`, `uint`, `uint8`, `uint16`, `uint64`, `float`, `double`, `bool`.
- You cannot typedef classes, interfaces, handles, arrays, enums, or other non-primitive types.
- The alias is fully interchangeable with the original type -- it is not a distinct type. A `real32` variable can be passed to a function expecting `float` with no conversion.

### Scope

- Typedefs are declared at global scope (or within a namespace).
- The alias name shares the global namespace with all other global entities and must not conflict.
- The alias is visible to all code in the module, regardless of declaration order.

### Type Identity

- A typedef does **not** create a new type. It creates a synonym.
- `real32` and `float` are the same type after typedef; they produce the same type hash and are indistinguishable in the type system.

## Examples

```angelscript
// Platform-specific float precision
typedef float real;

real gravity = 9.81f;

real ComputeForce(real mass, real acceleration)
{
    return mass * acceleration;
}

// Semantic clarity
typedef uint EntityId;
typedef uint ComponentId;

void AttachComponent(EntityId entity, ComponentId component)
{
    // EntityId and ComponentId are both uint, but the names
    // document intent (note: they are NOT distinct types)
}
```

## Compilation Notes

- **Module structure:** A typedef creates an entry in the module's type alias table mapping the new name to the existing primitive type identifier. No new type is generated -- the alias is resolved at compile time.
- **Symbol resolution:** When the compiler encounters a typedef name in a type position, it immediately resolves it to the underlying primitive type. This resolution happens during parsing/name resolution, before any bytecode is generated. The typedef name is registered in the global (or namespace) symbol table.
- **Initialization:** Typedefs require no runtime initialization. They are purely a compile-time name mapping.
- **Type system:** Since typedefs are transparent aliases, they do not affect type checking, overload resolution, or any other type-system operation. Two typedefs that alias the same primitive are interchangeable. This means `typedef uint EntityId` and `typedef uint ComponentId` do NOT provide type safety between each other -- both are just `uint`.
- **Special cases:** Because typedefs only work with primitives, they cannot be used to alias template instantiations (e.g. `array<int>`), handles, or class types. This limits their utility compared to typedefs in languages like C++. The restriction exists because non-primitive type aliasing would require more complex resolution in the type system.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Typedef` | Top-level item variant for typedef declarations | Wraps `TypedefDecl` |
| `TypedefDecl` | Typedef declaration | `base_type: TypeExpr<'ast>`, `name: Ident<'ast>`, `span: Span` |

**Notes:**
- `TypedefDecl` has no `DeclModifiers` field, meaning typedefs cannot be declared as `shared` or `external`. This is consistent with the documentation's restriction that typedefs only alias primitive types.
- `base_type` is a `TypeExpr`, which in practice must resolve to a primitive type. The parser does not enforce the "primitives only" restriction at the AST level -- that validation happens during semantic analysis.

## Related Features

- [Enums](./enums.md) -- enums provide named integer constants with distinct types (stronger typing than typedefs)
- [Funcdefs](./funcdefs.md) -- funcdefs define function signature types (not type aliases)
- [Namespaces](./namespaces.md) -- typedefs can be declared inside namespaces
