//! Built-in standard library modules for AngelScript.
//!
//! This crate provides runtime types and FFI registration for built-in
//! AngelScript types like string, array, and dictionary.
//!
//! # Runtime Types
//!
//! - [`ScriptString`] - Value type for string operations
//! - [`ScriptArray`] - Reference-counted array container
//! - [`ScriptDict`] - Reference-counted dictionary container
//!
//! # Built-in Modules
//!
//! - [`std_module`] - I/O functions (print, println, eprint, eprintln)
//! - [`string_module`] - Parse and format functions (parseInt, formatInt, etc.)
//! - [`math_module`] - Math constants and functions (sin, cos, sqrt, etc.)
//! - [`array_module`] - Array template type (array<T>)
//! - [`dictionary_module`] - Dictionary template type (dictionary<K,V>)
//!
//! # Getting All Default Modules
//!
//! Use [`default_modules`] to get all built-in modules at once:
//!
//! ```ignore
//! use angelscript_modules::default_modules;
//!
//! let modules = default_modules().expect("failed to create modules");
//! // modules is a Vec<Module> containing all built-in modules
//! ```

mod array;
mod dict;
mod math;
mod std;
mod string;

use angelscript_module::{RegistrationError, Module};

pub use array::{array_module, ScriptArray};
pub use dict::{dictionary_module, ScriptDict};
pub use math::math_module;
pub use std::std_module;
pub use string::{string_module, ScriptString};

/// Creates all default built-in modules.
///
/// Returns a vector containing:
/// - `std_module` - I/O functions (print, println, etc.)
/// - `string_module` - Parse and format functions
/// - `math_module` - Math constants and functions
/// - `array_module` - Array template type
/// - `dictionary_module` - Dictionary template type
///
/// # Example
///
/// ```ignore
/// use angelscript_modules::default_modules;
///
/// let modules = default_modules().expect("failed to create modules");
/// // Register with engine...
/// registry.import_modules(&modules)?;
/// ```
pub fn default_modules<'app>() -> Result<Vec<Module<'app>>, RegistrationError> {
    Ok(vec![
        std_module()?,
        string_module()?,
        math_module()?,
        array_module()?,
        dictionary_module()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_modules_creates_all() {
        let modules = default_modules().expect("default modules should build");
        assert_eq!(modules.len(), 5, "should have 5 default modules");
    }

    #[test]
    fn test_default_modules_contains_std() {
        let modules = default_modules().expect("default modules should build");
        // std_module is root namespace with print functions
        let std = &modules[0];
        assert!(std.is_root(), "std module should be root namespace");
        assert!(
            std.functions().iter().any(|f| f.0.name == "print"),
            "should have print function"
        );
    }

    #[test]
    fn test_default_modules_contains_math() {
        let modules = default_modules().expect("default modules should build");
        // math_module is "math" namespace
        let math = &modules[2];
        assert_eq!(
            math.namespace(),
            &["math".to_string()],
            "math module should be math namespace"
        );
    }

    #[test]
    fn test_default_modules_contains_array() {
        let modules = default_modules().expect("default modules should build");
        // array_module has array<T> template
        let array = &modules[3];
        assert!(
            array.types().iter().any(|t| t.name == "array"),
            "should have array type"
        );
    }

    #[test]
    fn test_default_modules_contains_dictionary() {
        let modules = default_modules().expect("default modules should build");
        // dictionary_module has dictionary<K,V> template
        let dict = &modules[4];
        assert!(
            dict.types().iter().any(|t| t.name == "dictionary"),
            "should have dictionary type"
        );
    }
}
