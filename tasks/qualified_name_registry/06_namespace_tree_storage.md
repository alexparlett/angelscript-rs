# Phase 6: NamespaceTree Type/Function Storage and Resolution

## Overview

Add type registration, function registration, and resolution methods to `NamespaceTree`.

**Files:**
- `crates/angelscript-registry/src/namespace_tree.rs` (extend)

---

## Type Registration

```rust
impl NamespaceTree {
    /// Register a type in the tree.
    pub fn register_type(
        &mut self,
        namespace_path: &[String],
        simple_name: &str,
        entry: TypeEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);
        let type_hash = entry.type_hash();

        let ns_data = self.graph.node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;

        // Check for duplicates
        if ns_data.types.contains_key(simple_name) {
            let qualified = self.qualified_name(ns_node, simple_name);
            return Err(RegistrationError::DuplicateType(qualified));
        }

        // Insert type and build reverse index
        ns_data.types.insert(simple_name.to_string(), entry);
        self.type_hash_index.insert(type_hash, (ns_node, simple_name.to_string()));

        Ok(())
    }

    /// Get a type by hash (for bytecode dispatch).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        let (ns_node, name) = self.type_hash_index.get(&hash)?;
        self.graph.node_weight(*ns_node)?.types.get(name)
    }

    /// Get a mutable type by hash.
    pub fn get_type_by_hash_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        let (ns_node, name) = self.type_hash_index.get(&hash)?.clone();
        self.graph.node_weight_mut(ns_node)?.types.get_mut(&name)
    }

    /// Get the qualified name for a type by its hash.
    pub fn get_type_qualified_name(&self, hash: TypeHash) -> Option<String> {
        let (ns_node, name) = self.type_hash_index.get(&hash)?;
        Some(self.qualified_name(*ns_node, name))
    }

    /// Get the location (namespace + name) for a type by its hash.
    pub fn get_type_location(&self, hash: TypeHash) -> Option<(NodeIndex, &str)> {
        let (ns_node, name) = self.type_hash_index.get(&hash)?;
        Some((*ns_node, name.as_str()))
    }
}
```

---

## Type Resolution

