# Script Classes

Script classes are **reference types** declared globally. They group properties and methods into logical units.

## Basic Syntax

```angelscript
class MyClass {
    // Properties
    int a;
    int b;

    // Methods
    void DoSomething() {}
}
```

## Memory Management

- Classes use **automatic memory management** (reference counting)
- Objects are destroyed when the last reference is released
- Multiple handles can reference the same object

## Constructors

Constructors have the **same name as the class** and **no return type**.

```angelscript
class MyClass {
    // Default constructor
    MyClass() {}

    // Copy constructor (enables optimized copies)
    MyClass(const MyClass &in other) {
        // Copy properties
    }

    // Parameterized constructors
    MyClass(int a, string b) {}
    MyClass(float x, float y, float z) {}

    // Conversion constructors
    MyClass(int a) {}                 // Allows implicit conversion from int
    MyClass(string a) explicit {}     // Only allows explicit conversion
}
```

### Constructor Rules

- If no constructors are declared, compiler provides a **default constructor**
- Default constructor calls member default constructors and sets handles to null
- Copy constructor enables more efficient object copies
- Single-argument constructors allow type conversions (unless `explicit`)
- Constructors **cannot call other constructors** - use a shared method instead

### Member Initialization

Members can be initialized at declaration:

```angelscript
class MyClass {
    int a = 10;              // Initialized in every constructor
    string name = "default";
}
```

## Destructor

```angelscript
class MyClass {
    ~MyClass() {
        // Cleanup code
    }
}
```

Called when object is destroyed. For derived classes, base destructor is called automatically after derived destructor.

## Methods

```angelscript
class MyClass {
    void Method() {}
    int Calculate(int x) { return x * 2; }

    // Const method - cannot modify object
    int GetValue() const { return value; }

    private int value;
}
```

**All methods are virtual** - no explicit `virtual` keyword needed.

## Inheritance

AngelScript supports **single inheritance** only. Multiple interfaces can be implemented.

```angelscript
class Derived : Base {
    Derived() {
        super(10);  // Call base constructor
    }

    // Override a method
    void DoSomething() override {
        Base::DoSomething();  // Call base implementation
        // Additional logic
    }
}
```

### super Keyword

Used to call base class constructor. If omitted, default constructor is called automatically.

### Scope Resolution (::)

Used to call base class methods: `Base::Method()`

### Casting

```angelscript
Base@ b = Derived();        // Implicit upcast - OK
Derived@ d = cast<Derived>(b);  // Explicit downcast - returns null if invalid
```

## Access Modifiers

| Modifier | Access |
|----------|--------|
| (default) | Public - accessible everywhere |
| `protected` | Class and derived classes only |
| `private` | Class only (not derived classes) |

```angelscript
class MyClass {
    void PublicMethod() {}
    int publicProp;

    protected void ProtectedMethod() {}
    protected int protectedProp;

    private void PrivateMethod() {}
    private int privateProp;
}
```

## Class Modifiers

### final

Prevents inheritance:

```angelscript
final class CannotInherit {}
```

Individual methods can also be `final`:

```angelscript
class MyClass {
    void Method() final {}  // Cannot be overridden
}
```

### abstract

Class cannot be instantiated, only derived from:

```angelscript
abstract class AbstractBase {
    // All methods must have implementations
    void Method() {}
}
```

**Note:** Individual methods cannot be marked abstract - all must have implementations.

### override

Explicitly marks a method as overriding a base method. Compiler error if no matching base method exists:

```angelscript
class Derived : Base {
    void Method() override {}       // OK if Base has Method()
    void Method(float) override {}  // Error if no matching Base method
}
```

## Property Accessors

Virtual properties using `get`/`set`:

```angelscript
class MyClass {
    int prop {
        get const { return realProp; }
        set { realProp = value; }  // 'value' is implicit parameter
    }
    private int realProp;
}
```

Equivalent explicit syntax:

```angelscript
class MyClass {
    int get_prop() const property { return realProp; }
    void set_prop(int value) property { realProp = value; }
    private int realProp;
}
```

### Interface Property Declarations

```angelscript
interface IMyInterface {
    int prop { get const; set; }  // Just declarations
}
```

### Read-Only / Write-Only

- Omit `set` for read-only property
- Omit `get` for write-only property

### Indexed Property Accessors

```angelscript
class MyClass {
    int get_items(int idx) const property { return arr[idx]; }
    void set_items(int idx, int value) property { arr[idx] = value; }
    private array<int> arr;
}

// Usage:
obj.items[0] = 10;
int x = obj.items[0];
```

### Property Accessor Limitations

- Compound assignment (`+=`, etc.) only works on reference types or global properties
- Increment/decrement (`++`/`--`) not supported - use `+= 1` instead

## Operator Overloads

See [06-operator-overloads.md](06-operator-overloads.md) for full details.

Classes can implement special methods for operator overloading:

```angelscript
class Vector {
    float x, y;

    Vector opAdd(const Vector &in other) const {
        Vector result;
        result.x = x + other.x;
        result.y = y + other.y;
        return result;
    }
}
```

## Default Behaviors

- **Default assignment** (`opAssign`): Bitwise copy of all members
- **Default constructor** (if none declared): Calls member default constructors, sets handles to null
