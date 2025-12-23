# Phase 5: NamespaceTree Implementation

## Overview

Implement the `NamespaceTree` data structure using `petgraph::DiGraph`. This is the core storage structure for the namespace hierarchy.

**Files:**
- `crates/angelscript-registry/src/namespace_tree.rs` (new)
- `crates/angelscript-registry/Cargo.toml` (add petgraph dependency)

---

## Dependencies

```toml
[dependencies]
petgraph = "0.6"
```

---

## Core Types

```rust
// crates/angelscript-registry/src/namespace_tree.rs

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use rustc_hash::FxHashMap;
use angelscript_core::{TypeHash, TypeEntry, FunctionEntry, GlobalPropertyEntry};

/// Edge types in the namespace graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceEdge {
    /// Parent namespace contains child namespace.
    /// The String is the child's simple name.
    Contains(String),
    /// `using namespace` directive.
    /// Source namespace imports target namespace for resolution.
    Uses,
}

/// Data stored in each namespace node.
#[derive(Debug, Default)]
pub struct NamespaceData {
    /// Types in this namespace by simple name.
    pub types: FxHashMap<String, TypeEntry>,

    /// Functions in this namespace by simple name.
    /// Vec holds overloads with same name, different signatures.
    pub functions: FxHashMap<String, Vec<FunctionEntry>>,

    /// Global properties in this namespace by simple name.
    pub globals: FxHashMap<String, GlobalPropertyEntry>,

    /// Type aliases (typedef) in this namespace.
    /// Maps alias name -> target TypeHash.
    pub type_aliases: FxHashMap<String, TypeHash>,
}

impl NamespaceData {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The namespace graph - hierarchical storage for all symbols.
///
/// Uses petgraph's DiGraph with:
/// - Nodes: NamespaceData (types, functions, globals at that level)
/// - Edges: Contains(name) for hierarchy, Uses for `using namespace`
pub struct NamespaceTree {
    /// The directed graph storing all namespaces.
    graph: DiGraph<NamespaceData, NamespaceEdge>,

    /// The root (global) namespace node.
    root: NodeIndex,

    /// Reverse index: TypeHash -> (NodeIndex, simple_name).
    /// Built during registration for O(1) hash lookups.
    type_hash_index: FxHashMap<TypeHash, (NodeIndex, String)>,

    /// Reverse index: func_hash -> (NodeIndex, simple_name, overload_index).
    func_hash_index: FxHashMap<TypeHash, (NodeIndex, String, usize)>,
}

impl Default for NamespaceTree {
    fn default() -> Self {
        Self::new()
    }
}

impl NamespaceTree {
    /// Create a new namespace tree with an empty root.
    pub fn new() -> Self {
        let mut graph = DiGraph::new();
        let root = graph.add_node(NamespaceData::new());
        Self {
            graph,
            root,
            type_hash_index: FxHashMap::default(),
            func_hash_index: FxHashMap::default(),
        }
    }

    /// Get the root namespace node index.
    pub fn root(&self) -> NodeIndex {
        self.root
    }

    /// Get a namespace node's data.
    pub fn get_namespace(&self, node: NodeIndex) -> Option<&NamespaceData> {
        self.graph.node_weight(node)
    }

    /// Get a mutable reference to a namespace node's data.
    pub fn get_namespace_mut(&mut self, node: NodeIndex) -> Option<&mut NamespaceData> {
        self.graph.node_weight_mut(node)
    }
}
```

---

## Path Navigation

