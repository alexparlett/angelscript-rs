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
//! - [`FfiRegistry`] - Immutable registry shared across all Units
//! - [`FfiRegistryBuilder`] - Builder for constructing FfiRegistry

mod ffi_data_type;
mod ffi_expr;
mod ffi_function;
mod ffi_registry;
mod type_kind;

pub use ffi_data_type::{FfiDataType, UnresolvedBaseType};
pub use ffi_expr::FfiExpr;
pub use ffi_function::{
    FfiFunctionDef, FfiParam, FfiResolutionError, ResolvedFfiFunctionDef, ResolvedFfiParam,
};
pub use ffi_registry::{FfiRegistry, FfiRegistryBuilder, FfiRegistryError};
pub use type_kind::{ReferenceKind, TypeKind};
