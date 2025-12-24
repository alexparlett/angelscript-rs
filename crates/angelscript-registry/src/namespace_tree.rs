//! Namespace Tree - hierarchical storage for all symbols.
//!
//! Uses `petgraph::StableDiGraph` with:
//! - Nodes: `NamespaceData` (types, functions, globals at that level)
//! - Edges: `Contains(name)` for hierarchy, `Uses` for `using namespace`, `Mirrors` for FFI/shared links
//!
//! Note: We use `StableDiGraph` instead of `DiGraph` because node removal must not
//! invalidate existing `NodeIndex` values stored in hash indexes and external references.

use angelscript_core::{
    FunctionEntry, GlobalPropertyEntry, QualifiedName, RegistrationError, TypeEntry, TypeHash,
};
use petgraph::Direction;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::visit::EdgeRef;
use rustc_hash::FxHashMap;

/// Result of name resolution that may be ambiguous.
///
/// When multiple `using namespace` directives bring the same name into scope,
/// resolution is ambiguous and must be reported as an error.
pub enum ResolutionResult<T> {
    /// Found exactly one match.
    Found(T),
    /// Found multiple matches from different using directives (ambiguous).
    /// Contains the namespace node and the value for each match.
    Ambiguous(Vec<(NodeIndex, T)>),
    /// Not found in any searched location.
    NotFound,
}

// Manual trait implementations to avoid requiring bounds on T for basic usage

impl<T: Clone> Clone for ResolutionResult<T> {
    fn clone(&self) -> Self {
        match self {
            ResolutionResult::Found(v) => ResolutionResult::Found(v.clone()),
            ResolutionResult::Ambiguous(v) => ResolutionResult::Ambiguous(v.clone()),
            ResolutionResult::NotFound => ResolutionResult::NotFound,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ResolutionResult<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionResult::Found(v) => f.debug_tuple("Found").field(v).finish(),
            ResolutionResult::Ambiguous(v) => f.debug_tuple("Ambiguous").field(v).finish(),
            ResolutionResult::NotFound => write!(f, "NotFound"),
        }
    }
}

impl<T: PartialEq> PartialEq for ResolutionResult<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ResolutionResult::Found(a), ResolutionResult::Found(b)) => a == b,
            (ResolutionResult::Ambiguous(a), ResolutionResult::Ambiguous(b)) => a == b,
            (ResolutionResult::NotFound, ResolutionResult::NotFound) => true,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for ResolutionResult<T> {}

impl<T> ResolutionResult<T> {
    /// Check if resolution found exactly one match.
    pub fn is_found(&self) -> bool {
        matches!(self, ResolutionResult::Found(_))
    }

    /// Check if resolution was ambiguous.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ResolutionResult::Ambiguous(_))
    }

    /// Check if the name was not found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, ResolutionResult::NotFound)
    }

    /// Convert to Option, returning Some for Found, None otherwise.
    pub fn ok(self) -> Option<T> {
        match self {
            ResolutionResult::Found(v) => Some(v),
            _ => None,
        }
    }
}

/// Context for type resolution within a specific namespace.
///
/// Provides the namespace context for unqualified name lookups.
#[derive(Debug, Clone)]
pub struct ResolutionContext {
    /// The namespace where resolution is happening.
    pub current_namespace: NodeIndex,
}

