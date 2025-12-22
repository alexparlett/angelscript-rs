# Phase 4: Registry Updates

## Overview

Update `SymbolRegistry` to support `QualifiedName`-based lookup alongside `TypeHash`. The registry only stores resolved entries - unresolved data never enters the registry.

**Files:**
- `crates/angelscript-registry/src/registry.rs` (update)
- `crates/angelscript-core/src/entries/*.rs` (minor updates)

---

## Key Changes

### 1. Add QualifiedName to Entries

Each entry type gets a `qualified_name` field:

```rust
// In ClassEntry, InterfaceEntry, FuncdefEntry, EnumEntry

pub struct ClassEntry {
    /// Qualified name for name-based lookup.
    pub qualified_name: QualifiedName,

    // Existing fields unchanged
    pub name: String,
    pub namespace: Vec<String>,
    // ... etc
}
```

Note: We keep the existing `name`, `namespace`, and `qualified_name: String` fields for backwards compatibility during migration. These can be removed later.

### 2. Add QualifiedName Index to Registry

```rust
// crates/angelscript-registry/src/registry.rs

use angelscript_core::QualifiedName;
use rustc_hash::FxHashMap;

pub struct SymbolRegistry {
    // === Existing storage (by TypeHash) ===
    types: FxHashMap<TypeHash, TypeEntry>,
    functions: FxHashMap<TypeHash, FunctionEntry>,
    globals: FxHashMap<TypeHash, GlobalPropertyEntry>,

    // === NEW: Name-based indexes ===

    /// Type lookup by qualified name.
    /// Populated during completion pass.
    types_by_name: FxHashMap<QualifiedName, TypeHash>,

    /// Function lookup by qualified name (name -> list of overload hashes).
    /// Populated during completion pass.
    functions_by_name: FxHashMap<QualifiedName, Vec<TypeHash>>,

    /// Global lookup by qualified name.
    globals_by_name: FxHashMap<QualifiedName, TypeHash>,

    // === NEW: Namespace indexes ===

    /// Types in each namespace: namespace_string -> simple_name -> TypeHash.
    types_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,

    /// Functions in each namespace.
    functions_by_namespace: FxHashMap<String, FxHashMap<String, Vec<TypeHash>>>,

    /// Registered namespaces.
    namespaces: FxHashSet<String>,
}
```

---

## New Registration Methods

```rust
impl SymbolRegistry {
    /// Register a type with both hash and name indexing.
    pub fn register_type_with_name(
        &mut self,
        entry: TypeEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        let hash = entry.type_hash();

        // Check for duplicate by name
        if self.types_by_name.contains_key(&name) {
            return Err(RegistrationError::DuplicateType(name.to_string()));
        }

        // Check for duplicate by hash (shouldn't happen if names are unique)
        if self.types.contains_key(&hash) {
            return Err(RegistrationError::DuplicateType(name.to_string()));
        }

        // Add to name index
        self.types_by_name.insert(name.clone(), hash);

        // Add to namespace index
        let ns_key = name.namespace_string();
        self.types_by_namespace
            .entry(ns_key)
            .or_default()
            .insert(name.simple_name().to_string(), hash);

        // Register namespace
        if !name.is_global() {
            self.namespaces.insert(name.namespace_string());
        }

        // Add to hash index
        self.types.insert(hash, entry);

        Ok(())
    }

    /// Register a function with both hash and name indexing.
    pub fn register_function_with_name(
        &mut self,
        entry: FunctionEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        let hash = entry.func_hash();

        // Check for duplicate by hash (same signature)
        if self.functions.contains_key(&hash) {
            return Err(RegistrationError::DuplicateFunction {
                name: name.to_string(),
            });
        }

        // Add to name index (functions can have overloads)
        self.functions_by_name
            .entry(name.clone())
            .or_default()
            .push(hash);

        // Add to namespace index (global functions only)
        if entry.is_global() {
            let ns_key = name.namespace_string();
            self.functions_by_namespace
                .entry(ns_key)
                .or_default()
                .entry(name.simple_name().to_string())
                .or_default()
                .push(hash);
        }

        // Add to hash index
        self.functions.insert(hash, entry);

        Ok(())
    }

    /// Register a global variable with both hash and name indexing.
    pub fn register_global_with_name(
        &mut self,
        entry: GlobalPropertyEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        let hash = entry.hash();

        if self.globals_by_name.contains_key(&name) {
            return Err(RegistrationError::DuplicateGlobal(name.to_string()));
        }

        self.globals_by_name.insert(name.clone(), hash);

        let ns_key = name.namespace_string();
        self.globals_by_namespace
            .entry(ns_key)
            .or_default()
            .insert(name.simple_name().to_string(), hash);

        self.globals.insert(hash, entry);

        Ok(())
    }
}
```

---

## New Lookup Methods

