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

1. **Pass 1 (Registration)** returns `RegistrationResult` - pure data, no registry mutation
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
│  │ Input:  AST                                                     │   │
│  │ Output: RegistrationResult (Vec<UnresolvedClass>, etc.)         │   │
│  │                                                                 │   │
│  │ - Collect type declarations as UnresolvedClass, etc.            │   │
│  │ - Store inheritance/signatures as UnresolvedType                │   │
│  │ - NO type lookups, NO TypeHash computation, NO registry access  │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              v                                          │
│  PASS 2: COMPLETION (No AST)                                            │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Input:  RegistrationResult + Global Registry (for FFI types)    │   │
│  │ Output: Populated SymbolRegistry                                │   │
│  │                                                                 │   │
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
- These are Pass 1 output types, distinct from resolved registry entries

### Phase 3: Registration Result (angelscript-compiler)
**Design:** [03_registration_result.md](qualified_name_registry/03_registration_result.md)

- Add `RegistrationResult` struct containing all unresolved entries
- Pass 1 returns this instead of mutating registry

### Phase 4: Registry Updates (angelscript-registry)
**Design:** [04_registry.md](qualified_name_registry/04_registry.md)

- Add `QualifiedName`-based lookup alongside `TypeHash`
- Add `hash_to_name` reverse index (built in completion)
- Registry only stores resolved entries

### Phase 4b: Namespace Tree Design
**Design:** [04b_namespace_tree.md](qualified_name_registry/04b_namespace_tree.md)

- Replace flat `HashMap<QualifiedName, TypeEntry>` with tree structure
- Remove redundant `name`, `namespace`, `qualified_name` fields from all entry types
- Tree-based type resolution for efficient `using` directive handling
- This fundamentally changes how Phases 5/6/7 work

### Phase 5: Registration Pass Rewrite (angelscript-compiler)
**Design:** [05_registration.md](qualified_name_registry/05_registration.md)

- Remove all `TypeResolver` usage
- Remove all registry mutations
- Return `RegistrationResult` with unresolved entries

### Phase 6: Completion Pass Rewrite (angelscript-compiler)
**Design:** [06_completion.md](qualified_name_registry/06_completion.md)

- Take `RegistrationResult` as input
- Transform unresolved entries → resolved entries
- Populate registry
- Build inheritance, vtables, hash indexes

### Phase 7: Compilation Pass Updates (angelscript-compiler)
**Design:** [07_compilation.md](qualified_name_registry/07_compilation.md)

- Use fully-resolved registry
- No changes to core logic, just use new lookup APIs

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
| Easier testing | Pass 1 is pure function, easy to unit test |

---

## Data Flow

```
AST
 │
 ▼
┌──────────────────────────────────────┐
│ Pass 1: Registration                 │
│ (Pure function - no side effects)    │
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
}
 │
 ▼
┌──────────────────────────────────────┐
│ Pass 2: Completion                   │
│ (Transforms + populates registry)    │
└──────────────────────────────────────┘
 │
 ▼
SymbolRegistry {
    types: HashMap<QualifiedName, TypeEntry>,     // All resolved
    functions: HashMap<QualifiedName, Vec<FunctionEntry>>,
    hash_to_name: HashMap<TypeHash, QualifiedName>,
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
namespace Game::Entities {
    class Player : Game::IEntity {
        void interact(Player@ p) {}
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
| angelscript-core | `entries/*.rs` | Minor updates for QualifiedName |
| angelscript-registry | `registry.rs` | Add QualifiedName lookup |
| angelscript-compiler | `passes/registration.rs` | Complete rewrite |
| angelscript-compiler | `passes/completion.rs` | Complete rewrite |
| angelscript-compiler | `passes/mod.rs` | Add RegistrationResult |
| angelscript-compiler | `type_resolver.rs` | Move to completion only |
