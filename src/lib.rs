// Re-export macros
#[cfg(feature = "macros")]
pub use angelscript_macros::*;

#[cfg(feature = "addons")]
pub mod addons {
    pub use angelscript_addons::*;
}

pub use angelscript_core::*;

// Re-export main types
pub mod prelude {
    pub use angelscript_core::core::context::*;
    pub use angelscript_core::core::engine::*;
    pub use angelscript_core::core::error::{ScriptError, ScriptResult};
    pub use angelscript_core::core::function::*;
    pub use angelscript_core::core::lockable_shared_bool::*;
    pub use angelscript_core::core::module::*;
    pub use angelscript_core::core::script_generic::*;
    pub use angelscript_core::core::script_object::*;
    pub use angelscript_core::core::typeinfo::*;
    pub use angelscript_core::types::enums::*;
    pub use angelscript_core::types::script_data::*;
    pub use angelscript_core::types::script_memory::*;
    pub use angelscript_core::types::script_value::*;
    pub use angelscript_core::types::user_data::*;
}
