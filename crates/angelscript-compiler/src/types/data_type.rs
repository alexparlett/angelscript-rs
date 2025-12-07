//! DataType - represents a type with modifiers.

use super::TypeHash;

/// A type with const/handle/reference modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataType {
    pub type_hash: TypeHash,
    pub is_const: bool,
    pub is_handle: bool,
    pub is_handle_to_const: bool,
}
