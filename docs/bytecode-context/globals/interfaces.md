# Interfaces

## Overview
Interfaces define contracts that classes must implement. An interface declares method signatures without implementations. Any class that lists an interface in its inheritance list is required to implement all methods declared by that interface. Interfaces enable polymorphism: a function can accept a handle to an interface type and call its methods without knowing the concrete class of the object.

## Syntax

### Interface Declaration

```angelscript
// Basic interface
interface MyInterface
{
    void DoSomething();
}

// Interface with multiple methods
interface IDrawable
{
    void Draw();
    int GetLayer();
}

// Interface with property accessors
interface IProperty
{
    int value { get const; set; }
}
```

### Implementing an Interface

```angelscript
// A class implementing a single interface
class MyClass : MyInterface
{
    void DoSomething()
    {
        // Implementation
    }
}

// A class implementing multiple interfaces
class Sprite : IDrawable, IClickable
{
    void Draw() { /* ... */ }
    int GetLayer() { return layer; }
    void OnClick() { /* ... */ }
    int layer;
}
```

### Using Interface Handles

```angelscript
void ProcessDrawable(IDrawable@ obj)
{
    obj.Draw();
    int layer = obj.GetLayer();
}
```

## Semantics

### Declaration Rules

- Interfaces can only contain method declarations (no implementations, no member variables).
- Methods in interfaces are implicitly public and virtual.
- Property accessor declarations are allowed (e.g. `int prop { get const; set; }`), which require implementing classes to provide the corresponding `get_prop()` and `set_prop()` methods.
- Interfaces cannot contain constructors, destructors, or static methods.
- Interface names share the global namespace with all other global entities.

### Implementation Rules

- A class implements an interface by listing it after a colon in the class declaration.
- A class can implement **multiple interfaces** by separating them with commas.
- The implementing class must provide implementations for **all methods** declared in the interface.
- If a method from the interface is missing, the compiler produces an error.
- The implementing methods must have matching signatures (same return type, same parameter types and qualifiers).

### Multiple Interface Implementation

```angelscript
interface IA
{
    void MethodA();
}

interface IB
{
    void MethodB();
}

class MyClass : IA, IB
{
    void MethodA() { /* ... */ }
    void MethodB() { /* ... */ }
}
```

### Interface Inheritance

- An interface can inherit from one or more other interfaces, requiring implementing classes to implement all inherited methods as well.

### Type Checking with Interfaces

- Objects can be cast to interface handles using `cast<IInterface>(obj)`.
- The `is` operator (or a reference cast) can be used to check whether an object implements an interface.
- Interface handles are reference-counted just like class handles.
- Interface handles can be compared with `is` and `!is` for identity checks and null checks.

## Examples

```angelscript
// Define interfaces
interface ISerializable
{
    string Serialize();
    void Deserialize(const string &in data);
}

interface IUpdatable
{
    void Update(float deltaTime);
}

// A class implementing both interfaces
class GameEntity : ISerializable, IUpdatable
{
    string name;
    float x, y;

    string Serialize()
    {
        return name + "," + x + "," + y;
    }

    void Deserialize(const string &in data)
    {
        // Parse data
    }

    void Update(float deltaTime)
    {
        x += deltaTime;
    }
}

// Polymorphic usage
void SaveEntity(ISerializable@ obj)
{
    string data = obj.Serialize();
    // Save data to file
}

void UpdateAll(array<IUpdatable@>@ objects, float dt)
{
    for (uint i = 0; i < objects.length(); i++)
    {
        objects[i].Update(dt);
    }
}

void main()
{
    GameEntity entity;
    entity.name = "player";

    // Pass as ISerializable
    SaveEntity(entity);

    // Pass as IUpdatable in an array
    array<IUpdatable@> updatables = {entity};
    UpdateAll(updatables, 0.016f);
}
```

## Compilation Notes

- **Module structure:** Each interface declaration creates a type entry in the engine's type system. The interface type stores its name, the list of method signatures (name, return type, parameter types), and any property accessor declarations. Interface types are registered during the type registration phase, before class compilation. The interface type has a unique type ID used in handle declarations and type checks.
- **Symbol resolution:** Interface names are registered in the global namespace (or the containing namespace). When a class declares that it implements an interface, the compiler resolves the interface name and retrieves its method list. Each required method is checked against the class's method table. If the class is in a different namespace from the interface, the interface must be referenced with a fully qualified name.
- **Initialization:** Interfaces have no runtime initialization. They are purely compile-time constructs that define contracts. The virtual function table (vtable) for a class is built to include slots for each interface method, enabling dynamic dispatch when calling through an interface handle.
- **Type system:** Interface types are reference types (always accessed via handles). A class that implements an interface can be implicitly cast to a handle of that interface type. The compiler generates the appropriate reference cast code. Multiple interfaces on a single class result in multiple vtable sections, one per interface. The type checker uses interface type IDs to validate assignments and casts. A `cast<IInterface>(obj)` returns null if the object's class does not implement the interface.
- **Special cases:** Interfaces can be declared as `shared` for cross-module sharing. Shared interfaces must have identical method declarations across all modules. The `external shared interface` form allows referencing a shared interface without re-declaring its methods. When a shared interface is used, the engine ensures that implementing classes across different modules are type-compatible through the shared interface.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `Item::Interface` | Top-level item variant for interface declarations | Wraps `InterfaceDecl` |
| `InterfaceDecl` | Interface declaration | `modifiers: DeclModifiers`, `name: Ident<'ast>`, `bases: &[Ident<'ast>]`, `members: &[InterfaceMember<'ast>]`, `span: Span` |
| `InterfaceMember` | Interface member enum | Enum: `Method(InterfaceMethod)`, `VirtualProperty(VirtualPropertyDecl)` |
| `InterfaceMethod` | Interface method signature (no body) | `return_type: ReturnType<'ast>`, `name: Ident<'ast>`, `params: &[FunctionParam<'ast>]`, `is_const: bool`, `span: Span` |
| `VirtualPropertyDecl` | Virtual property in an interface | `visibility: Visibility`, `ty: ReturnType<'ast>`, `name: Ident<'ast>`, `accessors: &[PropertyAccessor<'ast>]`, `span: Span` |

**Notes:**
- `InterfaceDecl.bases` is `&[Ident]` (simple identifiers), unlike `ClassDecl.inheritance` which uses `&[IdentExpr]` (supporting scoped names like `Namespace::Interface`). This means interface base references are unqualified names in the AST.
- `InterfaceMethod` is a separate struct from `FunctionDecl` -- it omits `modifiers`, `visibility`, `template_params`, `attrs`, `body`, and `is_destructor`, since interface methods are always public, virtual, and bodiless.
- Interface virtual property accessors have `body: None` in their `PropertyAccessor` entries (e.g., `int value { get const; set; }`).

## Related Features

- [Funcdefs](./funcdefs.md) -- funcdefs provide a different form of polymorphism through function pointers
- [Namespaces](./namespaces.md) -- interfaces can be declared inside namespaces
- [Shared Entities](./shared-entities.md) -- interfaces can be shared across modules
