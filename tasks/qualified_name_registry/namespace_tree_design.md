# Phase 4b: Namespace Tree Design Document

> Complete design for namespace tree structure using petgraph

## Executive Summary

This document describes a graph-based namespace structure using `petgraph` to replace the current flat `HashMap<QualifiedName, TypeEntry>` storage. The key innovations are:

1. **Graph-based storage** using `petgraph::DiGraph` where namespaces are nodes
2. **Edge types** for hierarchy (`Contains`) and imports (`Uses`)
3. **Elimination of redundant name fields** from entry types (name derived from graph position)
4. **Efficient resolution** via graph traversal

---

## Part 1: Core Data Structures

### 1.1 Dependencies

```toml
[dependencies]
petgraph = "0.6"
```

### 1.2 Graph Structure

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

### 1.3 Path Navigation

```rust
impl NamespaceTree {
    /// Find a child namespace by name.
    ///
    /// Iterates outgoing `Contains` edges to find the child.
    /// O(number of children) but namespaces typically have few direct children.
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

        // Create new child node
        let child = self.graph.add_node(NamespaceData::new());
        self.graph.add_edge(parent, child, NamespaceEdge::Contains(name.to_string()));
        child
    }

    /// Get or create a namespace path from root.
    ///
    /// Given `["Game", "Entities"]`, returns the NodeIndex for
    /// the `Game::Entities` namespace, creating intermediate nodes as needed.
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
        // Look for incoming Contains edge
        for edge in self.graph.edges_directed(node, Direction::Incoming) {
            if matches!(edge.weight(), NamespaceEdge::Contains(_)) {
                return Some(edge.source());
            }
        }
        None
    }

    /// Get the simple name of a namespace node.
    ///
    /// Returns None for the root namespace.
    pub fn get_namespace_name(&self, node: NodeIndex) -> Option<&str> {
        if node == self.root {
            return None;
        }
        // Find incoming Contains edge to get our name
        for edge in self.graph.edges_directed(node, Direction::Incoming) {
            if let NamespaceEdge::Contains(name) = edge.weight() {
                return Some(name.as_str());
            }
        }
        None
    }

    /// Get the full namespace path for a node.
    ///
    /// Returns `["Game", "Entities"]` for a node representing `Game::Entities`.
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

### 1.4 Using Directive Support

```rust
impl NamespaceTree {
    /// Add a `using namespace` directive.
    ///
    /// When resolving names from `from_ns`, also search `target_ns`.
    pub fn add_using_directive(
        &mut self,
        from_ns: NodeIndex,
        target_ns: NodeIndex,
    ) {
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

## Part 2: Type Storage and Lookup

### 2.1 Type Registration

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

### 2.2 Type Resolution

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
    /// 3. Namespaces imported via `using namespace` (non-transitive)
    ///
    /// Note: `using namespace` is NOT transitive. If namespace A uses B,
    /// and B uses C, resolving from A will NOT search C.
    pub fn resolve_type(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&TypeEntry> {
        // Handle qualified names (contains ::)
        if name.contains("::") {
            return self.resolve_qualified_type(name);
        }

        // 1. Check current namespace and walk up to root
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(entry) = ns_data.types.get(name) {
                    return Some(entry);
                }
            }
            current = self.find_parent(ns_node);
        }

        // 2. Check using directive namespaces (direct only, not transitive)
        // Only check the types directly in the imported namespace,
        // do NOT follow that namespace's own using directives.
        for using_ns in self.get_using_directives(ctx.current_namespace) {
            if let Some(ns_data) = self.graph.node_weight(using_ns) {
                if let Some(entry) = ns_data.types.get(name) {
                    return Some(entry);
                }
            }
        }

        None
    }

