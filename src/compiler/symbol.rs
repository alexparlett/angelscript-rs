// src/compiler/symbol.rs - Updated to match semantic.rs requirements

use crate::parser::ast::*;
use std::collections::HashMap;

// ==================== NAMESPACE STRUCTURE ====================

/// Represents a namespace node in the namespace tree
#[derive(Debug, Clone)]
pub struct NamespaceNode {
    pub name: String,
    pub full_path: Vec<String>,
    pub children: HashMap<String, NamespaceNode>,
}

impl NamespaceNode {
    pub fn new(name: String, full_path: Vec<String>) -> Self {
        Self {
            name,
            full_path,
            children: HashMap::new(),
        }
    }

    pub fn get_or_create_child(&mut self, name: &str) -> &mut NamespaceNode {
        if !self.children.contains_key(name) {
            let mut child_path = self.full_path.clone();
            child_path.push(name.to_string());
            self.children.insert(
                name.to_string(),
                NamespaceNode::new(name.to_string(), child_path),
            );
        }
        self.children.get_mut(name).unwrap()
    }

    pub fn find_child(&self, path: &[String]) -> Option<&NamespaceNode> {
        if path.is_empty() {
            return Some(self);
        }

        if let Some(child) = self.children.get(&path[0]) {
            child.find_child(&path[1..])
        } else {
            None
        }
    }

    pub fn find_child_mut(&mut self, path: &[String]) -> Option<&mut NamespaceNode> {
        if path.is_empty() {
            return Some(self);
        }

        if let Some(child) = self.children.get_mut(&path[0]) {
            child.find_child_mut(&path[1..])
        } else {
            None
        }
    }
}

// ==================== SYMBOL WITH NAMESPACE ====================

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_id: u32,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_reference: bool,
    pub namespace: Vec<String>,
}

impl Symbol {
    pub fn full_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }

    /// Create a new variable symbol
    pub fn variable(name: String, type_id: u32, namespace: Vec<String>) -> Self {
        Self {
            name,
            kind: SymbolKind::Variable,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace,
        }
    }

    /// Create a new function symbol
    pub fn function(name: String, type_id: u32, namespace: Vec<String>) -> Self {
        Self {
            name,
            kind: SymbolKind::Function,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace,
        }
    }

    /// Create a new type symbol
    pub fn type_symbol(name: String, type_id: u32, namespace: Vec<String>) -> Self {
        Self {
            name,
            kind: SymbolKind::Type,
            type_id,
            is_const: false,
            is_handle: false,
            is_reference: false,
            namespace,
        }
    }

    /// Mark as const
    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    /// Mark as handle
    pub fn with_handle(mut self) -> Self {
        self.is_handle = true;
        self
    }

    /// Mark as reference
    pub fn with_reference(mut self) -> Self {
        self.is_reference = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Type,
    EnumVariant,
    Member,
    Namespace,
}

// ==================== NAMESPACE-AWARE SYMBOL TABLE ====================

