# Current Task: Global Properties (Task 25)

**Status:** Phase 2 Complete
**Date:** 2025-12-09
**Branch:** 025-adding-global-properties

---

## Task 25: Global Properties

Enables FFI registration of global properties accessible from scripts.

### Phase 1: Core Types ✓ (Committed: 9106321)

| Type | Location | Purpose |
|------|----------|---------|
| `ConstantValue` | entries/global_property.rs | Primitive constants (Copy) |
| `GlobalPropertyEntry` | entries/global_property.rs | Complete entry with metadata |
| `GlobalPropertyImpl` | entries/global_property.rs | Storage: Constant/Mutable/Script |
| `GlobalPropertyAccessor` | entries/global_property.rs | Type-erased read/write for Arc<RwLock<T>> |
| `IntoGlobalProperty` | entries/global_property.rs | Conversion trait (primitives + Arc<RwLock<T>>) |
| `PropertyError` | entries/global_property.rs | Error type (→ RuntimeError) |
| `GlobalMeta` | meta.rs | Macro-generated metadata |

### Phase 2: Registry & Module ✓

- [x] **Step 6:** Add globals storage to SymbolRegistry
  - Added `globals: FxHashMap<TypeHash, GlobalPropertyEntry>` field
  - Added `register_global()`, `get_global()`, `get_global_by_name()`, `contains_global()`, `globals()`, `global_count()`
  - No separate `global_by_name` map needed - `TypeHash::from_name()` computes hash directly
- [x] **Step 7:** Add `global()` to Module
  - Added `globals: Vec<GlobalPropertyEntry>` field
  - Added `global<V: IntoGlobalProperty>(name, value)` builder method
  - Updated `is_empty()` and `len()` to include globals
  - Helper `qualify_name()` for namespace-qualified names
- [x] `global_meta()` not needed - uses `IntoGlobalProperty` trait

**Key fix:** Changed `GlobalPropertyImpl::data_type()` to return `DataType` directly instead of `Option<DataType>` since all variants always have a data type.

### Phase 3: Macro (Pending)

- [ ] **Step 9:** Add `#[angelscript::global]` attribute macro
- [ ] **Step 10:** Comprehensive tests

---

## Design Summary

**Two property types:**
1. **Constants** - Primitives via `ConstantValue`, immutable, can inline
2. **Mutable** - `Arc<RwLock<T>>` for shared state with scripts

**API:**
```rust
// Primitive constants
Module::new()
    .global("PI", 3.14159f64)           // const double PI
    .global("MAX", 100i32);             // const int MAX

// Mutable shared state
let score = Arc::new(RwLock::new(0i32));
Module::new()
    .global("score", score.clone());    // int score

// With namespace
Module::in_namespace(&["math"])
    .global("PI", 3.14159f64);          // math::PI
```

---

## Key Files

- `crates/angelscript-core/src/entries/global_property.rs` - Core types
- `crates/angelscript-core/src/meta.rs` - GlobalMeta
- `crates/angelscript-registry/src/registry.rs` - SymbolRegistry with globals
- `crates/angelscript-registry/src/module.rs` - Module API with global()
- `claude/tasks/25_global_properties.md` - Full task spec

---

## Next Step

Phase 3: Add `#[angelscript::global]` attribute macro
- Located in `crates/angelscript-macros/`
- Generate `GlobalMeta` from static declarations
