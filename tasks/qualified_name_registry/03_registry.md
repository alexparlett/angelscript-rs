# Phase 3: Registry Rewrite

## Overview

Rewrite `SymbolRegistry` to use `QualifiedName` as the primary key instead of `TypeHash`. TypeHash becomes a secondary index computed lazily for bytecode generation.

**Files:**
- `crates/angelscript-registry/src/registry.rs` (rewrite)

---

## New Storage Model

```rust
// crates/angelscript-registry/src/registry.rs

use rustc_hash::{FxHashMap, FxHashSet};
use angelscript_core::{
    QualifiedName, TypeEntry, FunctionEntry, GlobalPropertyEntry,
    TypeHash, RegistrationError,
};

/// Unified type and function registry.
///
/// Primary indexing by `QualifiedName` for deferred resolution.
/// Secondary indexing by `TypeHash` for bytecode generation.
#[derive(Default)]
pub struct SymbolRegistry {
    // === Primary Storage (by QualifiedName) ===

    /// All types by qualified name (O(1) lookup).
    types: FxHashMap<QualifiedName, TypeEntry>,

    /// All functions by qualified name -> overloads.
    /// Multiple functions can share the same name (overloading).
    functions: FxHashMap<QualifiedName, Vec<FunctionEntry>>,

    /// Global properties by qualified name.
    globals: FxHashMap<QualifiedName, GlobalPropertyEntry>,

    // === Secondary Indexes (built in completion) ===

    /// TypeHash -> QualifiedName reverse index.
    /// Built during completion pass for bytecode generation.
    hash_to_name: FxHashMap<TypeHash, QualifiedName>,

    /// Function hash -> (QualifiedName, overload_index).
    /// Built during completion pass for bytecode generation.
    func_hash_to_name: FxHashMap<TypeHash, (QualifiedName, usize)>,

    // === Namespace Indexes (for scope resolution) ===

    /// Types indexed by namespace: namespace -> {simple_name -> qualified_name}.
    types_by_namespace: FxHashMap<String, FxHashMap<String, QualifiedName>>,

    /// Functions indexed by namespace: namespace -> {simple_name -> qualified_name}.
    functions_by_namespace: FxHashMap<String, FxHashMap<String, QualifiedName>>,

    /// Globals indexed by namespace: namespace -> {simple_name -> qualified_name}.
    globals_by_namespace: FxHashMap<String, FxHashMap<String, QualifiedName>>,

    /// Registered namespaces.
    namespaces: FxHashSet<String>,

    // === Completion State ===

    /// Whether the hash indexes have been built.
    indexes_built: bool,
}
```

---

## API Changes

### Registration (Phase 1)

Registration uses `QualifiedName` only - no TypeHash computation.

```rust
impl SymbolRegistry {
    /// Register a type by qualified name.
    ///
    /// No TypeHash computed - that happens in completion.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let qualified_name = entry.qualified_name().clone();

        if self.types.contains_key(&qualified_name) {
            return Err(RegistrationError::DuplicateType(qualified_name.to_string()));
        }

        // Add to namespace index
        let ns_key = qualified_name.namespace().join("::");
        self.types_by_namespace
            .entry(ns_key)
            .or_default()
            .insert(qualified_name.simple_name().to_string(), qualified_name.clone());

        self.types.insert(qualified_name, entry);
        Ok(())
    }

    /// Register a function by qualified name.
    ///
    /// Adds to overload list for that name.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        let qualified_name = entry.qualified_name().clone();

        // Functions can have overloads, so we don't check for duplicates by name
        // Duplicate detection happens by signature during completion

        // Add to namespace index (only for global functions)
        if entry.is_global() {
            let ns_key = qualified_name.namespace().join("::");
            self.functions_by_namespace
                .entry(ns_key)
                .or_default()
                .insert(qualified_name.simple_name().to_string(), qualified_name.clone());
        }

        self.functions
            .entry(qualified_name)
            .or_default()
            .push(entry);
        Ok(())
    }

    /// Register a global property.
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        let qualified_name = entry.qualified_name().clone();

        if self.globals.contains_key(&qualified_name) {
            return Err(RegistrationError::DuplicateRegistration {
                name: qualified_name.to_string(),
                kind: "global property".to_string(),
            });
        }

        // Add to namespace index
        let ns_key = qualified_name.namespace().join("::");
        self.globals_by_namespace
            .entry(ns_key)
            .or_default()
            .insert(qualified_name.simple_name().to_string(), qualified_name.clone());

        self.globals.insert(qualified_name, entry);
        Ok(())
    }
}
```

