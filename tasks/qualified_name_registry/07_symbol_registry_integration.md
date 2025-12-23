# Phase 7: SymbolRegistry Integration

## Overview

Integrate `NamespaceTree` into `SymbolRegistry`, replacing the flat `HashMap` storage while maintaining backward compatibility via hash indexes.

**Files:**
- `crates/angelscript-registry/src/symbol_registry.rs` (rewrite)
- `crates/angelscript-registry/src/lib.rs` (update exports)

---

## Updated SymbolRegistry

```rust
// crates/angelscript-registry/src/symbol_registry.rs

use crate::namespace_tree::{NamespaceTree, RegistrationError, ResolutionContext};
use angelscript_core::{
    ClassEntry, EnumEntry, FuncdefEntry, FunctionEntry, GlobalPropertyEntry,
    InterfaceEntry, QualifiedName, TypeEntry, TypeHash,
};
use petgraph::graph::NodeIndex;

/// Registry for all symbols in a compilation unit or the global registry.
///
/// Uses a `NamespaceTree` for hierarchical storage with `using namespace` support.
/// Maintains hash indexes for O(1) bytecode dispatch.
pub struct SymbolRegistry {
    /// The namespace tree storing all symbols.
    tree: NamespaceTree,
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            tree: NamespaceTree::new(),
        }
    }

    // === Tree Access ===

    /// Get a reference to the namespace tree.
    pub fn tree(&self) -> &NamespaceTree {
        &self.tree
    }

    /// Get a mutable reference to the namespace tree.
    pub fn tree_mut(&mut self) -> &mut NamespaceTree {
        &mut self.tree
    }

    // === Type Registration ===

    /// Register a type using QualifiedName.
    pub fn register_type_with_name(
        &mut self,
        entry: TypeEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        self.tree.register_type(name.namespace_path(), name.simple_name(), entry)
    }

    /// Register a type (legacy - extracts name from entry).
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let name = entry.qualified_name().clone();
        self.register_type_with_name(entry, name)
    }

    // === Type Lookup by Hash (for bytecode) ===

    /// Get a type by its hash.
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.tree.get_type_by_hash(hash)
    }

    /// Get a mutable type by its hash.
    pub fn get_type_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        self.tree.get_type_by_hash_mut(hash)
    }

    /// Get a class entry by hash.
    pub fn get_class(&self, hash: TypeHash) -> Option<&ClassEntry> {
        self.get_type(hash)?.as_class()
    }

    /// Get a mutable class entry by hash.
    pub fn get_class_mut(&mut self, hash: TypeHash) -> Option<&mut ClassEntry> {
        self.get_type_mut(hash)?.as_class_mut()
    }

    /// Get an interface entry by hash.
    pub fn get_interface(&self, hash: TypeHash) -> Option<&InterfaceEntry> {
        self.get_type(hash)?.as_interface()
    }

    /// Get an enum entry by hash.
    pub fn get_enum(&self, hash: TypeHash) -> Option<&EnumEntry> {
        self.get_type(hash)?.as_enum()
    }

    /// Get a funcdef entry by hash.
    pub fn get_funcdef(&self, hash: TypeHash) -> Option<&FuncdefEntry> {
        self.get_type(hash)?.as_funcdef()
    }

    // === Type Lookup by Name ===

    /// Check if a type exists by QualifiedName.
    pub fn contains_type_name(&self, name: &QualifiedName) -> bool {
        self.tree.get_path(name.namespace_path())
            .and_then(|ns| self.tree.get_namespace(ns))
            .map(|data| data.types.contains_key(name.simple_name()))
            .unwrap_or(false)
    }

    /// Get a type by QualifiedName.
    pub fn get_type_by_name(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        let ns = self.tree.get_path(name.namespace_path())?;
        self.tree.get_namespace(ns)?.types.get(name.simple_name())
    }

    /// Get a type hash by QualifiedName.
    pub fn get_type_hash(&self, name: &QualifiedName) -> Option<TypeHash> {
        self.get_type_by_name(name).map(|e| e.type_hash())
    }

    // === Type Resolution ===

    /// Resolve a type name from a given namespace context.
    ///
    /// Uses the namespace tree resolution algorithm:
    /// 1. Current namespace and ancestors
    /// 2. Using directive namespaces at current and parent scopes (non-transitive)
    ///
    /// Handles `::Name` syntax for explicit global scope.
    /// Note: `::Name` does NOT honor using directives - it's a direct lookup in global scope only.
    pub fn resolve_type(
        &self,
        name: &str,
        context_namespace: &[String],
    ) -> Option<&TypeEntry> {
        // Handle explicit global scope (::Name)
        // This is a DIRECT lookup in global scope only - no using directives, no ancestors
        if name.starts_with("::") {
            let global_name = name.trim_start_matches("::");
            // Direct lookup in root namespace only (no resolution algorithm)
            return self.tree.get_namespace(self.tree.root())?
                .types.get(global_name);
        }

        let ctx = ResolutionContext {
            current_namespace: self.tree.get_path(context_namespace)
                .unwrap_or_else(|| self.tree.root()),
        };
        self.tree.resolve_type(name, &ctx)
    }

    /// Resolve and return the qualified name for a type.
    pub fn resolve_type_name(
        &self,
        name: &str,
        context_namespace: &[String],
    ) -> Option<QualifiedName> {
        // Handle explicit global scope (::Name)
        // Direct lookup in global scope only - no using directives
        if name.starts_with("::") {
            let global_name = name.trim_start_matches("::");
            // Direct lookup in root namespace only
            let root_data = self.tree.get_namespace(self.tree.root())?;
            if root_data.types.contains_key(global_name) {
                return Some(QualifiedName::global(global_name));
            }
            return None;
        }

        let ctx = ResolutionContext {
            current_namespace: self.tree.get_path(context_namespace)
                .unwrap_or_else(|| self.tree.root()),
        };
        self.tree.resolve_type_to_name(name, &ctx)
    }

    // === Function Registration and Lookup ===

    /// Register a function using QualifiedName.
    pub fn register_function_with_name(
        &mut self,
        entry: FunctionEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        self.tree.register_function(name.namespace_path(), name.simple_name(), entry)
    }

    /// Get a function by its hash.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.tree.get_function_by_hash(hash)
    }

    /// Resolve a function (returns all overloads).
    pub fn resolve_function(
        &self,
        name: &str,
        context_namespace: &[String],
    ) -> Option<&[FunctionEntry]> {
        // Handle explicit global scope
        // Direct lookup in global scope only - no using directives
        if name.starts_with("::") {
            let global_name = name.trim_start_matches("::");
            // Direct lookup in root namespace only
            return self.tree.get_namespace(self.tree.root())?
                .functions.get(global_name)
                .map(|v| v.as_slice());
        }

        let ctx = ResolutionContext {
            current_namespace: self.tree.get_path(context_namespace)
                .unwrap_or_else(|| self.tree.root()),
        };
        self.tree.resolve_function(name, &ctx)
    }

    // === Global Property Registration and Lookup ===

    /// Register a global property.
    pub fn register_global_with_name(
        &mut self,
        entry: GlobalPropertyEntry,
        name: QualifiedName,
    ) -> Result<(), RegistrationError> {
        self.tree.register_global(name.namespace_path(), name.simple_name(), entry)
    }

    /// Resolve a global property.
    pub fn resolve_global(
        &self,
        name: &str,
        context_namespace: &[String],
    ) -> Option<&GlobalPropertyEntry> {
        // Handle explicit global scope
        // Direct lookup in global scope only - no using directives
        if name.starts_with("::") {
            let global_name = name.trim_start_matches("::");
            // Direct lookup in root namespace only
            return self.tree.get_namespace(self.tree.root())?
                .globals.get(global_name);
        }

        let ctx = ResolutionContext {
            current_namespace: self.tree.get_path(context_namespace)
                .unwrap_or_else(|| self.tree.root()),
        };
        self.tree.resolve_global(name, &ctx)
    }

    // === Iteration ===

    /// Iterate over all types in the registry.
    pub fn types(&self) -> impl Iterator<Item = &TypeEntry> {
        // Traverse all namespace nodes and collect types
        TypeIterator::new(&self.tree)
    }

    /// Iterate over all classes.
    pub fn classes(&self) -> impl Iterator<Item = &ClassEntry> {
        self.types().filter_map(|e| e.as_class())
    }

    /// Iterate over all interfaces.
    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceEntry> {
        self.types().filter_map(|e| e.as_interface())
    }

    /// Iterate over all functions.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionEntry> {
        FunctionIterator::new(&self.tree)
    }
}
```

