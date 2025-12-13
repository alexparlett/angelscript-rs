# Current Task: Const-Correctness Implementation (Task 41d)

**Status:** ✅ Complete
**Date:** 2025-12-13
**Branch:** 041-expression-basics

---

## Summary

Implemented Task 41d: Const-correctness checks across the compiler. Non-const methods cannot be called on const objects, and this is now enforced during operator resolution and conversion method resolution.

### What Was Done

1. **Added `is_effectively_const()` helper to `DataType`** ✅
   - Returns true if `is_const || is_handle_to_const`
   - Centralized check for const-correctness validation
   - Location: `crates/angelscript-core/src/data_type.rs:400-429`

2. **Const-Correctness for Binary Operators** ✅
   - Added check in `try_operator_on_type()`
   - Non-const operator methods are skipped for const objects
   - Location: `crates/angelscript-compiler/src/operators/binary.rs:199-202`

3. **Const-Correctness for Unary Operators** ✅
   - Added check in `try_user_defined_unary()`
   - Non-const operator methods are skipped for const objects
   - Location: `crates/angelscript-compiler/src/operators/unary.rs:88-91`

4. **Const-Correctness for Conversion Methods** ✅
   - Updated `find_user_conversion()` to take `&DataType` instead of `TypeHash`
   - Updated `find_implicit_conv_method()` to take `&DataType` for source and target
   - Updated `find_cast_method()` to take `&DataType` for source and target
   - Non-const `opImplConv`/`opCast` methods are skipped for const objects
   - Location: `crates/angelscript-compiler/src/conversion/user_defined.rs`

5. **Updated Task 42 Requirements** ✅
   - Added const-correctness requirements section
   - Updated all code examples to use `is_effectively_const()`
   - Method calls, property writes, assignments, and reference parameters covered

### Files Modified

- **[data_type.rs](crates/angelscript-core/src/data_type.rs#L400-L429)**: Added `is_effectively_const()` + tests
- **[binary.rs](crates/angelscript-compiler/src/operators/binary.rs#L199-L202)**: Added const check
- **[unary.rs](crates/angelscript-compiler/src/operators/unary.rs#L88-L91)**: Added const check
- **[user_defined.rs](crates/angelscript-compiler/src/conversion/user_defined.rs)**: Updated signatures + added const checks
- **[mod.rs](crates/angelscript-compiler/src/conversion/mod.rs#L251)**: Updated call to `find_user_conversion`
- **[42_expression_calls.md](claude/tasks/42_expression_calls.md)**: Added const-correctness requirements

### Testing

All 326 tests pass ✅
No clippy warnings ✅

---

## Const-Correctness Architecture

### Helper Method
```rust
impl DataType {
    pub const fn is_effectively_const(&self) -> bool {
        self.is_const || self.is_handle_to_const
    }
}
```

### Check Pattern
```rust
// Non-const methods cannot be called on const objects
if obj_type.is_effectively_const() && !func_entry.def.is_const() {
    continue; // or return error
}
```

### Where Const-Correctness Is Checked

| Location | Status | Description |
|----------|--------|-------------|
| Binary operators | ✅ Task 41d | User-defined `opAdd`, `opMul`, etc. |
| Unary operators | ✅ Task 41d | User-defined `opNeg`, `opCom`, etc. |
| Conversion methods | ✅ Task 41d | `opImplConv`, `opCast`, converting constructors |
| Method calls | 📋 Task 42 | `obj.method()` |
| Property writes | 📋 Task 42 | `obj.field = x` |
| Assignment | 📋 Task 42 | Uses `ExprInfo.is_mutable` |
| Reference params | 📋 Task 42 | Handled by conversion system |

---

## Next Steps

**Immediate:**
- Task 42: Expression Compilation - Calls (function calls, method calls, member access)

**Future:**
- Task 43+: Statement compilation
- Task 46: Function body compilation

---

## Context for Next Session

### Completed Work
- ✅ Task 41b: Type Completion Pass
- ✅ Task 41c Phase 1: Inheritance validation (FFI + final checks)
- ✅ Task 41d: Const-correctness implementation
- ✅ All 326 tests passing

### Current State
- `DataType::is_effectively_const()` available for const checks
- Binary/unary operators check const-correctness
- Conversion methods check const-correctness
- Task 42 has requirements for remaining const-correctness (method calls, properties)
- Ready to continue with Task 42 (expression calls)
