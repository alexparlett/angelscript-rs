# Object Handles

## Overview

Object handles (`@`) are a type modifier that allows a variable to hold a reference to an object rather than owning it by value. Handles enable multiple variables to refer to the same physical object, support polymorphism through interfaces and inheritance, and extend object lifetimes beyond their original scope via reference counting. Not all types support handles -- primitives never do, and application-registered types may opt out.

## Syntax

```angelscript
// Handle declaration (initialized to null)
obj@ a;

// Handle declaration with initialization
obj@ b = @someObj;

// Handle assignment (change what the handle points to)
@a = @b;
@a = @someObj;
@a = null;              // Clear the handle

// Value assignment through handle (operates on the referenced object)
a = someObj;            // Copies value into the object a references

// Null check with identity operators
if (a is null) { }
if (a !is null) { }

// Identity comparison (same object?)
if (a is b) { }
if (a !is b) { }

// Value comparison through handles (calls opEquals/opCmp)
if (a == b) { }
if (a != b) { }

// Handle-level equality (equivalent to is/!is)
if (@a == @b) { }
if (@a != @b) { }

// Member access through handle (same as direct object access)
a.Method();
a.property = 42;

// Const handles
const obj@ c;                   // Handle to non-modifiable object
obj@ const d = obj();           // Read-only handle (cannot reassign)
const obj@ const e = obj();     // Both handle and object are const

// Auto handles
auto a = getObject();           // auto resolves to obj@ for reference types
auto@ b = getObject();          // Explicit handle syntax (same result)

// Handle + array combinations
array<obj@> handleArray;        // Array of handles
array<obj>@ arrayHandle;        // Handle to an array
array<obj@>@ both;              // Handle to array of handles

// Polymorphic handles
interface I {}
class A : I {}
I@ i = A();                     // Interface handle holds derived type
A@ a = cast<A>(i);              // Downcast with cast<>
```

## Semantics

### Handle vs value operations

The `@` prefix controls whether an operation targets the handle itself or the referenced object:

| Expression | Target | Effect |
|-----------|--------|--------|
| `@a = @b` | Handle | `a` now points to the same object as `b`; refcounts updated |
| `a = b` | Object | Copies the value of `b` into the object `a` references |
| `@a = null` | Handle | `a` is cleared; referenced object's refcount decremented |
| `a.Method()` | Object | Calls method on the referenced object |

The compiler can often determine implicitly whether the handle or the object is intended. For example, in `obj@ a = someObj;` the compiler knows a handle assignment is needed. Explicit `@` is always valid and serves as documentation.

### Null handles

- A handle declared without initialization is `null`.
- Accessing members or calling methods on a null handle raises a **script exception**.
- Use `is null` / `!is null` to check before access.

### Identity vs equality

| Operator | Meaning | Behavior |
|----------|---------|----------|
| `is` | Identity | Compares the pointer addresses (same object?) |
| `!is` | Non-identity | Inverse of `is` |
| `==` | Value equality | Calls `opEquals` on the referenced objects |
| `!=` | Value inequality | Inverse of `==` |
| `@a == @b` | Identity (alternate) | Same as `a is b` when both sides are explicitly handles |

### Const handles

Two independent `const` axes:

1. **Handle to const object** (`const obj@ c`): The object cannot be modified through this handle. The handle itself can be reassigned to point to a different object. Can refer to both const and non-const objects.
2. **Const handle** (`obj@ const d`): The handle cannot be reassigned after initialization. The referenced object can still be modified (unless also const).
3. **Both** (`const obj@ const e`): Neither the handle nor the object can be modified.

A read-only handle (`@ const`) can only be initialized at the point of declaration.

### Auto handles

When `auto` is used to declare a variable that receives a reference type, the type resolves to a handle rather than a value:

```angelscript
auto a = getObject();   // a is typed as obj@, not obj
auto@ b = getObject();  // Explicit handle syntax, same result
```

This is because handle assignment is more efficient than value copy for reference types.

### Reference counting and object lifetimes

- Creating a handle to an object increments its reference count.
- Clearing a handle (`@h = null`) or letting it go out of scope decrements the reference count.
- The object is destroyed when its reference count reaches zero.
- An object can outlive its original scope if handles outside that scope still reference it.

```angelscript
obj@ h;
{
    obj o;
    @h = @o;    // o's refcount is now 2 (variable + handle)
}               // o goes out of scope, refcount drops to 1
h.Method();     // Object still alive via h
@h = null;      // Refcount drops to 0, object destroyed
```

### Polymorphism

Handles to base classes or interfaces can hold references to derived types:

