# Mixin Classes

## Overview
Mixin classes provide a mechanism for code reuse in the absence of multiple inheritance. A mixin class declares a partial class structure (properties and methods) that can be included into multiple different class declarations. Mixin classes are not real types and cannot be instantiated. When a class includes a mixin, the mixin's properties and methods are replicated into the including class as if they had been written directly in the class body. This enables sharing common implementations across unrelated class hierarchies.

## Syntax
```angelscript
// Declare a mixin class
mixin class MyMixin
{
    void SomeMethod() { property++; }
    int property;
}

// Include the mixin in a class
class MyClass : MyMixin
{
    int OtherMethod()
    {
        SomeMethod();
        return property;
    }
}

// Mixin with inheritance and interfaces
class MyClass : BaseClass, MyMixin, ISerializable
{
}

// Mixin that requires interfaces
mixin class MyMixin : ISerializable
{
    void Serialize() { /* default implementation */ }
}

// Multiple mixins
class MyClass : MixinA, MixinB
{
}
```

## Semantics

### Basic Inclusion
- A mixin class is declared with the `mixin class` keywords.
- A mixin class is included into a regular class by listing it after the colon in the class declaration, alongside any base class and interfaces.
- When included, the mixin's properties and methods are **copied** into the including class as if they had been written there directly.
- The mixin class itself is **not a type**. It cannot be used as a variable type, parameter type, or handle type. It cannot be instantiated.

### Override and Deduplication Rules
- If the including class **already declares** a property or method with the same name as one in the mixin, the mixin's version is **not included**. The class's explicit declaration takes precedence.
- This allows a mixin to provide **default implementations** that a class can override by explicitly declaring its own version.

### Mixin Methods and Compilation Context
- Methods included from a mixin are compiled **in the context of the including class**, not the mixin class.
- A mixin method can reference properties and methods that are **not declared in the mixin itself**, as long as the including class provides them. This enables a form of "duck typing" where the mixin expects certain members to exist in the including class.

### Interaction with Inheritance
- Mixin methods **override** inherited methods from base classes, just as if the included method had been implemented directly in the derived class.
- Mixin properties are **not included** if a property with the same name is already inherited from a base class. The inherited property takes precedence.

```angelscript
class MyBase
{
    void MethodA() { print("Base behaviour"); }
    int property;
}

mixin class MyMixin
{
    void MethodA() { print("Mixin behaviour"); }
    float property;   // different type, but same name
}

// MyClass gets: property (int, from MyBase) and MethodA (from MyMixin)
class MyClass : MyBase, MyMixin
{
}
```

### Mixin and Interfaces
- A mixin class can declare a list of interfaces after its name (e.g., `mixin class M : IFoo`).
- When the mixin is included in a class, the class is **required to implement** those interfaces.
- The mixin can provide default implementations for the interface methods, or leave them for the including class to implement.

### Limitations
- A mixin class **cannot inherit** from other classes (no base class for mixins).
- A mixin class **cannot be instantiated**.
- A mixin class **cannot be used as a type** in variable declarations, function parameters, or handles.
- Constructors and destructors in mixin classes are not supported. Mixins provide only properties and methods.

## Examples
```angelscript
// Basic mixin for logging capability
mixin class Loggable
{
    void Log(string message)
    {
        print("[" + GetName() + "] " + message);
    }
}

class Player : Loggable
{
    string name;

    Player(string n) { name = n; }

    // Provides the GetName() that Loggable::Log() expects
    string GetName() { return name; }

    void TakeDamage(int amount)
    {
        Log("Took " + amount + " damage");
    }
}

class Enemy : Loggable
{
    string type;

    Enemy(string t) { type = t; }

    string GetName() { return type; }

    void Attack()
    {
        Log("Attacks!");
    }
}
```

```angelscript
// Mixin providing default method that can be overridden
mixin class Describable
{
    string Describe() { return "An object"; }
}

class Item : Describable
{
    string name;

    Item(string n) { name = n; }

    // Override the mixin's default
    string Describe() { return "Item: " + name; }
}

class NPC : Describable
{
    // Uses the mixin's default Describe()
}
```

