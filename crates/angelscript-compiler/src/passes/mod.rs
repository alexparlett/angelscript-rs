//! Compiler passes for the two-pass compilation model.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Walk AST and register all type and function declarations
//!   into the registry. Collects signatures without compiling function bodies.
//! - **Pass 1b (Type Completion)**: Copy inherited members from base classes to enable
//!   O(1) lookups without walking inheritance chains.
//! - **Pass 2 (Compilation)**: Type check function bodies and generate bytecode (future).
//!
//! ## Module Structure
//!
//! - [`registration`]: Pass 1 implementation
//! - [`completion`]: Pass 1b implementation

pub mod completion;
pub mod registration;

pub use completion::{CompletionOutput, TypeCompletionPass};
pub use registration::{RegistrationOutput, RegistrationPass};