```rust
impl NamespaceTree {
    /// Find a child namespace by name.
    pub fn find_child(&self, parent: NodeIndex, name: &str) -> Option<NodeIndex> {
        for edge in self.graph.edges(parent) {
            if let NamespaceEdge::Contains(child_name) = edge.weight() {
                if child_name == name {
                    return Some(edge.target());
                }
            }
        }
        None
    }

    /// Get or create a child namespace.
    pub fn get_or_create_child(&mut self, parent: NodeIndex, name: &str) -> NodeIndex {
        if let Some(child) = self.find_child(parent, name) {
            return child;
        }

        let child = self.graph.add_node(NamespaceData::new());
        self.graph.add_edge(parent, child, NamespaceEdge::Contains(name.to_string()));
        child
    }

    /// Get or create a namespace path from root.
    pub fn get_or_create_path(&mut self, path: &[String]) -> NodeIndex {
        let mut current = self.root;
        for segment in path {
            current = self.get_or_create_child(current, segment);
        }
        current
    }

    /// Get an existing namespace by path, or None if it doesn't exist.
    pub fn get_path(&self, path: &[String]) -> Option<NodeIndex> {
        let mut current = self.root;
        for segment in path {
            current = self.find_child(current, segment)?;
        }
        Some(current)
    }

    /// Find the parent namespace of a node.
    pub fn find_parent(&self, node: NodeIndex) -> Option<NodeIndex> {
        for edge in self.graph.edges_directed(node, Direction::Incoming) {
            if matches!(edge.weight(), NamespaceEdge::Contains(_)) {
                return Some(edge.source());
            }
        }
        None
    }

    /// Get the simple name of a namespace node.
    pub fn get_namespace_name(&self, node: NodeIndex) -> Option<&str> {
        if node == self.root {
            return None;
        }
        for edge in self.graph.edges_directed(node, Direction::Incoming) {
            if let NamespaceEdge::Contains(name) = edge.weight() {
                return Some(name.as_str());
            }
        }
        None
    }

    /// Get the full namespace path for a node.
    pub fn get_namespace_path(&self, node: NodeIndex) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = node;

        while current != self.root {
            if let Some(name) = self.get_namespace_name(current) {
                path.push(name.to_string());
            }
            match self.find_parent(current) {
                Some(parent) => current = parent,
                None => break,
            }
        }

        path.reverse();
        path
    }

    /// Get the qualified name string for a symbol in a namespace.
    pub fn qualified_name(&self, ns_node: NodeIndex, simple_name: &str) -> String {
        let path = self.get_namespace_path(ns_node);
        if path.is_empty() {
            simple_name.to_string()
        } else {
            format!("{}::{}", path.join("::"), simple_name)
        }
    }
}
```

---

## Using Directive Support

```rust
impl NamespaceTree {
    /// Add a `using namespace` directive.
    pub fn add_using_directive(&mut self, from_ns: NodeIndex, target_ns: NodeIndex) {
        // Avoid duplicate using edges
        for edge in self.graph.edges(from_ns) {
            if matches!(edge.weight(), NamespaceEdge::Uses) && edge.target() == target_ns {
                return;
            }
        }
        self.graph.add_edge(from_ns, target_ns, NamespaceEdge::Uses);
    }

    /// Get all namespaces imported via `using namespace` from a given namespace.
    pub fn get_using_directives(&self, ns: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .edges(ns)
            .filter_map(|edge| {
                if matches!(edge.weight(), NamespaceEdge::Uses) {
                    Some(edge.target())
                } else {
                    None
                }
            })
            .collect()
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
    fn create_namespace_path() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&["Game".into(), "Entities".into()]);

        let path = tree.get_namespace_path(node);
        assert_eq!(path, vec!["Game", "Entities"]);
    }

    #[test]
    fn find_existing_path() {
        let mut tree = NamespaceTree::new();
        tree.get_or_create_path(&["Game".into(), "Entities".into()]);

        let found = tree.get_path(&["Game".into(), "Entities".into()]);
        assert!(found.is_some());

        let not_found = tree.get_path(&["Other".into()]);
        assert!(not_found.is_none());
    }

    #[test]
    fn using_directives() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game".into()]);
        let utils = tree.get_or_create_path(&["Utils".into()]);

        tree.add_using_directive(game, utils);

        let usings = tree.get_using_directives(game);
        assert_eq!(usings.len(), 1);
        assert_eq!(usings[0], utils);
    }

    #[test]
    fn qualified_name() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&["Game".into(), "Entities".into()]);

        let qname = tree.qualified_name(node, "Player");
        assert_eq!(qname, "Game::Entities::Player");
    }
}
```

---

## Dependencies

- Phase 4: Registry updates (provides context for integration)

---

## What's Next

Phase 6 will add type/function storage and resolution to the NamespaceTree.
