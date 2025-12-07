# Current Task: Compiler Rewrite

**Status:** In Progress
**Date:** 2025-12-07
**Branch:** compiler-rewrite

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
| 7b | FFI + Parser Crates | Create angelscript-ffi and angelscript-parser, unify FunctionDef | ✅ Complete |
| 7c | CompilationContext | Implement unified CompilationContext with FfiRegistry + ScriptRegistry | ✅ Complete |

### Task 26.7c Summary (Just Completed)

**Implemented CompilationContext** in `crates/angelscript-compiler/src/context.rs`:
- Unified facade for FFI + Script registry lookups
- No `FunctionRef` enum needed - `get_function()` returns `Option<&FunctionDef>` directly
- Namespace management: `enter_namespace()`, `exit_namespace()`, `add_import()`
- Name resolution: `resolve_type()` with namespace rules
- All unified lookup methods for types, functions, behaviors, methods, operators, properties
- 36 unit tests passing

### Next Task

**Task 26.8: Pass 1: RegistrationPass** - Type + function registration with complete signatures

---

## Task 1: Unified Type Registry - Phase 2 Complete

See `/claude/tasks/01_unified-type-registry.md` for full details.

### Phase 2 Summary (Just Completed)

**Design fixes applied to `crates/angelscript-core/src/`:**

- Updated `ClassEntry.methods` from `Vec<FunctionEntry>` to `Vec<TypeHash>` (single source of truth)
- Updated `FunctionDef.template_params` from `Vec<String>` to `Vec<TypeHash>` (refs to TemplateParamEntry)
- Removed `FieldEntry` struct and all exports (all property access via PropertyEntry with getter/setter)
- Removed `ClassEntry.fields` field and related methods (`with_field`, `find_field`)
- Added `has_method(TypeHash)` to ClassEntry for method lookup by hash
- Verified `TemplateParamEntry.owner` and `TypeBehaviors` already use `TypeHash` correctly

**Tests:** 309 unit tests passing in angelscript-core

### Next Phase

**Phase 3: Create angelscript-registry** - TypeRegistry and Module builder

---

## Task 28: Unified Error Types - COMPLETE

See `/claude/tasks/28_unified_error_types.md` for full details.

### Completed Tasks

| # | Task | Description | Status |
|---|------|-------------|--------|
| 1 | Move Span to core | Moved `Span` from parser to angelscript-core | ✅ Complete |
| 2 | Create core error types | Defined `AngelScriptError` and phase-specific errors in core | ✅ Complete |
| 3 | Migrate parser errors | Parser now uses `LexError`, `ParseError`, `ParseErrorKind`, `ParseErrors` from core | ✅ Complete |
| 4 | Consolidate registration errors | Merged `FfiRegistryError` + `ModuleError` → `RegistrationError` | ✅ Complete |
| 5 | Migrate compiler errors | Compiler now uses `CompilationError` from core | ✅ Complete |
| 6 | Migrate main crate errors | Updated `ContextError`, `BuildError` with helper methods | ✅ Complete |
| 7 | Update public API | Exposed all error types in public API | ✅ Complete |

### Task 28.6 & 28.7 Summary (Just Completed)

**Updated public API exports in `src/lib.rs`:**
- Exported `AngelScriptError`, `LexError`, `ParseError`, `ParseErrorKind`, `ParseErrors`
- Exported `RegistrationError`, `CompilationError`, `RuntimeError`, `Span`

**Added helper methods to `BuildError`:**
- `into_errors()` - Converts to `Vec<AngelScriptError>` for unified handling
- `first_error()` - Gets first error as `AngelScriptError`

**Added helper methods to `ContextError`:**
- `into_errors()` - Converts to `Vec<AngelScriptError>` for unified handling
- `first_error()` - Gets first error as `AngelScriptError`

**Tests:** 47 library tests passing

### Next Task

**Task 26.8: Pass 1: RegistrationPass** - Type + function registration with complete signatures

### Deferred

**Task 19: FFI Default Args** - Deferred until after new compiler passes (Tasks 8-15) are built.
See `/claude/tasks/19_ffi_default_args.md` for details.

---

## Quick Reference

**Task File:** `/claude/tasks/26_compiler_rewrite.md`
**Decisions Log:** `/claude/decisions.md`

---

## Architecture Overview

### Crates (in `crates/`):
```
angelscript-core/       →  Shared types (TypeHash, DataType, TypeDef, FunctionDef, etc.)
angelscript-ffi/        →  FFI registry and type registration
angelscript-parser/     →  Lexer + AST + Parser
angelscript-compiler/   →  2-pass compiler (registration + compilation)
```

### Dependency Graph:
```
angelscript-core  ←─────────────────────────────┐
       ↑                                        │
       │                                        │
angelscript-parser    angelscript-ffi ──────────┤
       ↑                     ↑                  │
       │                     │                  │
       └─────── angelscript-compiler ───────────┘
                      ↑
                      │
               angelscript (main)
```

### Key Benefits
- 2 passes instead of 3 - Faster compilation
- No format!() overhead - Proper type resolution
- Better testability - Independent components
- DataType as Copy - Eliminates 175+ clone() calls
- Shared core types - No circular dependencies
- ~8,000 lines deleted after migration
