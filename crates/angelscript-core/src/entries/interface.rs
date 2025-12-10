//! Interface type entry.
//!
//! This module provides `InterfaceEntry` for interface types.

use crate::{MethodSignature, TypeHash};

use super::TypeSource;

/// Registry entry for an interface type.
///
/// Interfaces define a contract of methods that implementing classes must provide.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceEntry {
    /// Unqualified name.
    pub name: String,
    /// Namespace path (e.g., `["Game", "Interfaces"]`).
    pub namespace: Vec<String>,
    /// Fully qualified name (with namespace).
    pub qualified_name: String,
    /// Type hash for identity.
    pub type_hash: TypeHash,
    /// Source (FFI or script).
    pub source: TypeSource,
    /// Required method signatures.
    pub methods: Vec<MethodSignature>,
    /// Base interface type hashes.
    pub base_interfaces: Vec<TypeHash>,
}

impl InterfaceEntry {
    /// Create a new interface entry.
    pub fn new(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        source: TypeSource,
    ) -> Self {
        Self {
            name: name.into(),
            namespace,
            qualified_name: qualified_name.into(),
            type_hash,
            source,
            methods: Vec::new(),
            base_interfaces: Vec::new(),
        }
    }

    /// Create an FFI interface entry in the global namespace.
    pub fn ffi(name: impl Into<String>) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        Self {
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name,
            type_hash,
            source: TypeSource::ffi_untyped(),
            methods: Vec::new(),
            base_interfaces: Vec::new(),
        }
    }

    /// Add a method signature.
    pub fn with_method(mut self, method: MethodSignature) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a base interface.
    pub fn with_base(mut self, base: TypeHash) -> Self {
        self.base_interfaces.push(base);
        self
    }

    /// Find a method by name.
    pub fn find_method(&self, name: &str) -> Option<&MethodSignature> {
        self.methods.iter().find(|m| m.name == name)
    }

    /// Check if this interface has a specific base interface.
    pub fn has_base(&self, base: TypeHash) -> bool {
        self.base_interfaces.contains(&base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives, DataType};

    #[test]
    fn interface_entry_creation() {
        let entry = InterfaceEntry::ffi("IDrawable");

        assert_eq!(entry.name, "IDrawable");
        assert_eq!(entry.qualified_name, "IDrawable");
        assert!(entry.namespace.is_empty(), "ffi() should create empty namespace");
        assert!(entry.source.is_ffi());
        assert!(entry.methods.is_empty());
        assert!(entry.base_interfaces.is_empty());
    }

    #[test]
    fn interface_entry_with_namespace() {
        let entry = InterfaceEntry::new(
            "IUpdatable",
            vec!["Game".to_string(), "Interfaces".to_string()],
            "Game::Interfaces::IUpdatable",
            TypeHash::from_name("Game::Interfaces::IUpdatable"),
            TypeSource::ffi_untyped(),
        );

        assert_eq!(entry.name, "IUpdatable");
        assert_eq!(entry.namespace, vec!["Game".to_string(), "Interfaces".to_string()]);
        assert_eq!(entry.qualified_name, "Game::Interfaces::IUpdatable");
        assert_eq!(entry.type_hash, TypeHash::from_name("Game::Interfaces::IUpdatable"));
    }

    #[test]
    fn interface_entry_with_method() {
        let draw_method = MethodSignature::new(
            "draw",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::void(),
        );

        let entry = InterfaceEntry::ffi("IDrawable").with_method(draw_method);

        assert_eq!(entry.methods.len(), 1);
        assert_eq!(entry.methods[0].name, "draw");
    }

    #[test]
    fn interface_entry_find_method() {
        let update = MethodSignature::new(
            "update",
            vec![DataType::simple(primitives::FLOAT)],
            DataType::void(),
        );
        let render = MethodSignature::new_const("render", vec![], DataType::void());

        let entry = InterfaceEntry::ffi("IEntity")
            .with_method(update)
            .with_method(render);

        assert!(entry.find_method("update").is_some());
        assert!(entry.find_method("render").is_some());
        assert!(entry.find_method("nonexistent").is_none());
    }

    #[test]
    fn interface_entry_with_base() {
        let base = TypeHash::from_name("IBase");
        let entry = InterfaceEntry::ffi("IDerived").with_base(base);

        assert!(entry.has_base(base));
        assert!(!entry.has_base(TypeHash::from_name("IOther")));
    }
}
