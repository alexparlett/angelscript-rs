# Engine Architecture

## Core Components

### Script Engine (asIScriptEngine)

The central component that manages:
- Type registration (both reference and value types)
- Function registration
- Module compilation
- Context management
- Garbage collection

### Script Module (asIScriptModule)

An independent compilation unit containing:
- Script functions
- Global variables
- Classes
- Imports from other modules

**Key concept:** Each module has its own scope. Variables with the same name in different modules are independent.

### Script Context (asIScriptContext)

Represents an execution state:
- Call stack management
- Variable inspection
- Exception handling
- Can be suspended/resumed (coroutines)

One context per execution thread. Can be reused for multiple function calls.

## Memory Management

### Hybrid Approach

AngelScript uses **reference counting** as the primary mechanism, with **garbage collection** for backup.

**Reference Counting:**
- Objects track their own reference count
- Destroyed when count reaches zero
- Fast and deterministic

**Garbage Collection:**
- Only for types that can form circular references
- Incremental (doesn't halt execution)
- Uses mark-and-sweep algorithm

### GC Algorithm Steps

1. **Destroy garbage**: Free objects with only GC reference
2. **Clear counters**: Reset GC tracking counters
3. **Count GC references**: Track reachable objects
4. **Mark live objects**: Build list of alive objects
5. **Verify unmarked**: Double-check for concurrent changes
6. **Break circular refs**: Destroy unreachable cycles

### Generational GC

- **New generation**: Trivial GC only (destroy unreferenced)
- **Old generation**: Full cycle detection
- Objects promoted after surviving several iterations

## Type Categories

### Reference Types

- Allocated on heap
- Support object handles (`@`)
- Can outlive declaring scope
- All script classes are reference types

**Required behaviors:**
- `asBEHAVE_FACTORY` or `asBEHAVE_CONSTRUCT`
- `asBEHAVE_ADDREF` - increment ref count
- `asBEHAVE_RELEASE` - decrement and destroy if zero

### Value Types

- Allocated on stack (or inline in other objects)
- Cannot use handles
- Destroyed when scope ends
- Better for small, frequently-used types

**Required behaviors:**
- Constructor/destructor
- Copy behavior

### Type Flags

| Flag | Description |
|------|-------------|
| `asOBJ_REF` | Reference type |
| `asOBJ_VALUE` | Value type |
| `asOBJ_GC` | Participates in garbage collection |
| `asOBJ_TEMPLATE` | Template type |
| `asOBJ_NOCOUNT` | No reference counting (singleton-like) |

## Calling Conventions

> **C++ SPECIFIC:** The calling convention system is deeply tied to C++ ABI and platform-specific function calling.

### Native Calling (C++ Specific)

Direct function pointer calls using platform-specific ABI:
- `asCALL_CDECL` - C declaration calling convention
- `asCALL_STDCALL` - Standard call (Windows)
- `asCALL_THISCALL` - C++ class method (implicit this pointer)
- `asCALL_CDECL_OBJLAST` - Object pointer passed as last argument
- `asCALL_CDECL_OBJFIRST` - Object pointer passed as first argument

**C++ SPECIFIC:** These map directly to how C++ compilers generate function calls on different platforms (x86, x64, ARM). The engine must know the calling convention to correctly push arguments and retrieve return values.

### Generic Calling Convention

Portable fallback when native calling isn't available:

```cpp
// C++ SPECIFIC: The generic interface
void MyGenericFunction(asIScriptGeneric *gen) {
    int arg0 = gen->GetArgDWord(0);
    float arg1 = gen->GetArgFloat(1);
    // ... process ...
    gen->SetReturnDWord(result);
}
```

**When used in C++ impl:**
- Platforms without native calling convention support
- When `AS_MAX_PORTABILITY` is defined
- Complex parameter handling scenarios

## Template Types

Templates in AngelScript are **runtime-generic** (not compile-time like C++ templates).

> **C++ SPECIFIC:** The C++ implementation uses `asITypeInfo*` passed to factory functions to determine the subtype at runtime.

```cpp
// C++ registration example
engine->RegisterObjectType("array<class T>", 0, asOBJ_REF | asOBJ_TEMPLATE);

// Factory receives type info as hidden first parameter
myTemplate* ArrayFactory(asITypeInfo* typeInfo) {
    int subTypeId = typeInfo->GetSubTypeId();
    // Create appropriate instance based on subtype
}
```

**Key semantics:**
- Template callback validates instantiations at compile time
- Template specialization can override generic with optimized specific implementation
- Factory/constructor receives `asITypeInfo*` as hidden first parameter (declared as `int&in` in registration)

## Modules and Shared Entities

### Module Isolation

Each module is independent:
- Own global variables
- Own function implementations
- Own class definitions

### Cross-Module Communication

1. **Function binding**: Import functions from other modules
2. **Shared entities**: `shared` keyword for cross-module types
3. **Application proxy**: Through registered functions

### Shared Entity Rules

- Must be declared identically in all modules
- Cannot access non-shared entities
- `external shared` references already-compiled entities

## Object Handles Across Native Boundary

### Handle Conventions

When passing handles between script and application:

| Registration | Meaning |
|--------------|---------|
| `obj@` | Application manages reference counting |
| `obj@+` | Engine handles AddRef/Release automatically |

**Handle to const:** `const obj@` - can't modify object through this handle

> **C++ SPECIFIC:** The `@+` notation tells the engine to automatically call AddRef when receiving a handle and Release when the function returns, matching expected C++ pointer semantics.

## Bytecode

> **C++ SPECIFIC:** The bytecode format is defined by the C++ implementation. The instruction set is stack-based with 32-bit slots.

### Instruction Structure (C++ impl)

```cpp
struct asSBCInfo {
    asEBCInstr bc;    // Instruction ID (enum)
    asEBCType type;   // Argument layout type
    int stackInc;     // Stack effect (0xFFFF = variable based on args)
    const char* name; // Debug name
};
```

### Instruction Categories

- Stack operations (push, pop, dup)
- Arithmetic operations (add, sub, mul, div for each type)
- Comparison and branching
- Function calls (direct, virtual, interface)
- Object manipulation (construct, destruct, copy)
- Type conversions
- Handle operations

## Context Execution

### Call Stack

The context maintains:
- Local variables (stack-allocated)
- Return addresses
- Exception handlers
- Object registers

### Execution Control

> **C++ SPECIFIC:** These are methods on `asIScriptContext`.

- `Execute()` - run until completion or suspension
- `Suspend()` - pause for coroutine support
- `Abort()` - cancel execution
- `SetException()` - raise exception from application
- `GetExceptionInfo()` - inspect caught exceptions

### VM Registers (C++ impl)

```cpp
struct asSVMRegisters {
    asDWORD* stackFramePointer;  // Current stack frame
    asDWORD* stackPointer;       // Top of stack
    asDWORD* programPointer;     // Current instruction
    // ... additional registers
};
```

## Thread Safety

> **C++ SPECIFIC:** The C++ engine is NOT thread-safe by default.

- Engine can be shared between threads for read operations
- Each thread needs its own context for execution
- Global data modifications require synchronization
- GC behaviors must be thread-safe if used across threads
