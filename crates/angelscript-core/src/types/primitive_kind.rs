//! Primitive type kinds for AngelScript's built-in numeric and boolean types.

use std::fmt;

use crate::TypeHash;

/// Primitive type kinds.
///
/// These are the built-in numeric and boolean types in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float,
    Double,
}

impl PrimitiveKind {
    /// Get the TypeHash for this primitive type.
    pub const fn type_hash(self) -> TypeHash {
        use crate::primitives;
        match self {
            PrimitiveKind::Void => primitives::VOID,
            PrimitiveKind::Bool => primitives::BOOL,
            PrimitiveKind::Int8 => primitives::INT8,
            PrimitiveKind::Int16 => primitives::INT16,
            PrimitiveKind::Int32 => primitives::INT32,
            PrimitiveKind::Int64 => primitives::INT64,
            PrimitiveKind::Uint8 => primitives::UINT8,
            PrimitiveKind::Uint16 => primitives::UINT16,
            PrimitiveKind::Uint32 => primitives::UINT32,
            PrimitiveKind::Uint64 => primitives::UINT64,
            PrimitiveKind::Float => primitives::FLOAT,
            PrimitiveKind::Double => primitives::DOUBLE,
        }
    }

    /// Get the name of this primitive type.
    pub const fn name(self) -> &'static str {
        match self {
            PrimitiveKind::Void => "void",
            PrimitiveKind::Bool => "bool",
            PrimitiveKind::Int8 => "int8",
            PrimitiveKind::Int16 => "int16",
            PrimitiveKind::Int32 => "int",
            PrimitiveKind::Int64 => "int64",
            PrimitiveKind::Uint8 => "uint8",
            PrimitiveKind::Uint16 => "uint16",
            PrimitiveKind::Uint32 => "uint",
            PrimitiveKind::Uint64 => "uint64",
            PrimitiveKind::Float => "float",
            PrimitiveKind::Double => "double",
        }
    }
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