---

## Iterators

```rust
/// Iterator over all types in the tree.
pub struct TypeIterator<'a> {
    tree: &'a NamespaceTree,
    node_iter: petgraph::graph::NodeIndices<NamespaceData>,
    current_types: Option<std::collections::hash_map::Values<'a, String, TypeEntry>>,
}

impl<'a> TypeIterator<'a> {
    fn new(tree: &'a NamespaceTree) -> Self {
        let mut iter = Self {
            tree,
            node_iter: tree.graph().node_indices(),
            current_types: None,
        };
        iter.advance_node();
        iter
    }

    fn advance_node(&mut self) {
        while let Some(node) = self.node_iter.next() {
            if let Some(data) = self.tree.get_namespace(node) {
                if !data.types.is_empty() {
                    self.current_types = Some(data.types.values());
                    return;
                }
            }
        }
        self.current_types = None;
    }
}

impl<'a> Iterator for TypeIterator<'a> {
    type Item = &'a TypeEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut types) = self.current_types {
                if let Some(entry) = types.next() {
                    return Some(entry);
                }
            }
            self.advance_node();
            if self.current_types.is_none() {
                return None;
            }
        }
    }
}
```

---

## Entry Type Changes

Remove redundant name fields from entry types. The `NamespaceTree` provides names via position:

