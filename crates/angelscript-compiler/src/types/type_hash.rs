//! TypeHash - unique identifier for types.

/// A unique hash identifying a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeHash(pub u64);
