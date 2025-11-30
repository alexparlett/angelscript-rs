# Current Task: Fix Ignored Tests

**Status:** In Progress
**Date:** 2025-11-30
**Phase:** Semantic Analysis - Bug Fixes & Missing Features

---

## Current State Summary

**Parser:** 100% Complete
- All AngelScript syntax supported
- 20 comprehensive test files
- Lambda parameter disambiguation with lookahead

**Semantic Analysis:** 99% Complete (with known issues)
- Pass 1 (Registration): 100% Complete
- Pass 2a (Type Compilation): 100% Complete
- Pass 2b (Function Compilation): 100% Complete
- Phase 1 (Type Conversions): Tasks 1-25 Complete
- Tasks 26-29 (Lambda Expressions): Complete
- Tasks 30-34 (TODO Cleanup): Complete
- Tasks 35-38: Namespace, Enum, Funcdef & Interface Validation Complete
- Task 41: Mixin Support Complete
- Task 46: Void Expression Validation Complete
- Task 47: Constant Expression Evaluation Complete
- Task 48: Circular Dependency Detection Complete
- Task 49: Visibility Enforcement Complete
- Task 50: `this` Keyword and Implicit Member Access Complete
- Task 51: Switch Statement Bytecode Generation Complete
- Task 52: Remove CreateArray + Add Initialization List Instructions Complete
- Task 53: Deferred (runtime concern)
- Task 54: Fix invalid test expectations (float->int implicit allowed)
- Task 55: Fix Type Conversion Issues (const types, signed<->unsigned)
- Task 56: Fix Function Overload Registration Issue - COMPLETE
- Tasks 57-65: Fix remaining ignored tests (see below)

**Test Status:** 1605 tests passing, 22 ignored (exposing real bugs)

---

## Ignored Test Fix Tasks

Analysis found 31 ignored tests in `function_processor.rs`. These are now separate tasks.

---

### Task 54: Fix Invalid Test Expectations - COMPLETE

**Issue:** AngelScript allows implicit float->int truncation. Tests incorrectly expected an error.

| Test | Line | Status |
|------|------|--------|
| `variable_init_explicit_cast_required_error` | 8341 | Fixed |
| `assignment_float_to_int_implicit_error` | 8457 | Fixed |

---

### Task 55: Fix Type Conversion Issues - COMPLETE

**Issue:** Int literals not converting to unsigned types, const references failing.

| Test | Line | Status |
|------|------|--------|
| `various_int_types` | 10259 | Fixed - `uint8 x = 255` now works |
| `const_reference_parameter` | 10622 | Fixed - `const int &in` works |
| `global_const_variable` | 10735 | Fixed - Global const access works |

**Fixes Applied:**
1. Removed incorrect `is_const` check from `primitive_conversion()` - const doesn't affect type convertibility
2. Added same-type-id identity conversion for types differing only in const qualifier
3. Added all signed-to-unsigned conversions for different sizes (e.g., int32 -> uint8)
4. Added all unsigned-to-signed conversions for different sizes (e.g., uint16 -> int32)

---

### Task 56: Fix Function Overload Registration - COMPLETE

**Issue:** Function overloads were not being registered correctly. Multiple overloads with the same name were being overwritten instead of each getting their own params.

| Test | Line | Status |
|------|------|--------|
| `funcdef_call` | 9612 | Fixed - `op(5, 3)` through funcdef |
| `overload_resolution_exact_match` | 12436 | Fixed - Resolution picks correct overload |
| `overloaded_function_call_exact_match` | 9923 | Fixed - All overloads work |

**Root Cause:** In `update_function_signature()`, the heuristic for finding "which overload to update" was broken for global functions. It used a check based on traits (`!is_virtual && !is_final && !is_const && !is_abstract`) which is always true for all global function overloads even after being updated.

**Fix Applied:** Added `signature_filled` field to `FunctionDef`:
- Functions start with `signature_filled: false` in Pass 1 (Registration)
- Pass 2a sets `signature_filled: true` when updating the signature
- `update_function_signature` now uses this flag to find the next un-filled overload

---

### Task 57: Fix Operator Overload Issues

**Issue:** Various operator overloads not being found or called correctly.

