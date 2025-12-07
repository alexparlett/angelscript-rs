# Current Task: FFI Implementation

**Status:** In Progress
**Date:** 2025-12-06
**Phase:** FFI Performance Optimization (Phase 6.7 Complete)

---

## Current State Summary

**Parser:** 100% Complete
**Semantic Analysis:** 100% Complete
**Test Status:** 2423 library tests passing

**Recent Additions:**
- All FFI built-in modules implemented (std, string, math, array, dictionary)
- Integration tests and benchmarks using Context/Unit API
- SELF_TYPE placeholder for template self-references
- Template callback support for validation
- Multiple operator overload support (const/non-const)
- FFI_BIT for TypeId/FunctionId to distinguish FFI vs script-defined items
- FfiRegistry stored in Context, shared via Arc to Units
- Primitives now stored in FfiRegistry with TypeDef::Primitive entries
- CompilationContext as unified facade for FFI + Script registries
- TemplateInstantiator for template instantiation with specialization

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
| [08](tasks/08_ffi_builtin_modules.md) | Implement built-in modules via FFI (includes list behaviors) | ✅ Complete |
| [19](tasks/19_ffi_import_review.md) | FFI import system review & test migration | ✅ Complete |

### Phase 4: Migration
| Task | Description | Status |
|------|-------------|--------|
| [09](tasks/09_ffi_update_entry_points.md) | Update benches/tests to Context/Unit API | ✅ Complete |
| [10](tasks/10_ffi_extract_placeholders.md) | Remove FFI placeholders from test scripts | Not Started |
| [11](tasks/11_ffi_lib_exports.md) | Library exports and public API | Not Started |

### Phase 5: Performance & Advanced Features
| Task | Description | Status |
|------|-------------|--------|
| [20](tasks/20_ffi_import_performance.md) | FFI import performance optimization | ✅ Phase 6.7 Complete |
| [12](tasks/12_ffi_template_functions.md) | Template functions via register_fn_raw | Not Started |
| [13](tasks/13_ffi_variadic_args.md) | Variadic function arguments | Not Started |
| [14](tasks/14_ffi_advanced_templates.md) | Advanced templates (if_handle_then_const, funcdefs, specializations) | Not Started |
| [16](tasks/16_ffi_gc_weakref_behaviors.md) | GC and weak reference behaviors | Not Started |

### Phase 6: Standard Library Extensions
| Task | Description | Status |
|------|-------------|--------|
| [24](tasks/24_stdlib_random_time.md) | Random and time functions for standard library | Not Started |

---

## Key Design Decisions

- **Two-tier architecture**: `CompilationContext` = `Arc<FfiRegistry>` + `ScriptRegistry`
- **FFI_BIT high bit**: TypeId/FunctionId with bit 0x8000_0000 set = FFI, clear = Script
- **Compiler::compile(script, ffi)**: Main entry point, takes Arc<FfiRegistry>
- **No Registry alias**: Use `ScriptRegistry` or `CompilationContext` directly
- **Built-ins via FFI**: Replace ~800 lines of hardcoded registry.rs
- **Two-pass type import** - handles circular references between types in same module
- **SELF_TYPE (TypeId(u32::MAX - 1))** - placeholder for self-referential template types
- **Vec<FunctionId> for operator_methods** - supports const/non-const overloads

---

## Quick Reference

**Full FFI Design:** `/claude/ffi_plan.md`
**Decisions Log:** `/claude/decisions.md`

---

## Next Steps

**Task 20 Complete** - Phase 6.7 (Cleanup and Testing) finished:
- Removed deprecated `Compiler::compile()` (no args version)
- Renamed `Compiler::compile_with_ffi()` to `Compiler::compile(script, ffi)`
- Removed `Registry` type alias from semantic/mod.rs
- Updated all tests to use `default_ffi()` helper with new signature
- Fixed unused import warning in conversion.rs
- Updated doc comments to reference CompilationContext instead of Registry

**Next:** Phase 7 - Run benchmarks to verify performance improvement

---

## Future Tasks

### Task B: Enhanced Bytecode (After FFI)

1. Constant folding
2. Dead code elimination
3. Register allocation
4. Instruction optimization
