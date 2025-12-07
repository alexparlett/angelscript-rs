//! AngelScript Compiler
//!
//! A clean 2-pass compiler implementation for AngelScript.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Register all types and functions with complete signatures
//! - **Pass 2 (Compilation)**: Type check function bodies and generate bytecode
//!
//! ## Modules
//!
//! - [`types`]: Core type definitions (TypeHash, DataType, TypeDef, FunctionDef, ExprInfo)
//! - [`registry`]: Script type and function registry
//! - [`context`]: Compilation context with name resolution
//! - [`passes`]: Compiler passes (registration and compilation)

pub mod types;
pub mod registry;
pub mod context;
pub mod passes;

// Re-export commonly used types for convenience
pub use types::{TypeHash, DataType, RefModifier, TypeDef, FunctionDef, ExprInfo};
pub use registry::ScriptRegistry;
pub use context::CompilationContext;
