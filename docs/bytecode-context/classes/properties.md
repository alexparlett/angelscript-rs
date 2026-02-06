# Properties

## Overview
Class properties are member variables that store data within an object instance. AngelScript also supports virtual properties (property accessors), which look like member variables to the caller but are backed by getter and setter methods. Property accessors enable encapsulation, computed values, validation, and notification logic. Indexed property accessors extend this concept to array-like access patterns.

## Syntax

### Member Variables
```angelscript
class MyClass
{
    // Basic member variables
    int count;
    float x, y, z;
    string name;

    // Handle member
    OtherClass@ reference;

    // Member with default initialization
    int maxSize = 100;
    string label = "default";
}
```

### Virtual Properties (Property Accessors)
```angelscript
class MyClass
{
    // Block-style property accessor
    int prop
    {
        get const
        {
            return realProp;
        }
        set
        {
            realProp = value;  // 'value' is the implicit parameter name
        }
    }

    private int realProp;
}

// Equivalent explicit method syntax
class MyClass
{
    int get_prop() const property { return realProp; }
    void set_prop(int value) property { realProp = value; }
    private int realProp;
}
```

### Interface Property Declarations
```angelscript
interface IProp
{
    int prop { get const; set; }
}
```

### Indexed Property Accessors
```angelscript
class MyContainer
{
    float get_opIndex(int idx) const { return data[idx]; }
    void set_opIndex(int idx, float value) { data[idx] = value; }

    // Named indexed property
    string get_items(int idx) const property { return strings[idx]; }
    void set_items(int idx, const string &in value) property { strings[idx] = value; }
}
```

## Semantics
- **Member variables** are declared with a type and a name, optionally followed by an initialization expression.
- Members can be of any type: primitives, objects, handles, or arrays.
- Members are accessed using dot notation: `obj.member`.
- Members can have default values specified at declaration. These initialization expressions are compiled into every constructor. See [Member Initialization](./member-initialization.md).

### Virtual Property Rules
- A virtual property consists of a **getter** (`get`) and/or **setter** (`set`).
- If only a getter is declared, the property is **read-only**.
- If only a setter is declared, the property is **write-only**.
- The getter should be declared as `const` to allow access through const references.
- The setter receives the new value through an implicit parameter named `value`.
- Behind the scenes, the compiler transforms property accessors into methods prefixed with `get_` and `set_`, decorated with the `property` function decorator.
- The return type of the getter and the parameter type of the setter **must match**; otherwise the compiler cannot determine the property type.
- Virtual properties can coexist with a real member variable of the same name (the real variable can be private while the virtual property provides controlled access).

### Property Access in Expressions
- Reading a property: `x = obj.prop` is compiled as `x = obj.get_prop()`.
- Writing a property: `obj.prop = x` is compiled as `obj.set_prop(x)`.
- **Compound assignment** (`obj.prop += x`) works only when the owning object is a **reference type** (not a value type). This is because the compiler must ensure the object remains alive between the `get_` and `set_` calls.
- **Increment/decrement** (`obj.prop++`, `obj.prop--`) is **not supported** on virtual properties. Use `obj.prop += 1` instead (on reference types).

### Indexed Property Accessors
- Indexed property accessors emulate array-style access on an object.
- The get accessor takes the index as its only argument; the set accessor takes the index as the first argument and the new value as the second.
- Compound assignments are **not supported** for indexed properties.
- The `opIndex` name is used for the `[]` operator overload; named indexed properties use a custom name with `get_`/`set_` prefixes.

## Examples
```angelscript
class Player
{
    string name;
    private int _health = 100;
    private int _maxHealth = 100;

    // Virtual property with validation
    int health
    {
        get const { return _health; }
        set
        {
            if (value < 0) value = 0;
            if (value > _maxHealth) value = _maxHealth;
            _health = value;
        }
    }

    // Read-only computed property
    float healthPercent
    {
        get const { return float(_health) / float(_maxHealth) * 100.0f; }
    }
}

void example()
{
    Player p;
    p.name = "Hero";        // direct member access
    p.health = 150;         // calls set_health(150), clamped to 100
    float pct = p.healthPercent;  // calls get_healthPercent()
    p.health -= 30;         // compound assignment: get then set
}
```

