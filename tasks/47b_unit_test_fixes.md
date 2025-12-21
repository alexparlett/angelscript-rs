# Task 47b: Unit Test Fixes

## Overview

Fix remaining compiler issues discovered by running `cargo test --test unit -- --ignored`. These are the 13 failing tests that need either compiler fixes or test script corrections.

## Current Status

**Passing (2):** `test_functions`, `test_lambdas`
**Failing (13):** Listed below with root causes

## Sub-Tasks

| ID | Name | Status | Test(s) Affected |
|----|------|--------|------------------|
| 47b1 | Override Resolution | TODO | `test_inheritance` |
| 47b2 | Interface Handle Addref | TODO | `test_interface` |
| 47b3 | Init List Template Substitution | TODO | `test_large_function`, performance |
| 47b4 | Qualified Enum Paths | TODO | `test_using_namespace` |
| 47b5 | Typedef Support | TODO | `test_types` |
| 47b6 | Auto Type Inference | TODO | `test_types` |
| 47b7 | Qualified Constructor Calls | TODO | `test_nested` |
| 47b8 | Test Script Fixes | TODO | `test_control_flow`, `test_using_namespace` |
| 47b9 | Scoped Import Functions | TODO | `test_using_namespace` |

**Note:** Performance tests (`large_500`, `xlarge_1000`, `xxlarge_5000`) will pass once the above issues are fixed.

---

## Issue Categories

### 1. Test Script Issues (Fix the test, not the compiler)

#### 1.1 `test_control_flow` - Switch on handle type
**Error:** `type 'Animal' does not support switch (missing opEquals)` at line 142

**Problem:** Test uses switch on a handle (`Animal@ pet`) which we don't support (pattern matching removed).

**Fix:** Remove or rewrite the `testSwitchHandleNull()` function - switch on handles isn't valid in this version.

```angelscript
// REMOVE this function - switch on handles not supported
void testSwitchHandleNull() {
    Animal@ pet = null;
    switch (pet) {  // Invalid
```

---

#### 1.2 `test_inheritance` - Ambiguous override
**Error:** `AmbiguousOverload { name: "speak", candidates: "speak() and speak()" }` at line 98

**Problem:** Calling `dog.speak()` where Dog overrides Animal.speak() but compiler sees both as candidates.

**Analysis needed:** This could be:
- Test issue (duplicate method declarations)
- Compiler issue (not properly handling override resolution)

**Action:** Check if Dog properly overrides Animal.speak() - likely compiler needs to prefer derived class method over base.

---

#### 1.3 `test_interface` - addref behavior error
**Error:** `addref behavior only valid for class types` at line 104

**Problem:** Test does `IDrawable@ drawable = @obj;` - taking handle of object and assigning to interface handle.

**Analysis needed:** The interface handle assignment should work. Likely the addref validation is too strict.

**Action:** Check where addref behavior validation happens and ensure interface handles are allowed.

---

### 2. Typedef/Auto Not Implemented

#### 2.1 `test_types` - typedef and auto
**Errors:**
- `UnknownType { name: "EntityId" }` - typedef not resolved
- `UnknownType { name: "StringArray" }` - typedef not resolved
- `auto type cannot be resolved without inference context` - auto not working at file scope

**Problem:**
```angelscript
typedef int EntityId;
typedef array<string> StringArray;
EntityId id;              // UnknownType
auto inferredInt = 42;    // No inference context
```

**Fix Options:**
1. Implement typedef support in registration pass
2. Implement auto type inference for global variables
3. OR mark test as requiring features not yet implemented

---

### 3. Enum/Namespace Issues

