//! Standard library modules for the AngelScript scripting engine.
//!
//! This crate provides the built-in types and functions for AngelScript:
//!
//! - **array** - `array<T>` template type for dynamic arrays
//! - **dictionary** - `dictionary<K,V>` template type for key-value maps
//! - **math** - Mathematical functions (sin, cos, sqrt, etc.)
//! - **std** - Standard functions (print, println, etc.)
//!
//! # Usage
//!
//! Each module provides a function that returns a `Module` which can be
//! installed into a `Context`:
//!
//! ```ignore
//! use angelscript_modules::{array, math};
//!
//! // Create modules
//! let array_module = array::module();
//! let math_module = math::module();
//!
//! // Install into context
//! context.install(array_module);
//! context.install(math_module);
//! ```

pub mod array;
pub mod dictionary;
pub mod math;
pub mod std;

// Re-export the types for convenience
pub use array::ScriptArray;
pub use dictionary::{ScriptDict, ScriptKey};
