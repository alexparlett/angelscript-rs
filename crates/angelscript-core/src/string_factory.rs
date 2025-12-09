//! String factory trait for creating string values from raw byte data.
//!
//! This module provides the [`StringFactory`] trait that enables custom string
//! implementations for string literals in AngelScript. Similar to C++ AngelScript's
//! `asIStringFactory`, this allows users to use interned strings, OsString,
//! ASCII-optimized strings, or any other string implementation.
//!
//! ## Why Raw Bytes?
//!
//! The factory receives raw bytes rather than a Rust `&str` because:
//! - No UTF-8 assumption - factory interprets bytes however it wants
//! - Supports non-UTF8 escape sequences (`\xFF`, etc.)
//! - Enables OsString, ASCII-optimized strings, interned strings, etc.
//!
//! ## Usage
//!
//! ```ignore
//! use angelscript_core::{StringFactory, TypeHash};
//!
//! pub struct MyStringFactory;
//!
//! impl StringFactory for MyStringFactory {
//!     fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
//!         let s = String::from_utf8_lossy(data);
//!         Box::new(s.into_owned())
//!     }
//!
//!     fn type_hash(&self) -> TypeHash {
//!         TypeHash::from_name("string")
//!     }
//! }
//! ```

use crate::TypeHash;

/// Trait for creating string values from raw byte data.
///
/// Implement this trait to use custom string types for string literals.
/// The VM calls `create()` when loading string constants from the bytecode.
///
/// Similar to C++ AngelScript's `asIStringFactory`.
pub trait StringFactory: Send + Sync {
    /// Create a string value from raw bytes.
    ///
    /// Called by the VM when loading string constants. The factory
    /// interprets the bytes however it wants (UTF-8, ASCII, etc.).
    ///
    /// # Arguments
    ///
    /// * `data` - Raw byte data from the string literal
    ///
    /// # Returns
    ///
    /// A boxed value of the string type. The actual type must match
    /// what `type_hash()` returns.
    fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync>;

    /// The type hash of the string type this factory produces.
    ///
    /// Used by the compiler for type checking string literals.
    /// Must match the type registered in the symbol registry.
    fn type_hash(&self) -> TypeHash;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestStringFactory;

    impl StringFactory for TestStringFactory {
        fn create(&self, data: &[u8]) -> Box<dyn std::any::Any + Send + Sync> {
            let s = String::from_utf8_lossy(data);
            Box::new(s.into_owned())
        }

        fn type_hash(&self) -> TypeHash {
            TypeHash::from_name("test_string")
        }
    }

    #[test]
    fn factory_creates_string() {
        let factory = TestStringFactory;
        let value = factory.create(b"hello");
        let s = value.downcast::<String>().unwrap();
        assert_eq!(*s, "hello");
    }

    #[test]
    fn factory_type_hash() {
        let factory = TestStringFactory;
        assert_eq!(factory.type_hash(), TypeHash::from_name("test_string"));
    }

    #[test]
    fn factory_handles_non_utf8() {
        let factory = TestStringFactory;
        // Invalid UTF-8 sequence
        let data = vec![0xFF, 0xFE, b'a', b'b'];
        let value = factory.create(&data);
        let s = value.downcast::<String>().unwrap();
        // from_utf8_lossy replaces invalid sequences with replacement character
        assert!(s.contains("ab"));
    }
}
