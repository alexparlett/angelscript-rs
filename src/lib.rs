#[cfg(feature = "macros")]
pub mod macros {
    pub use angelscript_macros::*;
}

pub mod core;
mod internal;

pub mod types;
pub mod plugins;

// Add the inventory module
// Re-export macros
#[cfg(feature = "macros")]
pub use angelscript_macros::*;

// Re-export main types
pub mod prelude {
    pub use crate::core::context::*;
    pub use crate::core::engine::*;
    pub use crate::core::error::{ScriptError, ScriptResult};
    pub use crate::core::function::*;
    pub use crate::core::lockable_shared_bool::*;
    pub use crate::core::module::*;
    pub use crate::core::script_generic::*;
    pub use crate::core::script_object::*;
    pub use crate::core::typeinfo::*;
    pub use crate::plugins::plugin::*;
    pub use crate::types::enums::*;
    pub use crate::types::script_data::*;
    pub use crate::types::script_memory::*;
    pub use crate::types::script_value::*;
    pub use crate::types::user_data::*;
}
