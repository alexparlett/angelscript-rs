#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg(feature = "macros")]
pub mod macros {
    pub use angelscript_macros::*;
}

pub mod core;
mod internal;
pub mod plugins;
pub mod types;

// Re-export main types
pub mod prelude {
    pub use crate::types::enums::*;
    pub use crate::core::error::{ScriptError, ScriptResult};
    pub use crate::core::function::*;
    pub use crate::core::lockable_shared_bool::*;
    pub use crate::core::script_generic::*;
    pub use crate::core::script_object::*;
    pub use crate::core::typeinfo::*;
    pub use crate::types::script_value::*;
    pub use crate::types::user_data::*;
}
