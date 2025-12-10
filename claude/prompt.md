# Current Task: Template Instantiation (Task 35)

**Status:** Complete
**Date:** 2025-12-10
**Branch:** 035-template-instantiation

---

## Task 35: Template Instantiation

Implemented the template instantiation system in `crates/angelscript-compiler/src/template/`.

### Implementation Summary

| Component | Location | Purpose |
|-----------|----------|---------|
| `TemplateInstanceCache` | `template/cache.rs` | Caches (template, args) → instance hash |
| `SubstitutionMap` | `template/substitution.rs` | Maps template params to concrete types |
| `substitute_type` | `template/substitution.rs` | Replaces template params in types |
| `instantiate_template_type` | `template/instantiation.rs` | Instantiates template classes |
| `instantiate_template_function` | `template/instantiation.rs` | Instantiates template functions |
| `instantiate_child_funcdef` | `template/instantiation.rs` | Instantiates child funcdefs |
| `TemplateCallback` trait | `template/validation.rs` | Validates instantiation via callbacks |

### Key Features

- **Cache-first lookup**: Checks cache before computing hashes
- **FFI specialization priority**: Pre-registered instances take precedence
- **Method instantiation**: Automatically instantiates methods with substituted types
- **Modifier preservation**: const, handle, ref modifiers preserved through substitution
- **Validation callbacks**: Custom validation (e.g., hashable keys for dict)
- **Child funcdef support**: `array<int>::Callback` style nested types
- **TypeResolver integration**: Automatically instantiates templates when resolving `array<int>` style types
- **Nested template support**: `array<array<int>>` works via recursive resolution

### New Error Types (CompilationError)

- `TemplateArgCountMismatch { expected, got, span }`
- `NotATemplate { name, span }`
- `TemplateValidationFailed { template, message, span }`
- `FunctionNotFound { name, span }`
- `Internal { message }`

### Tests

122 compiler tests including:
- Cache operations (6 tests)
- Substitution logic (11 tests)
- Type instantiation with methods (4 tests)
- Validation callbacks (6 tests)
- TypeResolver template tests (4 tests): simple instantiation, nested templates, error handling, caching

E2E integration tests deferred until compiler is wired up (will be covered by `test_templates` in `tests/unit_tests.rs`).

---

## Fixes Applied (PR #30 Review)

1. **Fixed `format_type_args` TODO**: Now looks up actual type names from registry instead of printing `TypeHash(0x...)` debug output
2. **Fixed clippy warning**: Collapsed nested `if let` statements using `let && let` chains
3. **Wired template instantiation into TypeResolver**: `TypeResolver::resolve_base()` now calls `ctx.instantiate_template()` when template arguments are present
4. **Added nested template test**: `resolve_nested_template_instantiation` test verifies `array<array<int>>` works correctly

## Next Steps

- Task 36: Conversion System (type conversions with costs)
