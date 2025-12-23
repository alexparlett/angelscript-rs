//! Function definition (funcdef) type entry.
//!
//! This module provides `FuncdefEntry` for function pointer types.

use crate::{DataType, QualifiedName, TypeHash};

use super::TypeSource;

/// Registry entry for a function definition (funcdef) type.
///
/// Funcdefs are function pointer types in AngelScript, allowing functions
/// to be passed as values and stored in variables.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncdefEntry {
    /// Structured qualified name for name-based lookup.
    pub qname: QualifiedName,
    /// Unqualified name.
    pub name: String,
    /// Namespace path (e.g., `["Game", "Callbacks"]`).
    pub namespace: Vec<String>,
    /// Fully qualified name (with namespace).
    pub qualified_name: String,
    /// Type hash for identity.
    pub type_hash: TypeHash,
    /// Source (FFI or script).
    pub source: TypeSource,
    /// Parameter types.
    pub params: Vec<DataType>,
    /// Return type.
    pub return_type: DataType,
    /// Parent type for child funcdefs (e.g., `myTemplate<T>::callback`).
    /// None for global funcdefs.
    pub parent_type: Option<TypeHash>,
}

impl FuncdefEntry {
    /// Create a new funcdef entry.
    pub fn new(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        source: TypeSource,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        let name = name.into();
        let qualified_name = qualified_name.into();
        let qname = QualifiedName::new(name.clone(), namespace.clone());
        Self {
            qname,
            name,
            namespace,
            qualified_name,
            type_hash,
            source,
            params,
            return_type,
            parent_type: None,
        }
    }

    /// Create a funcdef entry from a QualifiedName.
    pub fn with_qname(
        qname: QualifiedName,
        source: TypeSource,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        let type_hash = qname.to_type_hash();
        Self {
            name: qname.simple_name().to_string(),
            namespace: qname.namespace_path().to_vec(),
            qualified_name: qname.to_string(),
            qname,
            type_hash,
            source,
            params,
            return_type,
            parent_type: None,
        }
    }

    /// Create a new funcdef entry with a parent type (child funcdef).
    ///
    /// For template instances, `parent_qname` should be the parent's QualifiedName
    /// (e.g., `QualifiedName::global("array<int>")`) to ensure unique identity.
    /// The child's namespace will be the parent's full path.
    #[allow(clippy::too_many_arguments)]
    pub fn new_child(
        name: impl Into<String>,
        parent_qname: &QualifiedName,
        type_hash: TypeHash,
        source: TypeSource,
        params: Vec<DataType>,
        return_type: DataType,
        parent_type: TypeHash,
    ) -> Self {
        let name = name.into();
        // Build namespace from parent's full qualified path
        // e.g., parent "Game::array<int>" -> namespace ["Game", "array<int>"]
        // e.g., parent "array<int>" (global) -> namespace ["array<int>"]
        let qname = parent_qname.child(name.clone());
        let namespace = qname.namespace_path().to_vec();
        let qualified_name = qname.to_string();
        Self {
            qname,
            name,
            namespace,
            qualified_name,
            type_hash,
            source,
            params,
            return_type,
            parent_type: Some(parent_type),
        }
    }