```rust
impl SymbolRegistry {
    // === Lookup by QualifiedName ===

    /// Get a type by qualified name.
    pub fn get_type_by_name(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.types_by_name
            .get(name)
            .and_then(|hash| self.types.get(hash))
    }

    /// Get a type hash by qualified name.
    pub fn get_type_hash(&self, name: &QualifiedName) -> Option<TypeHash> {
        self.types_by_name.get(name).copied()
    }

    /// Check if a type exists by name.
    pub fn contains_type_name(&self, name: &QualifiedName) -> bool {
        self.types_by_name.contains_key(name)
    }

    /// Get all function overloads by qualified name.
    pub fn get_functions_by_name(&self, name: &QualifiedName) -> Vec<&FunctionEntry> {
        self.functions_by_name
            .get(name)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|h| self.functions.get(h))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a global by qualified name.
    pub fn get_global_by_name(&self, name: &QualifiedName) -> Option<&GlobalPropertyEntry> {
        self.globals_by_name
            .get(name)
            .and_then(|hash| self.globals.get(hash))
    }

    // === Name Resolution ===

    /// Resolve a type name in context.
    ///
    /// Tries the following in order:
    /// 1. Already qualified name (contains ::)
    /// 2. Current namespace
    /// 3. Each import as prefix
    /// 4. Global namespace
    pub fn resolve_type_name(
        &self,
        name: &str,
        current_namespace: &[String],
        imports: &[String],
    ) -> Option<QualifiedName> {
        // 1. If already qualified, try direct lookup
        if name.contains("::") {
            let qn = QualifiedName::from_qualified_string(name);
            if self.types_by_name.contains_key(&qn) {
                return Some(qn);
            }
            return None;
        }

        // 2. Try current namespace (innermost to outermost)
        for i in (0..=current_namespace.len()).rev() {
            let ns = current_namespace[..i].to_vec();
            let qn = QualifiedName::new(name, ns);
            if self.types_by_name.contains_key(&qn) {
                return Some(qn);
            }
        }

        // 3. Try each import as prefix
        for import in imports {
            let ns: Vec<String> = import.split("::").map(|s| s.to_string()).collect();
            let qn = QualifiedName::new(name, ns);
            if self.types_by_name.contains_key(&qn) {
                return Some(qn);
            }
        }

        // 4. Try global namespace (already tried in step 2 when i=0)
        None
    }

    /// Resolve a function name in context (returns all overloads).
    pub fn resolve_function_name(
        &self,
        name: &str,
        current_namespace: &[String],
        imports: &[String],
    ) -> Option<(QualifiedName, Vec<&FunctionEntry>)> {
        // Similar logic to resolve_type_name
        if name.contains("::") {
            let qn = QualifiedName::from_qualified_string(name);
            let funcs = self.get_functions_by_name(&qn);
            if !funcs.is_empty() {
                return Some((qn, funcs));
            }
            return None;
        }

        for i in (0..=current_namespace.len()).rev() {
            let ns = current_namespace[..i].to_vec();
            let qn = QualifiedName::new(name, ns);
            let funcs = self.get_functions_by_name(&qn);
            if !funcs.is_empty() {
                return Some((qn, funcs));
            }
        }

        for import in imports {
            let ns: Vec<String> = import.split("::").map(|s| s.to_string()).collect();
            let qn = QualifiedName::new(name, ns);
            let funcs = self.get_functions_by_name(&qn);
            if !funcs.is_empty() {
                return Some((qn, funcs));
            }
        }

        None
    }

    // === Namespace queries ===

    /// Get all types in a namespace.
    pub fn types_in_namespace(&self, namespace: &str) -> Vec<&TypeEntry> {
        self.types_by_namespace
            .get(namespace)
            .map(|types| {
                types
                    .values()
                    .filter_map(|hash| self.types.get(hash))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all registered namespaces.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        self.namespaces.iter().map(|s| s.as_str())
    }

    /// Check if a namespace exists.
    pub fn has_namespace(&self, namespace: &str) -> bool {
        self.namespaces.contains(namespace)
    }
}
```

---

## Iteration Methods

```rust
impl SymbolRegistry {
    /// Iterate over all types with their qualified names.
    pub fn types_with_names(&self) -> impl Iterator<Item = (&QualifiedName, &TypeEntry)> {
        self.types_by_name.iter().filter_map(|(name, hash)| {
            self.types.get(hash).map(|entry| (name, entry))
        })
    }

    /// Iterate over all class entries with names.
    pub fn classes_with_names(&self) -> impl Iterator<Item = (&QualifiedName, &ClassEntry)> {
        self.types_with_names().filter_map(|(name, entry)| {
            entry.as_class().map(|class| (name, class))
        })
    }

    /// Iterate over all interface entries with names.
    pub fn interfaces_with_names(&self) -> impl Iterator<Item = (&QualifiedName, &InterfaceEntry)> {
        self.types_with_names().filter_map(|(name, entry)| {
            entry.as_interface().map(|iface| (name, iface))
        })
    }
}
```

