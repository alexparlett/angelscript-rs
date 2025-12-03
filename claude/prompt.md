# Current Task: FFI Implementation

**Status:** In Progress
**Date:** 2025-12-03
**Phase:** Post-Semantic Analysis

---

## Current State Summary

**Parser:** 100% Complete
**Semantic Analysis:** 100% Complete
**Test Status:** 1987 tests passing, 0 ignored

**Recent Additions:**
- Implemented `Registry::import_modules()` for converting FFI registrations to Registry entries
- Added `ImportError` type for import failure handling
- Two-pass type import for handling circular references within modules
- Type resolution helper for converting AST types to semantic DataType

---

## FFI Implementation Tasks

Detailed task files are in `/claude/tasks/`. Complete in order:

### Phase 1: Core Infrastructure
| Task | Description | Status |
|------|-------------|--------|
| [01](tasks/01_ffi_core_infrastructure.md) | Core types, traits (FromScript, ToScript, NativeType) | ✅ Complete |
| [02](tasks/02_ffi_module_and_context.md) | Module and Context API | ✅ Complete |

### Phase 2: Registration Builders
| Task | Description | Status |
|------|-------------|--------|
| [03](tasks/03_ffi_function_registration.md) | Function registration with declaration parsing | ✅ Complete |
| [04](tasks/04_ffi_class_builder.md) | ClassBuilder (value/reference types) | ✅ Complete |
| [05](tasks/05_ffi_enum_interface_funcdef.md) | Enum, Interface, Funcdef builders | ✅ Complete |

### Phase 3: Integration
| Task | Description | Status |
|------|-------------|--------|
| [07](tasks/07_ffi_apply_to_registry.md) | Apply FFI registrations to Registry | ✅ Complete |
| [08](tasks/08_ffi_builtin_modules.md) | Implement built-in modules via FFI | Not Started |

### Phase 4: Migration
| Task | Description | Status |
|------|-------------|--------|
| [09](tasks/09_ffi_update_entry_points.md) | Update benches/tests to Context/Unit API | Not Started |
| [10](tasks/10_ffi_extract_placeholders.md) | Remove FFI placeholders from test scripts | Not Started |
| [11](tasks/11_ffi_lib_exports.md) | Library exports and public API | Not Started |

---

## Key Design Decisions

- **Module owns arena** for storing parsed AST types (TypeExpr, Ident)
- **GlobalPropertyDef uses AST types** - `Ident<'ast>` and `TypeExpr<'ast>` instead of String/TypeSpec
- **Module has `'app` lifetime** for global property value references
- **Global properties on Module**, not Context (follows same pattern as functions)
- **Two calling conventions**: type-safe (closure) and raw (CallContext)
- **Built-ins via FFI**: Replace ~800 lines of hardcoded registry.rs
- **`import_modules()` on Registry** - processes all modules in one call
- **Two-pass type import** - handles circular references between types in same module

---

## Quick Reference

**Full FFI Design:** `/claude/ffi_plan.md`
**Decisions Log:** `/claude/decisions.md`

---

## Next Steps

**Task 08: Built-in Modules** - Implement std, string, array, dictionary, math modules via FFI

---

## Future Tasks

### Task B: Enhanced Bytecode (After FFI)

1. Constant folding
2. Dead code elimination
3. Register allocation
4. Instruction optimization
