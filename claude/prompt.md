# Current Task: FFI & Enhanced Bytecode

**Status:** Planning
**Date:** 2025-12-02
**Phase:** Post-Semantic Analysis

---

## Current State Summary

**Parser:** 100% Complete
**Semantic Analysis:** 100% Complete
**Test Status:** 1656 tests passing, 0 ignored

---

## Upcoming Tasks

### Task A: FFI (Foreign Function Interface)

Implement the ability to register and call native Rust functions from AngelScript scripts.

**Requirements:**
1. Register native functions with type signatures
2. Call native functions from script code
3. Pass arguments from script to native
4. Return values from native to script
5. Handle reference parameters (`&in`, `&out`, `&inout`)
6. Register native types/classes with methods and properties

**Example API:**
```rust
let mut engine = ScriptEngine::new();

// Register a native function
engine.register_fn("print", |s: &str| println!("{}", s));
engine.register_fn("sqrt", |x: f64| x.sqrt());

// Register a native type
engine.register_type::<Vec3>("Vec3")
    .with_constructor(Vec3::new)
    .with_method("length", Vec3::length)
    .with_property("x", Vec3::get_x, Vec3::set_x);
```

**AngelScript usage:**
```angelscript
void main() {
    print("Hello from script!");
    float x = sqrt(16.0);

    Vec3 v(1, 2, 3);
    float len = v.length();
}
```

---

### Task B: Enhanced Bytecode

Improve bytecode for better runtime execution:

1. **Constant folding** - Evaluate constant expressions at compile time
2. **Dead code elimination** - Remove unreachable code
3. **Register allocation** - Optimize local variable storage
4. **Instruction optimization** - Combine redundant instructions

**Example optimizations:**
```angelscript
// Before optimization:
int x = 2 + 3;        // LoadConst 2, LoadConst 3, Add
int y = x * 2;

// After constant folding:
int x = 5;            // LoadConst 5
int y = x * 2;
```

---

## Priority Order

1. **Task A: FFI** - Essential for any practical use
2. **Task B: Enhanced Bytecode** - Performance improvements

---

## References

- **Full Details:** `/claude/semantic_analysis_plan.md`
- **Decisions Log:** `/claude/decisions.md`
- **Bytecode:** `src/semantic/bytecode/instruction.rs`