| Test | Line | Status |
|------|------|--------|
| `class_with_opAdd` | 9785 | `a + b` doesn't find opAdd |
| `class_with_op_index_multi` | 9941 | `m[2, 3]` multi-index |
| `class_with_op_call` | 9969 | opCall not recognized |
| `class_with_get_op_index` | 10281 | get_opIndex accessor |

**Action:** Fix operator lookup in `lookup_method_chain()` and `check_index_access()`.

---

### Task 58: Implement is/!is Operators

**Issue:** Handle identity comparison operators not implemented.

| Test | Line | Status |
|------|------|--------|
| `is_operator_same_handle` | 9636 | `a is b` |
| `is_not_operator` | 9687 | `a !is b` |
| `is_operator_null_check` | 9711 | `a is null` |
| `is_operator_derived_types` | 9731 | Derived handle comparison |

**Action:** Implement `is` and `!is` operators in expression checking.

---

### Task 59: Fix &out Parameter Lvalue Validation

**Issue:** `&out` parameters not validating lvalue requirement.

| Test | Line | Status |
|------|------|--------|
| `out_param_requires_lvalue_error` | 12688 | `f(5)` should error |
| `reference_out_param_with_literal_error` | 13069 | Same issue |

**Action:** Add lvalue validation for `&out` parameters in `check_call()`.

---

### Task 60: Fix Init List Issues

**Issue:** Array initialization with `{1, 2, 3}` syntax not working.

| Test | Line | Status |
|------|------|--------|
| `init_list_basic` | 8687 | `array<int> arr = {1, 2, 3}` |
| `init_list_empty` | 8887 | `array<int> arr = {}` |
| `init_list_multidimensional` | 9177 | Nested init lists |

**Action:** Fix `check_init_list()` to properly handle array initialization.

---

### Task 61: Fix Lambda Issues

**Issue:** Lambda captures and lambda as function arguments not working.

| Test | Line | Status |
|------|------|--------|
| `lambda_with_captures` | 9532 | Capture outer variables |
| `lambda_in_function_call` | 9609 | Lambda as function argument |

**Action:** Fix capture analysis and lambda-to-funcdef conversion.

---

### Task 62: Fix Property Accessor Issues

**Issue:** `get_X`/`set_X` pattern not recognized as property access.

| Test | Line | Status |
|------|------|--------|
| `property_accessor_basic` | 13579 | `obj.prop` -> `get_prop()` |
| `property_accessor_set` | 13621 | `obj.prop = x` -> `set_prop(x)` |

**Action:** Implement property accessor detection in member access.

---

### Task 63: Implement Auto Type Inference

**Issue:** `auto` type inference not implemented.

| Test | Line | Status |
|------|------|--------|
| `auto_type_inference` | 13977 | `auto x = expr;` |
| `auto_type_in_for_loop` | 14000 | `for (auto x : arr)` |
| `auto_with_handle` | 14068 | `auto@ h = @obj;` |
| `auto_with_const` | 14096 | `const auto x = 5;` |

**Action:** Implement auto type inference in variable declarations.

---

### Task 64: Fix Ternary with Handles

| Test | Line | Issue | Status |
|------|------|-------|--------|
| `ternary_conditional_handles` | 15163 | Handle type in ternary | Pending |

---

## Priority Order

1. **Task 54** - Fix invalid test expectations (DONE)
2. **Task 55** - Type conversion issues (DONE)
3. **Task 56** - Function overload registration (DONE)
4. **Task 57** - Operator overload issues
5. **Task 58** - is/!is operators
6. **Task 59** - &out validation
7. **Task 60** - Init list issues
8. **Task 61** - Lambda issues
9. **Task 62** - Property accessors
10. **Task 63** - Auto type inference
11. **Task 64** - Ternary with handles

---

## Test Status

```
1605 tests passing
22 tests ignored (exposing real bugs - tracked in Tasks 57-64 above)
```

---

## References

- **Full Details:** `/claude/semantic_analysis_plan.md`
- **Decisions Log:** `/claude/decisions.md`
- **C++ Reference:** `reference/angelscript/source/as_builder.cpp`, `as_compiler.cpp`

---

**Current Work:** Task 57 - Fix Operator Overload Issues
**Next Work:** Continue through priority list (Tasks 58-64)
