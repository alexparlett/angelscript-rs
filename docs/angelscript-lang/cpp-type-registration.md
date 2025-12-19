# Type Registration

> **Note:** All code examples in this file show the **C++ API**. The concepts (behaviors, type flags, registration patterns) are language semantics that any implementation must support.

## Overview

Application types must be registered with the engine before scripts can use them. The registration tells AngelScript:
- Type name and size
- Whether it's a reference or value type
- What behaviors (methods) it supports
- Properties and methods available

## Reference Type Registration

### Basic Reference Type

```cpp
// C++ API example
engine->RegisterObjectType("MyClass", 0, asOBJ_REF);

// Register factory (constructor via factory function)
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_FACTORY,
    "MyClass@ f()", asFUNCTION(MyClassFactory), asCALL_CDECL);

// Register reference counting
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_ADDREF,
    "void f()", asMETHOD(MyClass, AddRef), asCALL_THISCALL);
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_RELEASE,
    "void f()", asMETHOD(MyClass, Release), asCALL_THISCALL);
```

### Garbage Collected Reference Type

Add `asOBJ_GC` flag and register GC behaviors:

```cpp
engine->RegisterObjectType("MyClass", 0, asOBJ_REF | asOBJ_GC);

// Additional GC behaviors required
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_SETGCFLAG, ...);
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_GETGCFLAG, ...);
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_GETREFCOUNT, ...);
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_ENUMREFS, ...);
engine->RegisterObjectBehaviour("MyClass", asBEHAVE_RELEASEREFS, ...);
```

### No-Count Reference Type (Singleton)

For types where scripts never manage lifetime:

```cpp
engine->RegisterObjectType("MyClass", 0, asOBJ_REF | asOBJ_NOCOUNT);
```

No AddRef/Release needed, but be careful about object lifetime.

## Value Type Registration

### Basic Value Type

```cpp
engine->RegisterObjectType("Vector3", sizeof(Vector3),
    asOBJ_VALUE | asOBJ_POD | asGetTypeTraits<Vector3>());

// Constructor
engine->RegisterObjectBehaviour("Vector3", asBEHAVE_CONSTRUCT,
    "void f()", asFUNCTION(Vector3Construct), asCALL_CDECL_OBJLAST);

// Destructor (if needed)
engine->RegisterObjectBehaviour("Vector3", asBEHAVE_DESTRUCT,
    "void f()", asFUNCTION(Vector3Destruct), asCALL_CDECL_OBJLAST);
```

### Value Type Flags

> **C++ SPECIFIC:** These flags describe C++ type traits for ABI compatibility.

| Flag | Meaning |
|------|---------|
| `asOBJ_POD` | Plain old data (trivially copyable, no constructor/destructor) |
| `asOBJ_APP_CLASS` | Has C++ class semantics |
| `asOBJ_APP_CLASS_CONSTRUCTOR` | Has non-trivial constructor |
| `asOBJ_APP_CLASS_DESTRUCTOR` | Has non-trivial destructor |
| `asOBJ_APP_CLASS_ASSIGNMENT` | Has non-trivial assignment operator |
| `asOBJ_APP_CLASS_COPY_CONSTRUCTOR` | Has copy constructor |
| `asOBJ_APP_PRIMITIVE` | Behaves like a primitive (passed in registers) |
| `asOBJ_APP_FLOAT` | Float-like (uses FPU registers on some platforms) |

> **C++ SPECIFIC:** `asGetTypeTraits<T>()` is a C++ template helper that determines correct flags automatically using SFINAE.

## Behavior Registration

### Factory/Constructor Behaviors

| Behavior | For | Description |
|----------|-----|-------------|
| `asBEHAVE_FACTORY` | Reference types | Returns new instance (allocates memory) |
| `asBEHAVE_CONSTRUCT` | Value types | Placement construction (memory already allocated) |
| `asBEHAVE_LIST_FACTORY` | Reference types | Init from `{...}` initializer list |
| `asBEHAVE_LIST_CONSTRUCT` | Value types | Init from `{...}` initializer list |

### Destructor Behaviors

| Behavior | For | Description |
|----------|-----|-------------|
| `asBEHAVE_DESTRUCT` | Value types | Cleanup before deallocation |

### Reference Counting Behaviors

| Behavior | Description |
|----------|-------------|
| `asBEHAVE_ADDREF` | Increment reference count |
| `asBEHAVE_RELEASE` | Decrement and potentially destroy when zero |

### Garbage Collection Behaviors

| Behavior | Description |
|----------|-------------|
| `asBEHAVE_SETGCFLAG` | Mark object for GC tracking |
| `asBEHAVE_GETGCFLAG` | Check if marked |
| `asBEHAVE_GETREFCOUNT` | Get current reference count |
| `asBEHAVE_ENUMREFS` | Enumerate contained references (for cycle detection) |
| `asBEHAVE_RELEASEREFS` | Break circular references during GC |

## Method Registration

```cpp
// C++ API examples

// Instance method
engine->RegisterObjectMethod("MyClass", "int GetValue() const",
    asMETHOD(MyClass, GetValue), asCALL_THISCALL);

// Method with parameters
engine->RegisterObjectMethod("MyClass", "void SetValue(int val)",
    asMETHOD(MyClass, SetValue), asCALL_THISCALL);

// Method returning handle
engine->RegisterObjectMethod("MyClass", "OtherClass@ GetOther()",
    asMETHOD(MyClass, GetOther), asCALL_THISCALL);
```