/// Edge types in the namespace graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceEdge {
    /// Parent namespace contains child namespace.
    /// The String is the child's simple name.
    Contains(String),
    /// `using namespace` directive.
    /// Source namespace imports target namespace for resolution.
    Uses,
    /// Mirrors edge - auto-link to same-named namespace in $ffi/$shared.
    /// When resolving from a unit namespace (e.g., $unit_0/Math), symbols
    /// from mirrored namespaces ($ffi/Math, $shared/Math) are also visible.
    Mirrors,
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
/// Uses petgraph's StableDiGraph with:
/// - Nodes: NamespaceData (types, functions, globals at that level)
/// - Edges: Contains(name) for hierarchy, Uses for `using namespace`, Mirrors for FFI/shared
///
/// We use `StableDiGraph` instead of `DiGraph` because node removal (e.g., `remove_unit`)
/// must not invalidate existing `NodeIndex` values stored in hash indexes and external
/// `ResolutionContext` references.
pub struct NamespaceTree {
    /// The directed graph storing all namespaces.
    graph: StableDiGraph<NamespaceData, NamespaceEdge>,

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
        let mut graph = StableDiGraph::new();
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
        self.graph
            .add_edge(parent, child, NamespaceEdge::Contains(name.to_string()));
        child
    }

    /// Get or create a namespace path from root.
    pub fn get_or_create_path<S: AsRef<str>>(&mut self, path: &[S]) -> NodeIndex {
        let mut current = self.root;
        for segment in path {
            current = self.get_or_create_child(current, segment.as_ref());
        }
        current
    }

    /// Get an existing namespace by path, or None if it doesn't exist.
    pub fn get_path<S: AsRef<str>>(&self, path: &[S]) -> Option<NodeIndex> {
        let mut current = self.root;
        for segment in path {
            current = self.find_child(current, segment.as_ref())?;
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

    /// Get all namespaces linked via `Mirrors` edges from a given namespace.
    pub fn get_mirrors_targets(&self, ns: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .edges(ns)
            .filter_map(|edge| {
                if matches!(edge.weight(), NamespaceEdge::Mirrors) {
                    Some(edge.target())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add a `Mirrors` edge from source to target namespace.
    ///
    /// Mirrors edges link a unit namespace to its counterpart in $ffi or $shared,
    /// enabling automatic symbol visibility across the boundary.
    pub fn add_mirrors_edge(&mut self, from_ns: NodeIndex, target_ns: NodeIndex) {
        // Avoid duplicate mirrors edges
        for edge in self.graph.edges(from_ns) {
            if matches!(edge.weight(), NamespaceEdge::Mirrors) && edge.target() == target_ns {
                return;
            }
        }
        self.graph
            .add_edge(from_ns, target_ns, NamespaceEdge::Mirrors);
    }

    // ========================================================================
    // Unit Management
    // ========================================================================

    /// Create a new compilation unit, returns its root node.
    ///
    /// Units are top-level nodes under the tree root (e.g., `$unit_0`, `$unit_1`).
    /// Each unit has its own namespace hierarchy isolated from other units.
    pub fn create_unit(&mut self, name: &str) -> NodeIndex {
        self.get_or_create_child(self.root, name)
    }

    /// Get or create the FFI namespace root (`$ffi`).
    ///
    /// FFI-registered types and functions live under this namespace.
    pub fn ffi_root(&mut self) -> NodeIndex {
        self.get_or_create_child(self.root, "$ffi")
    }

    /// Get the FFI namespace root if it exists.
    pub fn get_ffi_root(&self) -> Option<NodeIndex> {
        self.find_child(self.root, "$ffi")
    }

    /// Get or create the shared namespace root (`$shared`).
    ///
    /// Shared entities accessible across all units live under this namespace.
    pub fn shared_root(&mut self) -> NodeIndex {
        self.get_or_create_child(self.root, "$shared")
    }

    /// Get the shared namespace root if it exists.
    pub fn get_shared_root(&self) -> Option<NodeIndex> {
        self.find_child(self.root, "$shared")
    }

    /// Remove a compilation unit and all its contents.
    ///
    /// This removes the unit's entire subtree from the graph, including all
    /// namespaces, types, functions, and globals defined within the unit.
    /// Also cleans up the hash indexes for removed entries.
    pub fn remove_unit(&mut self, unit_root: NodeIndex) {
        // Collect all nodes in the subtree (BFS)
        let nodes_to_remove = self.collect_subtree_nodes(unit_root);

        // Clean up hash indexes for all types and functions in the subtree
        for &node in &nodes_to_remove {
            if let Some(ns_data) = self.graph.node_weight(node) {
                // Remove type hash entries
                for (name, entry) in &ns_data.types {
                    self.type_hash_index.remove(&entry.type_hash());
                    let _ = name; // silence unused warning
                }
                // Remove function hash entries
                for (name, overloads) in &ns_data.functions {
                    for func in overloads {
                        self.func_hash_index.remove(&func.def.func_hash);
                    }
                    let _ = name; // silence unused warning
                }
            }
        }

        // Remove nodes from graph (in reverse order to handle children first)
        for node in nodes_to_remove.into_iter().rev() {
            self.graph.remove_node(node);
        }
    }

    /// Collect all nodes in a subtree rooted at `start` (including start).
    fn collect_subtree_nodes(&self, start: NodeIndex) -> Vec<NodeIndex> {
        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            result.push(node);
            // Add all children (nodes connected via Contains edges)
            for edge in self.graph.edges(node) {
                if matches!(edge.weight(), NamespaceEdge::Contains(_)) {
                    queue.push_back(edge.target());
                }
            }
        }

        result
    }

    /// Check if a node is under a specific unit root.
    pub fn is_under_unit(&self, node: NodeIndex, unit_root: NodeIndex) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if n == unit_root {
                return true;
            }
            current = self.find_parent(n);
        }
        false
    }

    /// Get the unit root for a given node (if it's under a unit).
    ///
    /// Returns the immediate child of tree root that contains this node,
    /// or None if the node is the tree root itself.
    pub fn get_unit_root(&self, node: NodeIndex) -> Option<NodeIndex> {
        if node == self.root {
            return None;
        }

        let mut current = node;
        loop {
            if let Some(parent) = self.find_parent(current) {
                if parent == self.root {
                    return Some(current);
                }
                current = parent;
            } else {
                return None;
            }
        }
    }

    /// Get or create a child namespace within a unit, with automatic Mirrors edge creation.
    ///
    /// When creating a namespace in a unit (e.g., `$unit_0/Math`), this method
    /// automatically adds Mirrors edges to the same-named namespace in `$ffi` and
    /// `$shared` if they exist.
    ///
    /// # Arguments
    /// * `parent` - The parent namespace (must be within a unit subtree)
    /// * `name` - The name of the child namespace to get or create
    ///
    /// # Returns
    /// The NodeIndex of the child namespace
    ///
    /// # Panics (debug only)
    /// Panics if parent is not under a unit subtree (i.e., not under $unit_*, $ffi, or $shared).
    pub fn get_or_create_child_in_unit(&mut self, parent: NodeIndex, name: &str) -> NodeIndex {
        debug_assert!(
            self.get_unit_root(parent).is_some() || parent == self.root,
            "get_or_create_child_in_unit called with parent not under a unit subtree"
        );

        let child = self.get_or_create_child(parent, name);

        // Build the relative path from unit root to this child
        let path = self.get_relative_path_from_unit(child);

        // Add Mirrors edges to $ffi and $shared if same-named namespace exists
        if !path.is_empty() {
            self.add_mirrors_if_exists(child, &path);
        }

        child
    }

    /// Get or create a namespace path within a unit, with automatic Mirrors edge creation.
    ///
    /// Similar to `get_or_create_path`, but automatically creates Mirrors edges
    /// for each namespace level to its counterpart in `$ffi` and `$shared`.
    ///
    /// # Arguments
    /// * `unit_root` - The root node of the unit
    /// * `path` - The namespace path within the unit (e.g., ["Game", "Entities"])
    ///
    /// # Returns
    /// The NodeIndex of the deepest namespace
    pub fn get_or_create_path_in_unit<S: AsRef<str>>(
        &mut self,
        unit_root: NodeIndex,
        path: &[S],
    ) -> NodeIndex {
        let mut current = unit_root;
        let mut accumulated_path: Vec<String> = Vec::new();

        for segment in path {
            let name = segment.as_ref();
            accumulated_path.push(name.to_string());

            current = self.get_or_create_child(current, name);

            // Add Mirrors edges for this level
            self.add_mirrors_if_exists(current, &accumulated_path);
        }

        current
    }

    /// Get the relative path from the unit root to a node.
    ///
    /// Returns the namespace path excluding the unit root name itself.
    /// For example, for `$unit_0/Game/Entities`, returns `["Game", "Entities"]`.
    fn get_relative_path_from_unit(&self, node: NodeIndex) -> Vec<String> {
        let mut path = Vec::new();
        let mut current = node;

        while let Some(parent) = self.find_parent(current) {
            // Stop when we reach the tree root (parent of unit root)
            if parent == self.root {
                break;
            }
            if let Some(name) = self.get_namespace_name(current) {
                path.push(name.to_string());
            }
            current = parent;
        }

        path.reverse();
        path
    }

    /// Add Mirrors edges from a unit namespace to its counterparts in $ffi and $shared.
    ///
    /// # Arguments
    /// * `unit_ns` - The namespace node in the unit
    /// * `relative_path` - The path relative to the unit root (e.g., ["Game", "Entities"])
    fn add_mirrors_if_exists(&mut self, unit_ns: NodeIndex, relative_path: &[String]) {
        // Check $ffi for same-named namespace
        if let Some(ffi_root) = self.get_ffi_root() {
            if let Some(ffi_ns) = self.get_path_from(ffi_root, relative_path) {
                self.add_mirrors_edge(unit_ns, ffi_ns);
            }
        }

        // Check $shared for same-named namespace
        if let Some(shared_root) = self.get_shared_root() {
            if let Some(shared_ns) = self.get_path_from(shared_root, relative_path) {
                self.add_mirrors_edge(unit_ns, shared_ns);
            }
        }
    }

    /// Get an existing namespace by path starting from a given node.
    pub fn get_path_from<S: AsRef<str>>(&self, start: NodeIndex, path: &[S]) -> Option<NodeIndex> {
        let mut current = start;
        for segment in path {
            current = self.find_child(current, segment.as_ref())?;
        }
        Some(current)
    }

    /// Get the type hash index for O(1) lookups by TypeHash.
    pub fn type_hash_index(&self) -> &FxHashMap<TypeHash, (NodeIndex, String)> {
        &self.type_hash_index
    }

    /// Get a mutable reference to the type hash index.
    pub fn type_hash_index_mut(&mut self) -> &mut FxHashMap<TypeHash, (NodeIndex, String)> {
        &mut self.type_hash_index
    }

    /// Get the function hash index for O(1) lookups by function hash.
    pub fn func_hash_index(&self) -> &FxHashMap<TypeHash, (NodeIndex, String, usize)> {
        &self.func_hash_index
    }

    /// Get a mutable reference to the function hash index.
    pub fn func_hash_index_mut(&mut self) -> &mut FxHashMap<TypeHash, (NodeIndex, String, usize)> {
        &mut self.func_hash_index
    }

    // ========================================================================
    // Type Registration
    // ========================================================================

    /// Register a type in the tree.
    ///
    /// The type is stored in the specified namespace and indexed by its type hash
    /// for O(1) lookups.
    pub fn register_type<S: AsRef<str>>(
        &mut self,
        namespace_path: &[S],
        simple_name: &str,
        entry: TypeEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);
        let type_hash = entry.type_hash();

        // Check for duplicates first (before modifying)
        {
            let ns_data = self
                .graph
                .node_weight(ns_node)
                .ok_or(RegistrationError::InvalidNamespace)?;

            if ns_data.types.contains_key(simple_name) {
                let qualified = self.qualified_name(ns_node, simple_name);
                return Err(RegistrationError::DuplicateType(qualified));
            }
        }

        // Insert type
        let ns_data = self
            .graph
            .node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;
        ns_data.types.insert(simple_name.to_string(), entry);

        // Build reverse index
        self.type_hash_index
            .insert(type_hash, (ns_node, simple_name.to_string()));

        Ok(())
    }

    /// Get a type by hash (for bytecode dispatch).
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeEntry> {
        let (ns_node, name) = self.type_hash_index.get(&hash)?;
        self.graph.node_weight(*ns_node)?.types.get(name)
    }

    /// Get a mutable type by hash.
    pub fn get_type_by_hash_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        // Copy NodeIndex (it's Copy) and clone String separately to satisfy borrow checker
        // We need the name for the hashmap lookup after mutably borrowing the graph
        let &(ns_node, ref name) = self.type_hash_index.get(&hash)?;
        let name = name.clone();
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

    // ========================================================================
    // Type Resolution
    // ========================================================================

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
        // Delegate to resolve_type_with_location_checked and discard location
        match self.resolve_type_with_location_checked(name, ctx) {
            ResolutionResult::Found((entry, _)) => ResolutionResult::Found(entry),
            ResolutionResult::Ambiguous(matches) => {
                ResolutionResult::Ambiguous(matches.into_iter().map(|(n, (e, _))| (n, e)).collect())
            }
            ResolutionResult::NotFound => ResolutionResult::NotFound,
        }
    }

    /// Resolve an unqualified type name from a context.
    /// Convenience method that returns Option (returns first match, ignores ambiguity).
    /// Prefer `resolve_type_checked` for proper error handling.
    pub fn resolve_type(&self, name: &str, ctx: &ResolutionContext) -> Option<&TypeEntry> {
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
    ///
    /// Resolution order at each namespace level (walking up from current to root):
    /// 1. Check local symbols at current node
    /// 2. Follow Mirrors edges (same-named namespace in $ffi/$shared)
    /// 3. Follow Uses edges (explicit `using namespace` imports)
    /// 4. Walk up to parent namespace and repeat
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
            let entry = match self
                .graph
                .node_weight(ns_node)
                .and_then(|d| d.types.get(simple_name))
            {
                Some(e) => e,
                None => return ResolutionResult::NotFound,
            };
            return ResolutionResult::Found((entry, ns_node));
        }

        // Walk up namespace hierarchy, checking at each level:
        // 1. Local symbols
        // 2. Mirrors edges (FFI/shared counterparts)
        // 3. Uses edges (explicit imports)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            // 1. Check local symbols at this namespace
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(entry) = ns_data.types.get(name) {
                    return ResolutionResult::Found((entry, ns_node));
                }
            }

            // 2. Check Mirrors edges (FFI/shared counterparts) - iterate directly to avoid Vec allocation
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Mirrors) {
                    let mirror_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(mirror_ns) {
                        if let Some(entry) = ns_data.types.get(name) {
                            return ResolutionResult::Found((entry, mirror_ns));
                        }
                    }
                }
            }

            // 3. Check Uses edges (explicit imports) - may be ambiguous
            // Only allocate Vec if we find matches
            let mut uses_matches: Vec<(NodeIndex, (&TypeEntry, NodeIndex))> = Vec::new();
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Uses) {
                    let using_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(using_ns) {
                        if let Some(entry) = ns_data.types.get(name) {
                            if !uses_matches
                                .iter()
                                .any(|(_, (e, _))| e.type_hash() == entry.type_hash())
                            {
                                uses_matches.push((using_ns, (entry, using_ns)));
                            }
                        }
                    }
                }
            }

            // If we found matches via Uses edges at this level, return them
            match uses_matches.len() {
                0 => {} // Continue to parent
                1 => return ResolutionResult::Found(uses_matches.into_iter().next().unwrap().1),
                _ => {
                    return ResolutionResult::Ambiguous(
                        uses_matches.into_iter().map(|(ns, v)| (ns, v)).collect(),
                    );
                }
            }

            // 4. Walk up to parent namespace
            current = self.find_parent(ns_node);
        }

        ResolutionResult::NotFound
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
        Some(QualifiedName::new(entry.name(), path))
    }

    // ========================================================================
    // Function Registration
    // ========================================================================

    /// Register a function (allows overloads with same name).
    pub fn register_function<S: AsRef<str>>(
        &mut self,
        namespace_path: &[S],
        simple_name: &str,
        entry: FunctionEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);
        let func_hash = entry.def.func_hash;

        // Check for duplicate signature
        {
            let ns_data = self
                .graph
                .node_weight(ns_node)
                .ok_or(RegistrationError::InvalidNamespace)?;

            if let Some(overloads) = ns_data.functions.get(simple_name) {
                if overloads.iter().any(|f| f.def.func_hash == func_hash) {
                    let qualified = self.qualified_name(ns_node, simple_name);
                    return Err(RegistrationError::DuplicateFunction(qualified));
                }
            }
        }

        let ns_data = self
            .graph
            .node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;

        let overloads = ns_data
            .functions
            .entry(simple_name.to_string())
            .or_default();
        let overload_index = overloads.len();
        overloads.push(entry);

        self.func_hash_index.insert(
            func_hash,
            (ns_node, simple_name.to_string(), overload_index),
        );

        Ok(())
    }

    /// Get a function by hash.
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        let (ns_node, name, idx) = self.func_hash_index.get(&hash)?;
        self.graph
            .node_weight(*ns_node)?
            .functions
            .get(name)?
            .get(*idx)
    }

    // ========================================================================
    // Function Resolution
    // ========================================================================

    /// Resolve a function name (returns all overloads), with ambiguity detection.
    ///
    /// Resolution order at each namespace level (walking up from current to root):
    /// 1. Check local symbols at current node
    /// 2. Follow Mirrors edges (same-named namespace in $ffi/$shared)
    /// 3. Follow Uses edges (explicit `using namespace` imports)
    /// 4. Walk up to parent namespace and repeat
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
            return match self
                .graph
                .node_weight(ns_node)
                .and_then(|d| d.functions.get(simple_name))
            {
                Some(funcs) => ResolutionResult::Found(funcs.as_slice()),
                None => ResolutionResult::NotFound,
            };
        }

        // Walk up namespace hierarchy, checking at each level:
        // 1. Local symbols
        // 2. Mirrors edges (FFI/shared counterparts)
        // 3. Uses edges (explicit imports)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            // 1. Check local symbols at this namespace
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(funcs) = ns_data.functions.get(name) {
                    return ResolutionResult::Found(funcs.as_slice());
                }
            }

            // 2. Check Mirrors edges (FFI/shared counterparts) - iterate directly to avoid Vec allocation
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Mirrors) {
                    let mirror_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(mirror_ns) {
                        if let Some(funcs) = ns_data.functions.get(name) {
                            return ResolutionResult::Found(funcs.as_slice());
                        }
                    }
                }
            }

            // 3. Check Uses edges (explicit imports) - may be ambiguous
            // Only allocate Vec if we find matches
            let mut uses_matches: Vec<(NodeIndex, &[FunctionEntry])> = Vec::new();
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Uses) {
                    let using_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(using_ns) {
                        if let Some(funcs) = ns_data.functions.get(name) {
                            // Functions with same name in different namespaces are ambiguous
                            if !uses_matches.iter().any(|(ns, _)| *ns == using_ns) {
                                uses_matches.push((using_ns, funcs.as_slice()));
                            }
                        }
                    }
                }
            }

            // If we found matches via Uses edges at this level, return them
            match uses_matches.len() {
                0 => {} // Continue to parent
                1 => return ResolutionResult::Found(uses_matches.into_iter().next().unwrap().1),
                _ => return ResolutionResult::Ambiguous(uses_matches),
            }

            // 4. Walk up to parent namespace
            current = self.find_parent(ns_node);
        }

        ResolutionResult::NotFound
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

    // ========================================================================
    // Global Property Registration
    // ========================================================================

    /// Register a global property.
    pub fn register_global<S: AsRef<str>>(
        &mut self,
        namespace_path: &[S],
        simple_name: &str,
        entry: GlobalPropertyEntry,
    ) -> Result<(), RegistrationError> {
        let ns_node = self.get_or_create_path(namespace_path);

        // Check for duplicates first
        {
            let ns_data = self
                .graph
                .node_weight(ns_node)
                .ok_or(RegistrationError::InvalidNamespace)?;

            if ns_data.globals.contains_key(simple_name) {
                let qualified = self.qualified_name(ns_node, simple_name);
                return Err(RegistrationError::DuplicateGlobal(qualified));
            }
        }

        let ns_data = self
            .graph
            .node_weight_mut(ns_node)
            .ok_or(RegistrationError::InvalidNamespace)?;

        ns_data.globals.insert(simple_name.to_string(), entry);
        Ok(())
    }

    // ========================================================================
    // Global Property Resolution
    // ========================================================================

    /// Resolve a global property, with ambiguity detection.
    ///
    /// Resolution order at each namespace level (walking up from current to root):
    /// 1. Check local symbols at current node
    /// 2. Follow Mirrors edges (same-named namespace in $ffi/$shared)
    /// 3. Follow Uses edges (explicit `using namespace` imports)
    /// 4. Walk up to parent namespace and repeat
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
            return match self
                .graph
                .node_weight(ns_node)
                .and_then(|d| d.globals.get(simple_name))
            {
                Some(g) => ResolutionResult::Found(g),
                None => ResolutionResult::NotFound,
            };
        }

        // Walk up namespace hierarchy, checking at each level:
        // 1. Local symbols
        // 2. Mirrors edges (FFI/shared counterparts)
        // 3. Uses edges (explicit imports)
        let mut current = Some(ctx.current_namespace);
        while let Some(ns_node) = current {
            // 1. Check local symbols at this namespace
            if let Some(ns_data) = self.graph.node_weight(ns_node) {
                if let Some(global) = ns_data.globals.get(name) {
                    return ResolutionResult::Found(global);
                }
            }

            // 2. Check Mirrors edges (FFI/shared counterparts) - iterate directly to avoid Vec allocation
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Mirrors) {
                    let mirror_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(mirror_ns) {
                        if let Some(global) = ns_data.globals.get(name) {
                            return ResolutionResult::Found(global);
                        }
                    }
                }
            }

            // 3. Check Uses edges (explicit imports) - may be ambiguous
            // Only allocate Vec if we find matches
            let mut uses_matches: Vec<(NodeIndex, &GlobalPropertyEntry)> = Vec::new();
            for edge in self.graph.edges(ns_node) {
                if matches!(edge.weight(), NamespaceEdge::Uses) {
                    let using_ns = edge.target();
                    if let Some(ns_data) = self.graph.node_weight(using_ns) {
                        if let Some(global) = ns_data.globals.get(name) {
                            if !uses_matches.iter().any(|(ns, _)| *ns == using_ns) {
                                uses_matches.push((using_ns, global));
                            }
                        }
                    }
                }
            }

            // If we found matches via Uses edges at this level, return them
            match uses_matches.len() {
                0 => {} // Continue to parent
                1 => return ResolutionResult::Found(uses_matches.into_iter().next().unwrap().1),
                _ => return ResolutionResult::Ambiguous(uses_matches),
            }

            // 4. Walk up to parent namespace
            current = self.find_parent(ns_node);
        }

        ResolutionResult::NotFound
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

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        ClassEntry, ConstantValue, DataType, FunctionDef, FunctionTraits, GlobalPropertyEntry,
        TypeKind, TypeSource, Visibility,
    };

    /// Helper to create a test TypeEntry with a namespace.
    fn make_test_type_in_namespace(name: &str, namespace: &[&str]) -> TypeEntry {
        let ns: Vec<String> = namespace.iter().map(|s| s.to_string()).collect();
        let qualified_name = if ns.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", ns.join("::"), name)
        };
        let type_hash = TypeHash::from_name(&qualified_name);
        ClassEntry::new(
            name,
            ns,
            &qualified_name,
            type_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .into()
    }

    /// Helper to create a test FunctionEntry.
    fn make_test_function(name: &str) -> FunctionEntry {
        let def = FunctionDef::new(
            TypeHash::from_name(name),
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        FunctionEntry::ffi(def)
    }

    /// Helper to create a test FunctionEntry with a different hash (for overloads).
    fn make_test_function_with_hash(name: &str, hash_name: &str) -> FunctionEntry {
        let def = FunctionDef::new(
            TypeHash::from_name(hash_name),
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        FunctionEntry::ffi(def)
    }

    /// Helper to create a test GlobalPropertyEntry.
    fn make_test_global(name: &str) -> GlobalPropertyEntry {
        GlobalPropertyEntry::constant(name, ConstantValue::Int32(42))
    }

    // ========================================================================
    // Phase 6 TDD Tests - Type Registration and Resolution
    // ========================================================================

    #[test]
    fn register_and_resolve_type() {
        let mut tree = NamespaceTree::new();
        let entry = make_test_type_in_namespace("Player", &["Game"]);

        tree.register_type(&["Game"], "Player", entry).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game"]).unwrap(),
        };

        let resolved = tree.resolve_type("Player", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name(), "Player");
    }

    #[test]
    fn resolve_via_using_directive() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils"]);
        let game = tree.get_or_create_path(&["Game"]);

        // Register Helper in Utils
        let entry = make_test_type_in_namespace("Helper", &["Utils"]);
        tree.register_type(&["Utils"], "Helper", entry).unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game context
        let ctx = ResolutionContext {
            current_namespace: game,
        };
        let resolved = tree.resolve_type("Helper", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name(), "Helper");
    }

    #[test]
    fn using_not_transitive() {
        let mut tree = NamespaceTree::new();

        let a = tree.get_or_create_path(&["A"]);
        let b = tree.get_or_create_path(&["B"]);
        let c = tree.get_or_create_path(&["C"]);

        // Register CType in C
        let entry = make_test_type_in_namespace("CType", &["C"]);
        tree.register_type(&["C"], "CType", entry).unwrap();

        // A uses B, B uses C
        tree.add_using_directive(a, b);
        tree.add_using_directive(b, c);

        // CType should NOT be visible from A (non-transitive)
        let ctx = ResolutionContext {
            current_namespace: a,
        };
        let resolved = tree.resolve_type("CType", &ctx);
        assert!(
            resolved.is_none(),
            "using directives should not be transitive"
        );
    }

    #[test]
    fn parent_scope_using_inherited() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils"]);
        let game = tree.get_or_create_path(&["Game"]);
        let entities = tree.get_or_create_path(&["Game", "Entities"]);

        // Register Helper in Utils
        let entry = make_test_type_in_namespace("Helper", &["Utils"]);
        tree.register_type(&["Utils"], "Helper", entry).unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game::Entities context (should find via parent's using)
        let ctx = ResolutionContext {
            current_namespace: entities,
        };
        let resolved = tree.resolve_type("Helper", &ctx);
        assert!(
            resolved.is_some(),
            "should find Helper via parent namespace's using directive"
        );
        assert_eq!(resolved.unwrap().name(), "Helper");
    }

    #[test]
    fn ambiguous_using_detection() {
        let mut tree = NamespaceTree::new();

        let ns_b = tree.get_or_create_path(&["B"]);
        let ns_c = tree.get_or_create_path(&["C"]);
        let ns_a = tree.get_or_create_path(&["A"]);

        // Register Helper in both B and C with different type hashes
        let entry_b = make_test_type_in_namespace("Helper", &["B"]);
        let entry_c = make_test_type_in_namespace("Helper", &["C"]);
        tree.register_type(&["B"], "Helper", entry_b).unwrap();
        tree.register_type(&["C"], "Helper", entry_c).unwrap();

        // A uses both B and C
        tree.add_using_directive(ns_a, ns_b);
        tree.add_using_directive(ns_a, ns_c);

        // Resolve should detect ambiguity
        let ctx = ResolutionContext {
            current_namespace: ns_a,
        };
        match tree.resolve_type_checked("Helper", &ctx) {
            ResolutionResult::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2, "should find two ambiguous matches");
            }
            ResolutionResult::Found(_) => panic!("Expected ambiguity, got Found"),
            ResolutionResult::NotFound => panic!("Expected ambiguity, got NotFound"),
        }
    }

    #[test]
    fn resolve_type_to_name() {
        let mut tree = NamespaceTree::new();

        // Register Player in Game
        let entry = make_test_type_in_namespace("Player", &["Game"]);
        tree.register_type(&["Game"], "Player", entry).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game"]).unwrap(),
        };

        let qname = tree.resolve_type_to_name("Player", &ctx);
        assert!(qname.is_some());
        assert_eq!(qname.unwrap().to_string(), "Game::Player");
    }

    #[test]
    fn duplicate_type_detection() {
        let mut tree = NamespaceTree::new();

        let entry1 = make_test_type_in_namespace("Player", &["Game"]);
        let entry2 = make_test_type_in_namespace("Player", &["Game"]);

        tree.register_type(&["Game"], "Player", entry1).unwrap();

        let result = tree.register_type(&["Game"], "Player", entry2);
        assert!(result.is_err());
        match result {
            Err(RegistrationError::DuplicateType(name)) => {
                assert_eq!(name, "Game::Player");
            }
            _ => panic!("Expected DuplicateType error"),
        }
    }

    #[test]
    fn resolve_qualified_type() {
        let mut tree = NamespaceTree::new();

        let entry = make_test_type_in_namespace("Player", &["Game", "Entities"]);
        tree.register_type(&["Game", "Entities"], "Player", entry)
            .unwrap();

        // Resolve from root context using qualified name
        let ctx = ResolutionContext {
            current_namespace: tree.root(),
        };

        let resolved = tree.resolve_type("Game::Entities::Player", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name(), "Player");
    }

    #[test]
    fn get_type_by_hash() {
        let mut tree = NamespaceTree::new();

        let entry = make_test_type_in_namespace("Player", &["Game"]);
        let type_hash = entry.type_hash();

        tree.register_type(&["Game"], "Player", entry).unwrap();

        let found = tree.get_type_by_hash(type_hash);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Player");
    }

    // ========================================================================
    // Phase 6 TDD Tests - Function Registration and Resolution
    // ========================================================================

    #[test]
    fn register_and_resolve_function() {
        let mut tree = NamespaceTree::new();

        let entry = make_test_function("update");
        tree.register_function(&["Game"], "update", entry).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game"]).unwrap(),
        };

        let resolved = tree.resolve_function("update", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().len(), 1);
    }

    #[test]
    fn function_overloads() {
        let mut tree = NamespaceTree::new();

        // Register two functions with same name but different signatures (hashes)
        let entry1 = make_test_function_with_hash("update", "update()");
        let entry2 = make_test_function_with_hash("update", "update(int)");

        tree.register_function(&["Game"], "update", entry1).unwrap();
        tree.register_function(&["Game"], "update", entry2).unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game"]).unwrap(),
        };

        let resolved = tree.resolve_function("update", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().len(), 2, "should have two overloads");
    }

    #[test]
    fn duplicate_function_detection() {
        let mut tree = NamespaceTree::new();

        let entry1 = make_test_function("update");
        let entry2 = make_test_function("update"); // Same hash

        tree.register_function(&["Game"], "update", entry1).unwrap();

        let result = tree.register_function(&["Game"], "update", entry2);
        assert!(result.is_err());
        match result {
            Err(RegistrationError::DuplicateFunction(name)) => {
                assert_eq!(name, "Game::update");
            }
            _ => panic!("Expected DuplicateFunction error"),
        }
    }

    #[test]
    fn function_via_using_directive() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils"]);
        let game = tree.get_or_create_path(&["Game"]);

        // Register helper function in Utils
        let entry = make_test_function("helper");
        tree.register_function(&["Utils"], "helper", entry).unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game context
        let ctx = ResolutionContext {
            current_namespace: game,
        };
        let resolved = tree.resolve_function("helper", &ctx);
        assert!(resolved.is_some());
    }

    // ========================================================================
    // Phase 6 TDD Tests - Global Property Registration and Resolution
    // ========================================================================

    #[test]
    fn register_and_resolve_global() {
        let mut tree = NamespaceTree::new();

        let entry = make_test_global("MAX_PLAYERS");
        tree.register_global(&["Game"], "MAX_PLAYERS", entry)
            .unwrap();

        let ctx = ResolutionContext {
            current_namespace: tree.get_path(&["Game"]).unwrap(),
        };

        let resolved = tree.resolve_global("MAX_PLAYERS", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "MAX_PLAYERS");
    }

    #[test]
    fn duplicate_global_detection() {
        let mut tree = NamespaceTree::new();

        let entry1 = make_test_global("PI");
        let entry2 = make_test_global("PI");

        tree.register_global(&["Math"], "PI", entry1).unwrap();

        let result = tree.register_global(&["Math"], "PI", entry2);
        assert!(result.is_err());
        match result {
            Err(RegistrationError::DuplicateGlobal(name)) => {
                assert_eq!(name, "Math::PI");
            }
            _ => panic!("Expected DuplicateGlobal error"),
        }
    }

    #[test]
    fn global_via_using_directive() {
        let mut tree = NamespaceTree::new();

        let math = tree.get_or_create_path(&["Math"]);
        let game = tree.get_or_create_path(&["Game"]);

        // Register PI in Math
        let entry = make_test_global("PI");
        tree.register_global(&["Math"], "PI", entry).unwrap();

        // Add using directive from Game to Math
        tree.add_using_directive(game, math);

        // Resolve from Game context
        let ctx = ResolutionContext {
            current_namespace: game,
        };
        let resolved = tree.resolve_global("PI", &ctx);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "PI");
    }

    #[test]
    fn namespace_hierarchy_takes_precedence() {
        let mut tree = NamespaceTree::new();

        let utils = tree.get_or_create_path(&["Utils"]);
        let game = tree.get_or_create_path(&["Game"]);

        // Register Helper in both Game and Utils
        let entry_game = make_test_type_in_namespace("Helper", &["Game"]);
        let entry_utils = make_test_type_in_namespace("Helper", &["Utils"]);

        tree.register_type(&["Game"], "Helper", entry_game).unwrap();
        tree.register_type(&["Utils"], "Helper", entry_utils)
            .unwrap();

        // Add using directive from Game to Utils
        tree.add_using_directive(game, utils);

        // Resolve from Game context - should find Game::Helper, not Utils::Helper
        let ctx = ResolutionContext {
            current_namespace: game,
        };

        // This should NOT be ambiguous - namespace hierarchy takes precedence
        let result = tree.resolve_type_checked("Helper", &ctx);
        assert!(result.is_found(), "should find in current namespace");
    }

    // ========================================================================
    // Existing Phase 5 Tests
    // ========================================================================

    #[test]
    fn create_namespace_path() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&["Game", "Entities"]);

        let path = tree.get_namespace_path(node);
        assert_eq!(path, vec!["Game", "Entities"]);
    }

    #[test]
    fn find_existing_path() {
        let mut tree = NamespaceTree::new();
        tree.get_or_create_path(&["Game", "Entities"]);

        let found = tree.get_path(&["Game", "Entities"]);
        assert!(found.is_some());

        let not_found = tree.get_path(&["Other"]);
        assert!(not_found.is_none());
    }

    #[test]
    fn using_directives() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game"]);
        let utils = tree.get_or_create_path(&["Utils"]);

        tree.add_using_directive(game, utils);

        let usings = tree.get_using_directives(game);
        assert_eq!(usings.len(), 1);
        assert_eq!(usings[0], utils);
    }

    #[test]
    fn qualified_name() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&["Game", "Entities"]);

        let qname = tree.qualified_name(node, "Player");
        assert_eq!(qname, "Game::Entities::Player");
    }

    #[test]
    fn root_namespace_is_created_on_init() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        assert!(tree.get_namespace(root).is_some());
    }

    #[test]
    fn empty_path_returns_root() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path::<&str>(&[]);
        assert_eq!(node, tree.root());
    }

    #[test]
    fn find_child_returns_none_for_nonexistent() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        assert!(tree.find_child(root, "NonExistent").is_none());
    }

    #[test]
    fn find_parent_of_root_returns_none() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        assert!(tree.find_parent(root).is_none());
    }

    #[test]
    fn get_namespace_name_of_root_returns_none() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        assert!(tree.get_namespace_name(root).is_none());
    }

    #[test]
    fn qualified_name_at_root_returns_simple_name() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        let qname = tree.qualified_name(root, "GlobalFunc");
        assert_eq!(qname, "GlobalFunc");
    }

    #[test]
    fn get_or_create_child_returns_same_node_if_exists() {
        let mut tree = NamespaceTree::new();
        let root = tree.root();

        let child1 = tree.get_or_create_child(root, "Game");
        let child2 = tree.get_or_create_child(root, "Game");

        assert_eq!(child1, child2);
    }

    #[test]
    fn duplicate_using_directive_is_ignored() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game"]);
        let utils = tree.get_or_create_path(&["Utils"]);

        tree.add_using_directive(game, utils);
        tree.add_using_directive(game, utils); // duplicate

        let usings = tree.get_using_directives(game);
        assert_eq!(usings.len(), 1);
    }

    #[test]
    fn multiple_using_directives() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game"]);
        let utils = tree.get_or_create_path(&["Utils"]);
        let math = tree.get_or_create_path(&["Math"]);

        tree.add_using_directive(game, utils);
        tree.add_using_directive(game, math);

        let usings = tree.get_using_directives(game);
        assert_eq!(usings.len(), 2);
        assert!(usings.contains(&utils));
        assert!(usings.contains(&math));
    }

    #[test]
    fn deep_namespace_path() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&["Company", "Product", "Module", "SubModule"]);

        let path = tree.get_namespace_path(node);
        assert_eq!(path, vec!["Company", "Product", "Module", "SubModule"]);

        let qname = tree.qualified_name(node, "MyClass");
        assert_eq!(qname, "Company::Product::Module::SubModule::MyClass");
    }

    #[test]
    fn get_namespace_mut_allows_modification() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game"]);

        // Modify the namespace data
        if let Some(data) = tree.get_namespace_mut(game) {
            data.type_aliases
                .insert("MyAlias".to_string(), TypeHash(12345));
        }

        // Verify modification persists
        let data = tree.get_namespace(game).unwrap();
        assert!(data.type_aliases.contains_key("MyAlias"));
    }

    #[test]
    fn find_parent_returns_correct_parent() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game"]);
        let entities = tree.get_or_create_path(&["Game", "Entities"]);

        let parent = tree.find_parent(entities);
        assert_eq!(parent, Some(game));

        let grandparent = tree.find_parent(game);
        assert_eq!(grandparent, Some(tree.root()));
    }

    // ========================================================================
    // Phase 7 Tests - Unit Isolation and Mirrors Edges
    // ========================================================================

    #[test]
    fn create_unit_creates_top_level_node() {
        let mut tree = NamespaceTree::new();
        let unit = tree.create_unit("$unit_0");

        assert_eq!(tree.find_parent(unit), Some(tree.root()));
        assert_eq!(tree.get_namespace_name(unit), Some("$unit_0"));
    }

    #[test]
    fn ffi_root_creates_ffi_namespace() {
        let mut tree = NamespaceTree::new();
        let ffi = tree.ffi_root();

        assert_eq!(tree.find_parent(ffi), Some(tree.root()));
        assert_eq!(tree.get_namespace_name(ffi), Some("$ffi"));

        // Calling again returns the same node
        let ffi2 = tree.ffi_root();
        assert_eq!(ffi, ffi2);
    }

    #[test]
    fn shared_root_creates_shared_namespace() {
        let mut tree = NamespaceTree::new();
        let shared = tree.shared_root();

        assert_eq!(tree.find_parent(shared), Some(tree.root()));
        assert_eq!(tree.get_namespace_name(shared), Some("$shared"));

        // Calling again returns the same node
        let shared2 = tree.shared_root();
        assert_eq!(shared, shared2);
    }

    #[test]
    fn get_unit_root_returns_correct_unit() {
        let mut tree = NamespaceTree::new();
        let unit = tree.create_unit("$unit_0");
        let game = tree.get_or_create_child(unit, "Game");
        let entities = tree.get_or_create_child(game, "Entities");

        assert_eq!(tree.get_unit_root(entities), Some(unit));
        assert_eq!(tree.get_unit_root(game), Some(unit));
        assert_eq!(tree.get_unit_root(unit), Some(unit));
        assert_eq!(tree.get_unit_root(tree.root()), None);
    }

    #[test]
    fn is_under_unit_checks_ancestry() {
        let mut tree = NamespaceTree::new();
        let unit = tree.create_unit("$unit_0");
        let game = tree.get_or_create_child(unit, "Game");
        let entities = tree.get_or_create_child(game, "Entities");
        let other_unit = tree.create_unit("$unit_1");

        assert!(tree.is_under_unit(entities, unit));
        assert!(tree.is_under_unit(game, unit));
        assert!(tree.is_under_unit(unit, unit));
        assert!(!tree.is_under_unit(other_unit, unit));
        assert!(!tree.is_under_unit(entities, other_unit));
    }

    #[test]
    fn mirrors_edge_created_for_ffi_namespace() {
        let mut tree = NamespaceTree::new();

        // Create FFI namespace with Math
        let ffi = tree.ffi_root();
        let ffi_math = tree.get_or_create_child(ffi, "Math");

        // Register a function in $ffi/Math
        let entry = make_test_function("sin");
        tree.register_function(&["$ffi", "Math"], "sin", entry)
            .unwrap();

        // Create a unit with Math namespace (should auto-create Mirrors edge)
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Check Mirrors edge was created
        let mirrors = tree.get_mirrors_targets(unit_math);
        assert!(
            mirrors.contains(&ffi_math),
            "Should have Mirrors edge to $ffi/Math"
        );
    }

    #[test]
    fn resolve_type_via_mirrors_edge() {
        let mut tree = NamespaceTree::new();

        // Register a type in $ffi/Math
        let ffi = tree.ffi_root();
        let _ffi_math = tree.get_or_create_child(ffi, "Math");
        let entry = make_test_type_in_namespace("Vector", &["$ffi", "Math"]);
        tree.register_type(&["$ffi", "Math"], "Vector", entry)
            .unwrap();

        // Create unit with Math namespace (creates Mirrors edge)
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Resolve from unit's Math namespace - should find via Mirrors
        let ctx = ResolutionContext {
            current_namespace: unit_math,
        };
        let resolved = tree.resolve_type("Vector", &ctx);
        assert!(resolved.is_some(), "Should resolve Vector via Mirrors edge");
        assert_eq!(resolved.unwrap().name(), "Vector");
    }

    #[test]
    fn resolve_function_via_mirrors_edge() {
        let mut tree = NamespaceTree::new();

        // Register a function in $ffi/Math
        let ffi = tree.ffi_root();
        let _ffi_math = tree.get_or_create_child(ffi, "Math");
        let entry = make_test_function("sin");
        tree.register_function(&["$ffi", "Math"], "sin", entry)
            .unwrap();

        // Create unit with Math namespace (creates Mirrors edge)
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Resolve from unit's Math namespace - should find via Mirrors
        let ctx = ResolutionContext {
            current_namespace: unit_math,
        };
        let resolved = tree.resolve_function("sin", &ctx);
        assert!(resolved.is_some(), "Should resolve sin via Mirrors edge");
    }

    #[test]
    fn resolve_global_via_mirrors_edge() {
        let mut tree = NamespaceTree::new();

        // Register a global in $ffi/Math
        let ffi = tree.ffi_root();
        let _ffi_math = tree.get_or_create_child(ffi, "Math");
        let entry = make_test_global("PI");
        tree.register_global(&["$ffi", "Math"], "PI", entry)
            .unwrap();

        // Create unit with Math namespace (creates Mirrors edge)
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Resolve from unit's Math namespace - should find via Mirrors
        let ctx = ResolutionContext {
            current_namespace: unit_math,
        };
        let resolved = tree.resolve_global("PI", &ctx);
        assert!(resolved.is_some(), "Should resolve PI via Mirrors edge");
        assert_eq!(resolved.unwrap().name, "PI");
    }

    #[test]
    fn local_symbol_takes_precedence_over_mirrors() {
        let mut tree = NamespaceTree::new();

        // Register a type in $ffi/Math
        let ffi = tree.ffi_root();
        let _ffi_math = tree.get_or_create_child(ffi, "Math");
        let ffi_entry = make_test_type_in_namespace("Vector", &["$ffi", "Math"]);
        tree.register_type(&["$ffi", "Math"], "Vector", ffi_entry)
            .unwrap();

        // Create unit with Math namespace
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Register a local type with the same name in the unit's Math
        let unit_entry = make_test_type_in_namespace("Vector", &["$unit_0", "Math"]);
        tree.register_type(&["$unit_0", "Math"], "Vector", unit_entry)
            .unwrap();

        // Resolve from unit's Math namespace - should find local, not FFI
        let ctx = ResolutionContext {
            current_namespace: unit_math,
        };
        let resolved = tree.resolve_type_with_location("Vector", &ctx);
        assert!(resolved.is_some());
        let (_, location) = resolved.unwrap();
        assert_eq!(
            location, unit_math,
            "Should resolve to local unit namespace"
        );
    }

    #[test]
    fn mirrors_edge_for_nested_namespace() {
        let mut tree = NamespaceTree::new();

        // Create nested FFI namespace $ffi/Game/Math
        let ffi = tree.ffi_root();
        tree.get_or_create_path_from(ffi, &["Game", "Math"]);
        let entry = make_test_function("clamp");
        tree.register_function(&["$ffi", "Game", "Math"], "clamp", entry)
            .unwrap();

        // Create unit with same nested namespace
        let unit = tree.create_unit("$unit_0");
        let unit_game_math = tree.get_or_create_path_in_unit(unit, &["Game", "Math"]);

        // Resolve from unit's Game::Math namespace
        let ctx = ResolutionContext {
            current_namespace: unit_game_math,
        };
        let resolved = tree.resolve_function("clamp", &ctx);
        assert!(
            resolved.is_some(),
            "Should resolve clamp via nested Mirrors edge"
        );
    }

    #[test]
    fn remove_unit_cleans_up_subtree() {
        let mut tree = NamespaceTree::new();

        // Create a unit with some content
        let unit = tree.create_unit("$unit_0");
        let game = tree.get_or_create_child(unit, "Game");
        let _entities = tree.get_or_create_child(game, "Entities");

        // Register types and functions
        let type_entry = make_test_type_in_namespace("Player", &["$unit_0", "Game"]);
        let type_hash = type_entry.type_hash();
        tree.register_type(&["$unit_0", "Game"], "Player", type_entry)
            .unwrap();

        let func_entry = make_test_function("update");
        let func_hash = func_entry.def.func_hash;
        tree.register_function(&["$unit_0", "Game"], "update", func_entry)
            .unwrap();

        // Verify they exist
        assert!(tree.get_type_by_hash(type_hash).is_some());
        assert!(tree.get_function_by_hash(func_hash).is_some());

        // Remove the unit
        tree.remove_unit(unit);

        // Verify cleanup
        assert!(
            tree.get_type_by_hash(type_hash).is_none(),
            "Type should be removed from hash index"
        );
        assert!(
            tree.get_function_by_hash(func_hash).is_none(),
            "Function should be removed from hash index"
        );
        assert!(
            tree.find_child(tree.root(), "$unit_0").is_none(),
            "Unit node should be removed"
        );
    }

    #[test]
    fn remove_unit_preserves_ffi() {
        let mut tree = NamespaceTree::new();

        // Create FFI content
        let ffi = tree.ffi_root();
        let _ffi_math = tree.get_or_create_child(ffi, "Math");
        let ffi_entry = make_test_type_in_namespace("Vector", &["$ffi", "Math"]);
        let ffi_hash = ffi_entry.type_hash();
        tree.register_type(&["$ffi", "Math"], "Vector", ffi_entry)
            .unwrap();

        // Create unit
        let unit = tree.create_unit("$unit_0");
        let _unit_game = tree.get_or_create_child(unit, "Game");

        // Remove unit
        tree.remove_unit(unit);

        // FFI should still exist
        assert!(
            tree.get_type_by_hash(ffi_hash).is_some(),
            "FFI types should not be removed"
        );
        assert!(
            tree.get_ffi_root().is_some(),
            "$ffi namespace should still exist"
        );
    }

    #[test]
    fn mirrors_edges_to_both_ffi_and_shared() {
        let mut tree = NamespaceTree::new();

        // Create Math in both $ffi and $shared
        let ffi = tree.ffi_root();
        let ffi_math = tree.get_or_create_child(ffi, "Math");
        let shared = tree.shared_root();
        let shared_math = tree.get_or_create_child(shared, "Math");

        // Create unit with Math namespace
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Should have Mirrors edges to both
        let mirrors = tree.get_mirrors_targets(unit_math);
        assert!(
            mirrors.contains(&ffi_math),
            "Should have Mirrors edge to $ffi/Math"
        );
        assert!(
            mirrors.contains(&shared_math),
            "Should have Mirrors edge to $shared/Math"
        );
    }

    #[test]
    fn duplicate_mirrors_edge_ignored() {
        let mut tree = NamespaceTree::new();

        // Create FFI Math
        let ffi = tree.ffi_root();
        let ffi_math = tree.get_or_create_child(ffi, "Math");

        // Create unit Math
        let unit = tree.create_unit("$unit_0");
        let unit_math = tree.get_or_create_path_in_unit(unit, &["Math"]);

        // Try to add same Mirrors edge again
        tree.add_mirrors_edge(unit_math, ffi_math);
        tree.add_mirrors_edge(unit_math, ffi_math);

        // Should only have one
        let mirrors = tree.get_mirrors_targets(unit_math);
        assert_eq!(mirrors.len(), 1);
    }

    #[test]
    fn get_path_from_helper() {
        let mut tree = NamespaceTree::new();
        let ffi = tree.ffi_root();
        let ffi_game = tree.get_or_create_child(ffi, "Game");
        let ffi_game_math = tree.get_or_create_child(ffi_game, "Math");

        // Find existing path
        let found = tree.get_path_from(ffi, &["Game", "Math"]);
        assert_eq!(found, Some(ffi_game_math));

        // Non-existent path returns None
        let not_found = tree.get_path_from(ffi, &["Other"]);
        assert!(not_found.is_none());
    }

    // Helper for tests - get or create path from arbitrary start
    impl NamespaceTree {
        #[cfg(test)]
        fn get_or_create_path_from<S: AsRef<str>>(
            &mut self,
            start: NodeIndex,
            path: &[S],
        ) -> NodeIndex {
            let mut current = start;
            for segment in path {
                current = self.get_or_create_child(current, segment.as_ref());
            }
            current
        }
    }
}
