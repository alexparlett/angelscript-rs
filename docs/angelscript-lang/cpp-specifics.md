# C++ Specifics in AngelScript

This document notes C++ implementation details from the original AngelScript engine that may not apply to the Rust implementation.

## Application Registration

The original C++ implementation provides APIs for the host application to:

- Register custom types (both reference and value types)
- Register global functions and properties
- Register type behaviors (constructors, destructors, factories, operators)
- Define how objects are passed to/from script (by value, by reference, etc.)

### C++ Calling Conventions

The C++ engine handles different calling conventions:
- `cdecl`, `stdcall`, `thiscall`
- Generic calling convention for portability
- Platform-specific native calling

**Rust equivalent:** Would need FFI bindings or a Rust-native API.

## Type Registration

### Value Types vs Reference Types

In C++, the distinction matters for:
- Memory layout compatibility with C++ structures
- How the type is passed across the script/native boundary
- Who manages memory (script or application)

### Object Behaviors (C++ Engine)

The C++ engine uses behavior IDs to register special type operations:

| Behavior | Description |
|----------|-------------|
| `asBEHAVE_CONSTRUCT` | Constructor |
| `asBEHAVE_DESTRUCT` | Destructor |
| `asBEHAVE_FACTORY` | Factory function (for reference types) |
| `asBEHAVE_ADDREF` | Increment reference count |
| `asBEHAVE_RELEASE` | Decrement reference count |
| `asBEHAVE_GET_WEAKREF_FLAG` | For weak references |

**Rust equivalent:** These would be Rust traits or function pointers.

## Memory Management

### Reference Counting

The C++ engine uses manual reference counting:
- `AddRef()` - increment count
- `Release()` - decrement and destroy if zero

**Rust equivalent:** `Rc<T>`, `Arc<T>`, or custom reference counting.

### Garbage Collection

The C++ engine has optional garbage collection for circular references:
- GC behaviors registered on types
- Periodic sweep to find unreachable cycles

**Rust equivalent:** Could use `Rc<RefCell<T>>` with weak references, or a tracing GC.

## Native Calling

### Generic vs Native Calls

The C++ engine supports two ways to call native functions:
1. **Native calling** - Direct function pointer call (platform-specific)
2. **Generic calling** - Wrapper that manually pushes/pops arguments

**Rust equivalent:** Direct Rust function calls, potentially with FFI for C libraries.

### Wrapper Functions

C++ often needs wrapper functions to:
- Convert between script types and C++ types
- Handle different calling conventions
- Manage reference counting at boundaries

## Engine Configuration

### asIScriptEngine

The main C++ engine interface provides:
- Module management
- Type registration
- Global property registration
- Configuration options

**Rust equivalent:** Would be a Rust struct with similar methods.

### Configuration Options

C++ engine options that may or may not apply:

| Option | Description |
|--------|-------------|
| `asEP_ALLOW_UNSAFE_REFERENCES` | Allow dangerous reference patterns |
| `asEP_OPTIMIZE_BYTECODE` | Enable bytecode optimization |
| `asEP_BUILD_WITHOUT_LINE_CUES` | Smaller bytecode without debug info |
| `asEP_INIT_GLOBAL_VARS_AFTER_BUILD` | When to initialize globals |
| `asEP_REQUIRE_ENUM_SCOPE` | Require `EnumType::value` syntax |
| `asEP_PROPERTY_ACCESSOR_MODE` | Control property accessor feature |

## Bytecode

### C++ Bytecode Format

The C++ engine compiles to bytecode that can be:
- Saved to file
- Loaded without recompilation
- Inspected/modified

**Rust equivalent:** Could use same format for compatibility, or define a new one.

### JIT Compilation

The C++ engine supports JIT compilation through an interface:
- Script provides JIT compiler implementation
- Engine calls JIT for compiled functions

## Contexts

### asIScriptContext

Represents a script execution context:
- Call stack management
- Variable inspection
- Exception handling
- Debugging support

## Thread Safety

The C++ engine is **not thread-safe** by default:
- One context per thread
- Global data requires synchronization
- Context switching between threads requires care

**Rust equivalent:** Could leverage Rust's thread safety guarantees.

## Differences from Rust Implementation

When building a Rust AngelScript implementation, consider:

1. **Memory safety** - Rust eliminates many C++ pitfalls
2. **Reference counting** - Use Rust's `Rc`/`Arc` instead of manual counting
3. **Calling conventions** - Rust has simpler FFI requirements
4. **Error handling** - Use `Result<T, E>` instead of error codes
5. **Generics** - Rust generics vs C++ templates
6. **Type traits** - Use Rust traits for behavior contracts

## What to Keep

Language semantics that should be preserved:
- Object handle semantics (`@`)
- Reference modifiers (`&in`, `&out`, `&inout`)
- Operator overload method names (`opAdd`, etc.)
- Function overload resolution rules
- Implicit conversion precedence
- Class inheritance semantics
- Interface implementation

## What Could Change

Implementation details that could differ:
- Bytecode format
- Internal type representation
- Memory layout
- Error message format
- Debug information format
- Module loading mechanism
