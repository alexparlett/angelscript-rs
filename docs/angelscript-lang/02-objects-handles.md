# Objects and Handles

## Object Types

AngelScript has two kinds of object types:

### Value Types

- Allocated on the **stack**
- Deallocated when variable goes out of scope
- Only the application can register these types
- Behave like primitives (copied by value)

### Reference Types

- Allocated on the **heap**
- May outlive their declaring scope if references are kept
- All **script-declared classes are reference types**
- Interfaces are reference types that cannot be instantiated

```angelscript
obj o;       // Object instantiated
o = obj();   // Temporary created, value assigned to o
```

## Object Handles

Object handles hold **references** to objects. Multiple handles can reference the same object.

### Declaration

```angelscript
obj o;              // Object instance
obj@ a;             // Handle, initialized to null
obj@ b = @o;        // Handle referencing o
```

The `@` symbol declares a handle type.

### Usage

Handles work like the object itself for member access:

```angelscript
b.Method();         // Calls method on the referenced object
```

**Exception:** Accessing a null handle throws an exception.

### Handle Operations

Operators like `=` work on the **referenced object**:

```angelscript
obj@ h;
obj o;
h = o;              // Assigns value of o to object h references (exception if h is null!)
```

To operate on the **handle itself**, use `@`:

```angelscript
@h = @o;            // Make h reference the same object as o
@h = null;          // Clear the handle
```

The compiler often infers when you mean the handle vs the object, but explicit `@` is clearer.

### Identity vs Equality

| Operator | Meaning |
|----------|---------|
| `is` | Same object (address comparison) |
| `!is` | Different objects |
| `==` | Value equality (calls `opEquals`) |
| `!=` | Value inequality |

```angelscript
obj@ a, b;
if (a is b) { }       // Same object?
if (a !is null) { }   // Not null?
if (a == b) { }       // Equal values? (calls opEquals)
```

**Note:** `@a == @b` has the same meaning as `a is b`.

## Object Lifetimes

Objects normally live for their scope duration. But handles extend lifetime:

```angelscript
object@ h;
{
    object o;
    @h = @o;
    // o would normally die here, but h keeps it alive
}

h.Method();         // Object still alive via h

@h = null;          // Now object is destroyed
```

Object is destroyed when **all handles are released** (reference counting).

## Polymorphism

Handles enable polymorphic code through inheritance and interfaces:

```angelscript
interface I {}
class A : I {}
class B : I {}

I@ i1 = A();    // Handle to interface holds A
I@ i2 = B();    // Handle to interface holds B

void process(I@ i) {
    // Cast to check actual type
    A@ a = cast<A>(i);
    if (a !is null) {
        // It's an A
    }
}
```

## Const Handles

Two kinds of const:

### Handle to const object (prefix const)

```angelscript
const obj@ b;       // Can't modify the referenced object
```

- Can reference both const and non-const objects
- Cannot modify through this handle
- Cannot pass to a non-const handle

### Const handle (suffix const after @)

```angelscript
obj@ const c = obj();         // Handle itself is read-only
const obj@ const d = obj();   // Both handle and object are const
```

- Cannot reassign to different object
- Can only be initialized at declaration

## Not All Types Support Handles

- **Primitives** (bool, int, float, etc.) - never have handles
- **Value types** - depends on application registration
- **Reference types** - typically support handles

Check application documentation for which registered types support handles.

## Handle + Array Combinations

```angelscript
array<obj@>         // Array of handles
array<obj>@         // Handle to array
array<obj@>@        // Handle to array of handles
```
