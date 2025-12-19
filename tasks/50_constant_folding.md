# Task 50: Constant Folding Optimization

## Overview

Implement compile-time evaluation of constant expressions to reduce bytecode size and improve runtime performance.

## Goals

1. Fold binary operations on literals (`1 + 2` → `3`)
2. Fold unary operations on literals (`-42`, `!true`)
3. Fold comparison operations (`5 < 3` → `false`)
4. Propagate const variable values

## Priority

LOW - Optimization task, not required for correctness.

## Dependencies

- Task 43b: Assignment Expressions (compiler must be functionally complete first)

## Current State

No constant folding exists. All operations emit bytecode even when operands are known at compile time.

**Example:** `1 + 2` currently emits:
```
PushOne
Push(2)
Add
```

Should emit:
```
Push(3)
```

## Opportunities

| Category | Example | Savings |
|----------|---------|---------|
| Arithmetic | `3 + 4` → `7` | 2 opcodes |
| Unary | `-42` → `-42` | 1 opcode |
| Comparison | `5 < 3` → `false` | 2 opcodes |
| Bitwise | `0xFF & 0x0F` → `0x0F` | 2 opcodes |
| Boolean | `true && false` → `false` | 2 opcodes |

## Files to Modify

```
crates/angelscript-compiler/src/expr/
├── fold.rs       # NEW: Constant folding logic
├── binary.rs     # Check for foldable operands
├── unary.rs      # Check for foldable operand
└── mod.rs        # Integration
```

## Implementation Sketch

```rust
// In fold.rs
pub fn try_fold_binary(op: BinaryOp, left: &Expr, right: &Expr) -> Option<Literal> {
    let left_val = as_literal(left)?;
    let right_val = as_literal(right)?;

    match (op, left_val, right_val) {
        (BinaryOp::Add, Literal::Int(a), Literal::Int(b)) => {
            Some(Literal::Int(a.checked_add(b)?))
        }
        // ... other cases
        _ => None,
    }
}
```

## Edge Cases

1. **Overflow**: `i32::MAX + 1` - should not fold (runtime semantics)
2. **Division by zero**: `5 / 0` - should not fold
3. **Float precision**: May differ from runtime
4. **String concatenation**: Large strings could bloat constant pool

## Testing

```rust
#[test]
fn fold_int_addition() {
    // 1 + 2 should emit Push(3), not Push(1) Push(2) Add
}

#[test]
fn no_fold_overflow() {
    // i32::MAX + 1 should NOT fold (preserve runtime overflow behavior)
}
```

## Acceptance Criteria

- [ ] Binary operations on int/float literals fold
- [ ] Unary operations on literals fold
- [ ] Comparison operations fold to bool
- [ ] Overflow/error cases are NOT folded
- [ ] Bytecode size reduced for constant expressions
- [ ] All existing tests pass

## Notes

- This is a pure optimization with no semantic changes
- Should be implemented after compiler is functionally complete
- Can be expanded later with constant propagation
