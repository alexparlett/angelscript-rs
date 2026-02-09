mod engine;

// Re-export sub-crates for advanced use.
pub use angelscript_core as core;
pub use angelscript_macros as macros;
pub use angelscript_vm as vm;

// Re-export proc macros at the top level for ergonomic use.
pub use angelscript_macros::{script_function, ScriptType};

// Re-export commonly used types at the top level.
pub use angelscript_core::{
    DataType, Diagnostic, FunctionId, PrimitiveType, QualifiedName, Severity, Value,
};
pub use angelscript_vm::module::{Module, NativeCallContext, NativeFunction, ScriptFunction};

pub use engine::{CompiledModule, Engine, EngineError};

#[cfg(test)]
mod macro_tests;

#[cfg(test)]
mod engine_tests;
