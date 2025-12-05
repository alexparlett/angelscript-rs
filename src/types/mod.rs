//! Common type definitions shared across the crate.
//!
//! This module contains core type definitions used by both the FFI layer
//! and the semantic analysis layer.

mod ffi_data_type;
mod type_kind;

pub use ffi_data_type::{FfiDataType, UnresolvedBaseType};
pub use type_kind::{ReferenceKind, TypeKind};
