# Current Task: Fix Ignored Tests

**Status:** ⏳ In Progress
**Date:** 2025-11-30
**Phase:** Semantic Analysis - Bug Fixes & Missing Features

---

## Current State Summary

**Parser:** ✅ 100% Complete
- All AngelScript syntax supported
- 20 comprehensive test files
- Lambda parameter disambiguation with lookahead

**Semantic Analysis:** ✅ 99% Complete (with known issues)
- ✅ Pass 1 (Registration): 100% Complete
- ✅ Pass 2a (Type Compilation): 100% Complete
- ✅ Pass 2b (Function Compilation): 100% Complete
- ✅ Phase 1 (Type Conversions): Tasks 1-25 Complete
- ✅ Tasks 26-29 (Lambda Expressions): Complete
- ✅ Tasks 30-34 (TODO Cleanup): Complete
- ✅ Tasks 35-38: Namespace, Enum, Funcdef & Interface Validation Complete
- ✅ Task 41: Mixin Support Complete
- ✅ Task 46: Void Expression Validation Complete
- ✅ Task 47: Constant Expression Evaluation Complete
- ✅ Task 48: Circular Dependency Detection Complete
- ✅ Task 49: Visibility Enforcement Complete
- ✅ Task 50: `this` Keyword and Implicit Member Access Complete
- ✅ Task 51: Switch Statement Bytecode Generation Complete
- ✅ Task 52: Remove CreateArray + Add Initialization List Instructions Complete
- ❌ Task 53: Deferred (runtime concern)
- ✅ Task 54: Fix invalid test expectations (float→int implicit allowed)
- ✅ Task 55: Fix Type Conversion Issues (const types, signed↔unsigned)
- ⏳ Tasks 56-65: Fix remaining ignored tests (see below)

**Test Status:** ✅ 1602 tests passing, 25 ignored (exposing real bugs)

---

## Ignored Test Fix Tasks

Analysis found 31 ignored tests in `function_processor.rs`. These are now separate tasks.

---

### ✅ Task 54: Fix Invalid Test Expectations - COMPLETE

**Issue:** AngelScript allows implicit float→int truncation. Tests incorrectly expected an error.

| Test | Line | Status |
|------|------|--------|
| `variable_init_explicit_cast_required_error` | 8341 | ✅ Fixed |
| `assignment_float_to_int_implicit_error` | 8457 | ✅ Fixed |

---

### ✅ Task 55: Fix Type Conversion Issues - COMPLETE

**Issue:** Int literals not converting to unsigned types, const references failing.

| Test | Line | Status |
|------|------|--------|
| `various_int_types` | 10259 | ✅ Fixed - `uint8 x = 255` now works |
| `const_reference_parameter` | 10622 | ✅ Fixed - `const int &in` works |
| `global_const_variable` | 10735 | ✅ Fixed - Global const access works |

**Fixes Applied:**
1. Removed incorrect `is_const` check from `primitive_conversion()` - const doesn't affect type convertibility
2. Added same-type-id identity conversion for types differing only in const qualifier
3. Added all signed-to-unsigned conversions for different sizes (e.g., int32 → uint8)
4. Added all unsigned-to-signed conversions for different sizes (e.g., uint16 → int32)

---

### ⏳ Task 56: Fix Function Call / Overload Resolution Issues

**Issue:** Overload resolution and funcdef calls not working.

| Test | Line | Status |
|------|------|--------|
| `funcdef_call` | 10305 | ⏳ `op(5, 3)` through funcdef |
| `overload_resolution_exact_match` | 10669 | ⏳ Resolution picks wrong overload |
| `overloaded_function_call_exact_match` | 10783 | ⏳ Same issue |

**Action:** Fix overload resolution scoring and funcdef call detection.

---

### ⏳ Task 57: Fix Operator Overload Issues

**Issue:** Various operator overloads not being found or called correctly.

| Test | Line | Status |
|------|------|--------|
| `class_with_opAdd` | 9785 | ⏳ `a + b` doesn't find opAdd |
| `class_with_op_index_multi` | 9941 | ⏳ `m[2, 3]` multi-index |
| `class_with_op_call` | 9969 | ⏳ opCall not recognized |
| `class_with_get_op_index` | 10281 | ⏳ get_opIndex accessor |

**Action:** Fix operator lookup in `lookup_method_chain()` and `check_index_access()`.

