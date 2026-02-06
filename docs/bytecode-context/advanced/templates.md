# Template Types

## Overview

Template types in AngelScript allow the application to register generic container types that scripts can instantiate with different subtypes. They work similarly to C++ templates from the script writer's perspective, but the underlying implementation is a single generic class that adapts dynamically at runtime based on the subtype. For performance-critical subtypes, the application can register template specializations with dedicated implementations.

Template types are registered from the **host application** side. Scripts cannot define new template types, but they can instantiate registered template types with any valid subtype.

## Syntax

### Instantiation in Script

```angelscript
// Instantiate a template type with a subtype
array<int> intArray;
array<string> stringArray;
array<MyClass@> handleArray;

// Nested templates
array<array<int>> nestedArray;

// Multiple subtypes (if registered with multiple type parameters)
dictionary<string, int> map;
```

### Initialization list syntax

```angelscript
array<int> values = {1, 2, 3, 4, 5};
array<string> names = {"Alice", "Bob"};
```

## Semantics

### Registration from Host

Template types are registered with the `asOBJ_TEMPLATE` flag. The type name includes the subtype parameter(s) with `class` keyword:

```cpp
// Reference type template
engine->RegisterObjectType("myTemplate<class T>", 0,
    asOBJ_REF | asOBJ_GC | asOBJ_TEMPLATE);

// Value type template
engine->RegisterObjectType("myValueTemplate<class T>", sizeof(MyValueTempl),
    asOBJ_VALUE | asOBJ_TEMPLATE | asGetTypeTraits<MyValueTempl>());

// Multiple subtype parameters
engine->RegisterObjectType("myMap<class K, class V>", 0,
    asOBJ_REF | asOBJ_TEMPLATE);
```

### Factory/Constructor Hidden Parameter

Template factories and constructors receive the `asITypeInfo` of the template instance as a hidden first parameter (declared as `int &in`):

```cpp
// Factory for reference type template
engine->RegisterObjectBehaviour("myTemplate<T>", asBEHAVE_FACTORY,
    "myTemplate<T>@ f(int&in)",  // int&in = hidden asITypeInfo* param
    asFUNCTIONPR(myTemplateFactory, (asITypeInfo*), myTemplate*),
    asCALL_CDECL);

// Constructor for value type template
engine->RegisterObjectBehaviour("myValueTemplate<T>", asBEHAVE_CONSTRUCT,
    "void f(int&in)",  // int&in = hidden asITypeInfo* param
    asFUNCTIONPR(myValueTemplConstructor, (asITypeInfo*, void*), void),
    asCALL_CDECL_OBJLAST);
```

List factories/constructors follow the same pattern:

```cpp
// List factory
engine->RegisterObjectBehaviour("myTemplate<T>", asBEHAVE_LIST_FACTORY,
    "myTemplate<T>@ f(int&in, uint)",
    asFUNCTIONPR(myTemplateListFactory, (asITypeInfo*, unsigned int), myTemplate*),
    asCALL_CDECL);
```

### Subtype Constraints

- Methods and behaviours **cannot** accept or return the subtype by value, because the size is unknown at registration time.
- Subtype parameters must be passed by reference (`T&in`, `T&out`, `T&inout`).
- Object handles (`T@`) are allowed, but then the template cannot be instantiated with primitive or value types.
- Object properties **cannot** be of the template subtype.

### Subtype Replacement Rules

When a template is instantiated, the compiler replaces subtype placeholders with the concrete type. Special rules apply for `const` references:

**Without `if_handle_then_const`:**
```cpp
// Registration:
engine->RegisterObjectMethod("array<T>",
    "int find(const T&in value) const", ...);

// Instantiated as array<Obj@>, becomes:
int find(Obj @const &in value) const
// The handle is const, but the object it points to is NOT const
```

**With `if_handle_then_const`:**
```cpp
// Registration:
engine->RegisterObjectMethod("array<T>",
    "int find(const T&in if_handle_then_const value) const", ...);

// Instantiated as array<Obj@>, becomes:
int find(const Obj @const &in value) const
// Both the handle AND the object are const
```

This keyword ensures the method works with both read-only and non-read-only handles.

### Template Callback (Compile-Time Validation)

The `asBEHAVE_TEMPLATE_CALLBACK` behaviour allows the application to validate template instantiations at compile time:

```cpp
engine->RegisterObjectBehaviour("myTemplate<T>", asBEHAVE_TEMPLATE_CALLBACK,
    "bool f(int &in, bool&out)",
    asFUNCTION(myTemplateCallback), asCALL_CDECL);
```

The callback receives:
- `asITypeInfo*` -- The template instance type info
- `bool&` (output) -- Set to `true` to disable garbage collection for this instance

The callback returns `true` if the instantiation is valid, `false` to reject it (causing a compile error).