### Lookup by Name (All Passes)

```rust
impl SymbolRegistry {
    /// Get a type by qualified name.
    pub fn get(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.types.get(name)
    }

    /// Get a mutable type by qualified name.
    pub fn get_mut(&mut self, name: &QualifiedName) -> Option<&mut TypeEntry> {
        self.types.get_mut(name)
    }

    /// Check if a type exists by name.
    pub fn contains_type(&self, name: &QualifiedName) -> bool {
        self.types.contains_key(name)
    }

    /// Get all function overloads by qualified name.
    pub fn get_functions(&self, name: &QualifiedName) -> Option<&[FunctionEntry]> {
        self.functions.get(name).map(|v| v.as_slice())
    }

    /// Get a global by qualified name.
    pub fn get_global(&self, name: &QualifiedName) -> Option<&GlobalPropertyEntry> {
        self.globals.get(name)
    }
}
```

### Hash Index Building (Completion Pass)

```rust
impl SymbolRegistry {
    /// Build the TypeHash -> QualifiedName indexes.
    ///
    /// Called once at the end of completion pass.
    /// After this, `get_by_hash` and `get_function_by_hash` work.
    pub fn build_hash_indexes(&mut self) {
        if self.indexes_built {
            return;
        }

        // Build type hash index
        for (name, entry) in &self.types {
            let hash = entry.type_hash();
            self.hash_to_name.insert(hash, name.clone());
        }

        // Build function hash index
        for (name, overloads) in &self.functions {
            for (idx, entry) in overloads.iter().enumerate() {
                let hash = entry.func_hash();
                self.func_hash_to_name.insert(hash, (name.clone(), idx));
            }
        }

        self.indexes_built = true;
    }

    /// Get a type by its hash (requires indexes built).
    ///
    /// Used during bytecode generation.
    pub fn get_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        debug_assert!(self.indexes_built, "Hash indexes not built");
        self.hash_to_name.get(&hash)
            .and_then(|name| self.types.get(name))
    }

    /// Get a function by its hash (requires indexes built).
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        debug_assert!(self.indexes_built, "Hash indexes not built");
        self.func_hash_to_name.get(&hash)
            .and_then(|(name, idx)| {
                self.functions.get(name)
                    .and_then(|overloads| overloads.get(*idx))
            })
    }
}
```

### Namespace Resolution (Completion Pass)

```rust
impl SymbolRegistry {
    /// Resolve a type name in context.
    ///
    /// Tries the following in order:
    /// 1. Qualified name as-is
    /// 2. Relative to current namespace
    /// 3. Using each active import as prefix
    /// 4. Global namespace
    pub fn resolve_type_name(
        &self,
        name: &str,
        current_namespace: &[String],
        imports: &[String],
    ) -> Option<QualifiedName> {
        // 1. Try as qualified name
        let qualified = QualifiedName::from_qualified_string(name);
        if self.types.contains_key(&qualified) {
            return Some(qualified);
        }

        // 2. Try relative to current namespace
        if !current_namespace.is_empty() {
            let mut ns = current_namespace.to_vec();
            ns.push(name.to_string());
            let relative = QualifiedName::new(
                ns.pop().unwrap(),
                ns,
            );
            // Wait, this is wrong. Let me fix:
            let relative = QualifiedName::new(name, current_namespace.to_vec());
            if self.types.contains_key(&relative) {
                return Some(relative);
            }
        }

        // 3. Try each import as prefix
        for import in imports {
            let imported = QualifiedName::from_qualified_string(&format!("{}::{}", import, name));
            if self.types.contains_key(&imported) {
                return Some(imported);
            }
        }

        // 4. Try global namespace
        let global = QualifiedName::global(name);
        if self.types.contains_key(&global) {
            return Some(global);
        }

        None
    }

    /// Get types in a specific namespace.
    pub fn get_namespace_types(&self, namespace: &str) -> Option<&FxHashMap<String, QualifiedName>> {
        self.types_by_namespace.get(namespace)
    }

    /// Get functions in a specific namespace.
    pub fn get_namespace_functions(&self, namespace: &str) -> Option<&FxHashMap<String, QualifiedName>> {
        self.functions_by_namespace.get(namespace)
    }
}
```

---

## Iteration Methods

