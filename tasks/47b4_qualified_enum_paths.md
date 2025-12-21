# Task 47b4: Qualified Enum Path Resolution

## Problem

Fully qualified enum value paths like `Namespace::EnumType::Value` don't resolve.

**Error:** `UndefinedVariable { name: "test::Color::Green" }`

**Affected Test:** `test_using_namespace`

## Root Cause

When resolving `test::Color::Green`, the compiler treats the entire path as a variable name instead of recognizing it as `Namespace::EnumType::EnumValue`.

Enum values are not standalone variables - they're accessed through their enum type. The path resolution needs to:
1. Recognize `test::Color` as an enum type
2. Look up `Green` as a value within that enum

## Context

Valid AngelScript:
```angelscript
namespace test {
    enum Color { Red, Green, Blue }
}

void main() {
    test::Color c = test::Color::Green;  // Fully qualified
    Color c2 = Color::Red;               // After 'using namespace test'
}
```

## Solution

In the identifier/path resolution code, when encountering a path like `A::B::C`:
1. Try to resolve `A::B` as a type
2. If it's an enum, look up `C` as an enum value
3. Return the enum value's constant

### Where to Fix

The resolution likely happens in:
- `crates/angelscript-compiler/src/expr/ident.rs` or similar
- `crates/angelscript-compiler/src/context.rs` - `resolve_variable()` or path resolution

### Implementation

```rust
fn resolve_scoped_path(ctx: &CompilationContext, path: &[&str], span: Span) -> Result<ExprInfo> {
    // Try progressively longer prefixes as type names
    for i in (1..path.len()).rev() {
        let type_path = path[..i].join("::");
        let remainder = &path[i..];

        if let Some(type_hash) = ctx.resolve_type(&type_path) {
            if let Some(enum_entry) = ctx.get_type(type_hash).and_then(|e| e.as_enum()) {
                // It's an enum - look up the value
                if remainder.len() == 1 {
                    let value_name = remainder[0];
                    if let Some(value) = enum_entry.get_value(value_name) {
                        return Ok(ExprInfo::constant(value, DataType::simple(type_hash)));
                    }
                }
            }
        }
    }

    Err(CompilationError::UndefinedVariable { name: path.join("::"), span })
}
```

## Files to Modify

- `crates/angelscript-compiler/src/expr/ident.rs` or path resolution code
- `crates/angelscript-compiler/src/context.rs` - if resolution is centralized there

## Test Script Fix Required

The test also uses `Color c = Red` after `using namespace test`, which is invalid (enum values aren't brought into scope by `using namespace`). This needs to be fixed to `Color c = Color::Red`.

## Test Case

```angelscript
namespace test {
    enum Color { Red, Green, Blue }
}

void main() {
    // Fully qualified - should work
    test::Color c1 = test::Color::Green;

    // After using namespace
    using namespace test;
    Color c2 = Color::Red;  // Valid
    // Color c3 = Red;      // INVALID - enum values not brought into scope
}
```

## Acceptance Criteria

- [ ] `cargo test --test unit test_using_namespace` passes
- [ ] `Namespace::Enum::Value` syntax resolves correctly
- [ ] `Enum::Value` syntax works (unqualified enum, qualified value)
- [ ] Test script is updated to use valid syntax
- [ ] No regression in simple enum usage