```cpp
bool myTemplateCallback(asITypeInfo *ot, bool &dontGarbageCollect)
{
    int typeId = ot->GetSubTypeId();
    if (typeId & asTYPEID_MASK_OBJECT)
    {
        // Reject object subtypes
        return false;
    }
    dontGarbageCollect = true;  // Primitives don't need GC
    return true;
}
```

### Template Specializations

A template specialization overrides the generic template for a specific subtype. It is registered as a completely separate type with its own implementation:

```cpp
// Register specialization for float
engine->RegisterObjectType("myTemplate<float>", 0, asOBJ_REF);

// Factory has NO hidden parameter (concrete type is known)
engine->RegisterObjectBehaviour("myTemplate<float>", asBEHAVE_FACTORY,
    "myTemplate<float>@ f()",
    asFUNCTIONPR(myTemplateFloatFactory, (), myTemplateFloat*),
    asCALL_CDECL);
```

Key differences from generic template:
- No hidden `asITypeInfo*` parameter in factory/constructor
- Can have properties of the concrete subtype
- Can accept/return the subtype by value
- Must be registered in the same namespace as the generic template
- Should present the same API to scripts for transparency

## Examples

### Using the Built-in Array Template

```angelscript
// Integer array
array<int> numbers = {10, 20, 30};
numbers.insertLast(40);
int len = numbers.length();

// Handle array
array<Object@> objects;
objects.insertLast(Object());

// Finding elements
int idx = numbers.find(20);  // Returns 1

// Sorting
numbers.sortAsc();
```

### Nested Templates

```angelscript
// Array of arrays
array<array<int>> matrix;
matrix.insertLast({1, 2, 3});
matrix.insertLast({4, 5, 6});

int val = matrix[0][1];  // 2
```

## Compilation Notes

- **Type considerations:** Each unique template instantiation creates a distinct type in the engine's type system. `array<int>` and `array<float>` are different types with different type IDs. The compiler generates type-specific method calls based on the instantiated type.
- **Runtime support:** The generic template implementation must inspect the `asITypeInfo` at runtime to determine element sizes, alignment, and whether the subtype requires reference counting or garbage collection. This adds runtime overhead compared to specializations.
- **Stack behavior:** Template methods are registered application functions. Calling them follows the same bytecode sequence as any other registered function call (`asBC_CALLSYS`). The hidden `asITypeInfo*` parameter is pushed on the stack before the factory/constructor call.
- **Garbage collection:** Template instances are typically garbage collected (`asOBJ_GC`) because the subtype may form circular references. The template callback can disable GC for specific instantiations where circular references are impossible (e.g., primitive subtypes).
- **Template caching:** The engine caches template instance types. Once `array<int>` is instantiated, subsequent uses of `array<int>` reuse the same `asITypeInfo`. The template callback is invoked only once per unique instantiation.
- **Special cases:** Template specializations completely bypass the generic implementation. The compiler selects the specialization when available, and the generated bytecode is identical to calling methods on any non-template registered type.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`, `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::TemplateParam` | Template parameter declaration in FFI type registration (e.g., `class T` in `array<class T>`) | Wraps `Ident<'ast>` |
| `TypeExpr` | Type expression that can carry template arguments | `is_const: bool`, `scope: Option<Scope<'ast>>`, `base: TypeBase<'ast>`, `template_args: &[TypeExpr<'ast>]`, `suffixes: &[TypeSuffix]`, `span: Span` |
| `ClassDecl.template_params` | Template parameters on a class declaration | `&[Ident<'ast>]` |
| `FunctionDecl.template_params` | Template parameters on a function declaration | `&[Ident<'ast>]` |
| `FuncdefDecl.template_params` | Template parameters on a funcdef declaration | `&[Ident<'ast>]` |

**Notes:**
- Template instantiation in script code (e.g., `array<int>`) is represented through `TypeExpr.template_args`, which holds the list of concrete type arguments.
- `TypeBase::TemplateParam` is used specifically for FFI type registration strings (e.g., parsing `"array<class T>"` during host registration), not for script-level template syntax.
- `ClassDecl.template_params`, `FunctionDecl.template_params`, and `FuncdefDecl.template_params` are `&[Ident]` slices that store template parameter names for application-registered template types. These are empty for all script-declared entities since scripts cannot define new template types.
- Scripts cannot define new template types -- only the host application can register them. The AST supports template parameters to represent these registrations during FFI parsing.

## Related Features

- [arrays](../types/arrays.md) -- The built-in `array<T>` template type
- [dictionary](../types/dictionary.md) -- The `dictionary` type (related container)
- [objects](../types/objects.md) -- Value types vs reference types (affects template behaviour)
- [handles](../types/handles.md) -- Handle semantics within template types
- [type-conversions](../expressions/type-conversions.md) -- Type conversion rules for template subtypes
