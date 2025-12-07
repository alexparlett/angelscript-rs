//! FunctionDef - function definition.

use super::TypeHash;

/// Definition of a function.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub func_hash: TypeHash,
    pub name: String,
}
