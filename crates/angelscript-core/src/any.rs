//! Any trait for registrable types.
//!
//! This module provides the `Any` trait which must be implemented by all types
//! that can be registered with the AngelScript engine. It provides type identity
//! via `TypeHash` and the AngelScript type name.
//!
//! # Example
//!
//! ```
//! use angelscript_core::{Any, TypeHash};
//!
//! struct MyType {
//!     value: i32,
//! }
//!
//! impl Any for MyType {
//!     fn type_hash() -> TypeHash {
//!         TypeHash::from_name("MyType")
//!     }
//!
//!     fn type_name() -> &'static str {
//!         "MyType"
//!     }
//! }
//! ```
//!
//! With the `#[derive(Any)]` macro (from `angelscript-macros`):
//!
//! ```ignore
//! #[derive(Any)]
//! #[angelscript(name = "MyType")]
//! pub struct MyType {
//!     pub value: i32,
//! }
//! ```

use crate::{primitives, TypeHash};

/// Trait for types that can be registered with AngelScript.
///
/// This trait provides the type identity information needed for registration.
/// It should be implemented for all Rust types that will be exposed to scripts.
///
/// # Derive Macro
///
/// The recommended way to implement this trait is via `#[derive(Any)]` from
/// the `angelscript-macros` crate, which generates the implementation based
/// on struct attributes.
pub trait Any: 'static {
    /// Get the type hash for this type.
    ///
    /// This must return a consistent hash that uniquely identifies the type
    /// within the AngelScript type system.
    fn type_hash() -> TypeHash;

    /// Get the AngelScript type name.
    ///
    /// This is the name that will be used in scripts to refer to this type.
    fn type_name() -> &'static str;
}

// === Primitive Type Implementations ===

impl Any for () {
    fn type_hash() -> TypeHash {
        primitives::VOID
    }

    fn type_name() -> &'static str {
        "void"
    }
}

impl Any for bool {
    fn type_hash() -> TypeHash {
        primitives::BOOL
    }

    fn type_name() -> &'static str {
        "bool"
    }
}

impl Any for i8 {
    fn type_hash() -> TypeHash {
        primitives::INT8
    }

    fn type_name() -> &'static str {
        "int8"
    }
}

impl Any for i16 {
    fn type_hash() -> TypeHash {
        primitives::INT16
    }

    fn type_name() -> &'static str {
        "int16"
    }
}

impl Any for i32 {
    fn type_hash() -> TypeHash {
        primitives::INT32
    }

    fn type_name() -> &'static str {
        "int"
    }
}

impl Any for i64 {
    fn type_hash() -> TypeHash {
        primitives::INT64
    }

    fn type_name() -> &'static str {
        "int64"
    }
}

impl Any for u8 {
    fn type_hash() -> TypeHash {
        primitives::UINT8
    }

    fn type_name() -> &'static str {
        "uint8"
    }
}

impl Any for u16 {
    fn type_hash() -> TypeHash {
        primitives::UINT16
    }

    fn type_name() -> &'static str {
        "uint16"
    }
}

impl Any for u32 {
    fn type_hash() -> TypeHash {
        primitives::UINT32
    }

    fn type_name() -> &'static str {
        "uint"
    }
}

impl Any for u64 {
    fn type_hash() -> TypeHash {
        primitives::UINT64
    }

    fn type_name() -> &'static str {
        "uint64"
    }
}

impl Any for f32 {
    fn type_hash() -> TypeHash {
        primitives::FLOAT
    }

    fn type_name() -> &'static str {
        "float"
    }
}

impl Any for f64 {
    fn type_hash() -> TypeHash {
        primitives::DOUBLE
    }

    fn type_name() -> &'static str {
        "double"
    }
}

impl Any for String {
    fn type_hash() -> TypeHash {
        primitives::STRING
    }

    fn type_name() -> &'static str {
        "string"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_type_hashes() {
        assert_eq!(<()>::type_hash(), primitives::VOID);
        assert_eq!(bool::type_hash(), primitives::BOOL);
        assert_eq!(i32::type_hash(), primitives::INT32);
        assert_eq!(f32::type_hash(), primitives::FLOAT);
        assert_eq!(String::type_hash(), primitives::STRING);
    }

    #[test]
    fn primitive_type_names() {
        assert_eq!(<()>::type_name(), "void");
        assert_eq!(bool::type_name(), "bool");
        assert_eq!(i32::type_name(), "int");
        assert_eq!(i64::type_name(), "int64");
        assert_eq!(u32::type_name(), "uint");
        assert_eq!(f32::type_name(), "float");
        assert_eq!(f64::type_name(), "double");
        assert_eq!(String::type_name(), "string");
    }

    #[test]
    fn custom_type_implementation() {
        struct CustomType;

        impl Any for CustomType {
            fn type_hash() -> TypeHash {
                TypeHash::from_name("CustomType")
            }

            fn type_name() -> &'static str {
                "CustomType"
            }
        }

        assert_eq!(CustomType::type_name(), "CustomType");
        assert_eq!(CustomType::type_hash(), TypeHash::from_name("CustomType"));
    }
}
