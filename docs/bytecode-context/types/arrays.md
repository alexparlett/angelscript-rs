# Arrays

## Overview

The `array<T>` type is a dynamic, resizable container that holds elements of a single type `T`. Arrays are a reference type (heap-allocated, reference-counted) even when the elements are value types. This means handles to arrays can be used to avoid costly copies when passing arrays around. Arrays are only available if the host application registers support for them.

## Syntax

### Declaration and initialization

```angelscript
// Declaration forms
array<int> a, b, c;            // Multiple arrays of integers
array<Foo@> d;                  // Array of handles to Foo objects

// Empty array
array<int> a;                   // Zero-length array

// Sized array (default-initialized elements)
array<int> b(3);                // [0, 0, 0]

// Sized array with fill value
array<int> c(3, 1);             // [1, 1, 1]

// Initialization list
array<int> d = {5, 6, 7};      // [5, 6, 7]

// Multidimensional arrays (arrays of arrays)
array<array<int>> a;                        // Empty 2D array
array<array<int>> b = {{1,2},{3,4}};        // 2x2 with values
array<array<int>> c(10, array<int>(10));    // 10x10 array
```

### Element access

```angelscript
// Index access (0-based)
a[0] = some_value;

// Handle assignment in array of handles
array<Foo@> arr(1);
@arr[0] = Foo();
```

### Anonymous array construction

```angelscript
// Implicit type (when function signature is unambiguous)
foo({1, 2, 3, 4});

// Explicit type (when overloads require disambiguation)
foo2(array<int> = {1, 2, 3, 4});
```

## Semantics

### Operators

| Operator | Description |
|----------|-------------|
| `=` | Shallow copy of the array content |
| `[]` | Index access; returns a reference to the element. Raises an exception if the index is out of range. |
| `==` | Value comparison of each element; returns true if all elements are equal |
| `!=` | Value comparison; returns true if any element differs |

### Methods

| Method | Description |
|--------|-------------|
| `uint length() const` | Returns the number of elements |
| `void resize(uint)` | Sets the new length (adds default elements or truncates) |
| `void reverse()` | Reverses the order of elements in place |
| `void insertAt(uint index, const T &in value)` | Inserts a single element at the specified index |
| `void insertAt(uint index, const array<T> &in arr)` | Inserts all elements from another array at the specified index |
| `void insertLast(const T &in)` | Appends an element at the end |
| `void removeAt(uint index)` | Removes the element at the specified index |
| `void removeLast()` | Removes the last element |
| `void removeRange(uint start, uint count)` | Removes `count` elements starting from `start` |
| `void sortAsc()` | Sorts all elements in ascending order (uses `opCmp` for objects) |
| `void sortAsc(uint startAt, uint count)` | Sorts a sub-range ascending |
| `void sortDesc()` | Sorts all elements in descending order |
| `void sortDesc(uint startAt, uint count)` | Sorts a sub-range descending |
| `void sort(const less &in compareFunc, uint startAt = 0, uint count = uint(-1))` | Sorts using a custom comparison callback |
| `int find(const T &in)` | Returns the index of the first matching element, or -1 |
| `int find(uint startAt, const T &in)` | Searches starting from a given index |
| `int findByRef(const T &in)` | Finds by address (for handle arrays: finds the exact instance) |
| `int findByRef(uint startAt, const T &in)` | Searches by reference starting from a given index |

### Custom sort callbacks

The `sort` method accepts a callback function that takes two const references of the element type and returns `bool` (true if the first argument should come before the second):

```angelscript
// Using an anonymous function
array<int> arr = {3, 2, 1};
arr.sort(function(a, b) { return a < b; });

// Using a named function
bool lessForInt(const int &in a, const int &in b) {
    return a < b;
}
arr.sort(lessForInt);

// For handle arrays
bool lessForHandle(const obj @&in a, const obj @&in b) {
    return a < b;
}
array<obj@> objs;
objs.sort(lessForHandle);
```

### Multidimensional arrays

Multidimensional arrays are implemented as arrays of arrays. Each sub-array is an independent array object:

```angelscript
array<array<int>> grid(10, array<int>(10));
grid[0][0] = 42;

// Sub-arrays can have different lengths (jagged arrays)
array<array<int>> jagged;
jagged.insertLast(array<int> = {1, 2, 3});
jagged.insertLast(array<int> = {4, 5});
```

### Array of handles

When an array stores handles, elements are assigned using handle assignment:

