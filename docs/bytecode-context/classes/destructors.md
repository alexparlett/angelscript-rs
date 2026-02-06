# Destructors

## Overview
A class destructor is a special method that is called when an object instance is being destroyed. Destructors provide a hook for explicit cleanup logic. In most cases, implementing a destructor is not necessary because AngelScript automatically frees resources held by an object (releasing handles, destroying member objects). However, when explicit cleanup is required (e.g., closing external resources, logging, or notifying other systems), a destructor can be declared.

## Syntax
```angelscript
class MyClass
{
    ~MyClass()
    {
        // Explicit cleanup code
    }
}
```

## Semantics
- The destructor is declared with the **same name as the class**, prefixed with `~`, and takes **no parameters**.
- A class may declare at most **one destructor**.
- It is **not mandatory** to declare a destructor. If none is declared, AngelScript automatically handles cleanup by releasing all handles and destroying all member objects when the instance is destroyed.
- The destructor is called when the object's reference count drops to zero or when the garbage collector reclaims a cyclic reference group.
- AngelScript calls the destructor **only once**, even if the object is "resurrected" during destructor execution by storing a new reference to it.
- The destructor **cannot be invoked directly** from script code. If direct cleanup is needed, implement a separate public method and call it explicitly.
- Due to automatic memory management with garbage collection, the **exact timing** of destructor invocation is not always predictable. Objects may live beyond the point where the last visible reference is released if they are part of a cycle being processed by the garbage collector.
- For derived classes, the **base class destructor is called automatically** after the derived class destructor completes. There is no need to manually invoke the base destructor.

## Examples
```angelscript
class FileWrapper
{
    int fileHandle;

    FileWrapper(string path)
    {
        fileHandle = openFile(path);
    }

    ~FileWrapper()
    {
        // Close the file when the object is destroyed
        closeFile(fileHandle);
    }
}

class Resource
{
    string name;

    Resource(string n) { name = n; }

    ~Resource()
    {
        log("Releasing resource: " + name);
    }

    // Public method for explicit cleanup if needed
    void Release()
    {
        log("Explicitly releasing resource: " + name);
        // Do cleanup
    }
}
```

```angelscript
// Destructor ordering with inheritance
class Base
{
    ~Base()
    {
        log("Base destructor");
    }
}

class Derived : Base
{
    ~Derived()
    {
        log("Derived destructor");
    }
    // Output when destroyed:
    //   "Derived destructor"
    //   "Base destructor"
}
```

## Compilation Notes
- **Destruction sequence:** When an object is destroyed, the bytecode execution follows this order:
  1. Call the derived class destructor body (if declared).
  2. Call the base class destructor body (if declared), automatically chained.
  3. Release all handle members (decrement reference counts, potentially triggering further destructions).
  4. Destroy all object-type members by calling their destructors.
  5. Free the memory occupied by the object.
- **Reference count mechanics:** The destructor is triggered when `Release()` is called on the internal reference count and it reaches zero. The compiler emits `Release` calls at scope exits, handle reassignments, and function returns.
- **Garbage collector interaction:** For objects involved in reference cycles, the garbage collector breaks cycles by forcing destruction. The destructor is still called exactly once in this scenario. The GC marks the object as "being destroyed" to prevent re-entry.
- **Resurrection prevention:** Even if the destructor body stores a new handle to `this`, the runtime proceeds with destruction after the destructor returns. The stored handle becomes a dangling reference.
- **Stack behavior:** The destructor takes the `this` pointer as its only implicit argument. It has no return value. The destructor is not a regular callable method; it is invoked only by the runtime's reference management system.
- **No explicit call:** The compiler rejects any attempt to call the destructor directly (e.g., `obj.~MyClass()` is not valid syntax in AngelScript scripts).
- **Implicit destructor:** When no destructor is declared, the compiler does not generate a destructor function. The runtime still performs the member cleanup steps (releasing handles, destroying objects) as part of the deallocation process.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionDecl` | Destructor (as a `ClassMember::Method`) | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `is_destructor: bool`, `span: Span` |

**Notes:**
- A destructor is identified by `FunctionDecl.is_destructor == true`. The `return_type` is `None` and `params` is empty.
- The `~` prefix in the source syntax (`~MyClass()`) is consumed by the parser and sets the `is_destructor` flag to `true`. The `name` field contains the class name without the `~` prefix.
- There is no separate AST node for destructors; they reuse `FunctionDecl` with `is_destructor: true`, nested inside `ClassMember::Method`.

## Related Features
- [Constructors](./constructors.md) - object creation and initialization
- [Inheritance](./inheritance.md) - destructor chaining in class hierarchies
- [Class Declarations](./class-declarations.md) - class structure and memory management model
- [Member Initialization](./member-initialization.md) - member lifecycle from initialization to destruction