```angelscript
// Indexed property accessor
class StringMap
{
    private array<string> keys;
    private array<string> values;

    string get_opIndex(const string &in key) const
    {
        for (uint i = 0; i < keys.length(); i++)
        {
            if (keys[i] == key) return values[i];
        }
        return "";
    }

    void set_opIndex(const string &in key, const string &in val)
    {
        for (uint i = 0; i < keys.length(); i++)
        {
            if (keys[i] == key) { values[i] = val; return; }
        }
        keys.insertLast(key);
        values.insertLast(val);
    }
}

void example()
{
    StringMap map;
    map["greeting"] = "hello";   // calls set_opIndex("greeting", "hello")
    string s = map["greeting"];  // calls get_opIndex("greeting")
}
```

## Compilation Notes
- **Member layout:** Member variables are laid out in the object's memory block in declaration order. Inherited members come first (from the base class layout), followed by the derived class's own members. The compiler assigns each member an offset within the object structure.
- **Member access bytecode:** Accessing a member variable compiles to loading the object pointer, then accessing memory at the member's known offset. For primitives, this is a direct load/store. For handles, reference counting instructions (`AddRef`/`Release`) are emitted around assignments. For object members, copy constructors or assignment operators may be invoked.
- **Property accessor compilation:** When the compiler encounters a property access on a virtual property, it rewrites the expression into method calls:
  - `obj.prop` (read context) becomes `obj.get_prop()`.
  - `obj.prop = expr` (write context) becomes `obj.set_prop(expr)`.
  - `obj.prop += expr` (compound) becomes `obj.set_prop(obj.get_prop() + expr)`.
  The resulting method calls go through normal method dispatch (virtual if the accessor is inherited).
- **Compound assignment safety:** For compound assignments on virtual properties, the compiler must guarantee the owning object stays alive between the get and set calls. This is why compound assignment is restricted to reference types (whose lifetime is managed by reference counting) and global properties.
- **Indexed accessor compilation:** `obj[idx]` in read context becomes `obj.get_opIndex(idx)`. In write context, `obj[idx] = val` becomes `obj.set_opIndex(idx, val)`. The compiler checks for both the `opIndex` method and the `get_opIndex`/`set_opIndex` accessor pair, preferring the accessor pair when both forms exist for read/write disambiguation.
- **Stack behavior:** Member access through the `this` pointer uses the object pointer already on the stack. External member access loads the object reference first, then applies the offset. Virtual property calls push the object pointer as the `this` argument, just like any other method call.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `ClassMember::Field` | Member variable (field) | Contains a `FieldDecl` |
| `FieldDecl` | Field declaration | `visibility: Visibility`, `ty: TypeExpr`, `name: Ident`, `init: Option<&Expr>`, `span: Span` |
| `ClassMember::VirtualProperty` | Virtual property (block-style accessor) | Contains a `VirtualPropertyDecl` |
| `VirtualPropertyDecl` | Virtual property declaration | `visibility: Visibility`, `ty: ReturnType`, `name: Ident`, `accessors: &[PropertyAccessor]`, `span: Span` |
| `PropertyAccessor` | Get or set accessor | `kind: PropertyAccessorKind`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block>`, `span: Span` |
| `PropertyAccessorKind` | Accessor direction | Variants: `Get`, `Set` |
| `FuncAttr` | Property method attributes (for explicit `get_`/`set_` style) | `property: bool` (plus `override_`, `final_`, `explicit`, `delete`) |

**Notes:**
- Member variables with default initialization use `FieldDecl.init`. When `init` is `Some(&Expr)`, the field has an initializer expression. See [Member Initialization](./member-initialization.md).
- Block-style virtual properties (`int prop { get { ... } set { ... } }`) are parsed as `VirtualPropertyDecl` with `PropertyAccessor` entries.
- Explicit method-style property accessors (`int get_prop() const property`) are parsed as regular `FunctionDecl` nodes in `ClassMember::Method` with `FuncAttr.property == true`.
- `PropertyAccessor.is_const` maps to the `const` on a getter (e.g., `get const { ... }`).
- `PropertyAccessor.body` is `None` for interface property declarations (e.g., `int prop { get const; set; }`).
- Indexed property accessors (`get_opIndex`/`set_opIndex`) are parsed as regular methods with `FuncAttr.property == true`, not as `VirtualPropertyDecl`.

## Related Features
- [Member Initialization](./member-initialization.md) - default values and initialization ordering
- [Methods](./methods.md) - regular methods that property accessors compile down to
- [Access Modifiers](./access-modifiers.md) - controlling property visibility
- [Operator Overloads](./operator-overloads.md) - `opIndex` and other operator methods
- [Class Declarations](./class-declarations.md) - class body structure
