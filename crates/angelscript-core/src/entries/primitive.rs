//! Primitive type entry.
//!
//! This module provides `PrimitiveEntry` for built-in primitive types
//! like int, float, bool, etc.

use crate::{PrimitiveKind, TypeHash};

/// Registry entry for a primitive type.
///
/// Primitive types are the built-in numeric and boolean types in AngelScript.
/// They don't have methods, properties, or behaviors - just a kind and hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveEntry {
    /// The primitive kind (int, float, bool, etc.).
    pub kind: PrimitiveKind,
    /// Type hash for identity.
    pub type_hash: TypeHash,
}

impl PrimitiveEntry {
    /// Create a new primitive entry.
    pub fn new(kind: PrimitiveKind) -> Self {
        Self {
            kind,
            type_hash: kind.type_hash(),
        }
    }

    /// Get the name of this primitive type.
    pub fn name(&self) -> &'static str {
        self.kind.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn primitive_entry_int32() {
        let entry = PrimitiveEntry::new(PrimitiveKind::Int32);
        assert_eq!(entry.kind, PrimitiveKind::Int32);
        assert_eq!(entry.type_hash, primitives::INT32);
        assert_eq!(entry.name(), "int");
    }

    #[test]
    fn primitive_entry_float() {
        let entry = PrimitiveEntry::new(PrimitiveKind::Float);
        assert_eq!(entry.kind, PrimitiveKind::Float);
        assert_eq!(entry.type_hash, primitives::FLOAT);
        assert_eq!(entry.name(), "float");
    }

    #[test]
    fn primitive_entry_void() {
        let entry = PrimitiveEntry::new(PrimitiveKind::Void);
        assert_eq!(entry.kind, PrimitiveKind::Void);
        assert_eq!(entry.name(), "void");
    }
}
