# Current Task: Fix Remaining Ignored Tests

**Status:** In Progress
**Date:** 2025-12-01
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
- Task 54: Fix invalid test expectations (float->int implicit allowed) - COMPLETE
- Task 55: Fix Type Conversion Issues (const types, signed<->unsigned) - COMPLETE
- Task 56: Fix Function Overload Registration Issue - COMPLETE
- Task 57: Fix Operator Overload Issues (opAdd, opCall, get_opIndex) - COMPLETE
- Task 58: Implement is/!is Operators - COMPLETE
- Task 59: Fix &out Parameter Lvalue Validation - COMPLETE
- Task 60: Fix Init List Issues - COMPLETE
- Task 61: Fix Lambda Issues - COMPLETE
- Task 62: Fix Property Accessor Issues - COMPLETE
- Task 63: Implement Auto Type Inference - COMPLETE
- Tasks 64-70: Fix remaining ignored tests (see below)

**Test Status:** 1644 tests passing, 7 ignored (exposing real bugs)

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

### Task 57: Fix Operator Overload Issues - COMPLETE

**Issue:** Operator methods (opAdd, opCall, get_opIndex) not being registered or found correctly.

| Test | Line | Status |
|------|------|--------|
| `class_with_opAdd` | 9576 | Fixed - `a + b` finds opAdd |
| `class_with_op_call` | 12335 | Fixed - `f(5)` uses opCall |
| `class_with_get_op_index` | 13079 | Fixed - `arr[5]` uses get_opIndex |

**Fixes Applied:**
1. `parse_operator_method()` now uses `OperatorBehavior::from_method_name()` to recognize all operator methods (was only handling conversion operators)
2. `check_call()` now checks if a local variable's type has `opCall` before falling through to function lookup
3. `get_opIndex` is now registered via the canonical `from_method_name()` path

---

### Task 58: Implement is/!is Operators - COMPLETE

**Issue:** Handle identity comparison operators not implemented.

| Test | Line | Status |
|------|------|--------|
| `handle_comparison` | 8953 | Fixed - `a is b` and `a !is b` work |

**Fixes Applied:**
1. Added type checking in `check_binary_expression()` for `Is` and `NotIs` operators
2. Both operands must be handles or null (`NULL_TYPE`)
3. Reuses `Equal`/`NotEqual` instructions for pointer comparison (no new instruction needed)
4. Returns `bool` type

---

### Task 59: Fix &out Parameter Lvalue Validation - COMPLETE

**Issue:** `&out` parameters not validating lvalue requirement.

| Test | Line | Status |
|------|------|--------|
| `out_param_requires_lvalue_error` | 12790 | Fixed - `f(5+3)` now errors |
| `reference_out_param_with_literal_error` | 10984 | Fixed - `f(5)` now errors |

**Root Cause:** The `ref_kind` field from AST `ParamType` was not being converted to `ref_modifier` on `DataType` when compiling function parameters in Pass 2a.

**Fix Applied:** Added `resolve_param_type()` helper in `type_compilation.rs` that:
1. Resolves the base type via `resolve_type_expr()`
2. Converts AST `RefKind` to semantic `RefModifier`
3. Updated all 5 places that resolve function parameters to use this helper

---

### Task 60: Fix Init List Issues - COMPLETE

**Issue:** Array initialization with `{1, 2, 3}` syntax not working.

| Test | Line | Status |
|------|------|--------|
| `init_list_basic` | 8632 | Fixed - `array<int> arr = {1, 2, 3}` |
| `init_list_empty` | 9931 | Fixed - `array<int> arr = {}` |
| `init_list_multidimensional` | 9950 | Fixed - `array<array<int>> matrix = {{1, 2}, {3, 4}}` |

**Root Causes & Fixes:**

1. **Template types not instantiated in function bodies:** Pass 2a (`TypeCompiler`) wasn't scanning function bodies for type expressions. Added `scan_block()`, `scan_statement()`, and `scan_expression()` methods to walk function bodies and resolve all type expressions, triggering template instantiation before Pass 2b.

2. **Template instance names not registered:** `instantiate_template()` created template instances but didn't register their names (e.g., "array<int>") in the type lookup table. Added name generation and registration.

3. **Template instance types returning `"<template instance>"` as name:** Added a `name` field to `TypeDef::TemplateInstance` so they return proper names like `"array<int>"`.

4. **Array template types not recognized as handles:** In AngelScript, `array<T>` is always a reference type. Added check in `resolve_type_expr()` to automatically set `is_handle = true` for array template instances.

5. **Empty init lists couldn't infer type:** Added `expected_init_list_type` context propagation from variable declaration to `check_init_list()`, allowing empty init lists like `array<int> arr = {}` to work.

6. **Nested template lookups failing:** Fixed `resolve_type_expr()` to recursively resolve template arguments to get canonical names (e.g., `array<array<int>>` instead of `array<array>`).

7. **Template element type comparison failing:** Fixed array type lookups to compare only `type_id` rather than full `DataType` (which includes handle flags that differ between stored and runtime representations).

---

### Task 61: Fix Lambda Issues - COMPLETE

**Issue:** Lambda captures and lambda as function arguments not working.

| Test | Line | Status |
|------|------|--------|
| `lambda_with_captures` | 8861 | Fixed - Captures work |
| `lambda_in_function_call` | 10328 | Fixed - Lambda as argument works |