---

### ⏳ Task 58: Implement is/!is Operators

**Issue:** Handle identity comparison operators not implemented.

| Test | Line | Status |
|------|------|--------|
| `is_operator_same_handle` | 9636 | ⏳ `a is b` |
| `is_not_operator` | 9687 | ⏳ `a !is b` |
| `is_operator_null_check` | 9711 | ⏳ `a is null` |
| `is_operator_derived_types` | 9731 | ⏳ Derived handle comparison |

**Action:** Implement `is` and `!is` operators in expression checking.

---

### ⏳ Task 59: Fix &out Parameter Lvalue Validation

**Issue:** `&out` parameters not validating lvalue requirement.

| Test | Line | Status |
|------|------|--------|
| `out_param_requires_lvalue_error` | 12688 | ⏳ `f(5)` should error |
| `reference_out_param_with_literal_error` | 13069 | ⏳ Same issue |

**Action:** Add lvalue validation for `&out` parameters in `check_call()`.

---

### ⏳ Task 60: Fix Init List Issues

**Issue:** Array initialization with `{1, 2, 3}` syntax not working.

| Test | Line | Status |
|------|------|--------|
| `init_list_basic` | 8687 | ⏳ `array<int> arr = {1, 2, 3}` |
| `init_list_empty` | 8887 | ⏳ `array<int> arr = {}` |
| `init_list_multidimensional` | 9177 | ⏳ Nested init lists |

**Action:** Fix `check_init_list()` to properly handle array initialization.

---

### ⏳ Task 61: Fix Lambda Issues

**Issue:** Lambda captures and lambda as function arguments not working.

| Test | Line | Status |
|------|------|--------|
| `lambda_with_captures` | 9532 | ⏳ Capture outer variables |
| `lambda_in_function_call` | 9609 | ⏳ Lambda as function argument |

**Action:** Fix capture analysis and lambda-to-funcdef conversion.

---

### ⏳ Task 62: Fix Property Accessor Issues

**Issue:** `get_X`/`set_X` pattern not recognized as property access.

| Test | Line | Status |
|------|------|--------|
| `property_accessor_basic` | 13579 | ⏳ `obj.prop` → `get_prop()` |
| `property_accessor_set` | 13621 | ⏳ `obj.prop = x` → `set_prop(x)` |

**Action:** Implement property accessor detection in member access.

---

### ⏳ Task 63: Implement Auto Type Inference

**Issue:** `auto` type inference not implemented.

| Test | Line | Status |
|------|------|--------|
| `auto_type_inference` | 13977 | ⏳ `auto x = expr;` |
| `auto_type_in_for_loop` | 14000 | ⏳ `for (auto x : arr)` |
| `auto_with_handle` | 14068 | ⏳ `auto@ h = @obj;` |
| `auto_with_const` | 14096 | ⏳ `const auto x = 5;` |

**Action:** Implement auto type inference in variable declarations.

---

### ⏳ Task 64: Fix Ternary with Handles

| Test | Line | Issue | Status |
|------|------|-------|--------|
| `ternary_conditional_handles` | 15163 | Handle type in ternary | ⏳ |

---

## Priority Order

1. ✅ **Task 54** - Fix invalid test expectations (DONE)
2. ⏳ **Task 55** - Type conversion issues (foundational)
3. ⏳ **Task 56** - Function call/overload issues (high impact)
4. ⏳ **Task 57** - Operator overload issues
5. ⏳ **Task 58** - is/!is operators
6. ⏳ **Task 59** - &out validation
7. ⏳ **Task 60** - Init list issues
8. ⏳ **Task 61** - Lambda issues
9. ⏳ **Task 62** - Property accessors
10. ⏳ **Task 63** - Auto type inference
11. ⏳ **Task 64** - Ternary with handles

---

## Previous Work: Task 51 - Switch Statement Bytecode Generation - COMPLETE

Implemented proper switch statement dispatch bytecode using if-else chain approach.

---

## Complete Task List (70 Tasks)

### Documentation (Tasks 1-2) ✅ COMPLETE

1. ✅ Update semantic_analysis_plan.md with validated task list
2. ✅ Update prompt.md with continuation context

### Type Conversions (Tasks 3-9) ✅ COMPLETE