> **C++ SPECIFIC:** `asMETHOD` macro extracts the method pointer. `asCALL_THISCALL` indicates C++ member function calling convention.

## Property Registration

```cpp
// Direct property access (exposed struct member)
engine->RegisterObjectProperty("MyClass", "int value",
    asOFFSET(MyClass, value));

// Using accessors (virtual property)
engine->RegisterObjectMethod("MyClass", "int get_prop() const property",
    asMETHOD(MyClass, GetProp), asCALL_THISCALL);
engine->RegisterObjectMethod("MyClass", "void set_prop(int) property",
    asMETHOD(MyClass, SetProp), asCALL_THISCALL);
```

> **C++ SPECIFIC:** `asOFFSET` calculates the byte offset of a member within the struct, used for direct memory access.

## Operator Registration

Operators are registered as methods with special names (see [06-operator-overloads.md](06-operator-overloads.md)):

```cpp
// Binary operator: MyClass + MyClass
engine->RegisterObjectMethod("MyClass", "MyClass opAdd(const MyClass &in) const",
    asMETHOD(MyClass, operator+), asCALL_THISCALL);

// Reverse operator: int + MyClass (when class is on right side)
engine->RegisterObjectMethod("MyClass", "MyClass opAdd_r(int) const",
    asFUNCTION(MyClassAddInt), asCALL_CDECL_OBJLAST);

// Assignment
engine->RegisterObjectMethod("MyClass", "MyClass& opAssign(const MyClass &in)",
    asMETHOD(MyClass, operator=), asCALL_THISCALL);

// Comparison
engine->RegisterObjectMethod("MyClass", "bool opEquals(const MyClass &in) const",
    asMETHOD(MyClass, operator==), asCALL_THISCALL);
engine->RegisterObjectMethod("MyClass", "int opCmp(const MyClass &in) const",
    asMETHOD(MyClass, Compare), asCALL_THISCALL);

// Index operator
engine->RegisterObjectMethod("MyClass", "int opIndex(int) const",
    asMETHOD(MyClass, operator[]), asCALL_THISCALL);

// Conversion
engine->RegisterObjectMethod("MyClass", "int opConv() const",
    asMETHOD(MyClass, ToInt), asCALL_THISCALL);
```

## Global Function Registration

```cpp
// Free function
engine->RegisterGlobalFunction("void Print(const string &in)",
    asFUNCTION(Print), asCALL_CDECL);

// With overloads - use asFUNCTIONPR to disambiguate
engine->RegisterGlobalFunction("int abs(int)",
    asFUNCTIONPR(abs, (int), int), asCALL_CDECL);
engine->RegisterGlobalFunction("float abs(float)",
    asFUNCTIONPR(abs, (float), float), asCALL_CDECL);
```

> **C++ SPECIFIC:** `asFUNCTIONPR` macro specifies parameter types and return type to disambiguate overloaded C++ functions.

## Global Property Registration

```cpp
// Read-write property (exposes raw pointer to variable)
engine->RegisterGlobalProperty("int g_counter", &g_counter);

// Read-only via const (prevents script modification)
engine->RegisterGlobalProperty("const int g_maxValue", &g_maxValue);
```

## Enum Registration

```cpp
engine->RegisterEnum("MyEnum");
engine->RegisterEnumValue("MyEnum", "VALUE_A", 0);
engine->RegisterEnumValue("MyEnum", "VALUE_B", 1);
engine->RegisterEnumValue("MyEnum", "VALUE_C", 2);
```

## Interface Registration

```cpp
engine->RegisterInterface("IRenderable");
engine->RegisterInterfaceMethod("IRenderable", "void Render()");
engine->RegisterInterfaceMethod("IRenderable", "int GetLayer()");
```

## Funcdef Registration

```cpp
engine->RegisterFuncdef("bool CompareFunc(int, int)");
```

## Template Type Registration

> **C++ SPECIFIC:** Template factories receive `asITypeInfo*` as a hidden first parameter (declared as `int&in` in registration string).

```cpp
// Register template type
engine->RegisterObjectType("array<class T>", 0, asOBJ_REF | asOBJ_GC | asOBJ_TEMPLATE);

// Factory - receives type info as first parameter
engine->RegisterObjectBehaviour("array<T>", asBEHAVE_FACTORY,
    "array<T>@ f(int&in)", asFUNCTION(ArrayFactory), asCALL_CDECL);

// Template callback - validates instantiation at compile time
engine->RegisterObjectBehaviour("array<T>", asBEHAVE_TEMPLATE_CALLBACK,
    "bool f(int&in, bool&out)", asFUNCTION(ArrayTemplateCallback), asCALL_CDECL);
```

The `if_handle_then_const` modifier can be used in template method declarations to properly handle const handles:

```cpp
// Without: array<Obj@>.find() won't accept const Obj@ handles
// With: const Obj@ handles are accepted
engine->RegisterObjectMethod("array<T>",
    "int find(const T&in if_handle_then_const) const", ...);
```
