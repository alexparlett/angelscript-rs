//! Namespace Tree - hierarchical storage for all symbols.
//!
//! Uses `petgraph::DiGraph` with:
//! - Nodes: `NamespaceData` (types, functions, globals at that level)
//! - Edges: `Contains(name)` for hierarchy, `Uses` for `using namespace`

use angelscript_core::{FunctionEntry, GlobalPropertyEntry, TypeEntry, TypeHash};
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use rustc_hash::FxHashMap;

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
}

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

    #[test]
    fn root_namespace_is_created_on_init() {
        let tree = NamespaceTree::new();
        let root = tree.root();
        assert!(tree.get_namespace(root).is_some());
    }

    #[test]
    fn empty_path_returns_root() {
        let mut tree = NamespaceTree::new();
        let node = tree.get_or_create_path(&[]);
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
        let game = tree.get_or_create_path(&["Game".into()]);
        let utils = tree.get_or_create_path(&["Utils".into()]);

        tree.add_using_directive(game, utils);
        tree.add_using_directive(game, utils); // duplicate

        let usings = tree.get_using_directives(game);
        assert_eq!(usings.len(), 1);
    }

    #[test]
    fn multiple_using_directives() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game".into()]);
        let utils = tree.get_or_create_path(&["Utils".into()]);
        let math = tree.get_or_create_path(&["Math".into()]);

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
        let node = tree.get_or_create_path(&[
            "Company".into(),
            "Product".into(),
            "Module".into(),
            "SubModule".into(),
        ]);

        let path = tree.get_namespace_path(node);
        assert_eq!(path, vec!["Company", "Product", "Module", "SubModule"]);

        let qname = tree.qualified_name(node, "MyClass");
        assert_eq!(qname, "Company::Product::Module::SubModule::MyClass");
    }

    #[test]
    fn get_namespace_mut_allows_modification() {
        let mut tree = NamespaceTree::new();
        let game = tree.get_or_create_path(&["Game".into()]);

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
        let game = tree.get_or_create_path(&["Game".into()]);
        let entities = tree.get_or_create_path(&["Game".into(), "Entities".into()]);

        let parent = tree.find_parent(entities);
        assert_eq!(parent, Some(game));

        let grandparent = tree.find_parent(game);
        assert_eq!(grandparent, Some(tree.root()));
    }
}
