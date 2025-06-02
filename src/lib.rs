#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod ffi {
    pub use angelscript_bindings::*;
}

#[cfg(feature = "macros")]
pub mod macros {
    pub use angelscript_macros::*;
}

// Public modules - aligned with wrapper files
mod context;
mod engine;
mod enums;
mod error;
mod function;
mod module;
mod scriptobject;
mod typeinfo;
mod types;
mod utils;
mod stringfactory;

// Re-export main types
pub use context::*;
pub use engine::*;
pub use enums::*;
pub use error::{Error, Result};
pub use function::*;
pub use module::*;
pub use scriptobject::*;
pub use typeinfo::*;
pub use types::*;