```angelscript
interface I {}
class A : I {}
class B : I {}

I@ i1 = A();    // Interface handle holds A instance
I@ i2 = B();    // Interface handle holds B instance

// Downcast to specific type
A@ a = cast<A>(i1);    // Succeeds, returns handle to A
A@ a2 = cast<A>(i2);   // Fails, returns null
```

### Restrictions

- **Primitives** (`bool`, `int`, `float`, etc.) cannot have handles.
- Some application-registered object types may not support handles.
- Handle semantics do not apply to value types unless the application explicitly registers handle support.

## Examples

```angelscript
// Basic handle usage
class Entity {
    string name;
    void greet() { print("Hello from " + name + "\n"); }
}

Entity e;
e.name = "World";

Entity@ h1 = @e;
Entity@ h2 = @e;

h1.name = "Modified";
h2.greet();             // Prints "Hello from Modified"

// h1 and h2 point to the same object
if (h1 is h2) {
    print("Same object\n");   // This prints
}

// Null safety
Entity@ maybe = null;
if (maybe !is null) {
    maybe.greet();      // Skipped
}

// Const handle
const Entity@ reader = @e;
// reader.name = "X";  // ERROR: cannot modify through const handle
print(reader.name);     // OK: reading is fine

// Handle in function parameter
void process(Entity@ e) {
    if (e !is null) {
        e.greet();
    }
}
process(@e);
process(null);
```

## Compilation Notes

- **Memory layout:** A handle is stored as a single pointer-sized value on the stack (or within an object's field layout). It points to the heap-allocated object. Null is represented as a zero/null pointer.
- **Stack behavior:** Handles occupy one pointer-sized stack slot. When a handle variable goes out of scope, the compiler must emit a `RELEASE` (decref) instruction. When a handle is assigned, the compiler must emit `ADDREF` on the new target and `RELEASE` on the old target (if non-null).
- **Type considerations:**
  - Handle assignment (`@a = @b`) emits: load address of b's target, addref new, release old, store into a.
  - Value assignment (`a = b`) emits: load addresses of both targets, call the type's opAssign method.
  - Identity check (`is`/`!is`) emits: compare the two pointer values directly.
  - Null check: compare pointer against zero.
  - Downcast (`cast<T>`) requires runtime type information (RTTI) to verify type compatibility. Emit a type-check instruction followed by a conditional null or valid handle.
- **Lifecycle:**
  - Declaration without init: store null pointer.
  - Declaration with init: evaluate RHS, addref, store pointer.
  - Scope exit: release (decref). If refcount reaches zero, call destructor and free memory.
  - Handle reassignment: release old, addref new, store new pointer.
- **Special cases:**
  - Calling methods or accessing properties on a null handle must emit a null-check guard that raises an exception.
  - `const` handle constraints are enforced at compile time only; no runtime cost.
  - Read-only handles (`@ const`) must be verified at compile time to ensure no reassignment occurs after initialization.
  - When the compiler can prove a handle is non-null (e.g., immediately after construction), it may optimize away the null check.
  - Auto handles resolve the type at compile time during type inference; no special bytecode is needed.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeSuffix::Handle` | The `@` handle suffix on a type expression | `is_const: bool` -- `false` for `@`, `true` for `@ const` |
| `TypeExpr<'ast>` | Full type expression; handles appear in `suffixes` | `is_const: bool` (leading `const`), `suffixes: &'ast [TypeSuffix]` |

**Notes:**
- A handle type like `MyClass@` is represented as `TypeExpr { base: TypeBase::Named("MyClass"), suffixes: &[TypeSuffix::Handle { is_const: false }], ... }`.
- `const MyClass@` (handle to const object) sets `TypeExpr.is_const = true` with `TypeSuffix::Handle { is_const: false }`.
- `MyClass@ const` (const handle) sets `TypeExpr.is_const = false` with `TypeSuffix::Handle { is_const: true }`.
- `const MyClass@ const` (both const) sets `TypeExpr.is_const = true` with `TypeSuffix::Handle { is_const: true }`.
- `TypeExpr` provides helper methods `has_handle()` and `is_reference_type()` that check whether any suffix is `TypeSuffix::Handle`.
- The `@` prefix operator in expressions (handle-of, e.g., `@obj`) is an expression-level construct, not a type-level construct. It is not represented by `TypeSuffix::Handle`.
- Identity operators (`is`, `!is`) are expression operators, not part of the type AST.

## Related Features

- [Objects (value types vs reference types)](./objects.md)
- [ref and weakref](./ref-weakref.md)
- [Function pointers (funcdef handles)](./funcptr.md)
- [Auto declarations](./auto-declarations.md)
