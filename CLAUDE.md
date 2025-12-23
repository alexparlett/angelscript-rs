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
1. **Core Types** (`angelscript-core`): `QualifiedName`, `UnresolvedType`, `UnresolvedParam`, `UnresolvedSignature`
2. **Unresolved Entries** (`angelscript-core`): `UnresolvedClass`, `UnresolvedInterface`, etc.
3. **Registration Result** (`angelscript-compiler`): `RegistrationResult` struct
4. **Registry Updates** (`angelscript-registry`): `QualifiedName`-based lookup
5. **NamespaceTree** (`angelscript-registry`): Tree structure with petgraph
6. **NamespaceTree Storage** (`angelscript-registry`): Type/function registration and resolution
7. **SymbolRegistry Integration** (`angelscript-registry`): Integrate tree into registry
8. **Registration Pass** (`angelscript-compiler`): Build tree, collect unresolved entries
9. **Completion Pass** (`angelscript-compiler`): Resolve using directives, transform types
10. **Compilation Pass** (`angelscript-compiler`): Use resolved registry

### Design Documents
- `tasks/qualified_name_registry.md` - High-level design
- `tasks/qualified_name_registry/01_core_types.md` - Core type implementations
- `tasks/qualified_name_registry/02_unresolved_entries.md` - Unresolved entry types
- `tasks/qualified_name_registry/03_registration_result.md` - Registration result
- `tasks/qualified_name_registry/04_registry.md` - Registry updates
- `tasks/qualified_name_registry/05_namespace_tree.md` - NamespaceTree core
- `tasks/qualified_name_registry/06_namespace_tree_storage.md` - Tree storage/resolution
- `tasks/qualified_name_registry/07_symbol_registry_integration.md` - Registry integration
- `tasks/qualified_name_registry/08_registration.md` - Registration pass
- `tasks/qualified_name_registry/09_completion.md` - Completion pass
- `tasks/qualified_name_registry/10_compilation.md` - Compilation pass
- `tasks/qualified_name_registry/namespace_tree_design.md` - Comprehensive design reference