```rust
// ClassEntry changes - remove these fields:
// - name: String (get from tree position)
// - namespace: Vec<String> (get from tree position)
// - qualified_name: String (compute from tree)
// - qname: QualifiedName (stored separately in tree)

// Keep only:
pub struct ClassEntry {
    pub type_hash: TypeHash,
    pub type_kind: TypeKind,
    pub source: TypeSource,
    // ... other semantic fields
}

// Similarly for InterfaceEntry, EnumEntry, FuncdefEntry
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_lookup_by_hash() {
        let mut registry = SymbolRegistry::new();

        let name = QualifiedName::new("Player", vec!["Game".into()]);
        let hash = name.to_type_hash();
        let entry = /* create entry with hash */;

        registry.register_type_with_name(entry, name.clone()).unwrap();

        assert!(registry.get_type(hash).is_some());
        assert!(registry.get_type_by_name(&name).is_some());
    }

    #[test]
    fn resolve_from_context() {
        let mut registry = SymbolRegistry::new();

        // Register Player in Game namespace
        let name = QualifiedName::new("Player", vec!["Game".into()]);
        let entry = /* ... */;
        registry.register_type_with_name(entry, name).unwrap();

        // Resolve from Game namespace
        let resolved = registry.resolve_type("Player", &["Game".into()]);
        assert!(resolved.is_some());

        // Resolve from child namespace (walks up)
        let resolved = registry.resolve_type("Player", &["Game".into(), "Entities".into()]);
        assert!(resolved.is_some());
    }

    #[test]
    fn explicit_global_scope() {
        let mut registry = SymbolRegistry::new();

        // Register var in global scope
        let global_var = /* ... */;
        registry.register_global_with_name(
            global_var,
            QualifiedName::global("var"),
        ).unwrap();

        // Register var in Parent namespace
        let parent_var = /* ... */;
        registry.register_global_with_name(
            parent_var,
            QualifiedName::new("var", vec!["Parent".into()]),
        ).unwrap();

        // From Parent, "var" resolves to Parent::var
        let resolved = registry.resolve_global("var", &["Parent".into()]);
        // Should be Parent::var

        // From Parent, "::var" resolves to global var
        let resolved = registry.resolve_global("::var", &["Parent".into()]);
        // Should be global var
    }

    #[test]
    fn using_directive_resolution() {
        let mut registry = SymbolRegistry::new();

        // Register Helper in Utils
        let name = QualifiedName::new("Helper", vec!["Utils".into()]);
        let entry = /* ... */;
        registry.register_type_with_name(entry, name).unwrap();

        // Add using directive: Game uses Utils
        let game = registry.tree_mut().get_or_create_path(&["Game".into()]);
        let utils = registry.tree().get_path(&["Utils".into()]).unwrap();
        registry.tree_mut().add_using_directive(game, utils);

        // Resolve Helper from Game
        let resolved = registry.resolve_type("Helper", &["Game".into()]);
        assert!(resolved.is_some());
    }
}
```

---

## Dependencies

- Phase 5: NamespaceTree core
- Phase 6: NamespaceTree storage/resolution

---

## What's Next

Phase 8 updates the Registration pass to build the namespace tree and collect using directives.