#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// Root namespace (global namespace)
    root: NamespaceNode,

    /// Current namespace path during compilation
    current_namespace: Vec<String>,

    /// Symbols organized by namespace path
    /// Key: namespace path (e.g., ["A", "B"]), Value: symbols in that namespace
    namespace_symbols: HashMap<Vec<String>, HashMap<String, Symbol>>,

    /// Using declarations - namespaces imported in current scope
    using_namespaces: Vec<Vec<String>>,

    /// Local scopes for function-local variables (not namespace-aware)
    local_scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            root: NamespaceNode::new(String::new(), vec![]),
            current_namespace: vec![],
            namespace_symbols: HashMap::new(),
            using_namespaces: vec![],
            local_scopes: vec![],
        }
    }

    // ==================== NAMESPACE MANAGEMENT ====================

    /// Enter a namespace (for compilation context)
    pub fn enter_namespace(&mut self, path: &[String]) {
        // Ensure namespace exists in tree
        self.ensure_namespace_exists(path);
        self.current_namespace = path.to_vec();
    }

    /// Exit current namespace (go to parent)
    pub fn exit_namespace(&mut self) {
        if !self.current_namespace.is_empty() {
            self.current_namespace.pop();
        }
    }

    /// Get current namespace path
    pub fn current_namespace(&self) -> &[String] {
        &self.current_namespace
    }

    /// Ensure a namespace exists in the tree
    fn ensure_namespace_exists(&mut self, path: &[String]) {
        if path.is_empty() {
            return;
        }

        let mut current = &mut self.root;
        for segment in path {
            current = current.get_or_create_child(segment);
        }

        // Ensure symbol map exists for this namespace
        if !self.namespace_symbols.contains_key(path) {
            self.namespace_symbols.insert(path.to_vec(), HashMap::new());
        }
    }

    // ==================== USING DECLARATIONS ====================

    /// Add a using declaration
    pub fn add_using(&mut self, namespace_path: Vec<String>) {
        if !self.using_namespaces.contains(&namespace_path) {
            self.using_namespaces.push(namespace_path);
        }
    }

    /// Clear using declarations (typically when exiting a scope)
    pub fn clear_using(&mut self) {
        self.using_namespaces.clear();
    }

    /// Get all using namespaces
    pub fn using_namespaces(&self) -> &[Vec<String>] {
        &self.using_namespaces
    }

    // ==================== LOCAL SCOPE MANAGEMENT ====================

    /// Push a new local scope (for function bodies, blocks, etc.)
    pub fn push_scope(&mut self) {
        self.local_scopes.push(HashMap::new());
    }

    /// Pop the current local scope
    pub fn pop_scope(&mut self) {
        if !self.local_scopes.is_empty() {
            self.local_scopes.pop();
        }
    }

    /// Check if we're in a local scope
    pub fn in_local_scope(&self) -> bool {
        !self.local_scopes.is_empty()
    }

    // ==================== SYMBOL INSERTION ====================

    /// Insert a symbol in the current namespace
    pub fn insert(&mut self, name: String, mut symbol: Symbol) {
        if self.in_local_scope() {
            // Insert into local scope (no namespace)
            if let Some(scope) = self.local_scopes.last_mut() {
                scope.insert(name, symbol);
            }
        } else {
            // Insert into current namespace
            symbol.namespace = self.current_namespace.clone();
            self.insert_in_namespace(name, symbol, &self.current_namespace.clone());
        }
    }

    /// Insert a symbol in a specific namespace
    pub fn insert_in_namespace(&mut self, name: String, symbol: Symbol, namespace_path: &[String]) {
        self.ensure_namespace_exists(namespace_path);

        let symbols = self
            .namespace_symbols
            .entry(namespace_path.to_vec())
            .or_insert_with(HashMap::new);

        symbols.insert(name, symbol);
    }

    /// Insert a symbol in the global namespace
    pub fn insert_global(&mut self, name: String, mut symbol: Symbol) {
        symbol.namespace = vec![];
        self.insert_in_namespace(name, symbol, &[]);
    }

    // ==================== SYMBOL LOOKUP ====================

    /// Lookup a symbol using AST Scope information
    pub fn lookup_with_scope(&self, name: &str, scope: &Scope) -> Option<&Symbol> {
        if scope.is_global {
            // Absolute path from global namespace
            if scope.path.is_empty() {
                // ::name - look in global namespace only
                self.lookup_in_namespace(name, &[])
            } else {
                // ::A::B::name - look in specific namespace from global
                self.lookup_in_namespace(name, &scope.path)
            }
        } else if !scope.path.is_empty() {
            // Relative path: A::B::name
            // Try as absolute path first
            if let Some(symbol) = self.lookup_in_namespace(name, &scope.path) {
                return Some(symbol);
            }

            // Try relative to current namespace
            let mut full_path = self.current_namespace.clone();
            full_path.extend(scope.path.clone());
            self.lookup_in_namespace(name, &full_path)
        } else {
            // No scope specified - use hierarchical lookup
            self.lookup(name)
        }
    }

    /// Hierarchical lookup (follows AngelScript rules)
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // 1. Check local scopes (innermost to outermost)
        for scope in self.local_scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }

        // 2. Check current namespace
        if let Some(symbol) = self.lookup_in_namespace(name, &self.current_namespace) {
            return Some(symbol);
        }

        // 3. Check parent namespaces (walk up the hierarchy)
        let mut current_path = self.current_namespace.clone();
        while !current_path.is_empty() {
            current_path.pop();
            if let Some(symbol) = self.lookup_in_namespace(name, &current_path) {
                return Some(symbol);
            }
        }

        // 4. Check using namespaces
        for using_ns in &self.using_namespaces {
            if let Some(symbol) = self.lookup_in_namespace(name, using_ns) {
                return Some(symbol);
            }
        }

        // 5. Check global namespace
        self.lookup_in_namespace(name, &[])
    }

    /// Lookup a symbol in a specific namespace
    pub fn lookup_in_namespace(&self, name: &str, namespace_path: &[String]) -> Option<&Symbol> {
        self.namespace_symbols
            .get(namespace_path)
            .and_then(|symbols| symbols.get(name))
    }

    /// Lookup only in global namespace
    pub fn lookup_global(&self, name: &str) -> Option<&Symbol> {
        self.lookup_in_namespace(name, &[])
    }

    /// Check if a symbol exists in the current local scope
    pub fn exists_in_current_scope(&self, name: &str) -> bool {
        if let Some(scope) = self.local_scopes.last() {
            scope.contains_key(name)
        } else {
            // Check current namespace
            self.namespace_symbols
                .get(&self.current_namespace)
                .map_or(false, |symbols| symbols.contains_key(name))
        }
    }

    // ==================== NAMESPACE QUERIES ====================

    /// Check if a namespace exists
    pub fn namespace_exists(&self, path: &[String]) -> bool {
        self.root.find_child(path).is_some()
    }

    /// Get all symbols in a namespace
    pub fn get_namespace_symbols(&self, path: &[String]) -> Option<&HashMap<String, Symbol>> {
        self.namespace_symbols.get(path)
    }

    /// Get all symbols in current namespace
    pub fn current_namespace_symbols(&self) -> Option<&HashMap<String, Symbol>> {
        self.namespace_symbols.get(&self.current_namespace)
    }

    /// List all namespaces (for debugging/introspection)
    pub fn list_namespaces(&self) -> Vec<Vec<String>> {
        let mut namespaces = vec![];
        self.collect_namespaces(&self.root, &mut namespaces);
        namespaces
    }

    fn collect_namespaces(&self, node: &NamespaceNode, result: &mut Vec<Vec<String>>) {
        if !node.full_path.is_empty() {
            result.push(node.full_path.clone());
        }
        for child in node.children.values() {
            self.collect_namespaces(child, result);
        }
    }

    // ==================== DEBUGGING ====================

    /// Print the symbol table structure (for debugging)
    pub fn debug_print(&self) {
        println!("=== Symbol Table ===");
        println!("Current namespace: {}", self.current_namespace.join("::"));
        println!("\nNamespaces:");
        self.debug_print_namespace(&self.root, 0);
        println!("\nUsing declarations:");
        for using_ns in &self.using_namespaces {
            println!("  using {}", using_ns.join("::"));
        }
        println!("\nLocal scopes: {}", self.local_scopes.len());
    }

    fn debug_print_namespace(&self, node: &NamespaceNode, indent: usize) {
        let indent_str = "  ".repeat(indent);
        let ns_name = if node.full_path.is_empty() {
            "::".to_string()
        } else {
            node.full_path.join("::")
        };

        println!("{}namespace {}", indent_str, ns_name);

        // Print symbols in this namespace
        if let Some(symbols) = self.namespace_symbols.get(&node.full_path) {
            for symbol in symbols.values() {
                println!(
                    "{}  {:?}: {} (type_id: {})",
                    indent_str, symbol.kind, symbol.name, symbol.type_id
                );
            }
        }

        // Print child namespaces
        for child in node.children.values() {
            self.debug_print_namespace(child, indent + 1);
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_creation() {
        let mut table = SymbolTable::new();
        table.enter_namespace(&vec!["A".to_string()]);
        table.enter_namespace(&vec!["A".to_string(), "B".to_string()]);

        assert!(table.namespace_exists(&vec!["A".to_string()]));
        assert!(table.namespace_exists(&vec!["A".to_string(), "B".to_string()]));
        assert!(!table.namespace_exists(&vec!["C".to_string()]));
    }

    #[test]
    fn test_symbol_insertion_and_lookup() {
        let mut table = SymbolTable::new();

        // Insert in global namespace
        table.insert_global(
            "globalVar".to_string(),
            Symbol::variable("globalVar".to_string(), 1, vec![]),
        );

        // Insert in namespace A
        table.enter_namespace(&vec!["A".to_string()]);
        table.insert(
            "aVar".to_string(),
            Symbol::variable("aVar".to_string(), 1, vec!["A".to_string()]),
        );

        // Lookup from namespace A
        assert!(table.lookup("aVar").is_some());
        assert!(table.lookup("globalVar").is_some()); // Should find in parent

        // Lookup from global
        table.exit_namespace();
        assert!(table.lookup("globalVar").is_some());
        assert!(table.lookup("aVar").is_none()); // Not in scope
        assert!(
            table
                .lookup_in_namespace("aVar", &vec!["A".to_string()])
                .is_some()
        );
    }

    #[test]
    fn test_scoped_lookup() {
        let mut table = SymbolTable::new();

        table.enter_namespace(&vec!["A".to_string()]);
        table.insert(
            "func".to_string(),
            Symbol::function("func".to_string(), 0, vec!["A".to_string()]),
        );
        table.exit_namespace();

        // Test ::A::func (absolute path)
        let scope = Scope {
            is_global: true,
            path: vec!["A".to_string()],
        };
        assert!(table.lookup_with_scope("func", &scope).is_some());

        // Test A::func (relative path from global)
        let scope = Scope {
            is_global: false,
            path: vec!["A".to_string()],
        };
        assert!(table.lookup_with_scope("func", &scope).is_some());
    }

    #[test]
    fn test_using_declarations() {
        let mut table = SymbolTable::new();

        table.enter_namespace(&vec!["A".to_string()]);
        table.insert(
            "func".to_string(),
            Symbol::function("func".to_string(), 0, vec!["A".to_string()]),
        );
        table.exit_namespace();

        // Without using, func is not visible
        assert!(table.lookup("func").is_none());

        // With using, func becomes visible
        table.add_using(vec!["A".to_string()]);
        assert!(table.lookup("func").is_some());
    }

    #[test]
    fn test_local_scopes() {
        let mut table = SymbolTable::new();

        table.push_scope();
        table.insert(
            "localVar".to_string(),
            Symbol::variable("localVar".to_string(), 1, vec![]),
        );

        assert!(table.lookup("localVar").is_some());

        table.pop_scope();
        assert!(table.lookup("localVar").is_none());
    }

    #[test]
    fn test_nested_namespaces() {
        let mut table = SymbolTable::new();

        table.enter_namespace(&vec!["A".to_string(), "B".to_string()]);
        table.insert(
            "deepVar".to_string(),
            Symbol::variable(
                "deepVar".to_string(),
                1,
                vec!["A".to_string(), "B".to_string()],
            ),
        );

        // Should find in current namespace
        assert!(table.lookup("deepVar").is_some());

        // Should find with full path
        assert!(
            table
                .lookup_in_namespace("deepVar", &vec!["A".to_string(), "B".to_string()])
                .is_some()
        );
    }
}