```angelscript
// Mixin with interface requirement
interface IUpdatable
{
    void Update(float dt);
}

mixin class UpdateMixin : IUpdatable
{
    float elapsed = 0;

    void Update(float dt)
    {
        elapsed += dt;
        OnUpdate(dt);
    }
}

class GameEntity : UpdateMixin
{
    // Must provide OnUpdate since UpdateMixin::Update calls it
    void OnUpdate(float dt)
    {
        // Entity-specific update logic
    }
}
```

```angelscript
// Mixin overriding base class method
class Base
{
    void Render() { print("Base render"); }
    int layer;
}

mixin class RenderMixin
{
    void Render() { print("Enhanced render on layer " + layer); }
    int layer;   // Will NOT be included (already in Base)
}

class Widget : Base, RenderMixin
{
    // Gets: layer (from Base), Render() (from RenderMixin, overrides Base)
}
```

## Compilation Notes
- **Compile-time expansion:** Mixin inclusion is processed entirely at compile time. It is conceptually similar to a textual copy-paste of the mixin's members into the including class, followed by deduplication against existing members. No runtime mechanism or type relationship exists for mixins.
- **No type identity:** Mixin classes do not generate type entries, vtable entries, or any runtime metadata. They exist only during compilation as templates for member injection.
- **Method compilation context:** When a method from a mixin is included into a class, it is compiled as if it were written directly in that class. This means:
  - The `this` pointer type is the including class, not the mixin.
  - Name resolution uses the including class's full member list (including inherited and other mixin-included members).
  - If a mixin method references a name that the including class does not have, a compile error is emitted for that specific class (not for the mixin declaration itself).
- **Property deduplication:** During compilation, the compiler iterates over the mixin's properties. For each property, if the including class (or its base class) already has a property with the same name, the mixin property is skipped. Type compatibility is not checked; only the name matters for deduplication.
- **Method deduplication:** Similarly, the compiler checks if the including class already declares a method with the same name and signature. If so, the mixin method is skipped. If the base class has the method but the including class does not, the mixin method is included and effectively overrides the base class method (its vtable slot replaces the base implementation).
- **Interface propagation:** When a mixin declares interface requirements, the compiler adds those interfaces to the including class's interface list. The compiler then verifies that all interface methods are implemented, either by the class itself, by the mixin, or by the base class.
- **Object layout:** Since mixin properties are copied into the including class, they become regular members of the class and are laid out in the object's memory just like any other declared property. There is no separate "mixin section" in the object layout.
- **Vtable integration:** Mixin methods that are included become regular virtual methods in the including class's vtable. They occupy the same vtable slots as if they had been written directly in the class. There is no additional indirection or dispatch mechanism for mixin-origin methods.
- **Multiple mixin ordering:** When multiple mixins are included and they have overlapping member names, the first mixin listed takes precedence for methods (its version is included), while properties follow the same deduplication rule (first occurrence wins, including inherited properties).

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `MixinDecl` | Mixin class declaration (`mixin class ...`) | `class: ClassDecl`, `span: Span` |
| `ClassDecl` | The class body inside the mixin | `modifiers: DeclModifiers`, `name: Ident`, `template_params: &[Ident]`, `inheritance: &[IdentExpr]`, `members: &[ClassMember]`, `span: Span` |

**Notes:**
- A mixin declaration is a top-level `Item::Mixin(MixinDecl)`. The `MixinDecl` wraps a full `ClassDecl` which contains the mixin's name, members, and any interface requirements (in `inheritance`).
- Including a mixin in a regular class is done via `ClassDecl.inheritance`. The mixin name appears in the same list as base classes and interfaces. The parser does not distinguish mixin references from class or interface references; the compiler resolves this during type checking.
- `MixinDecl` has its own `span` that covers the `mixin` keyword plus the class body. The inner `ClassDecl.span` covers only the `class ...` portion.
- Mixin member expansion (copying members into the including class) and deduplication are compile-time operations not reflected in the AST.

## Related Features
- [Class Declarations](./class-declarations.md) - class syntax including mixin inclusion
- [Inheritance](./inheritance.md) - single inheritance that mixins complement
- [Methods](./methods.md) - mixin methods become regular class methods
- [Properties](./properties.md) - mixin properties become regular class properties
- [Access Modifiers](./access-modifiers.md) - mixin members can have access modifiers
