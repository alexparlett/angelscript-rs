# Phase 4: Registry Updates

## Overview

Update `SymbolRegistry` to use `QualifiedName` as the **primary key** for type storage. The `types: FxHashMap<TypeHash, TypeEntry>` map is **removed** - TypeHash is accessed directly from each entry when needed.

**Key Insight:** TypeHash is already stored on each entry (`ClassEntry.type_hash`, `FunctionDef.func_hash`, etc.), so storing entries again by hash is redundant. The VM will use `FxHashMap<TypeHash, CompiledFunction>` in the bytecode module for O(1) function dispatch at runtime - that's the appropriate place for hash-based lookup.

**Files:**
- `crates/angelscript-registry/src/registry.rs` (major update)
- `crates/angelscript-core/src/entries/*.rs` (add QualifiedName field)

---

## Key Changes

### 1. Remove Hash-Based Type Storage

The `types: FxHashMap<TypeHash, TypeEntry>` map is removed. Instead:
- Types are stored by `QualifiedName` as primary key
- TypeHash is accessed from the entry itself when needed
- A reverse index `hash_to_name` provides hash-based lookup when required

### 2. Add QualifiedName to Entries

Each entry type gets a `qname` field (using short name to avoid conflict with existing `qualified_name: String`):

```rust
// In ClassEntry, InterfaceEntry, FuncdefEntry, EnumEntry

pub struct ClassEntry {
    /// Structured qualified name for name-based lookup.
    pub qname: QualifiedName,

    // Existing fields unchanged (will be deprecated later)
    pub name: String,
    pub namespace: Vec<String>,
    pub qualified_name: String,
    pub type_hash: TypeHash,  // Hash accessed from here, not from map key
    // ... etc
}
```

### 3. New Registry Structure

```rust
// crates/angelscript-registry/src/registry.rs

use angelscript_core::QualifiedName;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct SymbolRegistry {
    // === PRIMARY: Name-based storage ===

    /// Types stored by qualified name (PRIMARY storage).
    types: FxHashMap<QualifiedName, TypeEntry>,

    /// Functions stored by hash (hash encodes signature for overload uniqueness).
    functions: FxHashMap<TypeHash, FunctionEntry>,

    /// Global properties by hash.
    globals: FxHashMap<TypeHash, GlobalPropertyEntry>,

    // === SECONDARY: Indexes for lookup ===

    /// Reverse index: hash -> name (for hash-based lookups).
    /// Built during registration.
    type_hash_to_name: FxHashMap<TypeHash, QualifiedName>,

    /// Function lookup by qualified name (name -> list of overload hashes).
    functions_by_name: FxHashMap<QualifiedName, Vec<TypeHash>>,

    /// Global lookup by qualified name.
    globals_by_name: FxHashMap<QualifiedName, TypeHash>,

    // === Namespace indexes (existing, kept for scope building) ===

    /// Types in each namespace: namespace_string -> simple_name -> QualifiedName.
    types_by_namespace: FxHashMap<String, FxHashMap<String, QualifiedName>>,

    /// Functions in each namespace.
    functions_by_namespace: FxHashMap<String, FxHashMap<String, Vec<TypeHash>>>,

    /// Globals in each namespace.
    globals_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,

    /// Registered namespaces.
    namespaces: FxHashSet<String>,

    // === Type aliases (existing) ===
    type_aliases: FxHashMap<String, TypeHash>,
    type_aliases_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,
}
```

---

## Registration Methods

