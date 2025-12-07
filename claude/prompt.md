# Current Task: Compiler Rewrite

**Status:** In Progress
**Date:** 2025-12-07
**Branch:** ffi-type-hash-improvements

---

## Current State Summary

**Parser:** 100% Complete
**Semantic Analysis:** 100% Complete
**Test Status:** 2440+ library tests passing

---

## Task 26: Compiler Rewrite - 2-Pass Architecture

See `/claude/tasks/26_compiler_rewrite.md` for full details.

### Completed Tasks

| # | Task | Description | Status |
|---|------|-------------|--------|
| 1 | Workspace Setup | Create workspace Cargo.toml, crate skeleton, lib.rs with re-exports | ✅ Complete |
| 2 | Types: TypeHash | Move TypeHash to compiler crate, add Display, make Copy | ✅ Complete |
| 3 | Types: DataType | Move DataType, make Copy, add Display, RefModifier | ✅ Complete |
| 4 | Types: TypeDef + FunctionDef | Create clean TypeDef enum and FunctionDef struct | ✅ Complete |
| 5 | Types: ExprInfo | Create ExprInfo (renamed from ExprContext) | ✅ Complete |
| 6 | ScriptRegistry + Registry trait | Implement clean registry, Registry trait for unification | ✅ Complete |
| 7a | angelscript-core crate | Shared types for FFI and compiler crates | ✅ Complete |

### Task 26.7a Summary (Just Completed)

**Created `angelscript-core` crate** - Shared types for both FFI and compiler:
- `TypeHash` - Deterministic 64-bit hash for type identity
- `DataType` - Complete type with modifiers (const, handle, reference)
- `TypeDef` - Type definitions with full `TypeKind` including generic methods
- `FunctionDef` - Function definitions with complete signatures
- `ExprInfo` - Expression type checking results
- `TypeBehaviors` - Lifecycle behaviors for types
- `FfiExpr` - Expressions for default argument values
- `BinaryOp`, `UnaryOp` - Operator enums without token dependencies

**Three-crate architecture:**
```
angelscript-core  →  angelscript-compiler
                 →  angelscript (main crate)
```

**Key changes:**
- Unified `DataType` (removed `FfiDataType`)
- `TypeKind` with generic methods: `value<T>()`, `pod<T>()`, `value_sized()`
- `DataTypeExt` extension trait for `can_convert_to()` method
- Backward-compatible re-exports in semantic module

**Test results:**
- 113 unit tests in core crate
- 19 unit tests in compiler crate
- 2284+ main crate tests passing
- All 2440+ library tests passing

### Next Task

**Task 26.7b: FfiRegistry updates** - Update FfiRegistry to use core types

---

## Quick Reference

**Task File:** `/claude/tasks/26_compiler_rewrite.md`
**Decisions Log:** `/claude/decisions.md`

---

## Architecture Overview

### New (in `crates/`):
```
angelscript-core/       →  Shared types (TypeHash, DataType, TypeDef, etc.)
angelscript-compiler/   →  2-pass compiler (registration + compilation)
```

### Key Benefits
- 2 passes instead of 3 - Faster compilation
- No format!() overhead - Proper type resolution
- Better testability - Independent components
- DataType as Copy - Eliminates 175+ clone() calls
- Shared core types - No circular dependencies
- ~8,000 lines deleted after migration
