# Task 54: QualifiedName-Based Registry Architecture

## Problem Summary

The current compilation pipeline has forward reference issues:

1. **Registration pass** tries to resolve type names immediately to `TypeHash`
2. **TypeHash** requires the qualified name (e.g., `"Game::Player"`)
3. **Forward references fail** when a type is used before it's declared

Example that fails:
```angelscript
interface IDamageable {
    void attack(Player@ p);  // FAIL: Player not registered yet
}
class Player : IDamageable { }
```

The root cause: We compute `TypeHash` during registration, which requires type lookup, which fails on forward references.

---

## Solution Overview

Follow the C++ AngelScript approach with a **clean separation** between passes:

1. **Pass 1 (Registration)** builds namespace tree, returns `RegistrationResult` - collects unresolved entries
2. **Pass 2 (Completion)** transforms `RegistrationResult` into resolved entries, populates registry
3. **Pass 3 (Compilation)** uses fully-resolved registry for bytecode generation

Key insight: **The registry only ever contains resolved types.** Unresolved data is an intermediate representation that never enters the registry.

---

## New 3-Pass Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        NEW 3-PASS ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PASS 1: REGISTRATION (Single AST Walk)                                 │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Input:  AST + NamespaceTree (mutable)                           │   │
│  │ Output: RegistrationResult (Vec<UnresolvedClass>, etc.)         │   │
│  │                                                                 │   │
│  │ - Build namespace tree structure via get_or_create_path         │   │
│  │ - Collect type declarations as UnresolvedClass, etc.            │   │
│  │ - Collect using directives as UnresolvedUsingDirective          │   │
│  │ - NO type lookups, NO TypeHash computation                      │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              v                                          │
│  PASS 2: COMPLETION (No AST)                                            │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Input:  RegistrationResult + NamespaceTree + Global Registry    │   │
│  │ Output: Populated SymbolRegistry                                │   │
│  │                                                                 │   │
│  │ Phase 0: Resolve using directives to graph edges                │   │
│  │ Phase 1: Build name index (QualifiedName → UnresolvedEntry)     │   │
│  │ Phase 2: Resolve all UnresolvedType → QualifiedName             │   │
│  │ Phase 3: Transform Unresolved* → resolved entries               │   │
│  │ Phase 4: Register resolved entries into SymbolRegistry          │   │
│  │ Phase 5: Resolve inheritance (base classes, interfaces)         │   │
│  │ Phase 6: Copy inherited members, apply mixins                   │   │
│  │ Phase 7: Validate interface compliance                          │   │
│  │ Phase 8: Build VTables and ITables                              │   │
│  │ Phase 9: Build TypeHash indexes for bytecode                    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              v                                          │
│  PASS 3: COMPILATION (AST Walk)                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Input:  AST + Fully resolved SymbolRegistry                     │   │
│  │ Output: Bytecode                                                │   │
│  │                                                                 │   │
│  │ - Type check function bodies                                    │   │
│  │ - Generate bytecode (uses TypeHashes from registry)             │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

Each phase has detailed design in the `qualified_name_registry/` subfolder.

### Phase 1: Core Types (angelscript-core)
**Design:** [01_core_types.md](qualified_name_registry/01_core_types.md)

- Add `QualifiedName` struct
- Add `UnresolvedType`, `UnresolvedParam`, `UnresolvedSignature`
- These form the intermediate representation for Pass 1 output

### Phase 2: Unresolved Entry Types (angelscript-core)
**Design:** [02_unresolved_entries.md](qualified_name_registry/02_unresolved_entries.md)

- Add `UnresolvedClass`, `UnresolvedInterface`, `UnresolvedFuncdef`
- Add `UnresolvedFunction`, `UnresolvedGlobal`, `UnresolvedEnum`
- Add `UnresolvedUsingDirective` for deferred using resolution
- These are Pass 1 output types, distinct from resolved registry entries

### Phase 3: Registration Result (angelscript-compiler)
**Design:** [03_registration_result.md](qualified_name_registry/03_registration_result.md)

- Add `RegistrationResult` struct containing all unresolved entries
- Includes `using_directives: Vec<UnresolvedUsingDirective>`
- Pass 1 returns this instead of mutating registry

### Phase 4: Registry Updates (angelscript-registry)
**Design:** [04_registry.md](qualified_name_registry/04_registry.md)

- Add `QualifiedName`-based lookup alongside `TypeHash`
- Add `hash_to_name` reverse index (built in completion)
- Registry only stores resolved entries

### Phase 5: NamespaceTree Implementation (angelscript-registry)
**Design:** [05_namespace_tree.md](qualified_name_registry/05_namespace_tree.md)

- Implement `NamespaceTree` using `petgraph::DiGraph`
- `NamespaceEdge::Contains(String)` for hierarchy
- `NamespaceEdge::Uses` for using directive edges
- Core navigation: `get_or_create_path`, `find_child`, `find_parent`

### Phase 6: NamespaceTree Storage and Resolution (angelscript-registry)
**Design:** [06_namespace_tree_storage.md](qualified_name_registry/06_namespace_tree_storage.md)

- Type/function/global registration in tree nodes
- `ResolutionContext` for namespace-aware lookups
- Resolution algorithm: current → ancestors → using edges (non-transitive)
- Hash indexes for bytecode dispatch

### Phase 7: SymbolRegistry Integration (angelscript-registry)
**Design:** [07_symbol_registry_integration.md](qualified_name_registry/07_symbol_registry_integration.md)

