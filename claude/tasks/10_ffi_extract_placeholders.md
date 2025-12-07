# Task 10: Extract FFI Placeholders from Test Scripts

**Status:** Not Started
**Depends On:** Tasks 08, 09
**Estimated Scope:** Test script cleanup

---

## Objective

Remove FFI placeholder stubs from AngelScript test files and ensure they work with proper FFI-registered functions.

## Current State

19 test scripts contain FFI placeholders like:
```angelscript
// FFI placeholder - will be replaced with proper FFI bindings
void print(const string &in msg) {}
```

## Files to Update

```
test_scripts/hello_world.as
test_scripts/expressions.as
test_scripts/utilities.as
test_scripts/interface.as
test_scripts/using_namespace.as
test_scripts/enum.as
test_scripts/templates.as
test_scripts/game_logic.as
test_scripts/inheritance.as
test_scripts/data_structures.as
test_scripts/nested.as
test_scripts/functions.as
test_scripts/literals.as
test_scripts/class_basic.as
test_scripts/control_flow.as
test_scripts/types.as
test_scripts/performance/large_500.as
test_scripts/performance/xlarge_1000.as
test_scripts/performance/xxlarge_5000.as
```

## Common Placeholders to Remove

```angelscript
// These will be provided by FFI:
void print(const string &in msg) {}
void println(const string &in msg) {}
string toString(int value) {}
string toString(float value) {}
string toString(bool value) {}
int abs(int x) {}
float sqrt(float x) {}
```

## Example: Before

```angelscript
// hello_world.as
// FFI placeholder - will be replaced with proper FFI bindings
void print(const string &in msg) {}

void main() {
    print("Hello, World!");
}
```

## Example: After

```angelscript
// hello_world.as
void main() {
    print("Hello, World!");  // Uses FFI-registered print function
}
```

## Implementation Steps

1. Identify all placeholder patterns in each file
2. Determine which functions come from which module:
   - `std` module: print, println, eprint, eprintln
   - `string` module: toString overloads (if we add them)
   - `math` module: abs, sqrt, sin, cos, etc.
3. Remove placeholder declarations
4. Verify tests still pass with FFI-provided functions
5. Update any tests that were testing the placeholders themselves

## Acceptance Criteria

- [ ] All placeholder declarations removed from test scripts
- [ ] All tests pass using FFI-registered functions
- [ ] No "FFI placeholder" comments remain
- [ ] Scripts are cleaner and more representative of real usage
- [ ] Performance scripts still compile and benchmark correctly