    /// Resolve a fully qualified type name like "Game::Entities::Player".
    pub fn resolve_qualified_type(&self, qualified_name: &str) -> Option<&TypeEntry> {
        // Handle leading :: (absolute path)
        let normalized = qualified_name.trim_start_matches("::");

        let parts: Vec<&str> = normalized.split("::").collect();
        if parts.is_empty() {
            return None;
        }

        // Last part is the type name, rest is namespace path
        let simple_name = *parts.last()?;
        let namespace_parts: Vec<String> = parts[..parts.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        let ns_node = self.get_path(&namespace_parts)?;
        self.graph.node_weight(ns_node)?.types.get(simple_name)
    }

    /// Resolve a type and return it with its location.
    pub fn resolve_type_with_location(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<(&TypeEntry, NodeIndex)> {
        // Handle qualified names
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
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
            let entry = self.graph.node_weight(ns_node)?.types.get(simple_name)?;
            return Some((entry, ns_node));
        }

        // Check current namespace and walk up
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(entry) = ns_data.types.get(name) {
                    return Some((entry, ns_node));
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives
        for using_ns in self.get_using_directives(ctx.current_namespace) {
            if let Some(ns_data) = self.graph.node_weight(using_ns) {
                if let Some(entry) = ns_data.types.get(name) {
                    return Some((entry, using_ns));
                }
            }
        }

        None
    }
}
```

---

## Part 3: Function Storage

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

        // Get or create overload list
        let overloads = ns_data.functions.entry(simple_name.to_string()).or_default();

        // Check for duplicate signature
        if overloads.iter().any(|f| f.def.func_hash == func_hash) {
            let qualified = self.qualified_name(ns_node, simple_name);
            return Err(RegistrationError::DuplicateFunction(qualified));
        }

        let overload_index = overloads.len();
        overloads.push(entry);

        // Build reverse index
        self.func_hash_index.insert(func_hash, (ns_node, simple_name.to_string(), overload_index));

        Ok(())
    }

    /// Get a function by hash.
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        let (ns_node, name, idx) = self.func_hash_index.get(&hash)?;
        self.graph.node_weight(*ns_node)?.functions.get(name)?.get(*idx)
    }

    /// Resolve a function name (returns all overloads).
    pub fn resolve_function(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&[FunctionEntry]> {
        // Handle qualified names
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
            let parts: Vec<&str> = normalized.split("::").collect();
            let simple_name = *parts.last()?;
            let namespace_parts: Vec<String> = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let ns_node = self.get_path(&namespace_parts)?;
            return self.graph.node_weight(ns_node)?
                .functions.get(simple_name)
                .map(|v| v.as_slice());
        }

        // Check current namespace up to root
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(funcs) = ns_data.functions.get(name) {
                    return Some(funcs.as_slice());
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives
        for using_ns in self.get_using_directives(ctx.current_namespace) {
            if let Some(ns_data) = self.graph.node_weight(using_ns) {
                if let Some(funcs) = ns_data.functions.get(name) {
                    return Some(funcs.as_slice());
                }
            }
        }

        None
    }
}
```

---

## Part 4: Global Property Storage

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

    /// Resolve a global property.
    pub fn resolve_global(
        &self,
        name: &str,
        ctx: &ResolutionContext,
    ) -> Option<&GlobalPropertyEntry> {
        // Handle qualified names
        if name.contains("::") {
            let normalized = name.trim_start_matches("::");
            let parts: Vec<&str> = normalized.split("::").collect();
            let simple_name = *parts.last()?;
            let namespace_parts: Vec<String> = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let ns_node = self.get_path(&namespace_parts)?;
            return self.graph.node_weight(ns_node)?.globals.get(simple_name);
        }

        // Check current namespace up to root
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(global) = ns_data.globals.get(name) {
                    return Some(global);
                }
            }
            current = self.find_parent(ns_node);
        }

        // Check using directives
        for using_ns in self.get_using_directives(ctx.current_namespace) {
            if let Some(ns_data) = self.graph.node_weight(using_ns) {
                if let Some(global) = ns_data.globals.get(name) {
                    return Some(global);
                }
            }
        }

        None
    }
}
```

---

## Part 5: Entry Type Changes

### 5.1 Fields to Remove

**ClassEntry** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `qualified_name: String`
- `qname: QualifiedName`

**InterfaceEntry** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `qualified_name: String`
- `qname: QualifiedName`

**EnumEntry** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `qualified_name: String`
- `qname: QualifiedName`

**FuncdefEntry** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `qualified_name: String`
- `qname: QualifiedName`

**FunctionDef** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `cached_qname: OnceCell<QualifiedName>`

**GlobalPropertyEntry** - Remove:
- `name: String`
- `namespace: Vec<String>`
- `qualified_name: String`

### 5.2 What Entries Keep

Each entry type retains:
- `type_hash: TypeHash` - for identity and bytecode
- All semantic data (fields, methods, inheritance, etc.)
- `source: TypeSource` - for error messages

Names are now derived from tree position via `NamespaceTree::get_type_qualified_name(hash)`.

### 5.3 Accessing Names

```rust
// Before:
let name = class_entry.qualified_name.clone();