- Integrate `NamespaceTree` into `SymbolRegistry`
- `::Name` syntax for explicit global scope
- Remove redundant name fields from entry types
- Iterators over types/functions

### Phase 8: Registration Pass Rewrite (angelscript-compiler)
**Design:** [08_registration.md](qualified_name_registry/08_registration.md)

- Takes mutable `NamespaceTree` reference
- Builds tree nodes on `namespace` declarations
- Collects `UnresolvedUsingDirective` (target may not exist yet)
- Returns `RegistrationResult` with unresolved entries

### Phase 9: Completion Pass Rewrite (angelscript-compiler)
**Design:** [09_completion.md](qualified_name_registry/09_completion.md)

- Phase 0: Resolve using directives to `Uses` edges
- Take `RegistrationResult` as input
- Transform unresolved entries → resolved entries
- Populate registry, build inheritance, vtables, hash indexes

### Phase 10: Compilation Pass Updates (angelscript-compiler)
**Design:** [10_compilation.md](qualified_name_registry/10_compilation.md)

- Use fully-resolved registry
- Use `ResolutionContext` for type lookups
- No changes to core logic, just use new lookup APIs

### Design Reference
**Design:** [namespace_tree_design.md](qualified_name_registry/namespace_tree_design.md)

- Comprehensive design document for namespace tree architecture
- Covers all parts: structure, edges, resolution, storage, integration

---

## Key Benefits

| Benefit | Description |
|---------|-------------|
| Type-safe phases | Can't use unresolved data in compilation |
| Registry always valid | No intermediate/partial state |
| Single AST walk | No two-phase hack in registration |
| Forward refs natural | Just store names as strings, resolve later |
| C++ alignment | Matches AngelScript C++ approach |
| Clean separation | Registration collects, Completion resolves |
| Easier testing | Pass 1 is pure function (except tree building) |
| Using directives | Graph edges, not per-type context |

---

## Data Flow

```
AST + NamespaceTree
 │
 ▼
┌──────────────────────────────────────┐
│ Pass 1: Registration                 │
│ (Builds tree, collects unresolved)   │
└──────────────────────────────────────┘
 │
 ▼
 RegistrationResult {
    classes: Vec<UnresolvedClass>,
    interfaces: Vec<UnresolvedInterface>,
    funcdefs: Vec<UnresolvedFuncdef>,
    functions: Vec<UnresolvedFunction>,
    globals: Vec<UnresolvedGlobal>,
    enums: Vec<UnresolvedEnum>,
    using_directives: Vec<UnresolvedUsingDirective>,
}
 │
 ▼
┌──────────────────────────────────────┐
│ Pass 2: Completion                   │
│ (Resolves using → transforms types)  │
└──────────────────────────────────────┘
 │
 ▼
 SymbolRegistry {
    tree: NamespaceTree {
        graph: DiGraph<NamespaceData, NamespaceEdge>,
        root: NodeIndex,
        type_hash_index: HashMap<TypeHash, (NodeIndex, String)>,
    }
}
 │
 ▼
┌──────────────────────────────────────┐
│ Pass 3: Compilation                  │
│ (Uses resolved registry)             │
└──────────────────────────────────────┘
 │
 ▼
Bytecode
```

---

## Test Cases

```angelscript
// Test 1: Forward ref in interface method
interface IDamageable {
    void takeDamage(Player@ attacker);
}
class Player : IDamageable {
    void takeDamage(Player@ attacker) {}
}

// Test 2: Forward ref in free function
void update(Enemy@ self, float dt) {}
class Enemy {}

// Test 3: Circular references
class Foo { void use(Bar@ b); }
class Bar { void use(Foo@ f); }

// Test 4: Funcdef with forward ref
funcdef void Callback(GameState@ state);
class GameState {}

// Test 5: Namespace forward ref
namespace Game {
    interface IEntity {
        void interact(Entities::Player@ p);
    }
}
namespace Game {
    namespace Entities {
        class Player : Game::IEntity {
            void interact(Player@ p) {}
        }
    }
}

// Test 6: Using namespace resolution
namespace Utils {
    class Helper {}
}
namespace Game {
    using Utils;
    class Player {
        Helper@ helper;  // Resolves via using directive
    }
}

// Test 7: Non-transitive using
namespace C { class CType {} }
namespace B { using C; }
namespace A {
    using B;
    class AType {
        CType@ c;  // ERROR: CType not visible (non-transitive)
    }
}

// Test 8: Explicit global scope
int var = 1;
namespace Parent {
    int var = 2;
    void foo() {
        Parent::var = ::var;  // ::var refers to global
    }
}
```

---

## Files Changed Summary

| Crate | Files | Change Type |
|-------|-------|-------------|
| angelscript-core | `qualified_name.rs` (new) | New struct |
| angelscript-core | `unresolved.rs` (new) | Unresolved types |
| angelscript-core | `unresolved_entries.rs` (new) | Unresolved entry types |
| angelscript-core | `entries/*.rs` | Remove redundant name fields |
| angelscript-registry | `namespace_tree.rs` (new) | Tree structure with petgraph |
| angelscript-registry | `registry.rs` | Integrate NamespaceTree |
| angelscript-compiler | `passes/registration.rs` | Complete rewrite |
| angelscript-compiler | `passes/completion.rs` | Complete rewrite |
| angelscript-compiler | `passes/mod.rs` | Add RegistrationResult |
