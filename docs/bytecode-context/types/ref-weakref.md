# ref and weakref Types

## Overview

The `ref` type is a generic (type-agnostic) object handle that can hold a reference to any reference type, regardless of the type hierarchy. It serves as a universal handle when the specific type is not known at compile time. The `weakref<T>` and `const_weakref<T>` types hold weak references to objects -- references that do not prevent the object from being destroyed when all strong references are released. Both `ref` and `weakref` are only available if the host application registers support for them.

## Syntax

### ref

```angelscript
// Declaration
ref@ r;

// Assign any reference type
class Car {}
class Banana {}

ref@ r = Car();
@r = Banana();          // Reassign to a completely unrelated type

// Cast to retrieve the actual type
Car@ c = cast<Car>(r);
if (c !is null) {
    // r refers to a Car
}

Banana@ b = cast<Banana>(r);
if (b !is null) {
    // r refers to a Banana
}

// Null check
if (r !is null) { }

// Pass as function parameter
void process(ref@ handle) {
    Car@ c = cast<Car>(handle);
    if (c !is null) { /* handle Car */ }
}
```

### weakref

```angelscript
// Create a weak reference
class MyClass {}
MyClass@ obj = MyClass();

weakref<MyClass> w(obj);                // Weak reference to mutable object
const_weakref<MyClass> cw(obj);         // Weak reference to const object

// Retrieve strong reference (returns null if object is dead)
MyClass@ strong = w.get();
const MyClass@ constStrong = cw.get();

// Implicit cast (equivalent to get())
MyClass@ strong2 = w;

// Assignment
weakref<MyClass> w2;
@w2 = @obj;             // Handle assignment: point to same object
w2 = w;                 // Value assignment: copy weakref

// Identity check
if (w is null) { }
if (w !is w2) { }
```

## Semantics

### ref type

The `ref` type is a completely generic handle. Unlike a typed handle (`obj@`), which can only reference objects of a specific type or its subtypes, `ref` can reference any reference type.

**Operators:**

| Operator | Description |
|----------|-------------|
| `@=` | Handle assignment: sets the `ref` to reference a specific object |
| `is`, `!is` | Identity comparison: compares the address of the referenced object |
| `cast<T>` | Dynamic cast: returns a typed handle if the object is of the requested type, otherwise null |

**Key behavior:**
- `ref` increments the reference count of the object it holds (strong reference).
- Assigning a new object releases the old one and acquires the new one.
- The `cast<T>` operator performs a runtime type check. If the object's actual type is `T` or a subtype of `T`, the cast succeeds. Otherwise, it returns null.
- `ref` cannot be used to hold primitives or value types.

### weakref type

A `weakref<T>` holds a reference to an object without preventing its destruction. When all strong references (`T@` handles) to an object are released, the object is destroyed. After destruction, the `weakref` returns null when queried.

**Constructors:**

| Constructor | Description |
|-------------|-------------|
| `weakref<T>()` | Default: creates a null weak reference |
| `weakref<T>(T@)` | Explicit: creates a weak reference to the given object |
| `const_weakref<T>()` | Default: creates a null const weak reference |
| `const_weakref<T>(const T@)` | Explicit: creates a const weak reference |

**Operators:**

| Operator | Description |
|----------|-------------|
| `@=` | Handle assignment: sets the weak reference target |
| `=` | Value assignment: copies one weakref to another |
| `is`, `!is` | Identity comparison: compares the address of the referenced object |
| `cast<T>` (implicit) | Implicit cast to strong reference; returns null if the object is dead |

**Methods:**

| Method | Description |
|--------|-------------|
| `T@ get() const` | Returns a strong handle to the object, or null if the object has been destroyed |

**Key behavior:**
- A weak reference does **not** increment the reference count. The object can be destroyed while weak references to it exist.
- After the object is destroyed, `get()` and the implicit cast operator return null.
- `const_weakref<T>` returns `const T@` from `get()`, ensuring the object cannot be modified through the weak reference.
- Weak references are useful for breaking circular reference chains and for observer patterns.

## Examples