    /// Create an FFI funcdef entry in the global namespace.
    pub fn ffi(name: impl Into<String>, params: Vec<DataType>, return_type: DataType) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        let qname = QualifiedName::global(name.clone());
        Self {
            qname,
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name,
            type_hash,
            source: TypeSource::ffi_untyped(),
            params,
            return_type,
            parent_type: None,
        }
    }

    /// Get the structured qualified name.
    pub fn qname(&self) -> &QualifiedName {
        &self.qname
    }

    /// Check if this is a child funcdef (belongs to a parent type).
    pub fn is_child(&self) -> bool {
        self.parent_type.is_some()
    }

    /// Get the number of parameters.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }

    /// Check if this funcdef returns void.
    pub fn returns_void(&self) -> bool {
        self.return_type.is_void()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn funcdef_entry_creation() {
        let entry = FuncdefEntry::ffi(
            "Callback",
            vec![DataType::simple(primitives::INT32)],
            DataType::simple(primitives::BOOL),
        );

        assert_eq!(entry.name, "Callback");
        assert_eq!(entry.qualified_name, "Callback");
        assert!(
            entry.namespace.is_empty(),
            "ffi() should create empty namespace"
        );
        assert_eq!(entry.param_count(), 1);
        assert!(!entry.returns_void());
        assert!(entry.source.is_ffi());
    }

    #[test]
    fn funcdef_entry_with_namespace() {
        let entry = FuncdefEntry::new(
            "EventHandler",
            vec!["Events".to_string()],
            "Events::EventHandler",
            TypeHash::from_name("Events::EventHandler"),
            TypeSource::ffi_untyped(),
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );

        assert_eq!(entry.name, "EventHandler");
        assert_eq!(entry.namespace, vec!["Events".to_string()]);
        assert_eq!(entry.qualified_name, "Events::EventHandler");
        assert_eq!(entry.type_hash, TypeHash::from_name("Events::EventHandler"));
    }

    #[test]
    fn funcdef_entry_void_return() {
        let entry = FuncdefEntry::ffi("VoidCallback", vec![], DataType::void());

        assert!(entry.returns_void());
        assert_eq!(entry.param_count(), 0);
    }

    #[test]
    fn funcdef_entry_multiple_params() {
        let entry = FuncdefEntry::ffi(
            "BinaryOp",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::simple(primitives::INT32),
        );

        assert_eq!(entry.param_count(), 2);
        assert_eq!(entry.params[0].type_hash, primitives::INT32);
        assert_eq!(entry.params[1].type_hash, primitives::INT32);
    }

    #[test]
    fn funcdef_child_of_namespaced_parent() {
        // Setup parent: Game::Container
        let parent_qname = QualifiedName::new("Container", vec!["Game".to_string()]);
        let parent_hash = TypeHash::from_name("Game::Container");

        let entry = FuncdefEntry::new_child(
            "Callback",
            &parent_qname,
            TypeHash::from_name("Game::Container::Callback"),
            TypeSource::ffi_untyped(),
            vec![],
            DataType::void(),
            parent_hash,
        );

        assert_eq!(entry.name, "Callback");
        // Verify child inherits parent's namespace + parent name
        assert_eq!(
            entry.namespace,
            vec!["Game".to_string(), "Container".to_string()]
        );
        assert_eq!(entry.qualified_name, "Game::Container::Callback");
        assert!(entry.is_child());
    }

    #[test]
    fn funcdef_child_of_global_parent() {
        // Setup parent: array (global namespace)
        let parent_qname = QualifiedName::global("array");
        let parent_hash = TypeHash::from_name("array");

        let entry = FuncdefEntry::new_child(
            "Callback",
            &parent_qname,
            TypeHash::from_name("array::Callback"),
            TypeSource::ffi_untyped(),
            vec![DataType::simple(primitives::INT32)],
            DataType::simple(primitives::BOOL),
            parent_hash,
        );

        assert_eq!(entry.name, "Callback");
        // For global parent, namespace is just the parent name
        assert_eq!(entry.namespace, vec!["array".to_string()]);
        assert_eq!(entry.qualified_name, "array::Callback");
        assert!(entry.is_child());
    }

    #[test]
    fn funcdef_child_of_template_instance() {
        // Setup parent: Game::array<int> (template instance in namespace)
        let parent_qname = QualifiedName::new("array<int>", vec!["Game".to_string()]);
        let parent_hash = TypeHash::from_name("Game::array<int>");

        let entry = FuncdefEntry::new_child(
            "Less",
            &parent_qname,
            TypeHash::from_name("Game::array<int>::Less"),
            TypeSource::ffi_untyped(),
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::simple(primitives::BOOL),
            parent_hash,
        );

        assert_eq!(entry.name, "Less");
        // Namespace must include the full template instance name
        assert_eq!(
            entry.namespace,
            vec!["Game".to_string(), "array<int>".to_string()]
        );
        assert_eq!(entry.qualified_name, "Game::array<int>::Less");
        assert!(entry.is_child());
    }
}
