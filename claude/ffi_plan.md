# Foreign Function Interface (FFI) Design

> **Status**: INITIAL DRAFT - Design decisions pending

## Overview

This document covers how Rust host functions are registered and called from scripts.

## Host Function Registration

**Decision Pending**: Registration API design

Options to consider:
1. **Macro-based** (like Rhai's `#[export_fn]`)
2. **Builder pattern** (current architecture.md design)
3. **Trait-based** (implement trait for callable types)

Current architecture.md shows builder pattern:
```rust
engine.register_fn("print", |s: &str| println!("{}", s));
engine.register_fn("sqrt", f64::sqrt);
```

## Calling Convention

**Decision Pending**: How `CallSystem` instruction invokes Rust functions

Considerations:
- Type conversion: `Value` → Rust types
- Error handling: Rust `Result` → script exceptions
- Runtime access: Some functions need `&mut Runtime`

### CallSystem Instruction
Need to add `CallSystem(FunctionId)` instruction that:
1. Looks up registered function by ID
2. Pops arguments from stack
3. Converts `Value` → Rust types
4. Invokes Rust function
5. Converts return value → `Value`
6. Pushes result onto stack

## Type Marshalling

| Script Type | Rust Type |
|-------------|-----------|
| `int` | `i32` |
| `int64` | `i64` |
| `uint` | `u32` |
| `float` | `f32` |
| `double` | `f64` |
| `bool` | `bool` |
| `string` | `String` or `&str` |
| `T@` (handle) | `Handle` → lookup in Runtime |

## Type Registration

From architecture.md - types registered via builder:
```rust
engine.register_type::<Vec3>("Vec3")
    .value_type()
    .constructor(|x, y, z| Vec3::new(x, y, z))
    .property("x", |v| v.x, |v, x| v.x = x)
    .method("length", |v| v.length())
    .build()?;
```

### Value Types vs Reference Types
| Kind | Storage | Example |
|------|---------|---------|
| Value | Inline in `Value` | `Vec3`, primitives |
| Reference | Handle into ObjectPool | `AiAgent`, script classes |

## Open Questions

1. **Async functions**: Support `async fn` registration?
2. **Generics**: How to handle generic Rust functions?
3. **Lifetimes**: How to handle `&str` vs `String`?

---

## References
- [architecture.md](../docs/architecture.md) - Registration API design
- [vm_plan.md](vm_plan.md) - VM execution model
