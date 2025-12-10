//! Function definition (funcdef) type entry.
//!
//! This module provides `FuncdefEntry` for function pointer types.

use crate::{DataType, TypeHash};

use super::TypeSource;

/// Registry entry for a function definition (funcdef) type.
///
/// Funcdefs are function pointer types in AngelScript, allowing functions
/// to be passed as values and stored in variables.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncdefEntry {
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
        Self {
            name: name.into(),
            namespace,
            qualified_name: qualified_name.into(),
            type_hash,
            source,
            params,
            return_type,
            parent_type: None,
        }
    }

    /// Create a new funcdef entry with a parent type (child funcdef).
    pub fn new_child(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        source: TypeSource,
        params: Vec<DataType>,
        return_type: DataType,
        parent_type: TypeHash,
    ) -> Self {
        Self {
            name: name.into(),
            namespace,
            qualified_name: qualified_name.into(),
            type_hash,
            source,
            params,
            return_type,
            parent_type: Some(parent_type),
        }
    }

    /// Create an FFI funcdef entry in the global namespace.
    pub fn ffi(
        name: impl Into<String>,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        Self {
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
        assert!(entry.namespace.is_empty(), "ffi() should create empty namespace");
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
}