```rust
impl SymbolRegistry {
    /// Register a type by qualified name (primary method).
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let name = entry.qname().clone();
        let hash = entry.type_hash();

        // Check for duplicate by name
        if self.types.contains_key(&name) {
            return Err(RegistrationError::DuplicateType(name.to_string()));
        }

        // Build reverse index
        self.type_hash_to_name.insert(hash, name.clone());

        // Add to namespace index
        if !entry.is_template_param() {
            let ns_key = name.namespace_string();
            self.types_by_namespace
                .entry(ns_key)
                .or_default()
                .insert(name.simple_name().to_string(), name.clone());
        }

        // Register namespace
        if !name.is_global() {
            self.namespaces.insert(name.namespace_string());
        }

        // Store by name (primary)
        self.types.insert(name, entry);

        Ok(())
    }

    /// Register a function.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        let hash = entry.def.func_hash;
        let name = entry.def.qname().clone();

        // Check for duplicate by hash (same signature)
        if self.functions.contains_key(&hash) {
            return Err(RegistrationError::DuplicateRegistration {
                name: name.to_string(),
                kind: "function".to_string(),
            });
        }

        // Add to name index (functions can have overloads)
        self.functions_by_name
            .entry(name.clone())
            .or_default()
            .push(hash);

        // Add to namespace index (global functions only)
        if entry.def.object_type.is_none() {
            let ns_key = name.namespace_string();
            self.functions_by_namespace
                .entry(ns_key)
                .or_default()
                .entry(name.simple_name().to_string())
                .or_default()
                .push(hash);
        }

        // Store by hash (hash encodes signature uniqueness)
        self.functions.insert(hash, entry);

        Ok(())
    }

    /// Register a global property.
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        let hash = entry.type_hash;
        let name = entry.qname().clone();

        if self.globals_by_name.contains_key(&name) {
            return Err(RegistrationError::DuplicateRegistration {
                name: name.to_string(),
                kind: "global property".to_string(),
            });
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

## Lookup Methods

```rust
impl SymbolRegistry {
    // === Primary: Lookup by QualifiedName ===

    /// Get a type by qualified name.
    pub fn get_type(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.types.get(name)
    }

    /// Get a mutable type by qualified name.
    pub fn get_type_mut(&mut self, name: &QualifiedName) -> Option<&mut TypeEntry> {
        self.types.get_mut(name)
    }

    /// Check if a type exists by name.
    pub fn contains_type(&self, name: &QualifiedName) -> bool {
        self.types.contains_key(name)
    }

    /// Get a type's hash by qualified name.
    pub fn get_type_hash(&self, name: &QualifiedName) -> Option<TypeHash> {
        self.types.get(name).map(|e| e.type_hash())
    }

    // === Secondary: Lookup by TypeHash (via reverse index) ===

    /// Get a type by hash (uses reverse index).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.type_hash_to_name
            .get(&hash)
            .and_then(|name| self.types.get(name))
    }

    /// Get a mutable type by hash (uses reverse index).
    pub fn get_type_by_hash_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        self.type_hash_to_name
            .get(&hash)
            .cloned()
            .and_then(move |name| self.types.get_mut(&name))
    }

    /// Check if a type exists by hash.
    pub fn contains_type_hash(&self, hash: TypeHash) -> bool {
        self.type_hash_to_name.contains_key(&hash)
    }

    // === Function lookup (unchanged - still by hash) ===

    /// Get a function by its hash.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.functions.get(&hash)
    }

    /// Get all overloads for a function by qualified name.
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

    // === Global lookup ===

    /// Get a global by qualified name.
    pub fn get_global(&self, name: &QualifiedName) -> Option<&GlobalPropertyEntry> {
        self.globals_by_name
            .get(name)
            .and_then(|hash| self.globals.get(hash))
    }

    /// Get a global by hash.
    pub fn get_global_by_hash(&self, hash: TypeHash) -> Option<&GlobalPropertyEntry> {
        self.globals.get(&hash)
    }
}
```

---

## Name Resolution

```rust
impl SymbolRegistry {
    /// Resolve a type name in context.
    ///
    /// Tries the following in order:
    /// 1. Already qualified name (contains ::)
    /// 2. Current namespace (innermost to outermost)
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
            if self.types.contains_key(&qn) {
                return Some(qn);
            }
            return None;
        }

        // 2. Try current namespace (innermost to outermost)
        for i in (0..=current_namespace.len()).rev() {
            let ns = current_namespace[..i].to_vec();
            let qn = QualifiedName::new(name, ns);
            if self.types.contains_key(&qn) {
                return Some(qn);
            }
        }

        // 3. Try each import as prefix
        for import in imports {
            let ns: Vec<String> = import.split("::").map(|s| s.to_string()).collect();
            let qn = QualifiedName::new(name, ns);
            if self.types.contains_key(&qn) {
                return Some(qn);
            }
        }

        // 4. Global namespace already tried in step 2 when i=0
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
}
```

---

## Iteration Methods

```rust
impl SymbolRegistry {
    /// Iterate over all types.
    pub fn types(&self) -> impl Iterator<Item = &TypeEntry> {
        self.types.values()
    }

