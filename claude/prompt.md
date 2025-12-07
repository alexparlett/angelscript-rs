# Current Task: Compiler Rewrite

**Status:** In Progress
**Date:** 2025-12-07
**Branch:** compiler-rewrite

---

## Current State Summary

**Parser:** 100% Complete
**Semantic Analysis:** 100% Complete
**Test Status:** 2411 library tests passing

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

### Task 26.6 Summary (Just Completed)

**TypeBehaviors struct** - Lifecycle behaviors for types:
- Constructors/factories (multiple overloads)
- Destructor, addref, release (single behaviors)
- List initialization (list_construct, list_factory)
- Template callback, weak reference support

**Registry trait** - Common interface for FFI and Script registries:
- Consistent naming: `get_*` (by hash), `lookup_*` (by name), `has_*` (existence), `find_*` (complex)
- Type lookups, function lookups, behavior lookups
- Method/operator/property lookups
- Inheritance queries (base class, is_subclass_of, interfaces)
- Enum and template support

**ScriptRegistry** - Clean implementation with no redundant maps:
- `types: FxHashMap<TypeHash, TypeDef>` (primary)
- `functions: FxHashMap<TypeHash, FunctionDef>` (primary)
- `behaviors: FxHashMap<TypeHash, TypeBehaviors>`
- Name indexes are secondary lookups returning TypeHash

**Key design decision:**
- `FunctionDef.Param.has_default: bool` (not `Option<&'ast Expr>`)
- Registry stores metadata only; default value expressions accessed during Pass 2 AST walk
- This keeps registry lifetime-free while allowing full compilation

**Test results:**
- 120 unit tests passing in compiler crate
- 24 doctests passing in compiler crate
- All 2411 main crate tests still passing

### Next Task

**Task 26.7: CompilationContext** - Implement context with name resolution

---

## Quick Reference

**Task File:** `/claude/tasks/26_compiler_rewrite.md`
**Decisions Log:** `/claude/decisions.md`

---

## Architecture Overview

### New (in `crates/angelscript-compiler/`):
```
Pass 1 (registration.rs):  Register types → Register functions with COMPLETE signatures
Pass 2 (compilation/):     Type check function bodies + generate bytecode
```

### Key Benefits
- 2 passes instead of 3 - Faster compilation
- No format!() overhead - Proper type resolution
- Better testability - Independent components
- DataType as Copy - Eliminates 175+ clone() calls
- ~8,000 lines deleted after migration
