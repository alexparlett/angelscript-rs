//! Runtime value type for VM slots.

use std::any::Any;
use std::fmt;

use super::ObjectHandle;

/// A dynamic value that can be stored in VM slots.
///
/// This enum represents all possible values that can be stored in the VM's
/// stack or registers. It uses safe Rust constructs - no raw pointers.
///
/// Similar to Rhai's `Dynamic` type, this provides a unified runtime
/// representation for all AngelScript values.
///
/// Note: Dynamic does not implement Clone because Native values may not be cloneable.
/// Use `Dynamic::clone_if_possible()` for slots that don't contain Native values.
pub enum Dynamic {
    /// Void/empty
    Void,
    /// Integer value (i8, i16, i32, i64, u8, u16, u32, u64 all stored as i64)
    Int(i64),
    /// Floating point value (f32, f64 both stored as f64)
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value (owned)
    String(String),
    /// Handle to heap-allocated object (reference types)
    Object(ObjectHandle),
    /// Inline native value (small registered types stored directly)
    /// Uses Box<dyn Any> for type safety - no raw pointer casting
    Native(Box<dyn Any + Send + Sync>),
    /// Null handle
    NullHandle,
}

impl Dynamic {
    /// Get a human-readable name for this slot's type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Dynamic::Void => "void",
            Dynamic::Int(_) => "int",
            Dynamic::Float(_) => "float",
            Dynamic::Bool(_) => "bool",
            Dynamic::String(_) => "string",
            Dynamic::Object(_) => "object",
            Dynamic::Native(_) => "native",
            Dynamic::NullHandle => "null",
        }
    }

    /// Check if this slot is void.
    pub fn is_void(&self) -> bool {
        matches!(self, Dynamic::Void)
    }

    /// Check if this slot is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Dynamic::NullHandle)
    }

    /// Clone the slot if it doesn't contain a Native value.
    ///
    /// Returns None for Native values since they may not be cloneable.
    pub fn clone_if_possible(&self) -> Option<Self> {
        match self {
            Dynamic::Void => Some(Dynamic::Void),
            Dynamic::Int(v) => Some(Dynamic::Int(*v)),
            Dynamic::Float(v) => Some(Dynamic::Float(*v)),
            Dynamic::Bool(v) => Some(Dynamic::Bool(*v)),
            Dynamic::String(s) => Some(Dynamic::String(s.clone())),
            Dynamic::Object(h) => Some(Dynamic::Object(*h)),
            Dynamic::Native(_) => None,
            Dynamic::NullHandle => Some(Dynamic::NullHandle),
        }
    }
}

impl fmt::Debug for Dynamic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dynamic::Void => write!(f, "Void"),
            Dynamic::Int(v) => write!(f, "Int({})", v),
            Dynamic::Float(v) => write!(f, "Float({})", v),
            Dynamic::Bool(v) => write!(f, "Bool({})", v),
            Dynamic::String(s) => write!(f, "String({:?})", s),
            Dynamic::Object(h) => write!(f, "Object({:?})", h),
            Dynamic::Native(_) => write!(f, "Native(...)"),
            Dynamic::NullHandle => write!(f, "NullHandle"),
        }
    }
}

impl PartialEq for Dynamic {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Dynamic::Void, Dynamic::Void) => true,
            (Dynamic::Int(a), Dynamic::Int(b)) => a == b,
            (Dynamic::Float(a), Dynamic::Float(b)) => a == b,
            (Dynamic::Bool(a), Dynamic::Bool(b)) => a == b,
            (Dynamic::String(a), Dynamic::String(b)) => a == b,
            (Dynamic::Object(a), Dynamic::Object(b)) => a == b,
            (Dynamic::NullHandle, Dynamic::NullHandle) => true,
            // Native values can't be compared for equality
            (Dynamic::Native(_), Dynamic::Native(_)) => false,
            _ => false,
        }
    }
}
