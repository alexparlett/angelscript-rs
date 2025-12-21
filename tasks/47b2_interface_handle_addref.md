# Task 47b2: Interface Handle Addref Validation

## Problem

Taking a handle to an object and assigning to an interface handle fails with:

**Error:** `addref behavior only valid for class types`

**Affected Test:** `test_interface`

## Root Cause

In `crates/angelscript-compiler/src/stmt/mod.rs:208-212`:

```rust
let Some(class) = type_entry.as_class() else {
    return Err(CompilationError::Other {
        message: "addref behavior only valid for class types".to_string(),
        span,
    });
};
```

When we have an interface handle (`IDrawable@`), `as_class()` returns `None` because interfaces are stored as `InterfaceEntry`, not `ClassEntry`.

## Context

The test does:
```angelscript
GameObject obj;
IDrawable@ drawable = @obj;  // Taking handle of obj, assigning to interface handle
```

When compiling `@obj` (handle-of expression), the compiler needs to emit addref.

## Design Question

**Is this actually a compiler concern or a VM concern?**

The compiler shouldn't need to look up specific addref function hashes. Instead:
- Compiler emits a generic "addref what's on the stack" instruction
- VM inspects the value on the stack and calls the appropriate addref

This would simplify the compiler - it doesn't need to know the specific addref behavior, just that addref needs to happen.

## Solution

The compiler must emit a generic addref opcode. It cannot resolve the specific addref function at compile time because:

1. Interface handles point to concrete objects whose type is only known at runtime
2. The addref must happen on the **actual object in memory**, not on the interface
3. The VM has the runtime type information needed to call the correct addref

### Implementation

Instead of `emit_call(addref_function_hash, 1)`, emit a dedicated opcode:

```rust
// In bytecode emission
pub fn emit_addref(&mut self) {
    self.emit(OpCode::AddRef);  // VM figures out which addref to call
}

pub fn emit_release(&mut self) {
    self.emit(OpCode::Release);
}
```

The VM then:
1. Inspects the value on the stack (gets the actual object pointer)
2. Looks up the object's concrete type (stored in object header or similar)
3. Calls the appropriate addref for that type

This is the only correct approach - the compiler cannot know the concrete type behind an interface handle.

## Files to Modify

- `crates/angelscript-compiler/src/bytecode/opcode.rs` - Add AddRef/Release opcodes
- `crates/angelscript-compiler/src/emit/mod.rs` - Emit generic opcodes instead of Call
- `crates/angelscript-compiler/src/stmt/mod.rs` - Remove `get_addref_behavior()` or simplify it
- VM code (future) - Handle the opcodes at runtime

## Test Case

```angelscript
interface IDrawable {
    void draw();
}

class GameObject : IDrawable {
    void draw() { print("Drawing"); }
}

void test() {
    GameObject obj;
    IDrawable@ drawable = @obj;  // Should work
    drawable.draw();
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_interface` passes
- [ ] Interface handle assignment from implementing class works
- [ ] Addref/release are properly emitted for interface handles
- [ ] No regression in class handle tests
