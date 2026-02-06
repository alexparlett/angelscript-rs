# Class Declarations

## Overview
Script classes are reference types declared at global scope that group properties and methods into logical units. They provide the primary mechanism for user-defined structured data in AngelScript. Classes support constructors, destructors, methods, property accessors, operator overloading, single inheritance, interface implementation, and mixin inclusion.

## Syntax
```angelscript
// Basic class declaration
class MyClass
{
    int a;
    int b;
    void DoSomething() {}
}

// Class with inheritance
class Derived : Base
{
}

// Class implementing interfaces
class MyClass : ISerializable, IComparable
{
}

// Class with inheritance and interfaces
class MyClass : Base, ISerializable
{
}

// Class including a mixin
class MyClass : MyMixin
{
}

// Class with inheritance, mixin, and interfaces
class MyClass : Base, MyMixin, ISerializable
{
}

// Final class - cannot be inherited from
final class Sealed
{
}

// Abstract class - cannot be instantiated, only derived from
abstract class AbstractBase
{
}

// Forward declaration (external class, declared elsewhere)
external class ExternalClass;
```

## Semantics
- Script classes are always **reference types**. Multiple references or handles can point to the same object instance.
- Classes use **automatic memory management** with reference counting and garbage collection. Object instances are destroyed when the last reference is released.
- Classes are declared at **global scope** only. They cannot be nested inside functions or other classes.
- A class body may contain: member variable declarations, constructors, a destructor, methods, property accessors, and operator overload methods.
- The order of declarations within the class body does not matter for visibility; all members are visible to all methods regardless of declaration order.
- **Final classes** are marked with the `final` keyword before the class name. They cannot serve as base classes. This is enforced at compile time.
- **Abstract classes** are marked with the `abstract` keyword before the class name. They cannot be instantiated directly but can be derived from. Note that individual methods cannot be marked abstract; all methods in an abstract class must have implementations.
- A class can inherit from at most one other class (single inheritance), implement any number of interfaces, and include any number of mixin classes, all specified after the colon in the declaration.
- When the compiler does not find any explicitly declared constructor, it automatically provides a default constructor that initializes all object members via their default constructors and sets all handles to null.
- All script classes have a **default assignment operator** (`opAssign`) that performs a bitwise copy of the class contents, unless explicitly overridden.

## Examples
```angelscript
// A simple class with properties and a method
class Vector2
{
    float x;
    float y;

    float Length()
    {
        return sqrt(x * x + y * y);
    }
}

// Using an abstract base with a concrete derived class
abstract class Shape
{
    float Area() { return 0; }
}

class Circle : Shape
{
    float radius;

    Circle(float r) { radius = r; }

    float Area() override
    {
        return 3.14159f * radius * radius;
    }
}

// A final class that cannot be extended
final class Singleton
{
    private Singleton() {}
    int value;
}
```

## Compilation Notes
- **Object layout:** Each class instance is a heap-allocated block containing: a reference count field, a type info pointer (or vtable pointer), and the member variables in declaration order. The reference count is used for automatic memory management. Inherited members from the base class are laid out first, followed by the derived class's own members.
- **Type registration:** During compilation, each class declaration registers a new type in the type system. The type entry includes the class name, its base class (if any), implemented interfaces, the member list with offsets, and the method table.
- **Reference counting:** All class instances are reference-counted. The compiler emits `AddRef` and `Release` calls at handle assignments, function argument passing, and scope exits. When the reference count reaches zero, the destructor (if any) is called and the memory is freed.
- **Garbage collection:** For classes that can form circular references (e.g., a class holding a handle to its own type), the garbage collector supplements reference counting to detect and break cycles.
- **Final optimization:** When a class is marked `final`, the compiler knows that no derived class can override its methods. This allows the compiler to emit direct calls instead of virtual calls for methods on instances whose static type is the final class.
- **Abstract enforcement:** The compiler prevents instantiation of abstract classes by rejecting constructor calls at compile time. References and handles to abstract classes are still permitted.
- **Forward declarations:** External class declarations allow referencing a class type before its full definition is available, enabling cross-module references.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `ClassDecl` | Class declaration | `modifiers: DeclModifiers`, `name: Ident`, `template_params: &[Ident]`, `inheritance: &[IdentExpr]`, `members: &[ClassMember]`, `span: Span` |
| `DeclModifiers` | Class-level modifiers (`abstract`, `final`, `shared`, `external`) | `shared: bool`, `external: bool`, `abstract_: bool`, `final_: bool` |
| `ClassMember` | Member within a class body | Variants: `Method(FunctionDecl)`, `Field(FieldDecl)`, `VirtualProperty(VirtualPropertyDecl)`, `Funcdef(FuncdefDecl)` |
| `Ident` | Class name / identifier | `name: &str`, `span: Span` |
| `IdentExpr` | Base class or interface name in inheritance list | Scoped name supporting `Namespace::Type` syntax |

**Notes:**
- `ClassDecl.modifiers.abstract_` maps to the `abstract` keyword; `ClassDecl.modifiers.final_` maps to the `final` keyword.
- `ClassDecl.inheritance` is a flat list of `IdentExpr` entries. The parser does not distinguish between a base class, an interface, or a mixin in this list; that distinction is resolved semantically during compilation.
- `ClassDecl.template_params` is for application-registered template classes and is not part of script-level AngelScript syntax.
- `ClassMember::Funcdef` represents a nested funcdef declaration inside a class. See [Function References](../functions/function-references.md) for the `FuncdefDecl` type.
- Forward declarations (`external class Foo;`) produce a `ClassDecl` with `modifiers.external == true` and an empty `members` slice.

## Related Features
- [Constructors](./constructors.md) - constructor declarations and overloading
- [Destructors](./destructors.md) - destructor declarations and cleanup
- [Methods](./methods.md) - class method declarations
- [Properties](./properties.md) - member variables and property accessors
- [Access Modifiers](./access-modifiers.md) - private and protected members
- [Inheritance](./inheritance.md) - single inheritance and polymorphism
- [Mixin Classes](./mixin-classes.md) - code reuse through mixins
- [Operator Overloads](./operator-overloads.md) - operator method declarations