```rust
/// Context for type resolution within a specific namespace.
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// The namespace where resolution is happening.
    pub current_namespace: NodeIndex,
}

impl NamespaceTree {
    /// Resolve an unqualified type name from a context.
    ///
    /// Search order:
    /// 1. Current namespace
    /// 2. Parent namespaces (walking up to root)
    /// 3. Namespaces imported via `using namespace` at current and parent scopes (non-transitive)
    ///
    /// Returns an error if multiple using directives bring in the same name (ambiguity).
    pub fn resolve_type_checked(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> ResolutionResult<&TypeEntry> {
        // Handle qualified names (contains ::)
        if name.contains("::") {
            return match self.resolve_qualified_type(name) {
                Some(entry) => ResolutionResult::Found(entry),
                None => ResolutionResult::NotFound,
            };
        }

        // 1. Check current namespace and walk up to root
        // Types found in the namespace hierarchy take precedence (no ambiguity possible)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(entry) = ns_data.types.get(name) {
                    return ResolutionResult::Found(entry);
                }
            }
            current = self.find_parent(ns_node);
        }

        // 2. Check using directive namespaces from current and all parent scopes
        // Collect all matches to detect ambiguity
        let mut matches: Vec<(NodeIndex, &TypeEntry)> = Vec::new();

        // Walk from current namespace up to root, collecting using directive matches
        let mut scope = Some(ctx.current_namespace);
        while let Some(ns_node) = scope {
            for using_ns in self.get_using_directives(ns_node) {
                if let Some(ns_data) = self.graph.node_weight(using_ns) {
                    if let Some(entry) = ns_data.types.get(name) {
                        // Check if we already found this same type (same hash)
                        if !matches.iter().any(|(_, e)| e.type_hash() == entry.type_hash()) {
                            matches.push((using_ns, entry));
                        }
                    }
                }
            }
            scope = self.find_parent(ns_node);
        }

        match matches.len() {
            0 => ResolutionResult::NotFound,
            1 => ResolutionResult::Found(matches.into_iter().next().unwrap().1),
            _ => ResolutionResult::Ambiguous(matches),
        }
    }

    /// Resolve an unqualified type name from a context.
    /// Convenience method that returns Option (returns first match, ignores ambiguity).
    /// Prefer `resolve_type_checked` for proper error handling.
    pub fn resolve_type(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&TypeEntry> {
        match self.resolve_type_checked(name, ctx) {
            ResolutionResult::Found(entry) => Some(entry),
            ResolutionResult::Ambiguous(matches) => Some(matches.into_iter().next().unwrap().1),
            ResolutionResult::NotFound => None,
        }
    }

    /// Resolve a fully qualified type name like "Game::Entities::Player".
    pub fn resolve_qualified_type(&self, qualified_name: &str) -> Option<&TypeEntry> {
        let normalized = qualified_name.trim_start_matches("::");

        let parts: Vec<&str> = normalized.split("::").collect();
        if parts.is_empty() {
            return None;
        }

        let simple_name = *parts.last()?;
        let namespace_parts: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let ns_node = self.get_path(&namespace_parts)?;
        self.graph.node_weight(ns_node)?.types.get(simple_name)
    }

    /// Resolve a type and return it with its location, with ambiguity detection.
    pub fn resolve_type_with_location_checked(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> ResolutionResult<(&TypeEntry, NodeIndex)> {
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
            let parts: Vec<&str> = normalized.split("::").collect();
            if parts.is_empty() {
                return ResolutionResult::NotFound;
            }
            let simple_name = match parts.last() {
                Some(s) => *s,
                None => return ResolutionResult::NotFound,
            };
            let namespace_parts: Vec<String> = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let ns_node = match self.get_path(&namespace_parts) {
                Some(n) => n,
                None => return ResolutionResult::NotFound,
            };
            let entry = match self.graph.node_weight(ns_node).and_then(|d| d.types.get(simple_name)) {
                Some(e) => e,
                None => return ResolutionResult::NotFound,
            };
            return ResolutionResult::Found((entry, ns_node));
        }

        // Check namespace hierarchy (no ambiguity possible here)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(entry) = ns_data.types.get(name) {
                    return ResolutionResult::Found((entry, ns_node));
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives from all parent scopes
        let mut matches: Vec<(NodeIndex, (&TypeEntry, NodeIndex))> = Vec::new();

        let mut scope = Some(ctx.current_namespace);
        while let Some(ns_node) = scope {
            for using_ns in self.get_using_directives(ns_node) {
                if let Some(ns_data) = self.graph.node_weight(using_ns) {
                    if let Some(entry) = ns_data.types.get(name) {
                        if !matches.iter().any(|(_, (e, _))| e.type_hash() == entry.type_hash()) {
                            matches.push((using_ns, (entry, using_ns)));
                        }
                    }
                }
            }
            scope = self.find_parent(ns_node);
        }

        match matches.len() {
            0 => ResolutionResult::NotFound,
            1 => ResolutionResult::Found(matches.into_iter().next().unwrap().1),
            _ => ResolutionResult::Ambiguous(matches.into_iter().map(|(ns, v)| (ns, v)).collect()),
        }
    }

    /// Resolve a type and return it with its location.
    /// Convenience method - prefer `resolve_type_with_location_checked`.
    pub fn resolve_type_with_location(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<(&TypeEntry, NodeIndex)> {
        match self.resolve_type_with_location_checked(name, ctx) {
            ResolutionResult::Found(result) => Some(result),
            ResolutionResult::Ambiguous(matches) => Some(matches.into_iter().next().unwrap().1),
            ResolutionResult::NotFound => None,
        }
    }

    /// Resolve a type name and return its QualifiedName.
    pub fn resolve_type_to_name(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<QualifiedName> {
        let (entry, ns_node) = self.resolve_type_with_location(name, ctx)?;
        let path = self.get_namespace_path(ns_node);
        Some(QualifiedName::new(entry.simple_name(), path))
    }
}
```

---

## Function Storage