3. ✅ Extend DataType with conversion methods
4. ✅ Implement primitive conversion logic (88+ conversions)
5. ✅ Implement handle conversions
6. ✅ Implement user-defined conversions
7. ✅ Implement constructor system
8. ✅ Implement constructor call detection
9. ✅ Implement initializer list support

### Reference Parameters & Handles (Tasks 10-13) ✅ COMPLETE

10. ✅ Extend DataType with reference modifiers
11. ✅ Implement reference parameter validation
12. ✅ Implement handle semantics
13. ✅ Document @+ as VM responsibility

### Constructors & super() (Tasks 14-16) ✅ COMPLETE

14. ✅ Implement member initialization order
15. ✅ Call base class constructor automatically
16. ✅ Implement copy constructor detection

### Operator Overloading (Tasks 17-20) ✅ COMPLETE

17. ✅ Extend TypeDef with operator_methods map
18. ✅ Implement operator overload lookup
19. ✅ Integrate operator overloading with binary, unary, postfix ops
20. ✅ Implement comparison operators

### Properties & Default Arguments (Tasks 21-25) ✅ COMPLETE

21. ✅ Implement property accessor detection
22. ✅ Transform property access to method calls
23. ✅ Implement default argument storage
24. ✅ Implement default argument compilation
25. ✅ Support accessors on opIndex

### Lambda Expressions (Tasks 26-29) ✅ COMPLETE

26. ✅ Implement lambda parsing (function keyword)
27. ✅ Implement capture environment (by reference)
28. ✅ Generate anonymous function (unique FunctionIds)
29. ✅ Emit lambda creation bytecode (FuncPtr, CallPtr)

### TODOs & Bug Fixes (Tasks 30-34) ✅ COMPLETE

30. ✅ Resolve all TODOs in function_processor.rs
31. ✅ Resolve all TODOs in type_compilation.rs
32. ✅ Resolve all TODOs in registration.rs
33. ✅ Fix switch/break bug
34. ✅ Fix method overload resolution bugs

### Remaining Features (Tasks 35-53)

35. ✅ Implement namespace resolution in call expressions
36. ✅ Implement enum value resolution (EnumName::VALUE)
37. ✅ Implement funcdef type checking
38. ✅ Implement interface method validation
39. ❌ REMOVED (Auto handle @+ is VM responsibility)
40. ❌ DEFERRED (Template constraints are FFI-level - defer to host API design)
41. ✅ Implement mixin support
42. ❌ DEFERRED (Scoped types are FFI-level - defer to host API design)
43. ❌ REMOVED (Null coalescing ?? is not part of AngelScript)
44. ❌ REMOVED (Elvis ?: is not part of AngelScript - ternary already implemented)
45. ✅ Bitwise assignment operators (already implemented)
46. ✅ Implement void expression validation
47. ✅ Constant expression evaluation (full implementation)
48. ✅ Implement circular dependency detection
49. ✅ Implement visibility enforcement
50. ✅ Implement `this` keyword and implicit member access
51. ✅ Complete switch statement bytecode generation
52. ✅ Remove CreateArray instruction - use CallConstructor for array<T> instead
53. ❌ DEFERRED (Null safety is a runtime concern - VM handles null pointer checks)

### Integration & Testing (Tasks 65-67)

65. ⏳ Add unit tests
66. ⏳ Add integration tests
67. ⏳ Add performance benchmarks

### Documentation (Tasks 68-70)

68. ⏳ Update architecture documentation
69. ✅ Update semantic_analysis_plan.md
70. ✅ Update prompt.md

---

## What's Next

**Current:** Task 56 - Fix Function Call / Overload Resolution Issues
- Fix funcdef call through handle (`op(5, 3)`)
- Fix overload resolution exact match preference

**After Tasks 56-64:** Tasks 65-67 (Integration & Testing)
- Add integration tests
- Performance benchmarks
- Stress tests

---

## Test Status

```
✅ 1602 tests passing
⏳ 25 tests ignored (exposing real bugs - tracked in Tasks 56-64 above)
```

---

## References

- **Full Details:** `/claude/semantic_analysis_plan.md`
- **Decisions Log:** `/claude/decisions.md`
- **C++ Reference:** `reference/angelscript/source/as_builder.cpp`, `as_compiler.cpp`

---

**Current Work:** Task 56 - Fix Function Call / Overload Resolution Issues
**Next Work:** Continue through priority list (Tasks 57-64)
