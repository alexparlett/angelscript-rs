//! FFI Registration System for native Rust functions and types.
//!
//! This module provides the infrastructure for registering native Rust functions
//! and types that can be called from AngelScript code. It includes:
//!
//! - Type conversion traits (`FromScript`, `ToScript`)
//! - Native function storage (`NativeFn`, `CallContext`)
//! - Type specifications for registration (`TypeSpec`, `ParamDef`)
//! - Variable parameter type support (`AnyRef`, `AnyRefMut`)
//!
//! # Architecture
//!
//! The FFI system is designed for registration only - it stores type metadata
//! and function pointers for semantic analysis. VM execution is separate.
//!
//! ```text
//! Module (registration) -> apply_to_registry() -> Registry (semantic analysis)
//! ```

mod any_type;
mod error;
mod native_fn;
mod traits;
mod types;

// Re-export core types
pub use any_type::{AnyRef, AnyRefMut};
pub use error::{ConversionError, NativeError};
pub use native_fn::{CallContext, NativeCallable, NativeFn, ObjectHandle, ObjectHeap, VmSlot};
pub use traits::{FromScript, NativeType, ToScript};
pub use types::{
    Behaviors, NativeFunctionDef, NativeTypeDef, ParamDef, TypeKind, TypeSpec,
};
