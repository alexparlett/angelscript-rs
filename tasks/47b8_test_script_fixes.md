# Task 47b8: Test Script Fixes

## Overview

Some test scripts contain invalid AngelScript or patterns not supported in this compiler version. These need to be updated.

## Fixes Required

### 1. `test_scripts/control_flow.as` - Remove switch on handle

**Problem:** Switch on handle type (`Animal@ pet`) not supported (pattern matching removed).

**Fix:** Remove or rewrite `testSwitchHandleNull()`:

```angelscript
// REMOVE this function entirely:
void testSwitchHandleNull() {
    Animal@ pet = null;
    switch (pet) {  // Invalid - can't switch on handle
        case null:
            print("no pet");
            break;
        default:
            print("has a pet");
            break;
    }
}

// REPLACE WITH: (if null check is needed)
void testHandleNull() {
    Animal@ pet = null;
    if (pet is null) {
        print("no pet");
    } else {
        print("has a pet");
    }
}
```

### 2. `test_scripts/using_namespace.as` - Fix enum value access

**Problem:** Enum values aren't brought into scope by `using namespace`.

**Current (invalid):**
```angelscript
using namespace test;
Color c = Red;  // INVALID - Red not in scope
```

**Fixed:**
```angelscript
using namespace test;
Color c = Color::Red;  // VALID - access through enum type
```

### 3. `test_scripts/game_logic.as` - Float to int cast

**Problem:** Implicit cast from float to int not allowed.

**Error:** `InvalidCast { from: "float", to: "int" }`

**Fix:** Add explicit cast:
```angelscript
// Before:
if (int(elapsedTime) % 10 == 0) {

// This is already explicit, so check the actual line 174 for the issue
// Likely something like:
int x = someFloatValue;  // Needs explicit cast
int x = int(someFloatValue);  // Fixed
```

### 4. `test_scripts/utilities.as` - Namespace issues

**Problem:**
- `sqrt` is under `math` namespace - needs `math::sqrt()` or `using namespace math`
- `substr` is present on `string` - may be a method call issue, not missing function

**Fix for sqrt:**
```angelscript
// Either add at top:
using namespace math;

// Or qualify calls:
return math::sqrt(dx * dx + dy * dy);
```

**Fix for substr:**
Check if it's being called as a free function instead of a method:
```angelscript
// Wrong:
substr(str, 0, 5)

// Right:
str.substr(0, 5)
```

## Files to Modify

- `test_scripts/control_flow.as`
- `test_scripts/using_namespace.as`
- `test_scripts/game_logic.as`
- `test_scripts/utilities.as` (or test setup)

## Acceptance Criteria

- [ ] All modified test scripts are syntactically valid AngelScript
- [ ] Scripts don't rely on unimplemented features (pattern matching, etc.)
- [ ] Scripts don't rely on unregistered FFI functions
- [ ] After fixes, remaining failures are compiler bugs (not test bugs)
