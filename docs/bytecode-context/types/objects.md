# Objects

## Overview

AngelScript distinguishes between two fundamental kinds of object types: **value types** and **reference types**. Value types behave like primitives (stack-allocated, copied by value). Reference types are heap-allocated and managed via reference counting, potentially outliving the scope in which they were created. All script-declared classes are reference types. Only the host application can register value types.

## Syntax

```angelscript
// Value type usage (application-registered)
vec3 position;                  // Constructed on the stack
vec3 other = position;          // Copied by value
position = vec3(1.0f, 2.0f, 3.0f);  // Assign from temporary

// Reference type usage (script class or application-registered)
obj o;                          // Heap-allocated, reference counted
o = obj();                      // Temporary created, value assigned to o

// Passing by value vs reference
void byValue(vec3 v) { }       // Receives a copy (value type)
void byRef(vec3 &in v) { }     // Receives a const reference (no copy)
void byRefOut(vec3 &out v) { } // Output reference
void byRefInout(vec3 &inout v) { }  // Mutable reference

// Const objects
const obj co;                   // Const object -- cannot modify
void readOnly(const obj &in o) { }  // Const reference parameter
```

## Semantics

### Value types

- Allocated on the **stack** (or inline within other objects).
- Deallocated automatically when the variable goes out of scope.
- Assignment (`=`) performs a **deep copy** of the value.
- Only the application can register value types; scripts cannot declare them.
- Behave like primitives for parameter passing: by default, passed by value (copied).
- Can be passed by reference using `&in`, `&out`, or `&inout` modifiers.

### Reference types

- Allocated on the **memory heap**.
- Managed by **reference counting**. The object lives as long as at least one reference (variable or handle) exists.
- All **script-declared classes** are reference types.
- **Interfaces** are a special form of reference type that cannot be instantiated directly, but can be used to access objects that implement the interface.
- Assignment (`=`) on a reference-type variable calls the type's assignment operator (typically copies the value into the existing object, does NOT change which object the variable refers to).
- Creating a new instance: `obj o;` allocates on the heap and binds the variable to it.
- Temporary instances: `o = obj();` creates a temporary, copies its value into `o`, then destroys the temporary.

### Object lifecycle

1. **Construction:** Object is allocated and its constructor is called. For reference types, a reference count is initialized to 1.
2. **Copy:** Value assignment calls the copy constructor or `opAssign`. For reference types, this copies data but does not create a new object.
3. **Destruction:** When the last reference is released (variable goes out of scope, handle set to null), the destructor is called and memory is freed.

### Passing conventions

| Modifier | Behavior | Value types | Reference types |
|----------|----------|------------|----------------|
| (none) | By value | Copies the object | Passes reference (implicit) |
| `&in` | Input reference | No copy, read-only | No copy, read-only |
| `&out` | Output reference | Caller provides storage | Caller provides storage |
| `&inout` | Mutable reference | Caller's object is modified | Caller's object is modified |
| `const` | Immutable | Cannot modify through this reference | Cannot modify through this reference |

### Const objects

- A `const` qualifier prevents modification of the object's state.
- `const` methods can be called on const objects; non-const methods cannot.
- `const` references can refer to both const and non-const objects, but modification is forbidden through the const reference.

## Examples

```angelscript
// Value type behavior
vec3 a(1, 2, 3);
vec3 b = a;          // b is a copy of a
b.x = 10;           // Only b is modified; a is unchanged

// Reference type behavior
class MyObj {
    int value;
}

MyObj o1;
o1.value = 42;
MyObj o2;
o2 = o1;            // Copies value (o2.value is now 42)
o2.value = 100;     // Only o2 is modified; o1.value is still 42

// But with handles, both refer to same object
MyObj@ h = @o1;
h.value = 999;      // o1.value is now 999 too

// Interfaces
interface IDrawable {
    void draw();
}

class Circle : IDrawable {
    void draw() { /* ... */ }
}

IDrawable@ d = Circle();   // Interface handle to a Circle
d.draw();                   // Calls Circle::draw()
```

## Compilation Notes

- **Memory layout:** Value types are stored inline on the stack frame, with size known at compile time. Reference types are stored as a pointer on the stack pointing to a heap-allocated block. The heap block contains the reference count followed by the object's fields.
- **Stack behavior:** Value types occupy their full size on the stack. Reference types occupy one pointer-sized slot on the stack (the pointer to the heap object). When a reference-type variable goes out of scope, the compiler must emit a release/decref instruction.
- **Type considerations:** The compiler must track whether a type is a value type or reference type to emit correct construction, copy, and destruction bytecodes. Value types use stack-local operations; reference types use heap allocation and reference counting.
- **Lifecycle:**
  - *Value types:* Constructor called at declaration, destructor called at scope exit. Copy constructor called for by-value parameter passing and `=` assignment.
  - *Reference types:* `ALLOC` bytecode to allocate on heap. `ADDREF` when a new reference is created. `RELEASE` when a reference is dropped. Destructor called when refcount reaches zero.
  - *Temporaries:* For expressions like `o = obj()`, the compiler creates a temporary (alloc + construct), calls the assignment operator on the target, then releases the temporary.
- **Special cases:**
  - Interfaces require virtual dispatch through a function table. The compiler emits indirect calls via the interface's vtable offset.
  - Forward-declared types may not have their full layout known during compilation; the registry must resolve these before bytecode emission.
  - Const correctness must be checked at compile time; no runtime cost.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeExpr<'ast>` | Complete type expression for any type (primitives, objects, handles, templates) | `is_const: bool`, `scope: Option<Scope<'ast>>`, `base: TypeBase<'ast>`, `template_args: &'ast [TypeExpr<'ast>]`, `suffixes: &'ast [TypeSuffix]`, `span: Span` |
| `TypeBase::Named(Ident<'ast>)` | User-defined or application-registered type name | Wraps an `Ident` with `name: &'ast str` and `span: Span` |
| `Scope<'ast>` | Namespace qualification for scoped types (e.g., `Namespace::Type`) | `is_absolute: bool`, `segments: &'ast [Ident<'ast>]`, `span: Span` |
| `TypeSuffix::Handle` | Handle modifier (`@`) turning a type into a handle type | `is_const: bool` (trailing `const` on the handle) |
| `RefKind` | Reference-passing modifier for parameters | Variants: `None`, `Ref`, `RefIn`, `RefOut`, `RefInOut` |
| `ParamType<'ast>` | Parameter type with reference kind | `ty: TypeExpr<'ast>`, `ref_kind: RefKind`, `span: Span` |
| `ReturnType<'ast>` | Function return type with optional reference | `ty: TypeExpr<'ast>`, `is_ref: bool`, `span: Span` |

**Notes:**
- Object types (both value types and reference types) are represented as `TypeBase::Named(ident)` in the parser AST. The parser does not distinguish between value types and reference types -- that distinction is determined during semantic analysis by consulting the type registry.
- The `is_const` field on `TypeExpr` represents the leading `const` keyword (e.g., `const MyClass`), which makes the object immutable.
- The `scope` field enables namespace-qualified types like `Namespace::MyClass`.
- Passing conventions (`&in`, `&out`, `&inout`) are captured by `RefKind` on `ParamType`, not on `TypeExpr` itself.
- See also: [function-declarations.md](../functions/function-declarations.md) for `ReturnType` and `ParamType` usage in function signatures.

## Related Features

- [Primitive types](./primitives.md)
- [Object handles](./handles.md)
- [Strings (reference type)](./strings.md)
- [Arrays (reference type)](./arrays.md)