```rust
impl SymbolRegistry {
    /// Iterate over all types.
    pub fn types(&self) -> impl Iterator<Item = (&QualifiedName, &TypeEntry)> {
        self.types.iter()
    }

    /// Iterate over all type entries (values only).
    pub fn type_entries(&self) -> impl Iterator<Item = &TypeEntry> {
        self.types.values()
    }

    /// Iterate over all class entries.
    pub fn classes(&self) -> impl Iterator<Item = &ClassEntry> {
        self.types.values().filter_map(|t| t.as_class())
    }

    /// Iterate over all interface entries.
    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceEntry> {
        self.types.values().filter_map(|t| t.as_interface())
    }

    /// Iterate over all functions.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionEntry> {
        self.functions.values().flatten()
    }

    /// Get number of registered types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.values().map(|v| v.len()).sum()
    }
}
```

---

## Inheritance Helpers

```rust
impl SymbolRegistry {
    /// Get the inheritance chain for a class.
    ///
    /// Requires inheritance to be resolved (completion pass done).
    pub fn base_class_chain(&self, name: &QualifiedName) -> Vec<&ClassEntry> {
        let mut chain = Vec::new();
        let mut current = name.clone();

        while let Some(entry) = self.types.get(&current)
            && let Some(class) = entry.as_class()
            && let InheritanceRef::Resolved(base_name) = &class.base_class
            && !base_name.simple_name().is_empty() // Skip void/no-base
            && let Some(base_entry) = self.types.get(base_name)
            && let Some(base_class) = base_entry.as_class()
        {
            chain.push(base_class);
            current = base_name.clone();
        }

        chain
    }

    /// Get all interfaces implemented by a class (including inherited).
    pub fn all_interfaces(&self, name: &QualifiedName) -> Vec<&InterfaceEntry> {
        let mut interfaces = Vec::new();

        // Own interfaces
        if let Some(class) = self.types.get(name).and_then(|t| t.as_class()) {
            for iface_ref in &class.interfaces {
                if let InheritanceRef::Resolved(iface_name) = iface_ref {
                    if let Some(iface) = self.types.get(iface_name).and_then(|t| t.as_interface()) {
                        interfaces.push(iface);
                    }
                }
            }
        }

        // Inherited interfaces
        for base in self.base_class_chain(name) {
            for iface_ref in &base.interfaces {
                if let InheritanceRef::Resolved(iface_name) = iface_ref {
                    if let Some(iface) = self.types.get(iface_name).and_then(|t| t.as_interface()) {
                        if !interfaces.iter().any(|i| i.qualified_name == iface.qualified_name) {
                            interfaces.push(iface);
                        }
                    }
                }
            }
        }

        interfaces
    }
}
```

---

## FFI Entry Creation

FFI entries can still use convenience constructors that compute QualifiedName internally:

```rust
impl ClassEntry {
    /// Create an FFI class in global namespace.
    pub fn ffi(name: impl Into<String>, type_kind: TypeKind) -> Self {
        let name = name.into();
        Self::new(
            QualifiedName::global(&name),
            type_kind,
            TypeSource::ffi_untyped(),
        )
    }

    /// Create an FFI class in a specific namespace.
    pub fn ffi_namespaced(
        name: impl Into<String>,
        namespace: Vec<String>,
        type_kind: TypeKind,
    ) -> Self {
        Self::new(
            QualifiedName::new(name, namespace),
            type_kind,
            TypeSource::ffi_untyped(),
        )
    }
}
```

---

## Migration Path

### Breaking Changes

1. `get(TypeHash)` replaced with `get(&QualifiedName)`
2. `get_by_hash(TypeHash)` only works after `build_hash_indexes()`
3. Registration methods take entries with `QualifiedName` instead of computing hash
4. Iteration returns `(&QualifiedName, &TypeEntry)` pairs

### Compatibility Layer (Optional)

For gradual migration, can add deprecated methods:

```rust
impl SymbolRegistry {
    #[deprecated(note = "Use get_by_hash after build_hash_indexes")]
    pub fn get_legacy(&self, hash: TypeHash) -> Option<&TypeEntry> {
        // Linear scan fallback
        self.types.values().find(|e| e.type_hash() == hash)
    }
}
```

---

## Dependencies

- Phase 1: `QualifiedName` struct must exist
- Phase 2: Entry types must use `QualifiedName` and `InheritanceRef`

Phase 4 (Registration) and Phase 5 (Completion) will use this new registry API.
