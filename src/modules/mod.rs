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
//! - [`string_module`] - Parse and format functions (parseInt, formatInt, etc.)
//! - [`math_module`] - Math constants and functions (sin, cos, sqrt, etc.)

mod array;
mod dict;
mod math;
mod std_io;
mod string;

pub use array::ScriptArray;
pub use dict::ScriptDict;
pub use math::math_module;
pub use std_io::std_module;
pub use string::{string_module, ScriptString};