```angelscript
array<Foo@> arr(1);
@arr[0] = Foo();        // Handle assignment to array element

// find/findByRef behavior with handle arrays
// find() uses opEquals for comparison, skips null handles
// findByRef() compares addresses (finds exact instance)
```

## Examples

```angelscript
int main() {
    array<int> arr = {1, 2, 3};   // [1, 2, 3]
    arr.insertLast(0);             // [1, 2, 3, 0]
    arr.insertAt(2, 4);            // [1, 2, 4, 3, 0]
    arr.removeAt(1);               // [1, 4, 3, 0]

    arr.sortAsc();                 // [0, 1, 3, 4]

    int sum = 0;
    for (uint n = 0; n < arr.length(); n++)
        sum += arr[n];

    return sum;                    // 8
}

// Passing arrays efficiently
void processArray(array<int>@ arr) {
    // arr is a handle, no copy made
    for (uint i = 0; i < arr.length(); i++) {
        arr[i] *= 2;
    }
}

// 2D array example
array<array<int>> matrix = {{1, 2}, {3, 4}};
int topLeft = matrix[0][0];    // 1
int bottomRight = matrix[1][1]; // 4
```

## Compilation Notes

- **Memory layout:** The array object is heap-allocated and contains a reference count, the element type info, and a dynamically-sized buffer for the elements. The array variable on the stack is a pointer to this heap object. Element storage is a contiguous buffer for value types, or a buffer of pointers for handle types.
- **Stack behavior:** An `array<T>` variable occupies one pointer-sized slot on the stack. The compiler must emit `ADDREF` when new references are created and `RELEASE` when references are dropped.
- **Type considerations:**
  - The array is a template type parameterized by the element type `T`. The compiler must resolve the element type and register appropriate specializations.
  - The `[]` operator must perform a bounds check and raise an exception on out-of-range access. The bytecode should emit a bounds check before the element access.
  - For arrays of handles, element assignment must go through handle assignment semantics (addref new, release old).
  - Initialization lists (`{1,2,3}`) are compiled as: allocate array, then call `insertLast` or equivalent for each element. The compiler may optimize this into a bulk initialization.
- **Lifecycle:**
  - Construction: Allocate the array on the heap (refcount = 1). For sized constructors, allocate the internal buffer and default-construct or fill elements.
  - Copy (assignment `=`): Performs a shallow copy. For value-type elements, each element is copied. For handle-type elements, each handle is copied (addref each).
  - Destruction: When refcount reaches zero, destroy all elements (call destructors for value types, release for handles), free the internal buffer, free the array object.
- **Special cases:**
  - Anonymous arrays in expressions (`foo({1,2,3})`) create a temporary array that must be properly reference-counted and released after the function call.
  - The `sort` method with a callback involves calling a script function (or delegate) for comparisons, which requires the bytecode to set up function calls within the sort operation.
  - Multidimensional arrays are just nested array objects, each independently reference-counted. The compiler does not special-case them.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Named(Ident { name: "array", .. })` | Array type as a named type | Wraps `Ident` with `name = "array"` |
| `TypeExpr.template_args` | Element type `T` in `array<T>` | `&'ast [TypeExpr<'ast>]` -- contains the element type(s) |

**Notes:**
- `array<T>` is **not** a parser-level built-in. It is a runtime-registered template type provided by the host application. The parser sees it as `TypeBase::Named(Ident { name: "array", .. })` with `template_args` containing the element type.
- For example, `array<int>` is represented as `TypeExpr { base: TypeBase::Named("array"), template_args: &[TypeExpr { base: TypeBase::Primitive(PrimitiveType::Int), .. }], .. }`.
- `array<obj@>` (array of handles) nests a handle suffix inside the template argument: `template_args: &[TypeExpr { base: TypeBase::Named("obj"), suffixes: &[TypeSuffix::Handle { is_const: false }], .. }]`.
- `array<int>@` (handle to array) uses the outer `TypeExpr.suffixes` field: `suffixes: &[TypeSuffix::Handle { is_const: false }]`.
- Multidimensional arrays like `array<array<int>>` are nested `TypeExpr` nodes in `template_args`.
- The parser does not validate that `array` is a valid registered type or that the template argument count is correct; that is deferred to semantic analysis.

## Related Features

- [Objects (reference types)](./objects.md)
- [Handles (array of handles)](./handles.md)
- [Strings (arrays of characters)](./strings.md)
- [Dictionary (alternative container)](./dictionary.md)
- [Function pointers (sort callbacks)](./funcptr.md)
