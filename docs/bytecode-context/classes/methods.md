# Methods

## Overview
Class methods are functions defined within a class body that operate on the class instance. They can access the instance's properties directly and are used to encapsulate behavior with the class's data. All class methods in AngelScript are virtual by default, meaning they can be overridden by derived classes without any special keyword. Methods support const overloading, where different implementations can be selected based on whether the object reference is read-only.

## Syntax
```angelscript
class MyClass
{
    // Basic method
    void DoSomething() {}

    // Method with parameters and return value
    int Calculate(int x, int y) { return x + y; }

    // Const method - cannot modify the object
    int GetValue() const { return value; }

    // Non-const and const overloads of the same method
    int Method()       { count++; return count; }
    int Method() const {          return count; }

    // Method with the 'final' decorator - cannot be overridden
    void Locked() final {}

    // Method with the 'override' decorator - must override a base method
    void BaseMethod() override {}

    int value;
    int count;
}
```

## Semantics
- Methods are declared inside the class body with a return type, name, parameter list, and optional decorators.
- Methods can access class properties **directly by name**. When a local variable shadows a class property, the property must be accessed via the `this` keyword (e.g., `this.name`).
- The implicit `this` reference is always available inside a method body and refers to the current object instance.
- **All methods are virtual.** There is no `virtual` keyword. Any method can be overridden by a derived class.
- **Const methods** are declared by adding `const` after the parameter list. A const method promises not to modify the object's state:
  - When an object is accessed through a `const` handle or reference, only const methods can be called.
  - When accessed through a non-const reference, both const and non-const overloads are available, with the non-const version preferred.
- **Const overloading** is a form of function overloading unique to class methods. Two methods can have identical names and parameters but differ in their const qualification. The compiler selects the appropriate overload based on the constness of the object reference.
- The `final` decorator on a method prevents derived classes from overriding it. The class itself can still be inherited from (unlike `final` on the class).
- The `override` decorator tells the compiler that the method is intended to override a base class method. If no matching base method exists, the compiler emits an error.
- Methods may call other methods of the same class, including inherited methods. Base class methods can be called explicitly using scope resolution: `BaseClass::MethodName()`.

## Examples
```angelscript
class MyClass
{
    void DoSomething()
    {
        // Direct access to class property
        a *= 2;

        // Local variable shadows class property 'b'
        int b = 42;

        // Must use 'this' to access the shadowed property
        this.b = b;
    }

    int a;
    int b;
}
```

```angelscript
class Counter
{
    int count;

    // Non-const: modifies state and returns
    int Next()       { count++; return count; }

    // Const: read-only access
    int Current() const { return count; }
}

void example()
{
    Counter c;
    c.Next();           // calls non-const Next()

    const Counter@ h = c;
    int val = h.Current();  // OK - const method on const handle
    // h.Next();            // Error - non-const method on const handle
}
```

```angelscript
class Base
{
    void Method() { print("Base"); }
    void Method(int x) { print("Base int"); }
}

class Derived : Base
{
    // Override the no-arg version
    void Method() override
    {
        Base::Method();  // call base implementation
        print("Derived");
    }

    // This would cause a compiler error:
    // void Method(float x) override {}  // no matching base method
}
```

## Compilation Notes
- **Virtual dispatch:** Since all methods are virtual, method calls on object references go through a vtable (virtual method table). Each class has a vtable containing function pointers for all its methods. When a derived class overrides a method, its vtable entry points to the new implementation.
- **this pointer passing:** Every method call implicitly passes the object pointer as the first argument. For `obj.Method(x)`, the compiled call pushes `obj` (the this pointer) onto the stack first, then `x`, then invokes the method.
- **Const method enforcement:** The compiler tracks whether the `this` reference is const inside a const method. Any attempt to modify a member or call a non-const method on `this` within a const method body generates a compile error. At runtime, there is no difference in how const and non-const methods execute; the enforcement is purely compile-time.
- **Const overload selection:** During overload resolution, the compiler checks the constness of the object expression. If the expression is const, only const overloads are candidates. If non-const, both are candidates but non-const is preferred. This is resolved entirely at compile time.
- **Final method optimization:** When a method is marked `final`, the compiler may optimize calls to that method by using direct dispatch instead of vtable lookup, since no derived class can override it.
- **Override checking:** The `override` decorator triggers a compile-time check that a matching method signature exists in the base class. It generates no additional runtime code.
- **Scope resolution calls:** `Base::Method()` bypasses virtual dispatch and calls the base class implementation directly. The compiler emits a direct call instruction to the specific base class function rather than going through the vtable.
- **Stack frame:** Method calls create a new stack frame containing: the this pointer, the method parameters, and local variables. The return value is placed on the caller's stack or in a designated return register.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `ClassMember::Method` | Method within a class | Contains a `FunctionDecl` |
| `FunctionDecl` | Method declaration | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `is_destructor: bool`, `span: Span` |
| `FuncAttr` | Method attributes | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |

**Notes:**
- `FunctionDecl.is_const` maps to the `const` keyword after the parameter list, marking a const method.
- `FuncAttr.override_` maps to the `override` decorator; `FuncAttr.final_` maps to the `final` decorator on methods.
- `FuncAttr.delete` represents a deleted method. This attribute is supported in the AST but is not explicitly documented in this file.
- `FuncAttr.property` marks a method as a property accessor (see [Properties](./properties.md)).
- Const overloading is represented by two `FunctionDecl` nodes with the same name and parameters, differing in `is_const`.

## Related Features
- [Class Declarations](./class-declarations.md) - class structure and method declarations
- [Inheritance](./inheritance.md) - method overriding and the `super` keyword
- [Access Modifiers](./access-modifiers.md) - controlling method visibility
- [Properties](./properties.md) - property accessors as special methods
- [Operator Overloads](./operator-overloads.md) - operator methods
