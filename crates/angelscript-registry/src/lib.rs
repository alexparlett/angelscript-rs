//! AngelScript Registry crate.
//!
//! This crate provides the unified type and function registry for AngelScript.
//!
//! # Overview
//!
//! The [`TypeRegistry`] provides central storage for all types and functions:
//!
//! - **Types**: All type entries stored by `TypeHash` for O(1) lookup
//! - **Functions**: All functions (global, methods, operators, behaviors) in one map
//! - **Template Callbacks**: Validation callbacks for template instantiation
//!
//! # Example
//!
//! ```
//! use angelscript_registry::TypeRegistry;
//! use angelscript_core::primitives;
//!
//! let registry = TypeRegistry::with_primitives();
//! assert!(registry.get(primitives::INT32).is_some());
//! ```

mod registry;

pub use registry::{TemplateCallback, TypeRegistry};

// Re-export from core for backwards compatibility during transition
pub use angelscript_core::{
    // Native function types
    NativeFn, NativeCallable, VmSlot, ObjectHandle, ObjectHeap, CallContext,
    // Error types
    ConversionError, NativeError,
    // List buffer types
    ListBuffer, TupleListBuffer, ListPattern,
    // Template types
    TemplateInstanceInfo, TemplateValidation,
    // Any type support
    AnyRef, AnyRefMut,
    // Core types
    DataType, RefModifier, TypeHash, TypeKind,
};
