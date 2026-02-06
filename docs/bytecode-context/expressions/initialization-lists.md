# Initialization Lists

## Overview

Initialization lists provide a brace-enclosed `{...}` syntax for creating and populating collections and structured objects in a single expression. They are used to initialize arrays, dictionaries, and any registered type that declares a list constructor (for value types) or a list factory function (for reference types). The target type can be given explicitly or inferred from the surrounding context.

## Syntax

```angelscript
// Flat initialization list with explicit type
array<int> a = {1, 2, 3};

// Flat initialization list with implicit type (inferred from context)
funcExpectsArrayOfInts({1, 2, 3});

// Nested initialization list (key-value pairs)
dictionary d = {{"banana", 1}, {"apple", 2}, {"orange", 3}};

// Nested initialization list with nested sublists
array<array<int>> grid = {{1, 2}, {3, 4}};

// Explicit type annotation as anonymous object (for disambiguation among overloads)
foo(dictionary = {{"a", 1}, {"b", 2}});
foo(array<int> = {1, 2, 3, 4});

// Mixed-type values using the ? token (dictionary values can be any type)
dictionary d = {{"name", "Alice"}, {"age", 30}, {"handle", @obj}};

// Value type with fixed-element list constructor
vector3 v = {1.0f, 2.0f, 3.0f};
```

## Semantics

### Explicit vs implicit type resolution

- **Explicit type:** The programmer writes `TypeName = {list}`. The compiler knows the target type and matches the list against that type's registered list pattern.
- **Implicit type (contextual inference):** When the initialization list appears without a type name (e.g., as a function argument `foo({1,2,3})`), the compiler uses the expected type from the surrounding context -- typically the parameter type of the function being called. If there is exactly one candidate type that accepts initialization lists, the compiler selects it. If there are zero or multiple candidates, it is a compile-time error.

### Nested initialization lists

Initialization lists can be nested to arbitrary depth. Each pair of braces `{}` corresponds to a sublist in the list pattern. For example:
- `{1, 2, 3}` matches a flat `{repeat int}` pattern.
- `{{"key", value}, ...}` matches a `{repeat {string, ?}}` pattern (each inner `{}` is a sublist with a string and a variant value).
- `{{1, 2}, {3, 4}}` matches a `{repeat {repeat_same int}}` pattern (each inner `{}` is a repeated sequence that must all have the same length).

### List constructors for value types (asBEHAVE_LIST_CONSTRUCT)

Value types register a list constructor via `asBEHAVE_LIST_CONSTRUCT`. The constructor receives a pointer to a pre-populated buffer (declared as `int &in` in the registration signature) and a list pattern string. The list constructor is called like a regular constructor -- it operates on pre-allocated memory for the value type -- but receives the buffer instead of individual arguments.

Registration example:
```cpp
// A vector3 type initialized with {float, float, float}
engine->RegisterObjectBehaviour("vector3", asBEHAVE_LIST_CONSTRUCT,
    "void f(int &in) {float, float, float}", ...);
```

### List factory functions for reference types (asBEHAVE_LIST_FACTORY)

Reference types register a list factory function via `asBEHAVE_LIST_FACTORY`. The factory function takes a single pointer parameter (the buffer) and returns a handle to the newly created object. The list pattern is appended to the registration signature.

Registration examples:
```cpp
// Array: initialized with {repeat int}
engine->RegisterObjectBehaviour("intarray", asBEHAVE_LIST_FACTORY,
    "intarray@ f(int &in) {repeat int}", ...);

// Dictionary: initialized with {repeat {string, ?}}
engine->RegisterObjectBehaviour("dictionary", asBEHAVE_LIST_FACTORY,
    "dictionary @f(int &in) {repeat {string, ?}}", ...);

// Grid: initialized with {repeat {repeat_same int}}
engine->RegisterObjectBehaviour("grid", asBEHAVE_LIST_FACTORY,
    "grid @f(int &in) {repeat {repeat_same int}}", ...);
```

### List pattern tokens

The list pattern string uses a special mini-language with these tokens:

| Token | Meaning |
|-------|---------|
| `{ }` | Delimiters for a list or sublist of values |
| `repeat` | The next type or sublist can appear 0 or more times |
| `repeat_same` | Like `repeat`, but every repetition of the enclosing list must have the same length |
| `?` | A variable type -- any type can be placed here |
| Any type name | A specific value type expected at that position (e.g., `int`, `float`, `string`) |

**Pattern examples:**
- `{repeat int}` -- zero or more integers (used by `array<int>`)
- `{repeat {string, ?}}` -- zero or more pairs of (string, any-type) (used by `dictionary`)
- `{repeat {repeat_same int}}` -- zero or more sublists of integers where every sublist must have the same length (used by a grid type)
- `{float, float, float}` -- exactly three floats (used by `vector3`)

### Buffer layout rules

When the compiler builds the initialization list buffer, it follows these rules:

1. **repeat / repeat_same prefix:** Whenever the pattern expects a `repeat` or `repeat_same`, the buffer begins with a `uint32` (32-bit unsigned integer) containing the count of repeated elements that follow.

2. **Variable type (?) prefix:** Whenever the pattern expects `?`, the buffer contains a `int32` (32-bit integer) holding the `typeId` of the value that follows. This allows the receiving function to identify the type at runtime.

3. **Value types:** When the pattern expects a value type, the value itself is placed directly in the buffer at the appropriate size.

4. **Reference types:** When the pattern expects a reference type, a pointer to the object is placed in the buffer.

5. **Alignment:** All values in the buffer are aligned to a 32-bit (4-byte) boundary, unless the value being placed is smaller than 32 bits (in which case it occupies only its natural size, but the next entry still starts at a 32-bit boundary relative to the buffer start).