// After:
let name = tree.get_type_qualified_name(class_entry.type_hash)
    .unwrap_or_else(|| "<unknown>".to_string());
```

---

## Part 6: SymbolRegistry Integration

```rust
// crates/angelscript-registry/src/registry.rs

use crate::namespace_tree::{NamespaceTree, ResolutionContext};
use petgraph::graph::NodeIndex;

/// Unified symbol registry using namespace tree.
pub struct SymbolRegistry {
    /// The namespace tree (primary storage).
    tree: NamespaceTree,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self {
            tree: NamespaceTree::new(),
        }
    }

    /// Get the underlying tree for direct access.
    pub fn tree(&self) -> &NamespaceTree {
        &self.tree
    }

    /// Get mutable access to the tree.
    pub fn tree_mut(&mut self) -> &mut NamespaceTree {
        &mut self.tree
    }

    /// Register a type.
    pub fn register_type(
        &mut self,
        namespace: &[String],
        name: &str,
        entry: TypeEntry,
    ) -> Result<(), RegistrationError> {
        self.tree.register_type(namespace, name, entry)
    }

    /// Resolve a type name in context.
    pub fn resolve_type(
        &self,
        name: &str,
        current_namespace: NodeIndex,
    ) -> Option<&TypeEntry> {
        let ctx = ResolutionContext { current_namespace };
        self.tree.resolve_type(name, &ctx)
    }

    /// Get type by hash (for bytecode).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.tree.get_type_by_hash(hash)
    }

    /// Get qualified name for a type.
    pub fn get_type_qualified_name(&self, hash: TypeHash) -> Option<String> {
        self.tree.get_type_qualified_name(hash)
    }

    // ... similar methods for functions and globals ...
}
```

---

## Part 7: Updated Phase 5/6/7 Integration

### 7.1 Phase 5 (Registration) Updates

```rust
impl RegistrationPass {
    fn visit_class(&mut self, class: &ClassDecl<'_>) {
        let namespace = self.current_namespace();
        let simple_name = class.name.name.to_string();

        // Compute hash from qualified name
        let qualified = if namespace.is_empty() {
            simple_name.clone()
        } else {
            format!("{}::{}", namespace.join("::"), simple_name)
        };
        let type_hash = TypeHash::from_name(&qualified);

        // Create entry WITHOUT name fields
        let unresolved = UnresolvedClass {
            type_hash,
            namespace: namespace.clone(),
            simple_name: simple_name.clone(),
            span: class.span,
            unit_id: self.unit_id,
            // ... other fields ...
        };

        self.result.add_class(unresolved);
    }
}
```

### 7.2 Phase 6 (Completion) Updates

```rust
impl CompletionPass {
    fn register_class(&mut self, unresolved: &UnresolvedClass) -> Result<(), CompilationError> {
        // Create entry WITHOUT name fields
        let entry = ClassEntry {
            type_hash: unresolved.type_hash,
            type_kind: TypeKind::ScriptObject,
            source: TypeSource::script(unresolved.unit_id, unresolved.span),
            // ... other semantic fields ...
            // NO: name, namespace, qualified_name, qname
        };

        // Register with namespace path and simple name
        self.registry.tree_mut().register_type(
            &unresolved.namespace,
            &unresolved.simple_name,
            entry.into(),
        )?;

        Ok(())
    }
}
```

### 7.3 Phase 7 (Compilation) Updates

```rust
impl CompilationPass {
    fn resolve_type(&self, name: &str) -> Option<&TypeEntry> {
        let ctx = ResolutionContext {
            current_namespace: self.current_namespace_node,
        };
        self.registry.tree().resolve_type(name, &ctx)
    }

    fn emit_error_for_type(&self, hash: TypeHash, message: &str) {
        let type_name = self.registry.get_type_qualified_name(hash)
            .unwrap_or_else(|| format!("<hash:{:?}>", hash));
        self.emit_error(format!("{}: {}", type_name, message));
    }
}
```

---

## Part 8: Example Walkthrough

**Script:**
```angelscript
namespace Game {
    using Other;

    class Entity {}

    class Player : Entity {
        void attack(Enemy@ e) {}
    }
}

namespace Other {
    class Enemy {}
}
```

**Graph structure:**

```
Nodes:
  [0] Root (global namespace)
      types: {}
      functions: {}
      globals: {}

  [1] Game namespace
      types: { "Entity" -> ClassEntry, "Player" -> ClassEntry }
      functions: {}
      globals: {}

  [2] Other namespace
      types: { "Enemy" -> ClassEntry }
      functions: {}
      globals: {}