    /// Iterate over all types with their qualified names.
    pub fn types_with_names(&self) -> impl Iterator<Item = (&QualifiedName, &TypeEntry)> {
        self.types.iter()
    }

    /// Iterate over all class entries.
    pub fn classes(&self) -> impl Iterator<Item = &ClassEntry> {
        self.types.values().filter_map(|t| t.as_class())
    }

    /// Iterate over all class entries with names.
    pub fn classes_with_names(&self) -> impl Iterator<Item = (&QualifiedName, &ClassEntry)> {
        self.types.iter().filter_map(|(name, entry)| {
            entry.as_class().map(|class| (name, class))
        })
    }

    // ... similar for interfaces, enums, funcdefs
}
```

---

## Entry Type Updates

Add `qname` field to entry types:

```rust
// crates/angelscript-core/src/entries/class.rs

use crate::QualifiedName;

pub struct ClassEntry {
    // NEW: Structured qualified name
    pub qname: QualifiedName,

    // Existing (kept for compatibility during migration)
    pub name: String,
    pub namespace: Vec<String>,
    pub qualified_name: String,
    pub type_hash: TypeHash,

    // ... rest unchanged
}

impl ClassEntry {
    /// Create with QualifiedName (new preferred constructor).
    pub fn with_qname(qname: QualifiedName, type_kind: TypeKind, source: TypeSource) -> Self {
        let type_hash = qname.to_type_hash();
        Self {
            name: qname.simple_name().to_string(),
            namespace: qname.namespace_path().to_vec(),
            qualified_name: qname.to_string(),
            qname,
            type_hash,
            type_kind,
            source,
            // ... defaults for rest
        }
    }

    /// Get the structured qualified name.
    pub fn qname(&self) -> &QualifiedName {
        &self.qname
    }
}
```

---

## Backwards Compatibility

During migration, provide compatibility shims:

```rust
impl SymbolRegistry {
    /// Legacy: Get type by hash.
    ///
    /// DEPRECATED: Use `get_type()` with QualifiedName instead.
    #[deprecated(note = "Use get_type() with QualifiedName")]
    pub fn get(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.get_type_by_hash(hash)
    }

