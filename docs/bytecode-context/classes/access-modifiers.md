# Access Modifiers

## Overview
AngelScript provides `protected` and `private` access modifiers to control where class members (properties and methods) can be accessed from. Members without an access modifier are public by default. Access control is a compile-time mechanism that prevents unintended use of internal implementation details, making large codebases easier to manage and less error-prone.

## Syntax
```angelscript
class MyClass
{
    // Public members (default - no modifier)
    void PublicMethod() {}
    int publicProp;

    // Protected members - accessible from this class and derived classes
    protected void ProtectedMethod() {}
    protected int protectedProp;

    // Private members - accessible only from this class
    private void PrivateMethod() {}
    private int privateProp;
}
```

## Semantics
- **Public** (default): Members without an access modifier are public. They can be accessed from anywhere: within the class, from derived classes, and from external code.
- **Protected**: Members marked with `protected` can be accessed:
  - From within the class that declares them.
  - From within any class that derives from the declaring class.
  - They **cannot** be accessed from outside the class hierarchy (e.g., from global functions or unrelated classes).
- **Private**: Members marked with `private` can be accessed:
  - **Only** from within the class that declares them.
  - They **cannot** be accessed from derived classes.
  - They **cannot** be accessed from outside the class.
- Access modifiers apply to both **properties** and **methods**, including constructors.
- Access modifiers are specified as a **prefix keyword** before the member declaration. Each member must have its own access modifier; there are no access modifier blocks (unlike C++ sections).
- Access control is enforced **entirely at compile time**. There is no runtime overhead for access checking.
- There is no `friend` mechanism in AngelScript. Access boundaries are strictly defined by the class hierarchy.
- Private members of a base class are still present in derived class instances (they occupy memory in the object layout), but they cannot be referenced by name in derived class code.

## Examples
```angelscript
class MyBase
{
    // Public interface
    void PublicFunc()
    {
        // The class can access its own protected and private members
        ProtectedProp = 0;   // OK
        ProtectedFunc();     // OK
        PrivateProp = 0;     // OK
        PrivateFunc();       // OK
    }

    int PublicProp;

    // Protected members
    protected void ProtectedFunc() {}
    protected int ProtectedProp;

    // Private members
    private void PrivateFunc() {}
    private int PrivateProp;
}

class MyDerived : MyBase
{
    void Func()
    {
        // Derived class CAN access protected members of the base
        ProtectedProp = 1;   // OK
        ProtectedFunc();     // OK

        // Derived class CANNOT access private members of the base
        // PrivateProp = 1;  // Compiler error
        // PrivateFunc();    // Compiler error
    }
}

void GlobalFunc()
{
    MyBase obj;

    // External code can only access public members
    obj.PublicProp = 0;      // OK
    obj.PublicFunc();        // OK

    // Protected and private members are inaccessible
    // obj.ProtectedProp = 0;  // Compiler error
    // obj.ProtectedFunc();    // Compiler error
    // obj.PrivateProp = 0;    // Compiler error
    // obj.PrivateFunc();      // Compiler error
}
```

```angelscript
// Common pattern: private backing field with public property accessor
class Account
{
    private float _balance = 0;

    float balance
    {
        get const { return _balance; }
    }

    void Deposit(float amount)
    {
        if (amount > 0)
            _balance += amount;
    }

    void Withdraw(float amount)
    {
        if (amount > 0 && amount <= _balance)
            _balance -= amount;
    }
}
```

## Compilation Notes
- **Compile-time only:** Access modifiers produce no runtime bytecode or metadata checks. The compiler resolves access during name lookup and type checking. If access is denied, a compile error is emitted. At runtime, member access is by offset regardless of access level.
- **Object layout:** Access modifiers do not affect the physical layout of the object. Private, protected, and public members are all stored in the same contiguous memory block in declaration order. A derived class's memory layout includes all base class members (including private ones) followed by its own members.
- **Name lookup:** During compilation, when a member name is resolved, the compiler checks:
  1. Is the access from within the declaring class? If so, all access levels are permitted.
  2. Is the access from a derived class? If so, public and protected are permitted; private is rejected.
  3. Is the access from external code? If so, only public is permitted.
- **Vtable entries:** Private and protected virtual methods still occupy entries in the vtable. A derived class cannot override a private method (it cannot see it), but a protected virtual method can be overridden by a derived class. The vtable structure is the same regardless of access level.
- **Property accessors and access:** Property accessor methods (`get_`/`set_`) can be declared with access modifiers independently. For example, a public getter with a private setter creates a publicly readable but internally writable property.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/node.rs`, `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Visibility` | Access modifier enum | Variants: `Public` (default), `Private`, `Protected` |
| `FunctionDecl` | Method/constructor with visibility | `visibility: Visibility`, plus other fields |
| `FieldDecl` | Field with visibility | `visibility: Visibility`, plus other fields |
| `VirtualPropertyDecl` | Virtual property with visibility | `visibility: Visibility`, plus other fields |

**Notes:**
- `Visibility` defaults to `Public` (the `#[default]` derive). Members without an explicit access modifier are public.
- Each member carries its own `visibility` field. There are no "access modifier blocks" in the AST; every `FieldDecl`, `FunctionDecl` (as `ClassMember::Method`), and `VirtualPropertyDecl` independently stores its access level.
- Access control is enforced entirely at compile time during name resolution. The `Visibility` enum in the AST is metadata consumed by the compiler; it produces no runtime bytecode.

## Related Features
- [Class Declarations](./class-declarations.md) - class body structure
- [Methods](./methods.md) - method declarations that access modifiers apply to
- [Properties](./properties.md) - property accessors with access control
- [Inheritance](./inheritance.md) - how access modifiers interact with derived classes
