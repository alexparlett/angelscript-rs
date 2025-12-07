//! Owned interface definitions for FFI registry.
//!
//! This module provides `FfiInterfaceDef` and `FfiInterfaceMethod`, owned
//! interface definitions that can be stored in `Arc<FfiRegistry>` without
//! arena lifetimes.

use crate::types::{FfiDataType, FfiParam, TypeHash};

/// An interface method signature.
///
/// This is an owned interface method that can be stored in `FfiInterfaceDef`.
#[derive(Debug, Clone)]
pub struct FfiInterfaceMethod {
    /// Method name
    pub name: String,

    /// Method parameters (with deferred type resolution)
    pub params: Vec<FfiParam>,

    /// Return type (with deferred type resolution)
    pub return_type: FfiDataType,

    /// Whether this method is const
    pub is_const: bool,
}

impl FfiInterfaceMethod {
    /// Create a new interface method.
    pub fn new(
        name: impl Into<String>,
        params: Vec<FfiParam>,
        return_type: FfiDataType,
        is_const: bool,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const,
        }
    }
}

/// An interface definition.
///
/// This is the FFI equivalent of `NativeInterfaceDef<'ast>`, but fully owned
/// so it can be stored in `Arc<FfiRegistry>`.
///
/// Interfaces define abstract method signatures that script classes can implement.
#[derive(Debug, Clone)]
pub struct FfiInterfaceDef {
    /// Type ID assigned during build()
    pub id: TypeHash,

    /// Interface name
    pub name: String,

    /// Abstract method signatures
    pub methods: Vec<FfiInterfaceMethod>,
}

impl FfiInterfaceDef {
    /// Create a new interface definition.
    pub fn new(id: TypeHash, name: impl Into<String>, methods: Vec<FfiInterfaceMethod>) -> Self {
        Self {
            id,
            name: name.into(),
            methods,
        }
    }

    /// Get the interface name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the interface methods.
    pub fn methods(&self) -> &[FfiInterfaceMethod] {
        &self.methods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::DataType;
    use crate::types::primitive_hashes;

    #[test]
    fn interface_method_creation() {
        let method = FfiInterfaceMethod::new(
            "draw",
            vec![],
            FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
            true,
        );

        assert_eq!(method.name, "draw");
        assert!(method.params.is_empty());
        assert!(method.is_const);
    }

    #[test]
    fn interface_def_creation() {
        let methods = vec![
            FfiInterfaceMethod::new(
                "draw",
                vec![],
                FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
                true,
            ),
            FfiInterfaceMethod::new(
                "update",
                vec![],
                FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
                false,
            ),
        ];

        let interface = FfiInterfaceDef::new(TypeHash::from_name("test_type"), "IDrawable", methods);

        assert_eq!(interface.name(), "IDrawable");
        assert_eq!(interface.methods().len(), 2);
        assert_eq!(interface.methods()[0].name, "draw");
        assert_eq!(interface.methods()[1].name, "update");
    }

    #[test]
    fn debug_output() {
        let interface = FfiInterfaceDef::new(TypeHash::from_name("test_type"), "ITest", vec![]);
        let debug = format!("{:?}", interface);
        assert!(debug.contains("FfiInterfaceDef"));
        assert!(debug.contains("ITest"));
    }
}