```rust
impl NamespaceTree {
    /// Register a function (allows overloads with same name).
    pub fn register_function(
        &mut self,
        namespace_path: &[String],
        simple_name: &str,
        entry: FunctionEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);
        let func_hash = entry.def.func_hash;

        let ns_data = self.graph.node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;

        let overloads = ns_data.functions.entry(simple_name.to_string()).or_default();

        // Check for duplicate signature
        if overloads.iter().any(|f| f.def.func_hash == func_hash) {
            let qualified = self.qualified_name(ns_node, simple_name);
            return Err(RegistrationError::DuplicateFunction(qualified));
        }

        let overload_index = overloads.len();
        overloads.push(entry);

        self.func_hash_index.insert(func_hash, (ns_node, simple_name.to_string(), overload_index));

        Ok(())
    }

    /// Get a function by hash.
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        let (ns_node, name, idx) = self.func_hash_index.get(&hash)?;
        self.graph.node_weight(*ns_node)?.functions.get(name)?.get(*idx)
    }

    /// Resolve a function name (returns all overloads), with ambiguity detection.
    pub fn resolve_function_checked(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> ResolutionResult<&[FunctionEntry]> {
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
            let parts: Vec<&str> = normalized.split("::").collect();
            let simple_name = match parts.last() {
                Some(s) => *s,
                None => return ResolutionResult::NotFound,
            };
            let namespace_parts: Vec<String> = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let ns_node = match self.get_path(&namespace_parts) {
                Some(n) => n,
                None => return ResolutionResult::NotFound,
            };
            return match self.graph.node_weight(ns_node).and_then(|d| d.functions.get(simple_name)) {
                Some(funcs) => ResolutionResult::Found(funcs.as_slice()),
                None => ResolutionResult::NotFound,
            };
        }

        // Check namespace hierarchy (no ambiguity possible)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(funcs) = ns_data.functions.get(name) {
                    return ResolutionResult::Found(funcs.as_slice());
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives from all parent scopes
        let mut matches: Vec<(NodeIndex, &[FunctionEntry])> = Vec::new();

        let mut scope = Some(ctx.current_namespace);
        while let Some(ns_node) = scope {
            for using_ns in self.get_using_directives(ns_node) {
                if let Some(ns_data) = self.graph.node_weight(using_ns) {
                    if let Some(funcs) = ns_data.functions.get(name) {
                        // Functions with same name in different namespaces are ambiguous
                        if !matches.iter().any(|(ns, _)| *ns == using_ns) {
                            matches.push((using_ns, funcs.as_slice()));
                        }
                    }
                }
            }
            scope = self.find_parent(ns_node);
        }

        match matches.len() {
            0 => ResolutionResult::NotFound,
            1 => ResolutionResult::Found(matches.into_iter().next().unwrap().1),
            _ => ResolutionResult::Ambiguous(matches),
        }
    }

    /// Resolve a function name (returns all overloads).
    /// Convenience method - prefer `resolve_function_checked`.
    pub fn resolve_function(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&[FunctionEntry]> {
        match self.resolve_function_checked(name, ctx) {
            ResolutionResult::Found(funcs) => Some(funcs),
            ResolutionResult::Ambiguous(matches) => Some(matches.into_iter().next().unwrap().1),
            ResolutionResult::NotFound => None,
        }
    }
}
```

---

## Global Property Storage

```rust
impl NamespaceTree {
    /// Register a global property.
    pub fn register_global(
        &mut self,
        namespace_path: &[String],
        simple_name: &str,
        entry: GlobalPropertyEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);

        let ns_data = self.graph.node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;

        if ns_data.globals.contains_key(simple_name) {
            let qualified = self.qualified_name(ns_node, simple_name);
            return Err(RegistrationError::DuplicateGlobal(qualified));
        }

        ns_data.globals.insert(simple_name.to_string(), entry);
        Ok(())
    }

    /// Resolve a global property, with ambiguity detection.
    pub fn resolve_global_checked(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> ResolutionResult<&GlobalPropertyEntry> {
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
            let parts: Vec<&str> = normalized.split("::").collect();
            let simple_name = match parts.last() {
                Some(s) => *s,
                None => return ResolutionResult::NotFound,
            };
            let namespace_parts: Vec<String> = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let ns_node = match self.get_path(&namespace_parts) {
                Some(n) => n,
                None => return ResolutionResult::NotFound,
            };
            return match self.graph.node_weight(ns_node).and_then(|d| d.globals.get(simple_name)) {
                Some(g) => ResolutionResult::Found(g),
                None => ResolutionResult::NotFound,
            };
        }

        // Check namespace hierarchy (no ambiguity possible)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(global) = ns_data.globals.get(name) {
                    return ResolutionResult::Found(global);
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives from all parent scopes
        let mut matches: Vec<(NodeIndex, &GlobalPropertyEntry)> = Vec::new();

        let mut scope = Some(ctx.current_namespace);
        while let Some(ns_node) = scope {
            for using_ns in self.get_using_directives(ns_node) {
                if let Some(ns_data) = self.graph.node_weight(using_ns) {
                    if let Some(global) = ns_data.globals.get(name) {
                        if !matches.iter().any(|(ns, _)| *ns == using_ns) {
                            matches.push((using_ns, global));
                        }
                    }
                }
            }
            scope = self.find_parent(ns_node);
        }

        match matches.len() {
            0 => ResolutionResult::NotFound,
            1 => ResolutionResult::Found(matches.into_iter().next().unwrap().1),
            _ => ResolutionResult::Ambiguous(matches),
        }
    }

    /// Resolve a global property.
    /// Convenience method - prefer `resolve_global_checked`.
    pub fn resolve_global(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&GlobalPropertyEntry> {
        match self.resolve_global_checked(name, ctx) {
            ResolutionResult::Found(g) => Some(g),
            ResolutionResult::Ambiguous(matches) => Some(matches.into_iter().next().unwrap().1),
            ResolutionResult::NotFound => None,
        }
    }
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("invalid namespace")]
    InvalidNamespace,

    #[error("duplicate type: {0}")]
    DuplicateType(String),

    #[error("duplicate function: {0}")]
    DuplicateFunction(String),

    #[error("duplicate global: {0}")]
    DuplicateGlobal(String),

    #[error("unknown namespace: {0}")]
    UnknownNamespace(String),
}

/// Result of name resolution that may be ambiguous.
#[derive(Debug)]
pub enum ResolutionResult<T> {
    /// Found exactly one match.
    Found(T),
    /// Found multiple matches from different using directives (ambiguous).
    Ambiguous(Vec<(NodeIndex, T)>),
    /// Not found.
    NotFound,
}
```

