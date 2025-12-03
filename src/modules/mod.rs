//! Built-in modules for AngelScript.
//!
//! This module contains runtime types and FFI registration for built-in
//! AngelScript types like string, array, and dictionary.

mod string;

pub use string::ScriptString;
