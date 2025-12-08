//! Owned funcdef (function pointer type) definitions for FFI registry.
//!
//! This module provides `FfiFuncdefDef`, an owned funcdef definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.
//!
//! A funcdef defines a function signature type that can be used for callbacks,
//! delegates, or function pointers in scripts.

use angelscript_core::{DataType, Param, TypeHash};

/// A funcdef (function pointer type) definition.
///
/// This is an owned funcdef definition that can be stored in `Arc<FfiRegistry>`
/// without arena lifetimes.
///
/// Funcdefs define function signature types that scripts can use for callbacks
/// and delegates.
///
/// # Example
///
/// ```ignore
/// // Define a callback type: void Callback(int value)
/// let funcdef = FfiFuncdefDef::new(
///     TypeHash::from_name("test_type"),
///     "Callback",
///     vec![FfiParam::new("value", DataType::simple(primitive_hashes::INT32))],
///     DataType::simple(primitive_hashes::VOID),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FfiFuncdefDef {
    /// Type ID assigned during build()
    pub id: TypeHash,

    /// Funcdef name
    pub name: String,

    /// Parameter definitions (always resolved)
    pub params: Vec<Param>,

    /// Return type (always resolved)
    pub return_type: DataType,
}

impl FfiFuncdefDef {
    /// Create a new funcdef definition.
    pub fn new(
        id: TypeHash,
        name: impl Into<String>,
        params: Vec<Param>,
        return_type: DataType,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            params,
            return_type,
        }
    }

    /// Get the funcdef name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the parameters.
    pub fn params(&self) -> &[Param] {
        &self.params
    }

    /// Get the return type.
    pub fn return_type(&self) -> &DataType {
        &self.return_type
    }

    /// Get the number of parameters.
    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn funcdef_creation() {
        let funcdef = FfiFuncdefDef::new(
            TypeHash::from_name("test_type"),
            "Callback",
            vec![Param::new(
                "value",
                DataType::simple(primitives::INT32),
            )],
            DataType::simple(primitives::VOID),
        );

        assert_eq!(funcdef.name(), "Callback");
        assert_eq!(funcdef.param_count(), 1);
        assert_eq!(funcdef.params()[0].name, "value");
    }

    #[test]
    fn funcdef_no_params() {
        let funcdef = FfiFuncdefDef::new(
            TypeHash::from_name("test_type"),
            "NoArgCallback",
            vec![],
            DataType::simple(primitives::VOID),
        );

        assert_eq!(funcdef.name(), "NoArgCallback");
        assert_eq!(funcdef.param_count(), 0);
    }

    #[test]
    fn debug_output() {
        let funcdef = FfiFuncdefDef::new(
            TypeHash::from_name("test_type"),
            "TestFunc",
            vec![],
            DataType::simple(primitives::VOID),
        );
        let debug = format!("{:?}", funcdef);
        assert!(debug.contains("FfiFuncdefDef"));
        assert!(debug.contains("TestFunc"));
    }

    #[test]
    fn clone() {
        let original = FfiFuncdefDef::new(
            TypeHash::from_name("test_type"),
            "Cloneable",
            vec![Param::new(
                "x",
                DataType::simple(primitives::INT32),
            )],
            DataType::simple(primitives::INT32),
        );

        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        assert_eq!(cloned.param_count(), original.param_count());
    }
}
