//! Compiler passes.
//!
//! - [`registration`]: Pass 1 - register types and functions with complete signatures
//! - [`compilation`]: Pass 2 - type check function bodies and generate bytecode

pub mod compilation;
pub mod registration;

pub use registration::{RegistrationError, RegistrationOutput, RegistrationPass};