Edges:
  [0] --Contains("Game")--> [1]
  [0] --Contains("Other")--> [2]
  [1] --Uses--> [2]
```

**Resolution of `Enemy` from within `Game::Player::attack`:**
1. Check `Game` namespace types - not found
2. Check root (global) namespace types - not found
3. Check using directives from `Game` - finds `Other`
4. Check `Other` namespace types - FOUND!

---

## Part 9: Error Types

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
```

---

## Part 10: Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Register type | O(path depth) | Walk/create path + O(1) insert |
| Resolve unqualified | O(depth + using count) | Walk up + check using namespaces |
| Resolve qualified | O(path depth) | Walk down path + O(1) lookup |
| Lookup by hash | O(1) | Via reverse index |
| Add using directive | O(edges from node) | Check for duplicates |
| Find child namespace | O(children count) | Typically small |

---

## Part 11: Deferred Using Directive Resolution

### Problem

When processing `using namespace Foo;` during registration, the target namespace `Foo` may not exist yet (forward reference). We can't create a `Uses` edge to a namespace that doesn't exist.

### Solution

1. **Registration (Phase 5)** builds the namespace tree structure directly as it walks the AST
2. `using namespace` directives are collected as `UnresolvedUsingDirective` (since target may not exist yet)
3. **Completion (Phase 6)** resolves the using directives to `Uses` edges before type resolution

### 11.1 UnresolvedUsingDirective

Add to `angelscript-core/src/unresolved_entries.rs`:

```rust
/// Unresolved using namespace directive from Pass 1.
///
/// Collected during registration, resolved at the start of completion
/// when all namespaces are guaranteed to exist. Resolution creates
/// `Uses` edges in the namespace tree graph.
#[derive(Debug, Clone)]
pub struct UnresolvedUsingDirective {
    /// The namespace where this directive appears (source namespace).
    pub source_namespace: Vec<String>,

    /// The target namespace path (e.g., ["Game", "Utils"] for `using Game::Utils`).
    pub target_namespace: Vec<String>,

    /// Source span for error reporting.
    pub span: Span,
}

impl UnresolvedUsingDirective {
    pub fn new(
        source_namespace: Vec<String>,
        target_namespace: Vec<String>,
        span: Span,
    ) -> Self {
        Self {
            source_namespace,
            target_namespace,
            span,
        }
    }
}
```

### 11.2 RegistrationResult Update

Add to `RegistrationResult` (Phase 3):

```rust
pub struct RegistrationResult {
    // ... existing fields ...

    /// Using namespace directives to be resolved in completion.
    pub using_directives: Vec<UnresolvedUsingDirective>,
}

impl RegistrationResult {
    /// Add a using namespace directive.
    pub fn add_using_directive(&mut self, directive: UnresolvedUsingDirective) {
        self.using_directives.push(directive);
    }
}
```

### 11.3 Registration Pass Update (Phase 5)

The registration pass takes a mutable reference to the namespace tree and builds it directly:

```rust
impl<'tree> RegistrationPass<'tree> {
    fn visit_namespace(&mut self, ns: &NamespaceDecl<'_>) {
        // Namespace declaration is a single identifier (not a path)
        let ns_name = ns.name.name;

        self.enter_namespace(ns_name);

        // Build the namespace node in the tree directly
        self.tree.get_or_create_path(&self.current_namespace());

        for item in ns.items {
            self.visit_item(item);
        }

        self.exit_namespace(ns_name);
    }

    fn visit_using(&mut self, u: &UsingNamespaceDecl<'_>) {
        // Collect using directive for later resolution
        // (target namespace may not exist yet)
        let target: Vec<String> = u.path.iter()
            .map(|id| id.name.to_string())
            .collect();

        let directive = UnresolvedUsingDirective::new(
            self.current_namespace(),  // Source namespace
            target,                     // Target namespace path
            u.span,
        );

        self.result.add_using_directive(directive);
    }
}
```

### 11.4 Completion Pass Update (Phase 6)

Resolve using directives at the **start** of the completion pass, before type resolution:

