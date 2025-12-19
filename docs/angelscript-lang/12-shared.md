# Shared Script Entities

Shared entities allow script code to be reused across multiple script modules, reducing memory consumption and ensuring type compatibility between modules.

## What Can Be Shared

- Classes
- Interfaces
- Functions
- Enums
- Funcdefs

**Note:** Global variables cannot be shared (future versions may allow this).

## Declaration

Add `shared` keyword before the declaration:

```angelscript
shared class Foo {
    void MethodInFoo(int b) { bar = b; }
    int bar;
}

shared void GlobalFunc() {}

shared interface IShared {
    void Method();
}

shared enum SharedEnum {
    Value1,
    Value2
}

shared funcdef bool SharedCallback(int);
```

## Restrictions

Shared entities **cannot access non-shared entities**:

```angelscript
int globalVar;  // Not shared

shared class MyClass {
    void Method() {
        globalVar = 1;  // ERROR: Cannot access non-shared entity
    }
}
```

This is because non-shared entities are exclusive to each module.

## Implementation Consistency

All modules sharing an entity must implement it **identically**:

```angelscript
// Module A
shared class Foo {
    int value;
    void Method() { value = 1; }
}

// Module B - MUST be identical
shared class Foo {
    int value;
    void Method() { value = 1; }
}
```

If implementations differ, the compiler will error on modules compiled after the first.

**Best Practice:** Use the same source file for shared entities.

## External Shared Entities

For entities already compiled in another module, use `external` modifier to reference them without re-declaring the implementation:

```angelscript
// Reference a shared class (no body needed)
external shared class Foo;

// Reference a shared function (no body needed)
external shared void GlobalFunc();

// Reference a shared interface
external shared interface IShared;

// Reference a shared enum
external shared enum SharedEnum;

// Reference a shared funcdef
external shared funcdef bool SharedCallback(int);
```

### Syntax Rules

The `external` keyword:
- Must come **before** `shared`
- Declaration ends with semicolon (no body)
- Entity must already exist in a compiled module

### Benefits

- **Shorter source code** - no need to duplicate implementation
- **Faster compilation** - compiler just looks up existing entity
- **No consistency errors** - can't accidentally have mismatched implementations

### Requirement

The entity **must already be compiled** in another module. If not found, a compiler error occurs:
```
Error: External shared entity 'Foo' not found
```

### Typical Pattern

```angelscript
// shared_types.as - Core module compiled first
shared class Message {
    string content;
    int priority;
}

// plugin.as - Plugin module compiled after core
external shared class Message;  // Reference the existing type

void HandleMessage(Message@ msg) {
    // Can use Message because it's externally referenced
}
```

## Use Cases

### Cross-Module Communication

```angelscript
// Shared type allows modules to exchange data
shared class Message {
    string content;
    int priority;
}

// Module A can create Message objects
// Module B can receive and process them
// Type is compatible because it's shared
```

### Plugin Systems

```angelscript
// Core module defines shared interface
shared interface IPlugin {
    void Initialize();
    void Update();
    string GetName();
}

// Plugin modules implement the interface
// Core can load any plugin implementing IPlugin
```

### Common Utilities

```angelscript
// Shared utility functions available to all modules
shared int Clamp(int value, int min, int max) {
    if (value < min) return min;
    if (value > max) return max;
    return value;
}
```

## Memory Benefits

Without sharing:
- Each module has its own copy of the class
- Different type identities (can't pass between modules)

With sharing:
- Single implementation in memory
- Same type identity across all modules
- Objects can be freely passed between modules
