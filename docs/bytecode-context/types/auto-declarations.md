# Auto Declarations

## Overview

The `auto` keyword enables type inference for variable declarations in AngelScript. When used in an assignment-style declaration, the compiler automatically determines the variable's type from the initialization expression. This reduces redundancy, especially with long type names, while maintaining full static typing. The resolved type is fixed at compile time -- `auto` is purely syntactic sugar and has no runtime cost.

## Syntax

```angelscript
// Basic type inference
auto i = 18;                   // int
auto f = 18 + 5.f;            // float
auto d = 3.14;                 // double
auto b = true;                 // bool
auto s = "hello";             // string (if registered)

// Inferred from function return type
auto o = getLongObjectTypeNameById(id);   // Whatever the function returns

// Const auto
const auto c = 2;             // const int

// Auto with reference types (becomes a handle)
auto a = getObject();          // Resolves to obj@ (handle), not obj
auto@ b = getObject();         // Explicit handle syntax (same result)
```

## Semantics

### Type resolution rules

1. The variable **must** have an initialization expression. `auto x;` without an initializer is invalid.
2. The type is determined from the type of the right-hand side expression at compile time.
3. For **primitive types**, `auto` resolves to the exact type of the expression:
   - Integer literal: `int`
   - Float literal (with `f` suffix): `float`
   - Double literal: `double`
   - Boolean literal: `bool`
   - Mixed arithmetic follows normal promotion rules (e.g., `int + float` -> `float`).
4. For **reference types**, `auto` resolves to a **handle** (`@`) rather than a value type. This is because handle assignment is more efficient than value copy for reference types.
5. `auto@` is explicitly allowed as an alternative syntax to make the handle intent clear. It produces the same result as plain `auto` for reference types.
6. `const auto` applies the `const` qualifier to the inferred type.

### Restrictions

- `auto` **cannot** be used for class member declarations. The type of a class member cannot depend on a constructor, so the compiler cannot resolve it from context.
- `auto` cannot be used for function parameter types.
- `auto` cannot be used for function return types.
- `auto` requires an initializer -- it cannot be used with default initialization.

### Interaction with handles

For types that support handles, `auto` always resolves to a handle type:

```angelscript
auto a = getObject();   // a is obj@ (handle), NOT obj (value)
```

If you explicitly need a value (non-handle) variable, you must spell out the type:

```angelscript
obj a = getObject();    // a is obj (value copy)
```

## Examples

```angelscript
// Avoid redundant type names
class VeryLongClassName {
    int value;
}

VeryLongClassName@ createInstance() {
    VeryLongClassName obj;
    obj.value = 42;
    return @obj;
}

void main() {
    // Without auto: redundant
    VeryLongClassName@ instance = createInstance();

    // With auto: concise
    auto instance2 = createInstance();   // type is VeryLongClassName@

    // Const inference
    const auto pi = 3.14159;            // const double

    // Expression type inference
    auto sum = 10 + 20;                 // int
    auto product = 10 * 2.5;            // double (int promoted to double)
    auto flag = (sum > 25);             // bool

    // Array with auto
    auto arr = array<int> = {1, 2, 3};  // array<int>@

    // Loop variable (common use case)
    array<string> names = {"Alice", "Bob", "Charlie"};
    for (uint i = 0; i < names.length(); i++) {
        auto name = names[i];           // string (or string@ depending on registration)
        print(name + "\n");
    }
}

// auto with various return types
int getInt() { return 42; }
float getFloat() { return 3.14f; }
string getString() { return "hello"; }

void main() {
    auto a = getInt();      // int
    auto b = getFloat();    // float
    auto c = getString();   // string or string@ depending on registration
}
```

## Compilation Notes

- **Memory layout:** `auto` has no runtime representation. It is resolved at compile time to the concrete type, and the resulting variable has exactly the same memory layout as if the type had been written explicitly.
- **Stack behavior:** Identical to the resolved type. If `auto` resolves to `int`, the variable occupies a 4-byte stack slot. If it resolves to `obj@`, it occupies a pointer-sized slot. No additional overhead.
- **Type considerations:**
  - The compiler must fully evaluate the type of the initialization expression before the variable's type can be determined. This means the RHS must be unambiguous.
  - For reference types, the compiler must determine whether the type supports handles and automatically choose handle semantics.
  - `auto` does not introduce any new type into the type system. After resolution, the variable is indistinguishable from one declared with the explicit type.
  - `const auto` applies const to the resolved type. For handles, `const auto` produces `const obj@` (handle to const object), not `obj@ const` (const handle).
- **Lifecycle:** Identical to the resolved type. Construction, copy, and destruction behave as if the type were written explicitly.
- **Special cases:**
  - When `auto` resolves to a handle type, the compiler must emit handle assignment (addref) rather than value copy for the initialization.
  - If the initialization expression type is ambiguous (e.g., overloaded function returning different types), the compiler should report an error.
  - `auto@` is syntactic sugar: it is equivalent to `auto` when the resolved type is a handle, and may produce a clearer error if the resolved type does not support handles.
  - The compiler must reject `auto` in class member declarations at parse/semantic analysis time, not at bytecode generation time.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Auto` | The `auto` keyword used as a type base | No fields (unit variant) |
| `TypeExpr<'ast>` | Full type expression wrapping `auto` | `is_const: bool` (for `const auto`), `base: TypeBase::Auto`, `suffixes: &'ast [TypeSuffix]` (for `auto@`) |

**Notes:**
- `auto` is represented as `TypeExpr { base: TypeBase::Auto, .. }`.
- `const auto` sets `TypeExpr.is_const = true` with `TypeBase::Auto`.
- `auto@` (explicit handle auto) is represented as `TypeExpr { base: TypeBase::Auto, suffixes: &[TypeSuffix::Handle { is_const: false }], .. }`.
- `TypeBase::Auto` is purely a parser-level placeholder. During semantic analysis / type inference, it is resolved to the concrete type of the initialization expression. No `Auto` type survives past the compilation front-end.
- **Cross-reference -- other `TypeBase` variants without dedicated docs:**
  - `TypeBase::TemplateParam(Ident)`: Represents `class T` in FFI template type parameter declarations (e.g., `array<class T>`). See [advanced/templates.md](../advanced/templates.md) if available.
  - `TypeBase::Unknown`: Represents the `?` placeholder type, used in certain FFI/registration contexts (e.g., `dictionary.set(const string &in, ? &in)`). There is no dedicated documentation file for this variant.

## Related Features

- [Primitive types (auto with literals)](./primitives.md)
- [Object handles (auto resolves to handles)](./handles.md)
- [Objects (value vs reference type resolution)](./objects.md)
- [Function pointers (auto with funcdef handles)](./funcptr.md)