---

## Entry Type Updates

Add `qualified_name` field to entry types:

```rust
// crates/angelscript-core/src/entries/class.rs

use crate::QualifiedName;

pub struct ClassEntry {
    // NEW: Structured qualified name
    pub qualified_name: QualifiedName,

    // Existing (kept for compatibility, can be derived from qualified_name)
    pub name: String,
    pub namespace: Vec<String>,
    pub qualified_name_str: String,  // Renamed to avoid conflict

    // ... rest unchanged
}

impl ClassEntry {
    /// Create a new class entry with qualified name.
    pub fn new_with_name(
        qualified_name: QualifiedName,
        type_hash: TypeHash,
        type_kind: TypeKind,
        source: TypeSource,
    ) -> Self {
        Self {
            name: qualified_name.simple_name().to_string(),
            namespace: qualified_name.namespace_path().to_vec(),
            qualified_name_str: qualified_name.to_string(),
            qualified_name,
            type_hash,
            type_kind,
            source,
            // ... defaults for rest
        }
    }
}
```

---

## Backwards Compatibility

During migration, the registry supports both:
- Old `TypeHash`-based registration (existing FFI code)
- New `QualifiedName`-based registration (completion pass)

```rust
impl SymbolRegistry {
    /// Legacy: Register type by hash only (for FFI).
    ///
    /// Builds the name index from the entry's qualified_name field.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let name = entry.qualified_name().clone();
        self.register_type_with_name(entry, name)
    }

    /// Legacy: Get type by hash.
    pub fn get(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.types.get(&hash)
    }

    /// Legacy: Get mutable type by hash.
    pub fn get_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        self.types.get_mut(&hash)
    }
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_type_with_name() {
        let mut registry = SymbolRegistry::new();
        let name = QualifiedName::new("Player", vec!["Game".into()]);
        let entry = ClassEntry::new_with_name(
            name.clone(),
            name.to_type_hash(),
            TypeKind::ScriptObject,
            TypeSource::ffi_untyped(),
        );

        registry.register_type_with_name(entry.into(), name.clone()).unwrap();

        assert!(registry.contains_type_name(&name));
        assert!(registry.get_type_by_name(&name).is_some());
    }

    #[test]
    fn resolve_type_name_global() {
        let mut registry = SymbolRegistry::new();
        let name = QualifiedName::global("Player");
        let entry = ClassEntry::new_with_name(
            name.clone(),
            name.to_type_hash(),
            TypeKind::ScriptObject,
            TypeSource::ffi_untyped(),
        );
        registry.register_type_with_name(entry.into(), name).unwrap();

        let resolved = registry.resolve_type_name("Player", &[], &[]);
        assert_eq!(resolved, Some(QualifiedName::global("Player")));
    }

    #[test]
    fn resolve_type_name_in_namespace() {
        let mut registry = SymbolRegistry::new();
        let name = QualifiedName::new("Entity", vec!["Game".into()]);
        let entry = ClassEntry::new_with_name(
            name.clone(),
            name.to_type_hash(),
            TypeKind::ScriptObject,
            TypeSource::ffi_untyped(),
        );
        registry.register_type_with_name(entry.into(), name).unwrap();

        // Resolve from same namespace
        let resolved = registry.resolve_type_name(
            "Entity",
            &["Game".into()],
            &[],
        );
        assert_eq!(resolved, Some(QualifiedName::new("Entity", vec!["Game".into()])));
    }

    #[test]
    fn resolve_type_name_with_import() {
        let mut registry = SymbolRegistry::new();
        let name = QualifiedName::new("Vector3", vec!["Math".into()]);
        let entry = ClassEntry::new_with_name(
            name.clone(),
            name.to_type_hash(),
            TypeKind::value(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type_with_name(entry.into(), name).unwrap();

        // Resolve using import
        let resolved = registry.resolve_type_name(
            "Vector3",
            &[],  // Not in Math namespace
            &["Math".into()],  // But Math is imported
        );
        assert_eq!(resolved, Some(QualifiedName::new("Vector3", vec!["Math".into()])));
    }

    #[test]
    fn function_overloads_by_name() {
        let mut registry = SymbolRegistry::new();
        let name = QualifiedName::global("print");

        // Register two overloads
        let entry1 = FunctionEntry::new_simple("print", vec![], DataType::void());
        let entry2 = FunctionEntry::new_simple("print", vec![DataType::int32()], DataType::void());

        registry.register_function_with_name(entry1, name.clone()).unwrap();
        registry.register_function_with_name(entry2, name.clone()).unwrap();

        let overloads = registry.get_functions_by_name(&name);
        assert_eq!(overloads.len(), 2);
    }
}
```

---

## Dependencies

- Phase 1: `QualifiedName` struct

---

## What's Next

Phase 5 will rewrite the Registration pass to return `RegistrationResult` instead of mutating the registry.
