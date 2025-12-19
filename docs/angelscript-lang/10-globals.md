# Global Entities

All global declarations share the same namespace. Declarations are visible to all code regardless of order.

## Global Variables

```angelscript
int myGlobalVar = 42;
const float PI = 3.14159f;
```

## Global Virtual Properties

```angelscript
int globalProp {
    get { return _value; }
    set { _value = value; }
}
private int _value;
```

Same syntax as class property accessors, but at global scope.

## Interfaces

Interfaces define a contract that classes must implement. Cannot be instantiated directly.

```angelscript
interface IDrawable {
    void Draw();
    int GetLayer();
}

class Sprite : IDrawable {
    void Draw() { /* ... */ }
    int GetLayer() { return layer; }
    int layer;
}
```

- A class can implement **multiple interfaces** (comma-separated)
- Handles to interfaces allow polymorphic code

```angelscript
class MyClass : InterfaceA, InterfaceB {
    // Must implement all methods from both interfaces
}
```

### Interface Property Declarations

```angelscript
interface IProperty {
    int value { get const; set; }  // Just declarations
}
```

## Enums

Named integer constants:

```angelscript
enum MyEnum {
    eValue0,           // 0
    eValue2 = 2,       // 2
    eValue3,           // 3 (previous + 1)
    eValue200 = eValue2 * 100  // Expressions allowed
}
```

**Rules:**
- First value is 0 unless specified
- Subsequent values are previous + 1 unless specified
- **Cannot rely on enum variables only containing declared values** - always handle unexpected values

## Funcdefs (Function Definitions)

Define function pointer types:

```angelscript
funcdef bool CALLBACK(int, int);

void ProcessWithCallback(CALLBACK@ cb) {
    bool result = cb(10, 20);
}

// Usage with function
bool MyCallback(int a, int b) { return a < b; }
CALLBACK@ cb = @MyCallback;

// Usage with lambda
CALLBACK@ cb2 = function(int a, int b) { return a > b; };
```

See also: [Function handles](doc_datatypes_funcptr.html)

## Typedefs

Alias for an existing type:

```angelscript
typedef float real;
real x = 1.5f;
```

## Namespaces

Organize code into logical units:

```angelscript
namespace A {
    void function() { variable++; }
    int variable;
}

namespace B {
    // Same names allowed in different namespaces
    void function() { A::function(); }  // Use :: to access other namespace
}
```

### Nested Namespaces

```angelscript
int var;

namespace Parent {
    int var;
    namespace Child {
        int var;
        void func() {
            var = Parent::var;       // Access parent namespace
            Parent::var = ::var;     // :: alone = global scope
        }
    }
}

void func() {
    int v = Parent::Child::var;      // Fully qualified access
}
```

### Scope Resolution Rules

- Entities in same namespace see each other normally
- Use `Namespace::name` to access entities in other namespaces
- Use `::name` to access global scope from any namespace
- Parent namespace entities visible unless shadowed by child

## Mixin Classes

Provide code reuse without inheritance (since AngelScript only supports single inheritance):

```angelscript
mixin class MyMixin {
    void SomeMethod() { property++; }
    int property;
}

class MyClass : MyMixin {
    int OtherMethod() {
        SomeMethod();      // From mixin
        return property;   // From mixin
    }
}
```

### Mixin Behavior

- Mixin properties and methods are **copied** into the including class
- Already-declared members are **not** overwritten (class can override mixin defaults)
- Mixin methods are compiled in the context of the including class
- Mixin methods can reference properties not in the mixin (provided by including class)

### Mixin with Inheritance

```angelscript
class MyBase {
    void MethodA() { print("Base"); }
    int property;
}

mixin class MyMixin {
    void MethodA() { print("Mixin"); }
    float property;  // Different type!
}

// Inherits property from base (type preserved)
// Gets method from mixin (overrides base)
class MyClass : MyBase, MyMixin {
}
```

**Priority:**
- Mixin **methods** override inherited base class methods
- Mixin **properties** are NOT included if already inherited from base

### Mixin with Interfaces

```angelscript
interface I {
    void a();
    void b();
}

mixin class M : I {
    void a() { print("default a"); }  // Provide default
    // b() left for including class
}

class C : M {
    void b() { print("custom b"); }  // Must implement
    // a() comes from mixin
}
```

### Mixin Limitations

- Cannot be instantiated
- Cannot inherit from other classes
- Can list interfaces that including class must implement

## Imports

Import entities from other modules (application-defined):

```angelscript
import void ExternalFunc(int) from "othermodule";
```

Depends on how the host application configures module loading.
