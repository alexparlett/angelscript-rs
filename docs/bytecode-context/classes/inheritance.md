# Inheritance and Polymorphism

## Overview
AngelScript supports single inheritance, where a derived class inherits properties and methods from one base class. Multiple inheritance is not supported, but polymorphism is achieved through interface implementation. All class methods are virtual by default, so overriding a base class method in a derived class automatically produces polymorphic behavior. The language provides `super` for calling base constructors, scope resolution (`Base::Method()`) for calling base method implementations, and the keywords `final`, `abstract`, and `override` for additional control.

## Syntax
```angelscript
// Basic inheritance
class Derived : Base
{
}

// Inheritance with interface implementation
class Derived : Base, ISerializable, IComparable
{
}

// Calling the base class constructor
class Derived : Base
{
    Derived()
    {
        super(10);  // Call base constructor with argument
    }
}

// Calling base class method implementation
class Derived : Base
{
    void Method() override
    {
        Base::Method();  // Call base implementation explicitly
    }
}

// Final class - cannot be inherited from
final class Sealed
{
}

// Abstract class - cannot be instantiated
abstract class AbstractBase
{
    void Method() {}  // All methods must have implementations
}

// Final method - cannot be overridden
class MyClass
{
    void Method() final {}
}

// Override decorator - compiler verifies base method exists
class Derived : Base
{
    void Method() override {}
}
```

## Semantics

### Single Inheritance
- A class can inherit from at most **one** base class.
- The derived class inherits all public and protected properties and methods from the base class.
- Private members of the base class are present in the derived object but are not accessible from derived class code.
- Multiple interfaces can be implemented alongside a single base class.

### Virtual Methods and Overriding
- **All methods are virtual** by default. There is no `virtual` keyword.
- When a derived class declares a method with the same name and parameter list as a base class method, it overrides the base implementation.
- The `override` decorator is optional but recommended. When present, the compiler verifies that a matching base class method exists. If not, a compile error is emitted.
- Overriding respects const qualification: a const method can only override a const method, and a non-const method can only override a non-const method.

### The `super` Keyword
- Used inside a derived class constructor to explicitly call a base class constructor.
- If `super(...)` is not called, the compiler automatically inserts a call to the base class **default constructor** at the beginning of the derived constructor.
- `super` can only be used in constructors, not in regular methods.

### Scope Resolution (`Base::Method()`)
- Calls the base class implementation of a method directly, bypassing virtual dispatch.
- Commonly used inside an overriding method to extend rather than replace the base behavior.

### Implicit and Explicit Casting
- A derived class reference can be **implicitly cast** to a base class reference (upcast). This is always safe.
- A base class reference must be **explicitly cast** to a derived class reference using `cast<Derived>(baseRef)` (downcast). This returns `null` at runtime if the object is not actually an instance of the target derived type.

### `final` Keyword
- On a **class**: Prevents any class from inheriting from it.
- On a **method**: Allows inheritance but prevents that specific method from being overridden in derived classes.

### `abstract` Keyword
- On a **class**: Prevents direct instantiation. The class can only be used as a base class.
- Individual methods **cannot** be marked abstract. All methods in an abstract class must have implementations.
- Derived classes of an abstract class can be instantiated (unless they are also abstract).

### Destructor Chaining
- The base class destructor is **automatically called** after the derived class destructor completes.
- There is no need (and no way) to manually invoke the base destructor.

## Examples
```angelscript
class Animal
{
    string name;

    Animal(string n) { name = n; }

    void Speak()
    {
        print(name + " makes a sound");
    }
}

class Dog : Animal
{
    Dog(string n)
    {
        super(n);  // Call Animal(string)
    }

    void Speak() override
    {
        print(name + " barks");
    }
}

class GuideDog : Dog
{
    string owner;

    GuideDog(string n, string o)
    {
        super(n);
        owner = o;
    }

    void Speak() override
    {
        Dog::Speak();  // Call Dog's implementation
        print("  (guide dog for " + owner + ")");
    }
}
```

```angelscript
// Polymorphism via base class handles
void MakeAnimalSpeak(Animal@ a)
{
    a.Speak();  // Virtual dispatch - calls the actual type's Speak()
}

void example()
{
    Animal@ a = Dog("Rex");
    MakeAnimalSpeak(a);  // prints "Rex barks"

    // Explicit downcast
    Dog@ d = cast<Dog>(a);
    if (d !is null)
        print("It is a dog!");
}
```

