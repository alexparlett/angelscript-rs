//! Owned funcdef (function pointer type) definitions for FFI registry.
//!
//! This module provides `FfiFuncdefDef`, an owned funcdef definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.
//!
//! A funcdef defines a function signature type that can be used for callbacks,
//! delegates, or function pointers in scripts.

use crate::types::{FfiDataType, FfiParam, TypeHash};

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
///     vec![FfiParam::new("value", FfiDataType::resolved(DataType::simple(primitive_hashes::INT32)), None)],
///     FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FfiFuncdefDef {
    /// Type ID assigned during build()
    pub id: TypeHash,

    /// Funcdef name
    pub name: String,

    /// Parameter definitions (with deferred type resolution)
    pub params: Vec<FfiParam>,

    /// Return type (with deferred type resolution)
    pub return_type: FfiDataType,
}

impl FfiFuncdefDef {
    /// Create a new funcdef definition.
    pub fn new(
        id: TypeHash,
        name: impl Into<String>,
        params: Vec<FfiParam>,
        return_type: FfiDataType,
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
    pub fn params(&self) -> &[FfiParam] {
        &self.params
    }

    /// Get the return type.
    pub fn return_type(&self) -> &FfiDataType {
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
    use crate::semantic::types::DataType;
    use crate::types::primitive_hashes;

    #[test]
    fn funcdef_creation() {
        let funcdef = FfiFuncdefDef::new(
            TypeHash::from_name("test_type"),
            "Callback",
            vec![FfiParam::new(
                "value",
                FfiDataType::resolved(DataType::simple(primitive_hashes::INT32)),
            )],
            FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
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
            FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
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
            FfiDataType::resolved(DataType::simple(primitive_hashes::VOID)),
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
            vec![FfiParam::new(
                "x",
                FfiDataType::resolved(DataType::simple(primitive_hashes::INT32)),
            )],
            FfiDataType::resolved(DataType::simple(primitive_hashes::INT32)),
        );

        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        assert_eq!(cloned.param_count(), original.param_count());
    }
}
