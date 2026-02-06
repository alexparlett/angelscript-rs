# Enums

## Overview
Enums provide a way to declare a family of named integer constants that can be used throughout a script as readable literals instead of raw numeric values. They improve code readability by replacing magic numbers with descriptive names. Enums are registered as types in the type system with an underlying `int` representation.

## Syntax

```angelscript
// Basic enum declaration
enum MyEnum
{
    eValue0,                    // 0 (implicit)
    eValue2 = 2,               // 2 (explicit)
    eValue3,                    // 3 (previous + 1)
    eValue200 = eValue2 * 100  // Expressions referencing earlier values allowed
}

// Enum with all explicit values
enum Flags
{
    FLAG_NONE    = 0,
    FLAG_ACTIVE  = 0x01,
    FLAG_VISIBLE = 0x02,
    FLAG_ENABLED = 0x04
}
```

## Semantics

### Value Assignment

- The first constant receives the value `0` unless an explicit value is given.
- Each subsequent constant receives the value of the previous constant + 1, unless an explicit value is given.
- Explicit values can be expressions, including references to earlier constants in the same enum.

### Underlying Type

- Enum values are stored as `int` (32-bit signed integer).
- Enum types are distinct from `int` in the type system, but implicit conversions exist between them.

### Value Safety

- You **cannot rely** on an enum variable only containing values from the declared list.
- Enum variables can hold any integer value, including values not listed in the declaration.
- Always include a default case or guard when switching on enum values.

### Scope

- Enum values are injected into the enclosing scope (the global namespace or the containing namespace).
- Enum values can be referenced by their unqualified name (e.g. `eValue0`) or qualified with the enum name (e.g. `MyEnum::eValue0`) when disambiguation is needed.
- Two enums in the same namespace cannot define values with conflicting names.

## Examples

```angelscript
enum Color
{
    RED,       // 0
    GREEN,     // 1
    BLUE,      // 2
    ALPHA = 10 // 10
}

void ProcessColor(Color c)
{
    switch (c)
    {
    case RED:
        print("Red\n");
        break;
    case GREEN:
        print("Green\n");
        break;
    case BLUE:
        print("Blue\n");
        break;
    default:
        print("Unknown color: " + c + "\n");
        break;
    }
}

// Enum values used as integer constants
int flags = FLAG_ACTIVE | FLAG_VISIBLE;

// Enum as function parameter type
void SetState(Flags f)
{
    // ...
}
```

## Compilation Notes

- **Module structure:** Each enum declaration creates a type entry in the module's type table and a set of constant entries (one per enum value). The enum type itself is registered with the engine so it can be referenced by name. The enum values are stored as named constants with their integer values computed at compile time.
- **Symbol resolution:** Enum type names and their value names are registered in the global namespace (or the containing namespace). When the compiler encounters an identifier, it checks enum values in scope. Enum values can be referenced unqualified or qualified with the enum type name using the scope operator (e.g. `MyEnum::eValue0`). If an enum is in a namespace, the fully qualified form is `Namespace::MyEnum::eValue0` or just `Namespace::eValue0`.
- **Initialization:** Enum values are compile-time constants. Their values are computed during compilation, not at runtime. No initialization bytecode is generated for enum values -- they are embedded directly into the bytecode as immediate integer constants wherever they are used.
- **Type system:** Enums are distinct types from `int` but are implicitly convertible to and from `int`. In overload resolution, enum-to-int conversion has a specific cost that factors into which overload is selected. Enum types can be used in variable declarations, function parameters, return types, and class members. The type checker distinguishes between different enum types (e.g. `Color` vs `Flags`), but both can be implicitly converted to `int`.
- **Special cases:** Enums can be declared as `shared` for cross-module type sharing. Shared enums must have identical declarations across all modules that share them. The `external shared enum` form allows referencing a shared enum without re-declaring its values. When used in switch statements, the compiler can optimize case matching since enum values are compile-time constants.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Enum` | Top-level item variant for enum declarations | Wraps `EnumDecl` |
| `EnumDecl` | Enum declaration | `modifiers: DeclModifiers`, `name: Ident<'ast>`, `enumerators: &[Enumerator<'ast>]`, `span: Span` |
| `Enumerator` | A single enum value | `name: Ident<'ast>`, `value: Option<&Expr<'ast>>`, `span: Span` |
| `DeclModifiers` | Supports `shared` and `external` flags for enums | `shared: bool`, `external: bool`, `abstract_: bool`, `final_: bool` |

**Notes:**
- `Enumerator.value` is `Option<&Expr>` -- `None` when the value is implicitly assigned (previous + 1 or 0 for the first).
- Explicit values are represented as arbitrary `Expr` nodes, supporting expressions like `eValue2 * 100`.
- `DeclModifiers` on `EnumDecl` enables `shared` and `external shared` enum declarations. The `abstract_` and `final_` fields are not meaningful for enums.

## Related Features

- [Namespaces](./namespaces.md) -- enums can be declared inside namespaces
- [Shared Entities](./shared-entities.md) -- enums can be shared across modules
- [Typedefs](./typedefs.md) -- alternative mechanism for naming types (but only for primitives)
