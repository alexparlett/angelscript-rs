# Virtual Machine Design

> **Status**: INITIAL DRAFT - Design decisions pending

## Overview

This document covers the VM execution and memory management.

## Execution Model

**Decision Pending**: Pure stack-based (current bytecode design) vs hybrid with registers

Current bytecode is stack-based:
- All operations push/pop from value stack
- No explicit registers
- Simpler but potentially less efficient than C++ AngelScript's hybrid approach

## Instruction Set Status

Our bytecode uses generic instructions (single `Add` vs type-specific `ADDi`, `ADDf`).

**Trade-off:**
- Generic: Simpler bytecode, VM must track types at runtime
- Type-specific: Larger instruction set, faster dispatch

**Current decision**: Stay generic for simplicity, optimize later if needed.

### Missing Instructions (vs C++ AngelScript)

**Critical for functionality:**
| Category | Instructions | Purpose |
|----------|-------------|---------|
| FFI Calls | `CallSystem` | Call registered Rust host functions |
| Null Safety | `CheckNull` | Runtime null pointer checks (compiler also warns) |
| Object Lifecycle | `IncRef`, `DecRef` | Reference counting |
| Switch | `JumpSwitch` | Jump table for efficient switch dispatch (optional optimization) |

**May not need (architectural differences):**
- Register-based ops (`CpyVtoR`, `CpyRtoV`, etc.) - we're pure stack
- Immediate operand variants - optimization, not required

### Instructions We Have That C++ Doesn't
| Instruction | Purpose |
|-------------|---------|
| `LogicalAnd`, `LogicalOr`, `LogicalXor` | Explicit logical operators (C++ uses JZ/JNZ) |
| `PostIncrement`, `PostDecrement` | Separate post-increment ops |
| `LoadField`, `StoreField` | Direct field access |
| `Dup` | Stack duplication |
| Extensive `Convert*` variants | 56 conversion instructions |

### Instructions To Remove
| Instruction | Reason |
|-------------|--------|
| `CreateArray` | Use `CallConstructor` for `array<T>` instead |
| `Index`, `StoreIndex` | Use method calls to `opIndex` instead |

---

## Memory Management

### Reference Counting

**Decision**: Use reference counting (similar to C++ AngelScript)

Implementation approach:
- Objects in `Runtime::ObjectPool` have refcount
- `IncRef` instruction: increment count
- `DecRef` instruction: decrement, free if zero
- Handle assignment automatically adjusts refcounts

### When Refcount Changes
- Handle assignment: `DecRef` old, `IncRef` new
- Function parameter passing: `IncRef` on entry (for handles)
- Function return: Caller takes ownership
- Scope exit: `DecRef` all handles in scope

### Object Pool
From architecture.md - objects stored in `Runtime::ObjectPool`:
- Generational handles prevent dangling references
- Pool recycles slots for efficiency

---

## Open Questions

1. **Coroutines/Suspension**: Support `SUSPEND` for async scripts?
2. **Debug Info**: Embed line numbers for stack traces?
3. **JIT**: Consider JIT compilation path?

---

## References
- [architecture.md](../docs/architecture.md) - Overall system design
- C++ AngelScript bytecode: `reference/angelscript/include/angelscript.h:1434-1646`
