//! Built-in modules for AngelScript.
//!
//! This module contains runtime types and FFI registration for built-in
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

mod array;
mod dict;
mod std_io;
mod string;

pub use array::ScriptArray;
pub use dict::ScriptDict;
pub use std_io::std_module;
pub use string::ScriptString;