    /// Legacy: Get mutable type by hash.
    ///
    /// DEPRECATED: Use `get_type_mut()` with QualifiedName instead.
    #[deprecated(note = "Use get_type_mut() with QualifiedName")]
    pub fn get_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        self.get_type_by_hash_mut(hash)
    }

    /// Legacy: Check if type exists by hash.
    #[deprecated(note = "Use contains_type() with QualifiedName")]
    pub fn contains_type_by_hash(&self, hash: TypeHash) -> bool {
        self.type_hash_to_name.contains_key(&hash)
    }
}
```

---

## Migration Notes

### What Changes
1. `types: FxHashMap<TypeHash, TypeEntry>` → `types: FxHashMap<QualifiedName, TypeEntry>`
2. `type_by_name: FxHashMap<String, TypeHash>` → removed (QualifiedName is now the key)
3. New `type_hash_to_name: FxHashMap<TypeHash, QualifiedName>` reverse index
4. `types_by_namespace` now maps to `QualifiedName` instead of `TypeHash`

### What Stays the Same
1. Functions still stored by `TypeHash` (hash encodes signature for overload uniqueness)
2. Globals still stored by `TypeHash`
3. Namespace index structure
4. Type aliases

### Why Functions Stay Hash-Indexed
Functions need hash-based storage because:
- The hash encodes the full signature (name + parameter types)
- Overloads have the same name but different hashes
- The VM needs O(1) lookup by hash for function dispatch

Types don't need this because:
- Type names are unique (no "type overloading")
- QualifiedName provides the uniqueness we need
- Hash can be accessed from the entry when needed

### VM Runtime: Bytecode Module Owns Hash Lookups

The VM needs O(1) hash-based lookup for function dispatch at runtime. This belongs in the **bytecode module**, not the registry:

```rust
// crates/angelscript-compiler/src/emit/mod.rs (future change)

/// Compiled bytecode module ready for VM execution.
pub struct CompiledModule {
    /// Constant pool (strings, numbers, type hashes).
    pub constants: ConstantPool,

    /// Compiled functions indexed by hash for O(1) VM dispatch.
    /// Changed from Vec<CompiledFunctionEntry> to FxHashMap.
    pub functions: FxHashMap<TypeHash, CompiledFunction>,

    /// Global variable initializers.
    pub global_inits: Vec<GlobalInitEntry>,
}

/// A compiled function ready for execution.
pub struct CompiledFunction {
    /// Function name (for debugging/stack traces).
    pub name: String,
    /// Compiled bytecode.
    pub bytecode: BytecodeChunk,
}
```

**Separation of concerns:**
- **SymbolRegistry** (compile-time): Stores type metadata by `QualifiedName` for name resolution
- **CompiledModule** (runtime): Stores compiled bytecode by `TypeHash` for VM dispatch

The registry doesn't need hash-based type storage because:
1. During compilation, we resolve names → get entry → access `entry.type_hash` when needed
2. At runtime, the VM uses `CompiledModule.functions` (by hash) for dispatch
3. Type metadata at runtime comes from the compiled bytecode's constant pool

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get_type() {
        let mut registry = SymbolRegistry::new();
        let qname = QualifiedName::new("Player", vec!["Game".into()]);
        let entry = ClassEntry::with_qname(
            qname.clone(),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );

        registry.register_type(entry.into()).unwrap();

        // Primary lookup by name
        assert!(registry.contains_type(&qname));
        assert!(registry.get_type(&qname).is_some());

        // Secondary lookup by hash
        let hash = qname.to_type_hash();
        assert!(registry.get_type_by_hash(hash).is_some());
    }

    #[test]
    fn resolve_type_in_namespace() {
        let mut registry = SymbolRegistry::new();
        let qname = QualifiedName::new("Entity", vec!["Game".into()]);
        let entry = ClassEntry::with_qname(
            qname.clone(),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(entry.into()).unwrap();

        // Resolve from same namespace
        let resolved = registry.resolve_type_name(
            "Entity",
            &["Game".into()],
            &[],
        );
        assert_eq!(resolved, Some(qname));
    }

    #[test]
    fn duplicate_type_error() {
        let mut registry = SymbolRegistry::new();
        let qname = QualifiedName::global("Player");

        let entry1 = ClassEntry::with_qname(
            qname.clone(),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        let entry2 = ClassEntry::with_qname(
            qname.clone(),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );

        registry.register_type(entry1.into()).unwrap();
        let result = registry.register_type(entry2.into());

        assert!(result.is_err());
    }
}
```

---

## Dependencies

- Phase 1: `QualifiedName` struct (complete)
- Phase 2: Entry types with `qname` field (this phase adds it)
- Phase 3: `RegistrationResult` (complete)

---

## What's Next

Phase 5 will update the Registration pass to use the new QualifiedName-based registry methods.
