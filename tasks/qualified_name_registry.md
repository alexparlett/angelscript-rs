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

Follow the C++ AngelScript approach:

1. **Registry indexed by `QualifiedName`** instead of `TypeHash`
2. **TypeHash computed lazily** when needed for bytecode/overloads
3. **Forward references stored as `UnresolvedType`** - no resolution during registration
4. **Resolution deferred to Completion pass** when all types exist

---

## New 3-Pass Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        NEW 3-PASS ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  PASS 1: REGISTRATION (Single AST Walk)                                 │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ - Register types by QualifiedName                               │   │
│  │ - Store inheritance as UnresolvedType (no resolution)           │   │
│  │ - Store signatures with UnresolvedType (no resolution)          │   │
│  │ - NO type lookups, NO TypeHash computation                      │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              v                                          │
│  PASS 2: COMPLETION (No AST)                                            │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ Phase 1: Resolve all UnresolvedType -> QualifiedName            │   │
│  │ Phase 2: Resolve inheritance (base classes, interfaces)         │   │
│  │ Phase 3: Copy inherited members, apply mixins                   │   │
│  │ Phase 4: Validate interface compliance                          │   │
│  │ Phase 5: Build VTables and ITables                              │   │
│  │ Phase 6: Compute TypeHashes, build hash index                   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                              │                                          │
│                              v                                          │
│  PASS 3: COMPILATION (AST Walk)                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ - Type check function bodies                                    │   │
│  │ - Generate bytecode (uses TypeHashes from Completion)           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Implementation Phases

Each phase has detailed design in the `54_qualified_name_registry/` subfolder.

### Phase 1: Core Types (angelscript-core)
**Design:** [01_core_types.md](54_qualified_name_registry/01_core_types.md)

- Add `QualifiedName` struct
- Add `UnresolvedType` and `UnresolvedParam` structs
- These are the foundation everything else builds on

### Phase 2: Entry Type Updates (angelscript-core)
**Design:** [02_entry_types.md](54_qualified_name_registry/02_entry_types.md)

- Update `ClassEntry`, `InterfaceEntry`, `FuncdefEntry`
- Add `OnceCell<TypeHash>` for lazy hash computation
- Change inheritance fields to use `UnresolvedType`
- Add unresolved signature support to `FunctionDef`

### Phase 3: Registry Rewrite (angelscript-registry)
**Design:** [03_registry.md](54_qualified_name_registry/03_registry.md)

- Primary key: `QualifiedName` instead of `TypeHash`
- Add `hash_to_name` reverse index
- Update all lookup and registration methods

### Phase 4: Registration Pass (angelscript-compiler)
**Design:** [04_registration.md](54_qualified_name_registry/04_registration.md)

- Remove `TypeResolver` usage during registration
- Store `UnresolvedType` instead of resolved `DataType`
- Single AST walk, no type resolution

### Phase 5: Completion Pass (angelscript-compiler)
**Design:** [05_completion.md](54_qualified_name_registry/05_completion.md)

- Add type resolution phase
- Integrate with existing inheritance completion
- Build hash index after all types resolved

### Phase 6: Compilation Pass (angelscript-compiler)
**Design:** [06_compilation.md](54_qualified_name_registry/06_compilation.md)

- Update lookups to use resolved types
- TypeResolver moves here for expression type checking

---

## Key Benefits

| Benefit | Description |
|---------|-------------|
| Single AST walk | No two-phase hack in registration |
| Forward refs natural | Just store names as strings |
| C++ alignment | Matches AngelScript C++ approach |
| Clean separation | Registration gathers, Completion resolves |
| TypeHash for bytecode only | Not used for internal indexing |

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
| angelscript-core | `unresolved.rs` (new) | New structs |
| angelscript-core | `entries/class.rs` | Update inheritance types |
| angelscript-core | `entries/interface.rs` | Update base_interfaces type |
| angelscript-core | `entries/funcdef.rs` | Add unresolved signature |
| angelscript-core | `function_def.rs` | Add unresolved fields |
| angelscript-registry | `registry.rs` | Complete rewrite |
| angelscript-compiler | `passes/registration.rs` | Remove type resolution |
| angelscript-compiler | `passes/completion.rs` | Add type resolution phase |
| angelscript-compiler | `passes/compilation.rs` | Update lookups |
| angelscript-compiler | `type_resolver.rs` | Move to Completion/Compilation |