**Resolution:** These tests were already passing due to fixes from earlier tasks (likely Task 60's template instantiation fixes which also handled funcdef type resolution). Removed `#[ignore]` attributes.

---

### Task 62: Fix Property Accessor Issues - COMPLETE

**Issue:** Property accessors not recognized during member access.

**Note:** Tests were originally using invalid syntax (`int get_count()` without `property` keyword). AngelScript requires one of two valid forms:
1. Block syntax: `int prop { get const { ... } set { ... } }`
2. Explicit with `property` keyword: `int get_prop() const property { ... }`

| Test | Line | Status |
|------|------|--------|
| `property_getter_only` | 14808 | Fixed - `obj.count` calls `get_count()` |
| `property_getter_and_setter` | 14835 | Fixed - both get/set work |
| `property_virtual_block_syntax` | 14864 | Fixed - block syntax works |
| `property_read_only_virtual` | 14895 | Fixed - read-only property access |

**Fixes Applied:**
1. Fixed test cases to use valid property syntax (`property` keyword or block syntax)
2. Added property getter lookup in `check_member()` before field access
3. Added `check_member_property_assignment()` helper for setter calls
4. Modified `check_assign()` to check for property setters on member access

---

### Task 63: Implement Auto Type Inference - COMPLETE

**Issue:** `auto` type inference not implemented.

| Test | Line | Status |
|------|------|--------|
| `auto_with_function_call` | 14803 | Fixed - `auto x = getNumber()` |
| `auto_with_complex_expression` | 14824 | Fixed - `auto x = a * b + c` |
| `auto_with_const` | 14845 | Fixed - `const auto x = 42` |
| `auto_with_handle` | 14864 | Fixed - `auto@ h = @obj` |

**Fixes Applied:**
1. Modified `visit_var_decl()` to detect `TypeBase::Auto` and infer type from initializer
2. Added support for `const auto` - applies const qualifier to inferred type
3. Added support for `auto@` - makes inferred type a handle
4. Updated `scan_statement()` in type_compilation.rs to skip auto types (resolved later)
5. Added proper error handling for auto without initializer and auto with void expression

---

## Remaining Ignored Tests (Tasks 64-70)

7 ignored tests remain in `function_processor.rs`. Each needs investigation to determine if:
1. The test logic is correct
2. If so, fix the underlying implementation bug

---

### Task 64: Fix super_call_in_index_expr - PENDING

**Test:** `super_call_in_index_expr` (line 9687)
**Issue:** Array init list in constructor with super should work

```angelscript
class Base { Base() { } }
class Derived : Base {
    Derived() {
        super();
        array<int> arr = {1, 2, 3};  // Fails
    }
}
```

**Analysis:** TBD

---

### Task 65: Fix index_expression_multi - PENDING

**Test:** `index_expression_multi` (line 10118)
**Issue:** Multi-index opIndex should work with multiple arguments

```angelscript
class Matrix {
    int opIndex(int row, int col) { return row * 10 + col; }
}
void test() {
    Matrix m;
    int val = m[2, 3];  // Should call opIndex(2, 3)
}
```

**Analysis:** TBD

---

### Task 66: Fix super_detection_in_init_list - PENDING

**Test:** `super_detection_in_init_list` (line 10439)
**Issue:** Init list in constructor should work with super detection

```angelscript
class Base { Base() { } }
class Derived : Base {
    Derived() {
        super();
        array<int> arr = {1, 2, 3};  // Fails
    }
}
```

**Note:** This appears to be the same scenario as Task 64. May be a duplicate test.

**Analysis:** TBD

---

### Task 67: Fix array_access - PENDING

**Test:** `array_access` (line 10933)
**Issue:** Array index access should work

```angelscript
void test() {
    array<int> arr = {1, 2, 3, 4, 5};
    int first = arr[0];   // Read access
    arr[1] = 42;          // Write access
}
```

**Analysis:** TBD

---

### Task 68: Fix uint8_to_double_conversion - PENDING

**Test:** `uint8_to_double_conversion` (line 14407)
**Issue:** Implicit conversion from uint8 to double not working

```angelscript
void test() {
    uint8 a = 200;
    double b = a;  // Should implicitly convert
}
```

**Analysis:** TBD

---

### Task 69: Fix uint16_to_uint32_widening - PENDING

**Test:** `uint16_to_uint32_widening` (line 14449)
**Issue:** Widening conversion from uint16 to uint32 not working

```angelscript
void test() {
    uint16 a = 50000;  // Note: literal 50000 may need explicit cast
    uint32 b = a;      // Should implicitly widen
}
```

**Note:** There may be two issues here:
1. Literal `50000` not converting to `uint16`
2. `uint16` to `uint32` widening conversion

**Analysis:** TBD

---

### Task 70: Fix enum_with_explicit_values - PENDING

**Test:** `enum_with_explicit_values` (line 16130)
**Issue:** Enum with explicit values should work

```angelscript
enum Priority { Low = 0, Medium = 5, High = 10 }

void test() {
    Priority p = Priority::Medium;
    int v = p;  // Should convert enum to int
}
```

**Analysis:** TBD

---

## Priority Order

1. **Task 64** - super_call_in_index_expr (array init with super)
2. **Task 65** - index_expression_multi (multi-arg opIndex)
3. **Task 66** - super_detection_in_init_list (likely duplicate of 64)
4. **Task 67** - array_access (basic array indexing)
5. **Task 68** - uint8_to_double_conversion
6. **Task 69** - uint16_to_uint32_widening
7. **Task 70** - enum_with_explicit_values

**Note:** Tasks 64, 66, and 67 may all be related to array/init list handling.

---

## Test Status

```
1644 tests passing
7 tests ignored (exposing real bugs)
```

---

## References

- **Full Details:** `/claude/semantic_analysis_plan.md`
- **Decisions Log:** `/claude/decisions.md`
- **C++ Reference:** `reference/angelscript/source/as_builder.cpp`, `as_compiler.cpp`

---

**Current Work:** Planning Tasks 64-70
**Next Work:** Task 64 - Fix super_call_in_index_expr
