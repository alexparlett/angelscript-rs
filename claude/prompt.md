# Current Task: Expression Compilation - String Literals & Architecture Review

**Status:** Complete (with task 41b identified)
**Date:** 2025-12-11
**Branch:** 041-expression-basics

---

## Summary

Completed string literal compilation by integrating string factory support into CompilationContext. During implementation, discovered and addressed a critical architectural gap: script class inheritance was not properly handled. Created Task 41b to address this systematically.

### What Was Done

1. **String Factory Integration** ✅
   - Added `string_type_hash: Option<TypeHash>` to `CompilationContext`
   - Added `set_string_type()` and `string_type_hash()` methods
   - Updated `compile_string()` in `literals.rs` to use string factory type
   - Removed TODO comments - implementation now complete

2. **Architecture Review** ✅
   - Reviewed AngelScript's C++ compilation phases vs. our Rust implementation
   - Confirmed our hash-based approach doesn't need:
     - Byte offset calculation for fields (hash-based property access)
     - FieldDef for script classes (deprecated, properties use getters/setters)
     - Vtable pointer tables (hash-based method dispatch)
   - **Identified gap**: Script class inheritance needs member copying

3. **Created Task 41b: Type Completion Pass**
   - Script classes need inherited methods/properties copied during registration
   - Walking inheritance chain at compile time is O(depth) × O(n) lookups (expensive)
   - Solution: Copy public/protected members from base during a completion pass
   - Matches AngelScript's `CompileClasses()` phase architecture

### Key Architectural Decisions

**Hash-Based Runtime Model:**
- Properties accessed by hash → getter/setter methods (no byte offsets needed)
- Methods dispatched by hash (no vtables needed)
- Type completion copies inherited members for O(1) lookups

**Inheritance Model:**
- Only script classes (unit registry) have inheritance
- Global/FFI classes cannot be inherited from (only their interfaces)
- Visibility rules: public/protected inherited, private not inherited

### Files Modified

**[context.rs](crates/angelscript-compiler/src/context.rs)**
- Added string factory support (lines 95-113)
- Added TODO comment on `find_methods()` about inheritance (line 474-476)

**[literals.rs](crates/angelscript-compiler/src/expr/literals.rs)**
- Implemented proper string type lookup from context (lines 56-71)
- Removed TODO comments

### Testing

All 317 tests pass ✅

---

## Next Steps

**Immediate (Task 41):**
- Continue with remaining expression basics (binary operators, identifiers, etc.)

**Important (Task 41b - NEW):**
- Implement Type Completion Pass to copy inherited members
- This is critical for proper inheritance support
- Should be done before Task 46 (Function Compilation)

**Future (Task 42+):**
- Expression compilation: function calls, member access
- Statement compilation
- Function body compilation (Task 46)

---

## Context for Next Session

### Current State
- String literals properly integrated with string factory
- Inheritance gap identified and documented in Task 41b
- Current `find_methods()` only returns directly declared methods (inheritance TODO)

### What Inheritance Needs
From Task 41b, the type completion pass should:
1. Topologically sort classes (base before derived)
2. Copy public/protected methods from base to derived
3. Copy public/protected properties from base to derived
4. Handle FFI base classes from global registry
5. Detect circular inheritance

This will enable O(1) method lookups without walking chains or visibility checks.
