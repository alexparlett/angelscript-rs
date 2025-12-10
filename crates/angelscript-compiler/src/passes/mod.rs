//! Compiler passes for the two-pass compilation model.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Walk AST and register all type and function declarations
//!   into the registry. Collects signatures without compiling function bodies.
//! - **Pass 2 (Compilation)**: Type check function bodies and generate bytecode (future).
//!
//! ## Module Structure
//!
//! - [`registration`]: Pass 1 implementation

pub mod registration;

pub use registration::{RegistrationOutput, RegistrationPass};