```angelscript
// Abstract base class pattern
abstract class Shape
{
    float Area() { return 0; }
    string GetName() { return "unknown"; }
}

class Circle : Shape
{
    float radius;
    Circle(float r) { radius = r; }
    float Area() override { return 3.14159f * radius * radius; }
    string GetName() override { return "circle"; }
}

class Square : Shape
{
    float side;
    Square(float s) { side = s; }
    float Area() override { return side * side; }
    string GetName() override { return "square"; }
}

// Shape s;  // Compiler error - abstract class
Circle c(5.0f);  // OK
```

```angelscript
// Final class and final methods
final class Immutable
{
    int value;
    Immutable(int v) { value = v; }
}

// class Derived : Immutable {}  // Compiler error - Immutable is final

class Base2
{
    void Locked() final { print("cannot override"); }
    void Open() { print("can override"); }
}

class Derived2 : Base2
{
    // void Locked() override {}  // Compiler error - method is final
    void Open() override { print("overridden"); }  // OK
}
```

## Compilation Notes
- **Vtable construction:** Each class has a virtual method table (vtable). The base class defines the initial vtable layout. A derived class copies the base vtable, then overwrites entries for methods it overrides and appends entries for new methods. The vtable index for each method is determined at compile time and remains stable across the hierarchy.
- **Virtual dispatch:** A method call on an object reference is compiled as: load the object pointer, look up the vtable from the object's type info, index into the vtable at the method's known slot, and call the function pointer found there. This indirection allows the correct overridden implementation to be called regardless of the static type of the reference.
- **Scope resolution bypass:** `Base::Method()` compiles to a direct function call to the base class's implementation, skipping the vtable lookup entirely. The compiler resolves the specific function at compile time and emits a direct call instruction.
- **super() compilation:** The `super(args)` call compiles to a direct call to the base class's matching constructor. If omitted, the compiler inserts a call to the base default constructor at the start of the constructor's bytecode, before any other initialization code (but after non-explicit member initialization; see [Member Initialization](./member-initialization.md)).
- **Upcasting:** Implicit upcasts require no runtime work beyond reference counting. A derived object pointer is directly usable as a base object pointer since the base members are at the beginning of the derived layout.
- **Downcasting:** `cast<Derived>(base)` compiles to a runtime type check. The runtime inspects the object's actual type info to determine if it is or inherits from `Derived`. If the check passes, the same pointer is returned (with an incremented reference count); if it fails, `null` is returned.
- **Final optimizations:** When the compiler knows the exact type of an object (e.g., the class is `final` or the method is `final`), it can bypass the vtable and emit a direct call. This eliminates the indirection overhead.
- **Abstract enforcement:** The compiler prevents calling constructors (factory functions) of abstract classes at compile time. No special runtime checks are needed.
- **Object layout with inheritance:** The derived class object layout is: [base class members] [derived class members]. The base class portion is identical in layout to a standalone base class instance, which is what makes pointer upcasting work without adjustment.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `ClassDecl` | Class with inheritance | `inheritance: &[IdentExpr]`, `modifiers: DeclModifiers`, plus other fields |
| `DeclModifiers` | Class modifiers for `final` and `abstract` | `abstract_: bool`, `final_: bool`, `shared: bool`, `external: bool` |
| `FuncAttr` | Method decorators for `override` and `final` | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |
| `IdentExpr` | Base class or interface name in inheritance list | Scoped name (supports `Namespace::Type`) |

**Notes:**
- `ClassDecl.inheritance` is a flat list of `IdentExpr` entries. The parser does not syntactically distinguish between a base class, an interface, and a mixin in this list. The compiler resolves each entry's role during type checking.
- `DeclModifiers.final_` on the class prevents inheritance; `DeclModifiers.abstract_` prevents instantiation.
- `FuncAttr.override_` on a method triggers a compile-time check that a matching base method exists. `FuncAttr.final_` on a method prevents it from being overridden.
- The `super` keyword and scope resolution calls (`Base::Method()`) are handled in the expression/statement AST, not in the declaration AST.

## Related Features
- [Class Declarations](./class-declarations.md) - class declaration syntax including `final` and `abstract`
- [Constructors](./constructors.md) - the `super` keyword and constructor chaining
- [Destructors](./destructors.md) - automatic destructor chaining
- [Methods](./methods.md) - virtual methods and const overloading
- [Access Modifiers](./access-modifiers.md) - protected and private member inheritance
- [Mixin Classes](./mixin-classes.md) - code reuse as an alternative to multiple inheritance
