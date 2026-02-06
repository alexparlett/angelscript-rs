# Constructors

## Overview
Class constructors are special methods used to create and initialize new instances of a class. Constructors are declared without a return type and must have the same name as the class. AngelScript supports default constructors, copy constructors, parameterized constructors, and conversion constructors. A class may declare multiple constructors with different parameter lists (overloading).

## Syntax
```angelscript
class MyClass
{
    // Default constructor (no parameters)
    MyClass()
    {
    }

    // Copy constructor
    MyClass(const MyClass &in other)
    {
        a = other.a;
    }

    // Parameterized constructors
    MyClass(int x, string name) {}
    MyClass(float x, float y, float z) {}

    // Conversion constructor (implicit - single argument)
    MyClass(int value) {}

    // Explicit conversion constructor (cannot be used implicitly)
    MyClass(string text) explicit {}

    int a;
}
```

## Semantics
- Constructors have **no return type** and share the **same name as the class**.
- Multiple constructors can be declared with different parameter lists (constructor overloading).
- If **no constructors** are declared, the compiler automatically generates a **default constructor** that:
  - Calls the default constructor for all object-type members.
  - Sets all handle members to `null`.
  - If a member cannot be default-constructed (no default constructor available), a compiler error is emitted.
- The **copy constructor** takes a single `const ClassName &in` parameter. When present, the compiler can use it to create copies more efficiently. Without a copy constructor, the compiler must first default-construct a new instance, then call `opAssign` to copy the contents.
- **Conversion constructors** are constructors that take a single argument. The compiler can use these for implicit type conversions (e.g., assigning an `int` to a `MyClass` variable). The `explicit` decorator prevents implicit use; the conversion constructor can then only be used with explicit syntax like `MyClass(value)`.
- One constructor **cannot call another constructor** of the same class. Shared initialization logic should be extracted into a private method.
- When members have initialization expressions in their declarations, those expressions are automatically compiled into every constructor body. See [Member Initialization](./member-initialization.md) for ordering details.
- In a derived class, the base class constructor is invoked using the `super` keyword. If `super(...)` is not explicitly called, the compiler inserts a call to the base class default constructor at the beginning of the derived constructor.

## Examples
```angelscript
class Vector3
{
    float x, y, z;

    // Default constructor
    Vector3()
    {
        x = 0; y = 0; z = 0;
    }

    // Parameterized constructor
    Vector3(float _x, float _y, float _z)
    {
        x = _x; y = _y; z = _z;
    }

    // Copy constructor
    Vector3(const Vector3 &in other)
    {
        x = other.x; y = other.y; z = other.z;
    }
}

// Using constructors
void main()
{
    Vector3 a;                     // calls default constructor
    Vector3 b(1.0f, 2.0f, 3.0f);  // calls parameterized constructor
    Vector3 c(b);                  // calls copy constructor
    Vector3 d = b;                 // also calls copy constructor
}
```

```angelscript
// Conversion constructors
class Temperature
{
    float celsius;

    Temperature(float c) { celsius = c; }             // implicit conversion from float
    Temperature(string s) explicit { celsius = parseFloat(s); }  // explicit only
}

void example()
{
    Temperature t1 = 100.0f;        // OK - implicit conversion from float
    Temperature t2 = "36.6";        // Error - explicit required
    Temperature t3 = Temperature("36.6");  // OK - explicit conversion
}
```

## Compilation Notes
- **Construction sequence:** The compiler emits bytecode for constructors in this order:
  1. Allocate the object on the heap (or obtain memory from the allocator).
  2. Initialize the reference count to 1.
  3. Initialize member variables without explicit initializers (default-construct objects, null handles, leave primitives uninitialized or zero).
  4. Call the base class constructor (if derived; either explicitly via `super(...)` or implicitly via the default base constructor).
  5. Execute member initialization expressions (for members with explicit initialization in their declaration).
  6. Execute the constructor body.
- **Factory pattern:** Internally, AngelScript uses factory functions to create object instances. The compiler generates a factory function for each constructor. The factory allocates memory, then calls the actual constructor on the allocated memory. Callers invoke the factory rather than the constructor directly.
- **Copy constructor optimization:** When the copy constructor is available, the compiler can avoid the two-step default-construct-then-assign pattern. For example, `Vector3 b = a;` emits a single factory call using the copy constructor rather than default-construct plus `opAssign`.
- **Conversion constructor dispatch:** For implicit conversions, the compiler checks single-argument constructors of the target type (excluding those marked `explicit`). If a match is found, the compiler emits a factory call to the target type's conversion constructor, passing the source value.
- **Stack behavior:** Constructor calls push the `this` pointer (the newly allocated object) onto the stack as the first implicit argument. Parameters follow according to the calling convention. The constructor returns `void`; the factory function returns the handle to the new object.
- **Overload resolution:** When multiple constructors match, the compiler applies the same overload resolution rules as for regular functions: exact match preferred, then implicit conversions, with ambiguities resulting in a compiler error.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FunctionDecl` | Constructor (as a `ClassMember::Method`) | `modifiers: DeclModifiers`, `visibility: Visibility`, `return_type: Option<ReturnType>`, `name: Ident`, `template_params: &[Ident]`, `params: &[FunctionParam]`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `is_destructor: bool`, `span: Span` |
| `FuncAttr` | Constructor attributes (includes `explicit`) | `override_: bool`, `final_: bool`, `explicit: bool`, `property: bool`, `delete: bool` |
| `FunctionParam` | Constructor parameter | `ty: ParamType`, `name: Option<Ident>`, `default: Option<&Expr>`, `is_variadic: bool`, `span: Span` |

**Notes:**
- A constructor is identified by `FunctionDecl.is_constructor()` returning `true`, which checks `return_type.is_none() && !is_destructor`. There is no separate AST node for constructors; they are `FunctionDecl` nodes inside `ClassMember::Method`.
- The `explicit` keyword maps to `FuncAttr.explicit`. When `true`, the constructor cannot be used for implicit type conversions.
- Constructor overloading is represented by multiple `FunctionDecl` nodes (all with `return_type: None`, `is_destructor: false`) sharing the same `name` as the class.
- The `FuncAttr.delete` field can mark a constructor as deleted, preventing its use.

## Related Features
- [Member Initialization](./member-initialization.md) - initialization order and default values
- [Destructors](./destructors.md) - cleanup when objects are destroyed
- [Inheritance](./inheritance.md) - the `super` keyword and constructor chaining
- [Operator Overloads](./operator-overloads.md) - type conversion operators as alternatives to conversion constructors
- [Class Declarations](./class-declarations.md) - class structure and modifiers