#### 3.1 `test_using_namespace` - Enum value resolution
**Errors:**
- `UndefinedVariable { name: "Red" }` - test script is wrong (enum values don't come via using)
- `UndefinedVariable { name: "test::Color::Green" }` - fully qualified enum is BROKEN
- `UnknownFunction { name: "testScopedImport" }` - scoped import not working

**Problem:**
```angelscript
using namespace test;
Color c = Red;  // INVALID - enum values aren't brought into scope by using (FIX TEST)
test::Color c = test::Color::Green;  // VALID syntax but BROKEN in compiler (FIX COMPILER)
```

**Fix:**
1. **Test fix:** Change `Red` to `Color::Red` or `test::Color::Red`
2. **Compiler fix:** Fully-qualified enum paths (`Namespace::Enum::Value`) need proper resolution

---

### 4. Constructor Call Syntax

#### 4.1 `test_nested` - Constructor as function call
**Error:** `UnknownFunction { name: "Entity" }` at line 118

**Problem:**
```angelscript
Game::Entity entity(1, "Player");  // Trying to construct
```

**Analysis:** Constructor call syntax - either:
- Not recognizing qualified type as constructor
- Registration didn't create constructor function

---

#### 4.2 `test_game_logic` - Forward reference to method
**Errors:**
- `UnknownFunction { name: "spawnEnemy" }` at line 131
- `InvalidCast { from: "float", to: "int" }` at line 174

**Problem:**
```angelscript
void startGame() {
    spawnEnemy(10, 10, 50, 10);  // Calls method defined later
}
void spawnEnemy(...) { ... }
```

**Analysis:** Method called before it's defined in class body. Registration should handle this.

---

### 5. Template Type Parameter Resolution

#### 5.1 `test_large_function` - Init list with template
**Error:** `expected 'T', got 'int'` at line 63

**Problem:**
```angelscript
array<int> arr = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};
```

**Analysis:** Init list compilation isn't substituting `T` with `int` for `array<int>`.

---

#### 5.2 `test_data_structures` - Template operators
**Errors:**
- `No matching operator '==' for types TypeHash(...) and TypeHash(...)` - comparing template params
- `UnknownFunction { name: "isEmpty" }` - method not found
- `No matching operator '-' for types...` - arithmetic on template params

**Problem:** Template methods/operators not properly resolved when template parameter is used.

---

### 6. Missing Method/Function Resolution

#### 6.1 `test_utilities` - Various missing
**Errors:**
- `UnknownFunction { name: "sqrt" }` - math function not in scope
- `NoMatchingOverload { name: "substr" }` - string method signature mismatch

**Problem:** Test assumes FFI functions (`sqrt`, `substr`) that may not be registered.

**Fix Options:**
1. Register required math functions in test context
2. Fix test to not rely on unregistered FFI

---

## Priority Order

1. **High - Compiler bugs:**
   - 1.2 Override resolution (affects all inheritance)
   - 1.3 Interface handle assignment
   - 4.2 Forward method references in classes
   - 5.1 Init list template substitution
   - 5.2 Template operators

2. **Medium - Feature gaps:**
   - 2.1 Typedef support
   - 3.1 Enum scoping with `using namespace`
   - 4.1 Qualified constructor calls

3. **Low - Test fixes:**
   - 1.1 Remove switch-on-handle test
   - 6.1 Register missing FFI or simplify test

---

## Detailed Investigation Needed

For each compiler issue, investigate:

| Issue | File(s) to Check | Likely Location |
|-------|------------------|-----------------|
| Override resolution | `overload.rs`, `resolution.rs` | Method lookup preferring derived |
| Interface handles | `conversion.rs`, `expr.rs` | Addref validation |
| Forward refs | `pass.rs`, `registration.rs` | Two-pass class member collection |
| Init list templates | `expr.rs` (init list) | Template param substitution |
| Template operators | `resolution.rs`, `overload.rs` | Operator lookup with generic types |
| Typedef | `registration.rs`, `resolution.rs` | Need to register type aliases |
| Qualified enums | `resolution.rs` | `NS::Enum::Value` path resolution |
| Qualified constructors | `expr.rs`, `resolution.rs` | Path resolution for type::ctor |

---

## Acceptance Criteria

- [ ] `cargo test --test unit -- --ignored` passes all 15 tests
- [ ] No regressions in `cargo test --test unit` (non-ignored)
- [ ] No regressions in `cargo test --test test_harness`
- [ ] No regressions in `cargo test --test module_tests`
