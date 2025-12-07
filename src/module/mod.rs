//! FFI Registration System for native Rust functions and types.
//!
//! This module provides the infrastructure for registering native Rust functions
//! and types that can be called from AngelScript code.
//!
//! # Architecture
//!
//! The FFI system is designed for registration only - it stores type metadata
//! and function pointers for semantic analysis. VM execution is separate.
//!
//! ```text
//! Module (registration) -> apply_to_registry() -> Registry (semantic analysis)
//! ```

// Local builders (depend on Module from main crate)
mod class_builder;
mod enum_builder;
mod function_builder;
mod global_property;
mod interface_builder;
mod module;
pub mod stdlib;

// Re-export local builders
pub use class_builder::ClassBuilder;
pub use enum_builder::EnumBuilder;
pub use function_builder::FunctionBuilder;
pub use global_property::GlobalPropertyDef;
pub use interface_builder::InterfaceBuilder;
pub use module::{Module, FfiModuleError};

// Re-export everything from angelscript-ffi crate
pub use angelscript_ffi::{
    // Error types
    ConversionError, NativeError,
    // Native function types
    CallContext, NativeCallable, NativeFn, ObjectHandle, ObjectHeap, VmSlot,
    // Conversion traits
    FromScript, IntoNativeFn, NativeType, ToScript,
    // List buffer support
    ListBuffer, ListPattern, TupleListBuffer,
    // Template support
    ListBehavior, TemplateInstanceInfo, TemplateValidation,
    // Any type support
    AnyRef, AnyRefMut,
    // FFI type definitions
    FfiEnumDef, FfiExpr, FfiExprExt, FfiFuncdefDef, FfiInterfaceDef, FfiInterfaceMethod,
    FfiPropertyDef, FfiTypeDef,
    // Convert utilities
    function_param_to_ffi, param_type_to_data_type, return_type_to_data_type,
    type_expr_to_data_type,
    // Registry
    FfiRegistry, FfiRegistryBuilder, FfiRegistryError,
    // Core types re-exported from FFI crate
    DataType, RefModifier, TypeHash, TypeKind,
};
