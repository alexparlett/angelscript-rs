# Current Task: Bytecode Emitter (Task 40)

**Status:** Complete
**Date:** 2025-12-10
**Branch:** 040-bytecode-emitter

---

## Task 40: Bytecode Emitter

Implemented the bytecode emitter in `crates/angelscript-compiler/src/emit/`.

### Implementation Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| `BytecodeEmitter` | `emit/mod.rs` | High-level API for bytecode generation |
| `JumpManager` | `emit/jumps.rs` | Loop context tracking for break/continue |
| `JumpLabel` | `emit/mod.rs` | Forward jump targets for patching |
| `BreakError` | `emit/mod.rs` | Error type for break/continue outside loops |

### Key Features

- **Constants**: `emit_int()`, `emit_f32()`, `emit_f64()`, `emit_string()`, `emit_bool()`, `emit_null()`
  - Optimizes 0 and 1 to `PushZero`/`PushOne` opcodes
  - Uses narrow (8-bit) or wide (16-bit) constant indices based on pool size
- **Local Variables**: `emit_get_local()`, `emit_set_local()` with auto-wide selection
- **Global Variables**: `emit_get_global()`, `emit_set_global()` by TypeHash
- **Function Calls**: `emit_call()`, `emit_call_method()`, `emit_call_virtual()`
- **Object Operations**: `emit_new()`, `emit_new_factory()`, `emit_get_field()`, `emit_set_field()`, `emit_get_this()`
- **Type Operations**: `emit_conversion()`, `emit_cast()`, `emit_instanceof()`
- **Control Flow**: `emit_jump()`, `patch_jump()`, `emit_loop()`
- **Loop Control**: `enter_loop()`, `exit_loop()`, `emit_break()`, `emit_continue()`
- **Stack Operations**: `emit_pop()`, `emit_pop_n()`, `emit_dup()`
- **Reference Counting**: `emit_add_ref()`, `emit_release()`
- **Function Pointers**: `emit_func_ptr()`, `emit_call_func_ptr()`
- **Init Lists**: `emit_init_list_begin()`, `emit_init_list_end()`
- **Debug Info**: `set_line()` for source line tracking

### Tests

39 unit tests covering:
- Constant emission (int, float, string, bool, null)
- Special int optimization (0, 1)
- Constant deduplication
- Jump and patch mechanics
- Loop break/continue
- Nested loops
- Break/continue outside loop errors
- Wide constant indices (256+ constants)
- Local variable access (narrow and wide)
- Field access
- Function calls
- Type operations
- Stack operations

---

## Next Steps

- Task 41: Expression Compilation - Basics (literals, identifiers, binary ops)
