mod compiler;
mod vm;

mod parser;
mod core;

pub use core::engine::{ScriptEngine, TypeFlags, GetModuleFlag};
pub use core::context::{Context, ExecutionResult, ContextState};
pub use core::module::Module;
pub use compiler::AngelscriptCompiler;