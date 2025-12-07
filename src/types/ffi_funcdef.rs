//! Owned funcdef (function pointer type) definitions for FFI registry.
//!
//! This module provides `FfiFuncdefDef`, an owned funcdef definition
//! that can be stored in `Arc<FfiRegistry>` without arena lifetimes.
//!
//! A funcdef defines a function signature type that can be used for callbacks,
//! delegates, or function pointers in scripts.

use crate::semantic::types::type_def::TypeId;
use crate::types::{FfiDataType, FfiParam};

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
///     TypeId::next_ffi(),
///     "Callback",
///     vec![FfiParam::new("value", FfiDataType::resolved(DataType::simple(INT32_TYPE)), None)],
///     FfiDataType::resolved(DataType::simple(VOID_TYPE)),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FfiFuncdefDef {
    /// Type ID assigned during build()
    pub id: TypeId,

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
        id: TypeId,
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
    use crate::semantic::types::type_def::{INT32_TYPE, VOID_TYPE};

    #[test]
    fn funcdef_creation() {
        let funcdef = FfiFuncdefDef::new(
            TypeId::next_ffi(),
            "Callback",
            vec![FfiParam::new(
                "value",
                FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            )],
            FfiDataType::resolved(DataType::simple(VOID_TYPE)),
        );

        assert_eq!(funcdef.name(), "Callback");
        assert_eq!(funcdef.param_count(), 1);
        assert_eq!(funcdef.params()[0].name, "value");
    }

    #[test]
    fn funcdef_no_params() {
        let funcdef = FfiFuncdefDef::new(
            TypeId::next_ffi(),
            "NoArgCallback",
            vec![],
            FfiDataType::resolved(DataType::simple(VOID_TYPE)),
        );

        assert_eq!(funcdef.name(), "NoArgCallback");
        assert_eq!(funcdef.param_count(), 0);
    }

    #[test]
    fn debug_output() {
        let funcdef = FfiFuncdefDef::new(
            TypeId::next_ffi(),
            "TestFunc",
            vec![],
            FfiDataType::resolved(DataType::simple(VOID_TYPE)),
        );
        let debug = format!("{:?}", funcdef);
        assert!(debug.contains("FfiFuncdefDef"));
        assert!(debug.contains("TestFunc"));
    }

    #[test]
    fn clone() {
        let original = FfiFuncdefDef::new(
            TypeId::next_ffi(),
            "Cloneable",
            vec![FfiParam::new(
                "x",
                FfiDataType::resolved(DataType::simple(INT32_TYPE)),
            )],
            FfiDataType::resolved(DataType::simple(INT32_TYPE)),
        );

        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        assert_eq!(cloned.param_count(), original.param_count());
    }
}
