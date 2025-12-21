# Task 47b9: Scoped Import Function Resolution

## Problem

Functions defined inside a namespace with `using namespace` can't be called from outside.

**Error:** `UnknownFunction { name: "testScopedImport" }`

**Affected Test:** `test_using_namespace`

## Root Cause

```angelscript
namespace myspace {
    using namespace utils;

    void testScopedImport() {
        helper();  // Uses utils::helper via import
    }
}

void main() {
    myspace::testScopedImport();  // Can't find the function
}
```

The function `myspace::testScopedImport` should be registered and callable, but it's not being found when called with the qualified name.

## Context

This could be:
1. Registration issue - function not registered with correct qualified name
2. Resolution issue - `myspace::testScopedImport` not resolving correctly

## Investigation Points

1. Check how functions inside namespaces are registered in Pass 1
2. Check how qualified function names are resolved during calls
3. Verify the function hash uses the correct qualified name

## Solution

### If Registration Issue

In registration pass, ensure functions are registered with their full namespace path:

```rust
fn register_function(&mut self, func: &FuncDecl, namespace: &[String]) -> Result<()> {
    let qualified_name = if namespace.is_empty() {
        func.name.to_string()
    } else {
        format!("{}::{}", namespace.join("::"), func.name)
    };

    // Use qualified_name for hash and registration
    let func_hash = TypeHash::from_function(&qualified_name, &param_types);
    // ...
}
```

### If Resolution Issue

In function resolution, ensure qualified paths are looked up correctly:

```rust
fn resolve_function(&self, name: &str) -> Option<&[TypeHash]> {
    // Check if name is already qualified
    if name.contains("::") {
        return self.functions.get(name);
    }

    // Try with current namespace context
    // ...
}
```

## Files to Check

- `crates/angelscript-compiler/src/passes/registration.rs` - Function registration
- `crates/angelscript-compiler/src/context.rs` - Function resolution
- `crates/angelscript-compiler/src/expr/calls.rs` - Call site resolution

## Test Case

```angelscript
namespace outer {
    void innerFunc() {
        print("inner");
    }
}

void main() {
    outer::innerFunc();  // Should work
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_using_namespace` passes
- [ ] Functions in namespaces are callable with qualified names
- [ ] `using namespace` inside a namespace works correctly
- [ ] No regression in global function resolution
