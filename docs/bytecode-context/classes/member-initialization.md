# Member Initialization

## Overview
The order in which class member variables are initialized during object construction is significant in AngelScript, especially when using inheritance or when members have initialization expressions in their declarations. Incorrect initialization ordering can lead to null handle exceptions if a member is accessed before it has been initialized. AngelScript defines a specific initialization order to minimize such problems.

## Syntax
```angelscript
// Members with default initializers
class Foo
{
    string a;
    string b = a;       // initialized from 'a'
    string c;
    string d = b;       // initialized from 'b'
}

// Members in a derived class
class Bar
{
    string a;
    string b = a;
}

class Baz : Bar
{
    string c = a;       // 'a' comes from base class Bar
    string d;
}
```

## Semantics

### Initialization Order for Simple Classes
For a class without inheritance, members are initialized in the following order:
1. **Members without explicit initializers** are initialized first, in declaration order. Object-type members get default-constructed, handles are set to `null`, primitives are left at their default/zero state.
2. **Members with explicit initializers** (initialization expressions in the declaration) are initialized second, in declaration order.

This means that in the class:
```angelscript
class Foo
{
    string a;          // 1st: default-constructed
    string b = a;      // 3rd: initialized from 'a' (explicit init)
    string c;          // 2nd: default-constructed
    string d = b;      // 4th: initialized from 'b' (explicit init)
}
```
The order is: `a`, `c`, `b`, `d`.

### Initialization Order with Inheritance
When a class has a base class, the initialization follows a more complex ordering:
1. **Derived class members without explicit initializers** - initialized first, in declaration order.
2. **Base class member initialization** - the base class constructor runs (which initializes base members according to base class rules).
3. **Derived class members with explicit initializers** - initialized last, in declaration order.

This ordering ensures that:
- Members that can be initialized cheaply (default construction) are done before the base class is fully set up.
- Members that depend on base class state (explicit initializers referencing inherited members) run after the base class is ready.

Example:
```angelscript
class Bar
{
    string a;
    string b = a;
}

// Initialization order for Baz: d, (Bar: a, b), c
class Baz : Bar
{
    string c = a;   // depends on base class member 'a'
    string d;       // no dependency on base
}
```
Order: `d` (derived, no initializer), then `Bar` members (`a`, `b`), then `c` (derived, explicit initializer).

### Interaction with Constructors
- All member initialization happens **at the beginning** of the constructor body. By the time the explicit constructor code runs, all members are already initialized.
- **Exception:** When the constructor explicitly calls `super(...)`, the members with explicit initializers remain uninitialized until `super()` returns. Members without explicit initializers are initialized before the `super()` call.

```angelscript
class Bar
{
    Bar(string val) { a = val; }
    string a;
}

class Foo : Bar
{
    Foo()
    {
        // 'b' is already initialized here (no explicit initializer)
        super(b);   // Base class 'a' is initialized inside super()
        // 'c' is initialized right after super() returns
    }

    string b;
    string c = a;   // depends on base class member 'a'
}
```

### Danger: Virtual Methods During Construction
Be cautious when constructors or member initialization expressions call class methods. Since all methods are virtual, a base class constructor can unwittingly call a derived class's overridden method. If that method accesses a derived class member that has not yet been initialized (because derived explicit-initializer members are initialized after the base constructor), a **null handle exception** can occur.

```angelscript
class Bar
{
    Bar()
    {
        DoSomething();  // Virtual call - may call Foo::DoSomething()
    }
    void DoSomething() {}
}

class Foo : Bar
{
    string msg = "hello";

    void DoSomething()
    {
        // DANGER: 'msg' is not yet initialized when called from Bar()
        // because explicit initializers run AFTER base constructor
        print(msg);  // null handle exception!
    }
}
```

## Examples
```angelscript
// Safe initialization pattern
class Config
{
    int maxRetries = 3;
    float timeout = 30.0f;
    string name;

    Config()
    {
        // All members are already initialized by this point
        name = "default";
    }

    Config(string n, int retries)
    {
        // maxRetries and timeout already have their default values
        name = n;
        maxRetries = retries;
    }
}
```

```angelscript
// Inheritance initialization order
class Vehicle
{
    int wheels;
    string type = "unknown";

    Vehicle(int w) { wheels = w; }
}

class Car : Vehicle
{
    string model;
    string description = type + " - " + model;

    Car(string m)
    {
        super(4);
        model = m;
        // Note: 'description' was already initialized, but 'type' had its
        // explicit initializer value ("unknown") and 'model' was default empty
        // at that point. To get correct description, set it explicitly here:
        description = type + " - " + model;
    }
}
```

## Compilation Notes
- **Initialization bytecode placement:** The compiler inserts member initialization bytecode at the very beginning of each constructor's compiled body. For the auto-generated default constructor, the entire body is member initialization code.
- **Split initialization with super():** When `super(...)` is explicitly called in a constructor:
  1. First, emit initialization for derived members **without** explicit initializers.
  2. Emit the `super(...)` call.
  3. After `super()` returns, emit initialization for derived members **with** explicit initializers.
  4. Proceed with the rest of the constructor body.
  If `super()` is not explicitly called, the compiler inserts the implicit `super()` call at the point where step 2 would occur.
- **Member initialization expressions:** Each explicit initializer expression (`int x = expr`) is compiled as an assignment in the constructor. The compiler evaluates the expression and stores the result into the member's offset. These are not constant-folded into the object layout; they execute at runtime during construction.
- **Default initialization:** Members without explicit initializers are initialized based on their type:
  - Primitives: the compiler may emit zero-initialization or leave them at whatever value the allocated memory contains (implementation-dependent).
  - Handles: explicitly set to `null` (zero the pointer).
  - Object types: the default constructor of the member type is called.
- **Constructor-per-constructor:** The initialization code is duplicated in every constructor. Each constructor independently contains the full initialization sequence. This means if a class has three constructors, the member initialization bytecode appears three times (once per constructor).
- **Virtual method hazard:** The compiler does not warn about virtual method calls during construction. It is the programmer's responsibility to avoid accessing uninitialized members through virtual dispatch during base class construction.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `FieldDecl` | Field with optional initializer | `visibility: Visibility`, `ty: TypeExpr`, `name: Ident`, `init: Option<&Expr>`, `span: Span` |
| `FunctionDecl` | Constructor that receives initialization bytecode | `return_type: Option<ReturnType>` (None for constructors), `body: Option<Block>`, plus other fields |

**Notes:**
- `FieldDecl.init` is the key field for member initialization. When `init` is `Some(&Expr)`, the field has an explicit initialization expression (e.g., `int maxSize = 100`). When `None`, the member relies on default initialization (zero/null/default constructor).
- The initialization ordering rules (non-initialized members first, then base class, then initialized members) are enforced by the compiler when generating constructor bytecode. The AST only stores whether each field has an initializer; it does not encode ordering.
- Each `Expr` in `FieldDecl.init` can be any expression (literal, global variable, function call, etc.). The compiler inserts this expression's evaluation into every constructor.

## Related Features
- [Constructors](./constructors.md) - constructor syntax and the `super` keyword
- [Properties](./properties.md) - member variable declarations with default values
- [Inheritance](./inheritance.md) - how base class construction interleaves with member initialization
- [Destructors](./destructors.md) - the reverse process: member cleanup ordering