```rust
impl<'reg, 'global> CompletionPass<'reg, 'global> {
    pub fn run(mut self, input: RegistrationResult) -> CompletionResult {
        // Phase 0: Resolve using directives FIRST
        // Namespace tree was already built during registration.
        // Must happen before type resolution so lookups can traverse using edges.
        self.resolve_using_directives(&input);

        // Phase 1: Build name index from unresolved entries
        self.build_name_index(&input);

        // ... rest of phases ...
    }

    /// Resolve using directives to graph edges.
    fn resolve_using_directives(&mut self, input: &RegistrationResult) {
        for directive in &input.using_directives {
            if let Err(e) = self.resolve_single_using_directive(directive) {
                self.result.errors.push(e);
            }
        }
    }

    fn resolve_single_using_directive(
        &mut self,
        directive: &UnresolvedUsingDirective,
    ) -> Result<(), CompilationError> {
        let tree = self.registry.tree_mut();

        // Get source namespace (must exist - it was created during registration)
        let source_node = tree.get_path(&directive.source_namespace)
            .ok_or_else(|| CompilationError::InternalError {
                message: format!("Source namespace {} not found", directive.source_namespace.join("::")),
                span: directive.span,
            })?;

        // Get target namespace (must exist, or it's an error)
        let target_node = tree.get_path(&directive.target_namespace)
            .ok_or_else(|| CompilationError::UnknownNamespace {
                name: directive.target_namespace.join("::"),
                span: directive.span,
            })?;

        // Add the Uses edge
        tree.add_using_directive(source_node, target_node);

        Ok(())
    }
}
```

### 11.5 Why This Order Works

1. **Registration (Phase 5)**:
   - Walks AST, builds namespace tree nodes via `get_or_create_path`
   - Collects `UnresolvedClass`, `UnresolvedInterface`, etc.
   - Collects `UnresolvedUsingDirective` for each `using namespace` statement

2. **Completion (Phase 6) - Start**:
   - Resolves using directives to `Uses` edges
   - If target namespace doesn't exist → compile error with span

3. **Completion (Phase 6) - Type Resolution**:
   - Uses `NamespaceTree::resolve_type()` which follows `Uses` edges
   - All edges are in place, resolution works correctly

### 11.6 Non-Transitive Resolution

The `Uses` edge semantics remain non-transitive:

```rust
pub fn resolve_type(&self, name: &str, ctx: &ResolutionContext) -> Option<&TypeEntry> {
    // 1. Check current namespace and walk up to root
    // ...

    // 2. Check using directive namespaces (direct only, NOT transitive)
    for using_ns in self.get_using_directives(ctx.current_namespace) {
        // Only check types directly in using_ns
        // Do NOT follow using_ns's own using directives
        if let Some(ns_data) = self.graph.node_weight(using_ns) {
            if let Some(entry) = ns_data.types.get(name) {
                return Some(entry);
            }
        }
    }

    None
}
```

---

## Part 12: Updated UnresolvedType

With namespace tree resolution via edges, `UnresolvedType` no longer needs per-instance import context:

```rust
/// Unresolved type reference - stored during registration, resolved in completion.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct UnresolvedType {
    /// The type name as written (e.g., "Player", "Game::Entity", "array<int>")
    pub name: String,

    /// Namespace context where this reference appeared.
    /// Used for relative name resolution.
    pub context_namespace: Vec<String>,

    // REMOVED: pub imports: Vec<String>,
    // Using directives are now graph edges, not per-type context

    /// Leading `const` modifier.
    pub is_const: bool,

    /// Handle (`@`) modifier.
    pub is_handle: bool,

    /// Handle-to-const (`const@` or `@const`) modifier.
    pub is_handle_to_const: bool,

    /// Reference modifier for parameters (`&in`, `&out`, `&inout`).
    pub ref_modifier: RefModifier,

    /// Source span for error reporting.
    pub span: Span,
}
```

The resolution algorithm in `NamespaceTree::resolve_type()` handles the `using namespace` lookup via graph edges, so each `UnresolvedType` only needs to know its context namespace (where it was written).

---

## Part 13: Cleanup of Old Import Approach

The old approach stored imports per-type-reference. This needs to be removed:

### 13.1 Remove from RegistrationPass (Phase 5)

```rust
// REMOVE these fields and methods from RegistrationPass:

pub struct RegistrationPass {
    // ...
    // REMOVE: imports: Vec<String>,
}

impl RegistrationPass {
    // REMOVE:
    // fn current_imports(&self) -> Vec<String> {
    //     self.imports.clone()
    // }

    // UPDATE visit_using - no longer pushes to self.imports
    // (see Part 11.3 for new implementation)
}
```

### 13.2 Remove from UnresolvedType (Phase 1)