6. **Sublists:** Nested `{}` groups in the pattern produce nested structures in the buffer. Each sublist that uses `repeat` or `repeat_same` gets its own count prefix.

## Examples

```angelscript
// Array initialization with explicit type
array<int> numbers = {10, 20, 30, 40, 50};

// Array initialization with implicit type as function argument
void processScores(array<int> scores) { }
processScores({95, 87, 73, 100});

// Dictionary initialization
dictionary config = {
    {"width", 1920},
    {"height", 1080},
    {"title", "My Game"},
    {"fullscreen", true}
};

// Dictionary with handles
obj myObj;
dictionary registry = {
    {"player", @myObj},
    {"score", 42}
};

// Passing dictionary as anonymous object (explicit type for overload disambiguation)
void configure(dictionary@ opts) { }
configure(dictionary = {{"debug", true}, {"verbose", false}});

// 2D array (nested initialization list)
array<array<int>> matrix = {{1, 2, 3}, {4, 5, 6}, {7, 8, 9}};

// Value type with fixed list constructor (e.g., vector3)
vector3 pos = {1.0f, 2.0f, 3.0f};

// Nested dictionaries using anonymous objects
dictionary menus = {
    {"file", dictionary = {{"new", 1}, {"open", 2}, {"save", 3}}},
    {"edit", dictionary = {{"undo", 1}, {"redo", 2}}}
};
```

## Compilation Notes

- **Buffer construction:** The compiler evaluates each element expression in the initialization list in order (left to right, outer to inner for nested lists) and writes the resulting values into a contiguous buffer. For `repeat` patterns, the compiler first writes the element count as a `uint32`, then writes each element value. For `?` patterns, the compiler writes the `typeId` as an `int32` before each value. The buffer is fully populated before being passed to the list constructor or list factory function.

- **Type resolution:** The compiler determines the target type in one of two ways:
  1. **Explicit annotation:** When the programmer writes `TypeName = {list}`, the compiler looks up the type and retrieves its registered list pattern.
  2. **Context inference:** When only `{list}` appears (no type name), the compiler examines the expected type from the surrounding context. For function arguments, this is the parameter type. For variable declarations, this is the declared variable type. If exactly one registered type with a list constructor/factory matches, it is selected. Ambiguity (zero or multiple matches) is a compile-time error.

- **Memory layout:** The buffer is a flat byte array. The layout is determined entirely by the list pattern:
  - Each `repeat` or `repeat_same` token produces a 4-byte count prefix.
  - Each `?` token produces a 4-byte typeId prefix before the value.
  - Value types are placed inline at their natural size.
  - Reference types are placed as pointer-sized entries.
  - All entries are aligned to 4-byte boundaries. If a value is smaller than 4 bytes (e.g., `int8`), it still occupies its natural size, but padding is added so the next entry starts at a 4-byte boundary.

- **Control flow:** List elements are evaluated left to right. For nested sublists, the outer list is processed first (writing the repeat count), then each inner sublist is processed in order. There is no short-circuit evaluation -- all elements are always evaluated. If an element expression has side effects, they occur in this left-to-right order.

- **Special cases:**
  - **Nested lists:** Each level of nesting in the list pattern corresponds to a sublist in the source code. The compiler tracks the current depth and matches braces against the pattern structure. Mismatched nesting is a compile-time error.
  - **Mixed types with `?`:** When the pattern contains `?`, the compiler accepts any type for that position and records the `typeId` in the buffer. The receiving function must inspect the typeId at runtime to determine how to interpret the value. This is how dictionaries handle heterogeneous values.
  - **`repeat_same` constraints:** When `repeat_same` is used, the compiler enforces that every repetition of the enclosing sublist has the same number of elements. For example, in a grid pattern `{repeat {repeat_same int}}`, all rows must have the same column count. A mismatch is a compile-time error.
  - **Value type list constructors vs reference type list factories:** For value types, the list constructor receives a pointer to the buffer and operates on pre-allocated stack/object memory (like a regular constructor). For reference types, the list factory allocates a new object on the heap, initializes it from the buffer, and returns a handle (with refcount = 1). The factory must not return null without setting an exception.
  - **Temporary lifetime:** When an initialization list creates an anonymous object in an expression (e.g., as a function argument), the resulting object is a temporary. For reference types, the handle is released after the enclosing expression completes. For value types, the stack memory is reclaimed normally.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/expr.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Expr::InitList` | Initialization list expression variant | Wraps `InitListExpr` |
| `InitListExpr` | Brace-enclosed initializer list | `ty: Option<TypeExpr>`, `elements: &[InitElement]`, `span: Span` |
| `InitElement::Expr` | Expression element in init list | Wraps `&Expr` |
| `InitElement::InitList` | Nested initializer list element | Wraps `InitListExpr` |

**Notes:**
- `InitListExpr.ty` is `Some(TypeExpr)` for explicitly typed lists (`TypeName = {list}`) and `None` for implicitly typed lists (`{list}`).
- Nested initialization lists (e.g., `{{1,2},{3,4}}`) are represented via `InitElement::InitList`, which recursively wraps another `InitListExpr`.
- The `InitElement` enum distinguishes between expression elements and nested sublists at the AST level, enabling the compiler to match against list patterns during semantic analysis.
- `InitListExpr` is a value type (not behind a reference) in the `Expr::InitList` variant, unlike most other expression structs which are arena-allocated references.

## Related Features

- [anonymous-objects.md](anonymous-objects.md) - Anonymous object creation with initialization lists
- [function-calls.md](function-calls.md) - Passing initialization lists as function arguments
- [../types/arrays.md](../types/arrays.md) - Array type using initialization lists
- [../types/dictionary.md](../types/dictionary.md) - Dictionary type using initialization lists
