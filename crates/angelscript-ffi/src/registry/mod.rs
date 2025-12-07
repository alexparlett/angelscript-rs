//! FFI registry for storing native type and function definitions.

mod ffi_registry;

pub use ffi_registry::{FfiRegistry, FfiRegistryBuilder};

// Re-export RegistrationError from core for convenience
pub use angelscript_core::RegistrationError;
