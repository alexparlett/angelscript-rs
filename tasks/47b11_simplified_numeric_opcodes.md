# Task 47b11: Simplified Numeric Opcodes

## Overview

Simplify the bytecode by using generic numeric opcodes (`Add`, `Sub`, `Mul`, `Div`, `Mod`) instead of type-specific variants (`AddI32`, `AddI64`, `AddF32`, etc.). The VM determines the correct operation at runtime based on the Rust types on the stack.

## Current State

Currently we have many type-specific opcodes:
- `AddI32`, `AddI64`, `AddF32`, `AddF64`
- `SubI32`, `SubI64`, `SubF32`, `SubF64`
- `MulI32`, `MulI64`, `MulF32`, `MulF64`
- `DivI32`, `DivI64`, `DivF32`, `DivF64`
- `ModI32`, `ModI64`
- etc.

## Target State

Simple generic opcodes:
- `Add` - works for all numeric types
- `Sub` - works for all numeric types
- `Mul` - works for all numeric types
- `Div` - works for all numeric types (signed/unsigned handled by Rust type)
- `Mod` - works for all integer types

## Why This Works

The VM stack holds `Value` enums with Rust primitives inside:
```rust
enum Value {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    // ...
}
```

The VM matches on the actual types and Rust handles the semantics:
```rust
OpCode::Div => {
    match (stack.pop(), stack.pop()) {
        (Value::I32(b), Value::I32(a)) => stack.push(Value::I32(a / b)),  // signed div
        (Value::U32(b), Value::U32(a)) => stack.push(Value::U32(a / b)),  // unsigned div
        (Value::F32(b), Value::F32(a)) => stack.push(Value::F32(a / b)),  // float div
        // ...
    }
}
```

## Implementation Steps

### 1. Update OpCode enum
In `crates/angelscript-compiler/src/bytecode/opcode.rs`:
- Replace `AddI32`, `AddI64`, `AddF32`, `AddF64` with single `Add`
- Replace `SubI32`, `SubI64`, `SubF32`, `SubF64` with single `Sub`
- Replace `MulI32`, `MulI64`, `MulF32`, `MulF64` with single `Mul`
- Replace `DivI32`, `DivI64`, `DivF32`, `DivF64` with single `Div`
- Replace `ModI32`, `ModI64` with single `Mod`
- Similarly for comparison ops: `Lt`, `Le`, `Gt`, `Ge`, `Eq`, `Ne`
- Similarly for bitwise ops: `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`
- Similarly for unary ops: `Neg`, `BitNot`

### 2. Update primitive operator resolution
In `crates/angelscript-compiler/src/operators/primitive.rs`:
- Simplify `arithmetic_opcode()` to return generic opcodes
- Remove type-specific opcode selection
- Keep type promotion logic (compiler still needs to know result type)
- Remove conversion opcode tracking (VM handles mixed types)

### 3. Update OperatorResolution
In `crates/angelscript-compiler/src/operators/mod.rs`:
- Simplify `OperatorResolution::Primitive` - no longer needs `left_conv`/`right_conv`
- Just needs `opcode` and `result_type`

### 4. Update bytecode emission
Update any code that emits arithmetic opcodes to use the new generic versions.

### 5. Update VM (if exists)
If there's a VM implementation, update it to match on `Value` types for generic opcodes.

## Files to Modify

- `crates/angelscript-compiler/src/bytecode/opcode.rs` - Simplify OpCode enum
- `crates/angelscript-compiler/src/operators/primitive.rs` - Use generic opcodes
- `crates/angelscript-compiler/src/operators/mod.rs` - Simplify OperatorResolution

## Benefits

1. **Simpler bytecode** - Fewer opcodes to maintain
2. **Simpler compiler** - No type-specific opcode selection
3. **Correct semantics** - Rust handles signed/unsigned/float correctly
4. **Easier extension** - Adding new numeric types doesn't require new opcodes

## Acceptance Criteria

- [ ] Generic `Add`, `Sub`, `Mul`, `Div`, `Mod` opcodes
- [ ] Generic comparison opcodes
- [ ] Generic bitwise opcodes
- [ ] Compiler emits generic opcodes
- [ ] All existing tests pass
- [ ] `utilities.as` compiles (uint/int mixed operations work)
