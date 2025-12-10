# Current Task: Type Resolution (Task 34)

**Status:** Core Complete (templates pending Task 35)
**Date:** 2025-12-10
**Branch:** 034-type-resolution

---

## Task 34: Type Resolution

Implemented the `TypeResolver` that converts AST `TypeExpr` nodes into semantic `DataType` values.

### Implementation Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| `TypeResolver` | `crates/angelscript-compiler/src/type_resolver.rs` | Converts AST types to DataType |
| Primitive mapping | `primitive_to_hash()` | Maps PrimitiveType enum to TypeHash constants |
| Modifier handling | `resolve()` | Applies const, handle, handle-to-const flags |
| Parameter resolution | `resolve_param()` | Handles reference modifiers (&in, &out, &inout) |

### Key Features

- **Primitive resolution**: void, bool, int, int8, int16, int64, uint, uint8, uint16, uint64, float, double
- **Named type resolution**: Uses `ctx.resolve_type()` for O(1) scope lookup
- **Qualified types**: Builds qualified name from scope segments (e.g., `Game::Player`)
- **Type modifiers**:
  - `const` → `is_const = true`
  - `@` → `is_handle = true`
  - `@ const` → `is_handle_to_const = true`
- **Reference modifiers**: `&in` → In, `&out` → Out, `&inout` → InOut, `&` → InOut

### Tests

18 tests covering:
- All primitive types
- Named types (global and namespaced)
- Const and handle modifiers
- Handle-to-const combinations
- Qualified type paths
- All reference modifier types
- Error cases (unknown types, auto, template params)

---

## Complete

Core type resolution is complete. Template instantiation (Task 35) will add:
- Template type arguments (`array<int>`)
- Template instance caching
- Array type sugar (`int[]` → `array<int>`)

## Next Steps

- Task 35: Template Instantiation
- Task 36: Conversion System
