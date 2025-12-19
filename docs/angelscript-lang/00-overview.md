# AngelScript Language Reference

This directory contains converted documentation from the official AngelScript HTML docs, organized for use as Claude memory files when building this Rust implementation.

## Document Index

### Language Semantics (Universal)

| File | Description |
|------|-------------|
| [01-primitives.md](01-primitives.md) | Primitive data types (void, bool, integers, floats) |
| [02-objects-handles.md](02-objects-handles.md) | Objects, handles, and reference semantics |
| [03-statements.md](03-statements.md) | Statement types (if, while, for, switch, try-catch) |
| [04-expressions.md](04-expressions.md) | Expression types and evaluation |
| [05-operators.md](05-operators.md) | Operator precedence and behavior |
| [06-operator-overloads.md](06-operator-overloads.md) | Class operator overload methods |
| [07-classes.md](07-classes.md) | Script class definitions, inheritance, access modifiers |
| [08-functions.md](08-functions.md) | Function declarations, parameters, overloading |
| [09-type-conversions.md](09-type-conversions.md) | Implicit and explicit type conversions |
| [10-globals.md](10-globals.md) | Enums, interfaces, namespaces, mixins, funcdefs |
| [11-datatypes-advanced.md](11-datatypes-advanced.md) | Strings, arrays, function handles, auto, lambdas |
| [12-shared.md](12-shared.md) | Shared entities across modules |

### C++ Implementation Reference

| File | Description |
|------|-------------|
| [cpp-specifics.md](cpp-specifics.md) | Calling conventions, behaviors, platform-specific notes |
| [cpp-engine-architecture.md](cpp-engine-architecture.md) | Memory management, GC algorithm, modules, bytecode |
| [cpp-type-registration.md](cpp-type-registration.md) | Registering types, behaviors, methods, properties (C++ API) |

## Source

Original documentation from: https://www.angelcode.com/angelscript/sdk/docs/manual/

## Usage

These files can be referenced by Claude Code during development to understand AngelScript language semantics without needing to parse the HTML docs each session.

**Note:** Documents marked with `> **C++ SPECIFIC:**` contain implementation details specific to the C++ reference implementation. Language semantics (operator names, type categories, etc.) are universal.

## Quick Reference

### Type System
- **Primitives:** bool, int8/16/32/64, uint8/16/32/64, float, double
- **Objects:** Reference types (heap-allocated, ref-counted)
- **Value types:** Stack-allocated (application-registered only)
- **Handles:** `@` suffix for reference handles

### Key Syntax
- Handle declaration: `obj@ handle`
- Handle assignment: `@handle = @obj`
- Identity check: `a is b`, `a !is null`
- Reference params: `&in`, `&out`, `&inout` (or just `&`)
- Scope resolution: `Namespace::item`, `::global`

### Operator Methods
- Assignment: `opAssign`
- Comparison: `opEquals`, `opCmp`
- Binary: `opAdd`, `opSub`, `opMul`, etc. (and `_r` variants)
- Unary: `opNeg`, `opCom`, `opPreInc`, etc.
- Index: `opIndex` or `get_opIndex`/`set_opIndex`
- Conversion: `opConv`, `opImplConv`, `opCast`, `opImplCast`

### Type Registration Behaviors
| Behavior | Purpose |
|----------|---------|
| `asBEHAVE_FACTORY` | Create reference type instance |
| `asBEHAVE_CONSTRUCT` | Construct value type in-place |
| `asBEHAVE_DESTRUCT` | Destroy value type |
| `asBEHAVE_ADDREF` | Increment reference count |
| `asBEHAVE_RELEASE` | Decrement/destroy on zero |
| `asBEHAVE_LIST_FACTORY` | Create from initializer list |
| `asBEHAVE_TEMPLATE_CALLBACK` | Validate template instantiation |

### Memory Management
- **Reference counting:** Primary mechanism
- **Garbage collection:** Backup for circular references
- **GC is incremental:** Runs in small steps, doesn't halt
- Objects can opt-out of GC with `asOBJ_NOCOUNT`
