# Virtual Properties

## Overview
Virtual properties are properties with special behavior for reading and writing. They look like ordinary variables when accessed, but behind the scenes, the compiler transforms each read into a call to a `get` accessor and each write into a call to a `set` accessor. Virtual properties can be declared at global scope or as members of classes. They are particularly useful when specific logic must execute on every read or write, such as sending notifications, computing derived values, or validating assignments.

## Syntax

### Inline Accessor Syntax

```angelscript
// Global virtual property with both get and set
int prop
{
    get { return SomeValue(); }
    set { UpdateValue(value); }
}

// Read-only property (no set accessor)
int readOnlyProp
{
    get { return ComputeValue(); }
}

// Write-only property (no get accessor)
int writeOnlyProp
{
    set { StoreValue(value); }
}

// Get accessor with const qualifier
int prop
{
    get const { return _value; }
    set { _value = value; }
}
```

### Explicit Method Syntax

The inline syntax is transformed by the compiler into two methods prefixed with `get_` and `set_`, decorated with the `property` function decorator. You can write these explicitly instead:

```angelscript
// Equivalent to the inline syntax above
int get_prop() const property { return _value; }
void set_prop(int value) property { _value = value; }
```

### Indexed Property Accessors

Property accessors can emulate array-like access through an index parameter:

```angelscript
// Global indexed get accessor
string get_stringArray(int idx) property
{
    switch (idx)
    {
    case 0: return firstString;
    case 1: return secondString;
    }
    return "";
}

// Global indexed set accessor
void set_stringArray(int idx, const string &in value) property
{
    switch (idx)
    {
    case 0: firstString = value; break;
    case 1: secondString = value; break;
    }
}

// Usage
stringArray[0] = "Hello";
print(stringArray[0]);
```

### Interface Property Declarations

In interfaces, virtual properties can be declared without bodies:

```angelscript
interface IProperty
{
    int value { get const; set; }
}
```

## Semantics

### Accessor Rules

- The `get` accessor takes no arguments (or an index for indexed properties) and returns the property's type.
- The `set` accessor takes one argument named `value` of the property's type (or an index plus the value for indexed properties) and returns `void`.
- If using explicit method syntax, the return type of the `get` accessor and the parameter type of the `set` accessor **must match**, or the compiler will not be able to determine the correct property type.
- Either the `get` or `set` accessor may be omitted. Omitting `set` creates a read-only property; omitting `get` creates a write-only property.
- The `get` accessor can be declared `const` (useful for class members, indicating it does not modify the object state).

### Access Behavior

- Reading the property value is converted by the compiler to a call to the `get` accessor (`get_propName()`).
- Writing to the property is converted to a call to the `set` accessor (`set_propName(value)`).

### Compound Assignment

- Compound assignments (e.g. `prop += 5`) are supported **only if** the owning object is a reference type or the property is global. This is because the compiler must guarantee the object stays alive between the `get` call and the `set` call.
- For value type members, compound assignment on virtual properties is not allowed.
- Compound assignments currently do **not** work for indexed properties.

### Increment/Decrement

- The increment (`++`) and decrement (`--`) operators are **not supported** on virtual properties.
- You must rewrite `a++` as `a += 1` (and only if compound assignment is allowed for that context).

## Examples

```angelscript
// Global virtual property backed by a private variable
private int _health;

int health
{
    get { return _health; }
    set
    {
        if (value < 0) value = 0;
        if (value > 100) value = 100;
        _health = value;
    }
}

void main()
{
    // Accessed like a normal variable
    health = 150;         // Clamped to 100
    int h = health;       // Returns 100
    health += -50;        // Calls get, subtracts, calls set -> 50
}

// Indexed property simulating an array
string names0;
string names1;

string get_names(int idx) property
{
    if (idx == 0) return names0;
    if (idx == 1) return names1;
    return "";
}

void set_names(int idx, const string &in val) property
{
    if (idx == 0) names0 = val;
    else if (idx == 1) names1 = val;
}

void example()
{
    names[0] = "Alice";
    names[1] = "Bob";
    print(names[0] + " and " + names[1]);
}
```

## Compilation Notes

- **Module structure:** Virtual properties do not have their own storage slot in the module's global variable table. Instead, the compiler generates two global functions: `get_<name>()` and `set_<name>(<type> value)`, both marked with the `property` function decorator. These functions appear in the module's function table like any other global function. For indexed properties, the signatures include the index parameter.
- **Symbol resolution:** When the compiler encounters a property-like access on a name that has no matching variable, it searches for `get_<name>` and `set_<name>` functions with the `property` decorator. The property type is inferred from the return type of `get_` or the parameter type of `set_`. If both exist, their types must match. The property decorator flag prevents these functions from being called as ordinary functions (they are accessed only through property syntax).
- **Initialization:** Virtual properties have no initialization of their own. Any backing storage must be managed separately (e.g. a private global variable). The `get` and `set` functions are compiled as normal functions.
- **Type system:** The virtual property's type is determined by the accessor signatures. It participates in type checking like a regular variable of that type. The compiler ensures that assignments and reads go through the correct accessor. Compound assignment generates a temporary: `temp = get_prop(); temp op= rhs; set_prop(temp);`.
- **Special cases:** The application can optionally disable support for property accessors entirely. When property accessors are registered by the host application (native code), they follow the same naming convention (`get_`/`set_` prefix with `property` decorator). This means script-declared and application-declared virtual properties are interchangeable from the script's perspective.

## AST Mapping

> **Source:** `crates/angelscript-parser/src/ast/decl.rs`, `crates/angelscript-parser/src/ast/node.rs`

| AST Type | Role | Fields |
|----------|------|--------|
| `VirtualPropertyDecl` | Virtual property declaration | `visibility: Visibility`, `ty: ReturnType<'ast>`, `name: Ident<'ast>`, `accessors: &[PropertyAccessor<'ast>]`, `span: Span` |
| `PropertyAccessor` | A get or set accessor | `kind: PropertyAccessorKind`, `is_const: bool`, `attrs: FuncAttr`, `body: Option<Block<'ast>>`, `span: Span` |
| `PropertyAccessorKind` | Accessor kind enum | Enum: `Get`, `Set` |
| `ClassMember::VirtualProperty` | Virtual property as class member | Wraps `VirtualPropertyDecl` |
| `InterfaceMember::VirtualProperty` | Virtual property in interface | Wraps `VirtualPropertyDecl` |

**Notes:**
- **Discrepancy:** `VirtualPropertyDecl` does **not** appear as a top-level `Item` variant. It can only appear inside `ClassMember::VirtualProperty` or `InterfaceMember::VirtualProperty`. Global virtual properties declared with the inline accessor syntax (e.g., `int prop { get { ... } set { ... } }`) have no dedicated `Item` variant. Global virtual properties using explicit method syntax (e.g., `int get_prop() property`) are parsed as regular `Item::Function` declarations with `FuncAttr.property = true`.
- `PropertyAccessor.body` is `Option<Block>` -- `None` for interface property declarations (e.g., `int value { get const; set; }`).
- The `is_const` field on `PropertyAccessor` represents the `const` qualifier on the `get` accessor.

## Related Features

- [Global Variables](./global-variables.md) -- virtual properties are an alternative to direct variable access
- [Global Functions](./global-functions.md) -- virtual properties are implemented as pairs of functions
- [Namespaces](./namespaces.md) -- virtual properties can be declared within namespaces
