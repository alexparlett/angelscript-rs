//! FFI Registration System for native Rust functions and types.
//!
//! This module provides the infrastructure for registering native Rust functions
//! and types that can be called from AngelScript code. It includes:
//!
//! - Type conversion traits (`FromScript`, `ToScript`)
//! - Native function storage (`NativeFn`, `CallContext`)
//! - Type kind and behavior definitions (`TypeKind`, `ReferenceKind`, `Behaviors`)
//! - Variable parameter type support (`AnyRef`, `AnyRefMut`)
//! - Template support (`TemplateInstanceInfo`, `TemplateValidation`)
//! - List initialization support (`ListBuffer`, `TupleListBuffer`, `ListPattern`)
//!
//! # Architecture
//!
//! The FFI system is designed for registration only - it stores type metadata
//! and function pointers for semantic analysis. VM execution is separate.
//!
//! Type specifications (parameters, return types) use AST primitives parsed from
//! declaration strings, not FFI-specific types. This module provides only:
//! - Runtime value conversion (FromScript/ToScript)
//! - Type memory semantics (TypeKind, ReferenceKind)
//! - Lifecycle behaviors (stored directly on FfiTypeDef)
//!
//! ```text
//! Module (registration) -> apply_to_registry() -> Registry (semantic analysis)
//! ```

mod any_type;
mod class_builder;
mod enum_builder;
mod error;
mod ffi_registry;
mod global_property;
mod interface_builder;
mod list_buffer;
mod native_fn;
mod traits;
mod types;

// Re-export core types
pub use any_type::{AnyRef, AnyRefMut};
pub use class_builder::ClassBuilder;
pub use enum_builder::EnumBuilder;
pub use error::{ConversionError, NativeError};
pub use ffi_registry::{FfiRegistry, FfiRegistryBuilder, FfiRegistryError};
pub use global_property::GlobalPropertyDef;
pub use interface_builder::InterfaceBuilder;
pub use list_buffer::{ListBuffer, ListPattern, TupleListBuffer};
pub use native_fn::{CallContext, NativeCallable, NativeFn, ObjectHandle, ObjectHeap, VmSlot};
pub use traits::{FromScript, IntoNativeFn, NativeType, ThisMut, ThisRef, ToScript};
pub use types::{
    ListBehavior, ReferenceKind, TemplateInstanceInfo, TemplateValidation, TypeKind,
};