```rust
// REMOVE the imports field:

impl UnresolvedType {
    // CHANGE from:
    // pub fn with_context(
    //     name: impl Into<String>,
    //     context_namespace: Vec<String>,
    //     imports: Vec<String>,  // REMOVE this parameter
    // ) -> Self

    // TO:
    pub fn with_context(
        name: impl Into<String>,
        context_namespace: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            context_namespace,
            // No imports field
            ..Default::default()
        }
    }
}
```

### 13.3 Update collect_type_expr (Phase 5)

```rust
impl RegistrationPass {
    // CHANGE from:
    // fn collect_type_expr(&self, ty: &TypeExpr<'_>) -> UnresolvedType {
    //     let name = self.type_to_string(&ty.ty);
    //     UnresolvedType::with_context(name, self.current_namespace(), self.current_imports())
    //         ...
    // }

    // TO:
    fn collect_type_expr(&self, ty: &TypeExpr<'_>) -> UnresolvedType {
        let name = self.type_to_string(&ty.ty);
        UnresolvedType::with_context(name, self.current_namespace())
            .with_const(ty.is_const)
            .with_handle(ty.is_handle)
            .with_handle_to_const(ty.is_handle_to_const)
            .with_ref_modifier(convert_ref_modifier(ty.ref_modifier))
    }
}
```

### 13.4 Update Completion Pass Resolution (Phase 6)

```rust
impl CompletionPass {
    // CHANGE from:
    // fn resolve_type_name(&self, unresolved: &UnresolvedType) -> Result<QualifiedName, ...> {
    //     // ...
    //     // Try imports
    //     for import in &unresolved.imports {
    //         let ns: Vec<String> = import.split("::").map(|s| s.to_string()).collect();
    //         let qn = QualifiedName::new(&unresolved.name, ns);
    //         if self.type_exists(&qn) {
    //             return Ok(qn);
    //         }
    //     }
    //     // ...
    // }

    // TO:
    fn resolve_type_name(&self, unresolved: &UnresolvedType) -> Result<QualifiedName, ...> {
        // Use NamespaceTree::resolve_type() which handles using directives via graph edges
        let ctx = ResolutionContext {
            current_namespace: self.registry.tree().get_or_create_path(&unresolved.context_namespace),
        };

        // Tree resolution handles: current ns -> parent ns -> using directives
        self.registry.tree().resolve_type(&unresolved.name, &ctx)
            .ok_or_else(|| CompilationError::UnknownType {
                name: unresolved.name.clone(),
                span: unresolved.span,
            })
    }
}
```

---

## Part 14: Migration Strategy

1. **Add petgraph dependency**
2. **Create NamespaceTree** alongside existing storage
3. **Add `UnresolvedUsingDirective`** to `angelscript-core`
4. **Update `RegistrationResult`** to include `using_directives: Vec<UnresolvedUsingDirective>`
5. **Update Registration pass**:
   - Remove `imports: Vec<String>` field
   - Remove `current_imports()` helper
   - Update `visit_using()` to create `UnresolvedUsingDirective`
   - Update `collect_type_expr()` to not pass imports
6. **Update `UnresolvedType`**:
   - Remove `imports: Vec<String>` field
   - Update `with_context()` to not take imports parameter
7. **Update Completion pass**:
   - Add `resolve_using_directives()` at start of `run()`
   - Update `resolve_type_name()` to use tree resolution
8. **Update resolution** to use tree
9. **Remove redundant fields** from entry types (name, namespace, qualified_name, qname)
10. **Remove old flat storage**

---

## Summary

This design uses `petgraph::DiGraph` to model namespaces as a graph where:
- Nodes hold the actual symbols (types, functions, globals)
- `Contains` edges form the namespace hierarchy
- `Uses` edges represent `using namespace` directives
- Reverse indexes provide O(1) hash lookups for bytecode

Key design decisions:
- **Deferred using resolution**: `using namespace` directives are collected during Registration (Phase 5) as `UnresolvedUsingDirective` and resolved to graph edges at the start of Completion (Phase 6)
- **Non-transitive using**: If A uses B and B uses C, A does NOT see C's types
- **Error on unknown namespace**: If a `using namespace` target doesn't exist, it's a compile error with proper span

Benefits:
- Clean separation of structure (graph) and data (node contents)
- Well-tested graph library handles traversal
- Eliminates redundant name storage in entries
- Natural representation of `using` as graph edges
- Forward references for `using namespace` handled cleanly
