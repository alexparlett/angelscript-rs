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
mod callback_manager;
mod context;
mod engine;
mod enums;
mod error;
mod function;
mod jit_compiler;
mod lockable_shared_bool;
mod module;
mod scriptobject;
mod string;
mod stringfactory;
mod typeinfo;
mod types;
mod user_data;
mod utils;
mod globals;
mod thread_manager;
mod script_generic;

// Re-export main types
pub use context::*;
pub use engine::*;
pub use enums::*;
pub use error::{Error, Result};
pub use function::*;
pub use lockable_shared_bool::*;
pub use module::*;
pub use script_generic::*;
pub use scriptobject::*;
pub use typeinfo::*;
pub use types::*;
pub use globals::*;
pub use user_data::*;