---

## Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve_type() {
        let mut tree = NamespaceTree::new();

        // Create a mock TypeEntry
        let entry = /* ... */;

        tree.register_type(&["Game".into()], "Player", entry).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game".into()]).unwrap(),
        };

        let resolved = tree.resolve_type("Player", &ctx);
        assert!(resolved.is_some());
    }

    #[test]
    fn resolve_via_using_directive() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils".into()]);
        let game = tree.get_or_create_path(&["Game".into()]);

        // Register Helper in Utils
        let entry = /* ... */;
        tree.register_type(&["Utils".into()], "Helper", entry).unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game context
        let ctx = ResolutionContext { current_namespace: game };
        let resolved = tree.resolve_type("Helper", &ctx);
        assert!(resolved.is_some());
    }

    #[test]
    fn using_not_transitive() {
        let mut tree = NamespaceTree::new();

        let a = tree.get_or_create_path(&["A".into()]);
        let b = tree.get_or_create_path(&["B".into()]);
        let c = tree.get_or_create_path(&["C".into()]);

        // Register CType in C
        let entry = /* ... */;
        tree.register_type(&["C".into()], "CType", entry).unwrap();

        // A uses B, B uses C
        tree.add_using_directive(a, b);
        tree.add_using_directive(b, c);

        // CType should NOT be visible from A (non-transitive)
        let ctx = ResolutionContext { current_namespace: a };
        let resolved = tree.resolve_type("CType", &ctx);
        assert!(resolved.is_none());
    }

    #[test]
    fn parent_scope_using_inherited() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils".into()]);
        let game = tree.get_or_create_path(&["Game".into()]);
        let entities = tree.get_or_create_path(&["Game".into(), "Entities".into()]);

        // Register Helper in Utils
        let entry = /* ... */;
        tree.register_type(&["Utils".into()], "Helper", entry).unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game::Entities context (should find via parent's using)
        let ctx = ResolutionContext { current_namespace: entities };
        let resolved = tree.resolve_type("Helper", &ctx);
        assert!(resolved.is_some());
    }

    #[test]
    fn ambiguous_using_detection() {
        let mut tree = NamespaceTree::new();

        let ns_b = tree.get_or_create_path(&["B".into()]);
        let ns_c = tree.get_or_create_path(&["C".into()]);
        let ns_a = tree.get_or_create_path(&["A".into()]);

        // Register Helper in both B and C
        let entry_b = /* TypeEntry with different hash */;
        let entry_c = /* TypeEntry with different hash */;
        tree.register_type(&["B".into()], "Helper", entry_b).unwrap();
        tree.register_type(&["C".into()], "Helper", entry_c).unwrap();

        // A uses both B and C
        tree.add_using_directive(ns_a, ns_b);
        tree.add_using_directive(ns_a, ns_c);

        // Resolve should detect ambiguity
        let ctx = ResolutionContext { current_namespace: ns_a };
        match tree.resolve_type_checked("Helper", &ctx) {
            ResolutionResult::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2);
            }
            _ => panic!("Expected ambiguity"),
        }
    }

    #[test]
    fn resolve_type_to_name() {
        let mut tree = NamespaceTree::new();

        // Register Player in Game
        let entry = /* ... */;
        tree.register_type(&["Game".into()], "Player", entry).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game".into()]).unwrap(),
        };

        let qname = tree.resolve_type_to_name("Player", &ctx);
        assert!(qname.is_some());
        assert_eq!(qname.unwrap().to_string(), "Game::Player");
    }
}
```

---

## Dependencies

- Phase 5: NamespaceTree core structure

---

## What's Next

Phase 7 will integrate NamespaceTree into SymbolRegistry.
