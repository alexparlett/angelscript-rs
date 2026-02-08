//! Method signature type for interface method definitions.

use crate::{DataType, TypeHash};

/// A method signature for interfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    /// Method name.
    pub name: String,
    /// Parameter types.
    pub params: Vec<DataType>,
    /// Return type.
    pub return_type: DataType,
    /// Whether the method is const.
    pub is_const: bool,
}

impl MethodSignature {
    /// Create a new method signature.
    pub fn new(name: impl Into<String>, params: Vec<DataType>, return_type: DataType) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const: false,
        }
    }

    /// Create a new const method signature.
    pub fn new_const(
        name: impl Into<String>,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const: true,
        }
    }

    /// Compute the signature hash for vtable/itable matching.
    ///
    /// Uses name and parameter types with modifiers (excludes owner and return type)
    /// so that override matching works correctly in inheritance hierarchies.
    /// The signature hash includes parameter modifiers (const, handle, ref) so that
    /// `foo(int)` and `foo(int &in)` are treated as different signatures.
    /// Also includes const flag so `foo()` and `foo() const` are different.
    pub fn signature_hash(&self) -> u64 {
        let param_sig_hashes: Vec<u64> = self.params.iter().map(|p| p.signature_hash()).collect();
        TypeHash::from_signature(&self.name, &param_sig_hashes, self.is_const).0
    }
}
