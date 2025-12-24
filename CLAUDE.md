# Claude Code Instructions

## Project Structure
- `tasks/` - Task definitions (committed)

## Quick Lookup
- Primitives: docs/angelscript-lang/01-primitives.md
- Handles/Objects: docs/angelscript-lang/02-objects-handles.md
- Statements: docs/angelscript-lang/03-statements.md
- Expressions: docs/angelscript-lang/04-expressions.md
- Operators: docs/angelscript-lang/05-operators.md, docs/angelscript-lang/06-operator-overloads.md
- Classes: docs/angelscript-lang/07-classes.md
- Functions: docs/angelscript-lang/08-functions.md
- Type conversions: docs/angelscript-lang/09-type-conversions.md
- Globals (enums, interfaces, namespaces): docs/angelscript-lang/10-globals.md
- Advanced types (strings, arrays, lambdas): docs/angelscript-lang/11-datatypes-advanced.md
- Shared entities: docs/angelscript-lang/12-shared.md
- C++ specifics: docs/angelscript-lang/cpp-*.md files

## ⚠️ MANDATORY: Run Tests Before Completing Any Feature
You MUST run tests before marking any feature as complete:

```bash
# Rust
cargo nextest --lib
```

**Do NOT set `passes: true` unless tests actually pass!**

## Key Principles
1. **RUN TESTS** - no exceptions

## Current Task: QualifiedName-Based Registry Architecture

### Problem
Forward declarations fail because type resolution happens during Registration before all types are registered.

### Solution
Index registry by `QualifiedName` (namespace, name) instead of `TypeHash`. TypeHash computed lazily for bytecode.

### Implementation Phases
1. **Core Types** (`angelscript-core`): `QualifiedName`, `UnresolvedType`, `UnresolvedParam`, `UnresolvedSignature` - DONE
2. **Unresolved Entries** (`angelscript-core`): `UnresolvedClass`, `UnresolvedInterface`, etc. - DONE
3. **Registration Result** (`angelscript-compiler`): `RegistrationResult` struct - DONE
4. **Registry Updates** (`angelscript-registry`): `QualifiedName`-based lookup - DONE
5. **NamespaceTree** (`angelscript-registry`): Tree structure with petgraph - DONE
6. **NamespaceTree Storage** (`angelscript-registry`): Type/function registration and resolution - DONE
7. **Unified Tree** (`angelscript-registry`): Single tree with unit isolation - **NEXT**
8. **SymbolRegistry Integration** (`angelscript-registry`): Integrate tree into registry
9. **Registration Pass** (`angelscript-compiler`): Build tree, collect unresolved entries
10. **Completion Pass** (`angelscript-compiler`): Resolve using directives, transform types
11. **Compilation Pass** (`angelscript-compiler`): Use resolved registry

### Phase 7: Unified Tree (Current Priority)

Single tree with units as top-level nodes:
- `$ffi/` - FFI-registered types/functions
- `$shared/` - Shared entities across units
- `$unit_N/` - Per-compilation-unit namespaces

Edge types:
- `Contains(String)` - Parent contains child namespace
- `Uses` - Explicit `using namespace` directive
- `Mirrors` - Auto-link to same-named namespace in `$ffi`/`$shared`

Resolution order at each level:
1. Local symbols
2. `Mirrors` edges (FFI/shared counterparts)
3. `Uses` edges (explicit imports)
4. Walk up to parent

Stashed changes (`git stash show -p stash@{0}`):
- TypeBehaviors stores FunctionEntry (constructors, factories, operators)
- ClassEntry.methods stores FunctionEntry
- Overload resolution takes &[&FunctionEntry]
- VTable keeps TypeHash (runtime dispatch)

### Design Documents
- `tasks/qualified_name_registry.md` - High-level design
- `tasks/qualified_name_registry/01_core_types.md` - Core type implementations
- `tasks/qualified_name_registry/02_unresolved_entries.md` - Unresolved entry types
- `tasks/qualified_name_registry/03_registration_result.md` - Registration result
- `tasks/qualified_name_registry/04_registry.md` - Registry updates
- `tasks/qualified_name_registry/05_namespace_tree.md` - NamespaceTree core
- `tasks/qualified_name_registry/06_namespace_tree_storage.md` - Tree storage/resolution
- `tasks/qualified_name_registry/07_unified_tree.md` - Unified tree with unit isolation
- `tasks/qualified_name_registry/08_symbol_registry_integration.md` - Registry integration
- `tasks/qualified_name_registry/09_registration.md` - Registration pass
- `tasks/qualified_name_registry/10_completion.md` - Completion pass
- `tasks/qualified_name_registry/11_compilation.md` - Compilation pass
- `tasks/qualified_name_registry/namespace_tree_design.md` - Comprehensive design reference