```angelscript
// ref example: type-agnostic container
class Car {
    void drive() { print("Driving\n"); }
}
class Banana {
    void peel() { print("Peeling\n"); }
}

void handle(ref@ r) {
    Car@ c = cast<Car>(r);
    if (c !is null) {
        c.drive();
        return;
    }
    Banana@ b = cast<Banana>(r);
    if (b !is null) {
        b.peel();
        return;
    }
    if (r is null) {
        print("Null handle\n");
    } else {
        print("Unknown type\n");
    }
}

void main() {
    ref@ r = Car();
    handle(r);              // "Driving"
    @r = Banana();
    handle(r);              // "Peeling"
}

// weakref example: observer pattern
class Entity {
    string name;
    Entity(const string &in n) { name = n; }
}

void main() {
    Entity@ e = Entity("Player");
    weakref<Entity> observer(e);

    // Object is still alive
    Entity@ strong = observer.get();
    assert(strong !is null);
    print(strong.name + "\n");   // "Player"

    // Release all strong references
    @e = null;
    @strong = null;

    // Object is now dead
    Entity@ gone = observer.get();
    assert(gone is null);        // Confirmed dead
}

// const_weakref example
class Config {
    int value = 42;
}

void main() {
    Config@ cfg = Config();
    const_weakref<Config> w(cfg);

    const Config@ reader = w.get();
    if (reader !is null) {
        print("Value: " + reader.value + "\n");
        // reader.value = 0;  // ERROR: const reference
    }
}
```

## Compilation Notes

- **Memory layout:**
  - `ref`: Stored as a pointer-sized value on the stack, pointing to the referenced object. Additionally stores (or looks up) runtime type information to enable `cast<T>` operations.
  - `weakref<T>`: Stored as a pointer to a weak-reference control block (or equivalent mechanism). The control block tracks whether the target object is still alive and provides the pointer to it if so.
- **Stack behavior:** Both `ref` and `weakref` occupy one pointer-sized slot on the stack. `ref` behaves like a normal handle for refcounting. `weakref` does not affect the target's reference count.
- **Type considerations:**
  - `ref` requires runtime type information (RTTI) for the `cast<T>` operator. The compiler must emit a dynamic type check instruction that consults the object's actual type against the requested type `T`.
  - `weakref<T>` is a template type. The compiler must resolve `T` and ensure it is a valid reference type. The `get()` method returns `T@` (or `const T@` for `const_weakref`).
  - Implicit cast from `weakref<T>` to `T@` involves the same logic as `get()`: check if the object is alive, if so return a strong handle (which increments the refcount), otherwise return null.
- **Lifecycle:**
  - `ref` creation: Store the object pointer and addref.
  - `ref` reassignment: Release old, store new, addref new.
  - `ref` destruction (scope exit): Release the held reference.
  - `weakref` creation: Register the weak reference with the object's control block (or weak-ref tracking mechanism). No addref on the object.
  - `weakref` get(): Atomically check if the object is alive. If so, addref and return the pointer. If not, return null.
  - `weakref` destruction: Unregister from the control block.
- **Special cases:**
  - `cast<T>` on `ref` must work across unrelated type hierarchies (no inheritance relationship required). This requires a full RTTI lookup, not just a vtable check.
  - `weakref` must handle the race condition (in concurrent scenarios) where the object is being destroyed while `get()` is called. The control block provides the synchronization mechanism.
  - `const_weakref<T>` differs from `weakref<T>` only in the const qualification of the returned handle. The underlying mechanism is identical.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/types.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `TypeBase::Named(Ident { name: "ref", .. })` | Generic `ref` type as a named type | Wraps `Ident` with `name = "ref"` |
| `TypeBase::Named(Ident { name: "weakref", .. })` | Weak reference template type | Wraps `Ident` with `name = "weakref"` |
| `TypeBase::Named(Ident { name: "const_weakref", .. })` | Const weak reference template type | Wraps `Ident` with `name = "const_weakref"` |
| `TypeExpr.template_args` | Type parameter `T` in `weakref<T>` / `const_weakref<T>` | `&'ast [TypeExpr<'ast>]` -- contains the target type |

**Notes:**
- `ref`, `weakref<T>`, and `const_weakref<T>` are **not** parser-level built-ins. They are runtime-registered types provided by the host application. The parser sees them as `TypeBase::Named(...)`.
- `ref@` is represented as `TypeExpr { base: TypeBase::Named("ref"), suffixes: &[TypeSuffix::Handle { is_const: false }], .. }`.
- `weakref<MyClass>` is represented as `TypeExpr { base: TypeBase::Named("weakref"), template_args: &[TypeExpr { base: TypeBase::Named("MyClass"), .. }], .. }`.
- `const_weakref<MyClass>` follows the same pattern with `name = "const_weakref"`.
- The parser does not distinguish between `ref`, `weakref`, `const_weakref`, and any other named type; all validation is deferred to semantic analysis.

## Related Features

- [Object handles (strong references)](./handles.md)
- [Objects (reference types)](./objects.md)
- [Function pointers (funcdef handles)](./funcptr.md)
