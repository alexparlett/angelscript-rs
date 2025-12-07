//! Type system for AngelScript.
//!
//! This module contains all type-related functionality:
//! - Type definitions (TypeDef)
//! - Runtime data types (DataType)
//! - Script registry (ScriptRegistry) - for script-defined types only
//! - Type conversions (Conversion, ConversionKind)
//! - Type behaviors (TypeBehaviors)
//!
//! FFI types (including primitives) are stored in `FfiRegistry` and accessed
//! via `CompilationContext` which provides a unified lookup interface.
//!
//! ## Type Identity
//!
//! Types are identified by `TypeHash` - a deterministic 64-bit hash computed
//! from the type's qualified name. Primitive type hashes are available in
//! `crate::types::primitive_hashes`.

pub mod behaviors;
pub mod conversion;
pub mod data_type;
pub mod registry;
pub mod type_def;

// Re-export key types for ergonomic use
pub use behaviors::TypeBehaviors;
pub use conversion::{Conversion, ConversionKind};
pub use data_type::{DataType, RefModifier};
pub use registry::{FunctionDef, GlobalVarDef, ScriptParam, ScriptRegistry};
pub use type_def::{
    FieldDef, FunctionTraits, MethodSignature, OperatorBehavior, PrimitiveType,
    PropertyAccessors, TypeDef, Visibility,
};
