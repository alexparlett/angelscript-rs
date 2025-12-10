# Current Task: Template Instantiation (Task 35)

**Status:** Core Complete
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

### New Error Types (CompilationError)

- `TemplateArgCountMismatch { expected, got, span }`
- `NotATemplate { name, span }`
- `TemplateValidationFailed { template, message, span }`
- `FunctionNotFound { name, span }`
- `Internal { message }`

### Tests

103 compiler tests including:
- Cache operations (6 tests)
- Substitution logic (11 tests)
- Type instantiation with methods (4 tests)
- Validation callbacks (6 tests)

---

## Remaining Work

- **TypeResolver integration**: Wire template instantiation into `TypeResolver` when resolving template arguments
- **Nested templates**: `array<array<int>>` requires recursive instantiation in TypeResolver

## Next Steps

- Task 36: Conversion System (type conversions with costs)
