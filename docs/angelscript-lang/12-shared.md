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

For entities already compiled in another module, use `external`:

```angelscript
external shared class Foo;
external shared void GlobalFunc();
external shared interface IShared;
```

Benefits:
- Shorter source code
- Faster compilation
- No need to duplicate implementation

**Requirement:** The entity must already be compiled in another module, or a compiler error occurs.

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
