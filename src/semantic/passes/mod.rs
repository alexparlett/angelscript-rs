//! Compilation passes for semantic analysis.
//!
//! The semantic analysis is performed in multiple passes:
//! - **Pass 1 (Registration):** Register all global names (types, functions, globals)
//! - **Pass 2a (Type Compilation):** Fill in type details and resolve type expressions
//! - **Pass 2b (Function Processing):** Type check function bodies and emit bytecode

pub mod registration;
pub mod type_compilation;
pub mod function_processor;

pub use registration::{RegistrationData, Registrar};
pub use type_compilation::{TypeCompilationData, TypeCompiler};
pub use function_processor::{CompiledFunction, FunctionCompiler};
