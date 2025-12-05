//! Common type definitions shared across the crate.
//!
//! This module contains core type definitions used by both the FFI layer
//! and the semantic analysis layer.
//!
//! # Types for FFI Registry
//!
//! These types enable the two-tier registry architecture where FFI types
//! are registered once and shared across all compilation Units:
//!
//! - [`FfiDataType`] - Deferred type resolution for cross-type references
//! - [`FfiExpr`] - Owned expressions for default argument values
//! - [`FfiParam`] - Function parameter with deferred types
//! - [`FfiFunctionDef`] - Owned function definition for FFI registry
//! - [`FfiPropertyDef`] - Owned property definition for FFI registry
//! - [`FfiTypeDef`] - Owned type (class) definition for FFI registry
//! - [`FfiEnumDef`] - Owned enum definition for FFI registry
//! - [`FfiInterfaceDef`] - Owned interface definition for FFI registry
//! - [`FfiFuncdefDef`] - Owned funcdef (callback type) definition for FFI registry

mod ffi_convert;
mod ffi_data_type;
mod ffi_enum;
mod ffi_expr;
mod ffi_funcdef;
mod ffi_function;
mod ffi_interface;
mod ffi_property;
mod ffi_type;
mod type_kind;

pub use ffi_convert::{
    function_param_to_ffi, param_type_to_ffi, return_type_to_ffi, signature_to_ffi_function,
    type_expr_to_ffi,
};
pub use ffi_data_type::{FfiDataType, UnresolvedBaseType};
pub use ffi_enum::FfiEnumDef;
pub use ffi_expr::FfiExpr;
pub use ffi_funcdef::FfiFuncdefDef;
pub use ffi_function::{
    FfiFunctionDef, FfiParam, FfiResolutionError, ResolvedFfiFunctionDef, ResolvedFfiParam,
};
pub use ffi_interface::{FfiInterfaceDef, FfiInterfaceMethod};
pub use ffi_property::FfiPropertyDef;
pub use ffi_type::FfiTypeDef;
pub use type_kind::{ReferenceKind, TypeKind};
