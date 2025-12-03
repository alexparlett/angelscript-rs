//! Built-in modules for AngelScript.
//!
//! This module contains runtime types and FFI registration for built-in
//! AngelScript types like string, array, and dictionary.

mod array;
mod dict;
mod string;

pub use array::ScriptArray;
pub use dict::ScriptDict;
pub use string::ScriptString;
