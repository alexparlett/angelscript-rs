//! AngelScript FFI (Foreign Function Interface) crate.
//!
//! This crate provides the interface between Rust native code and AngelScript scripts.
//! It includes:
//! - Native function registration and invocation
//! - Value conversion traits (FromScript, ToScript)
//! - FFI type definitions (FfiTypeDef, FfiEnumDef, etc.)
//! - FFI registry for storing native definitions

// Error types
mod error;
pub use error::{ConversionError, NativeError};

// Native function invocation
mod native_fn;
pub use native_fn::{CallContext, NativeFn, NativeCallable, ObjectHandle, ObjectHeap, VmSlot};

// Type conversion traits
mod traits;
pub use traits::{FromScript, IntoNativeFn, NativeType, ToScript};

// List/initialization buffer support
mod list_buffer;
pub use list_buffer::{ListBuffer, ListPattern, TupleListBuffer};

// Template type support
mod template;
pub use template::{ListBehavior, TemplateInstanceInfo, TemplateValidation};

// Any type support
mod any_type;
pub use any_type::{AnyRef, AnyRefMut};

// FFI type definitions
mod types;
pub use types::{
    FfiEnumDef, FfiExpr, FfiExprExt, FfiFuncdefDef, FfiInterfaceDef, FfiInterfaceMethod,
    FfiPropertyDef, FfiTypeDef,
};

// AST to FFI conversion utilities
mod convert;
pub use convert::{
    function_param_to_ffi, param_type_to_data_type, return_type_to_data_type,
    type_expr_to_data_type,
};

// FFI registry
pub mod registry;
pub use registry::{FfiRegistry, FfiRegistryBuilder, FfiRegistryError};

// Re-export core types for convenience
pub use angelscript_core::{DataType, RefModifier, TypeHash, TypeKind};
