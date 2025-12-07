//! TypeDef - type definition.

use super::TypeHash;

/// Definition of a type (class, interface, enum, etc.).
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub type_hash: TypeHash,
    pub name: String,
}
