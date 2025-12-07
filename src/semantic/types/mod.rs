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
    FieldDef, FunctionId, FunctionTraits, MethodSignature, OperatorBehavior, PrimitiveType,
    PropertyAccessors, TypeDef, TypeId, Visibility, BOOL_TYPE,
    DOUBLE_TYPE, FFI_BIT, FIRST_USER_TYPE_ID, FLOAT_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE,
    INT8_TYPE, NULL_TYPE, SELF_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE, UINT8_TYPE,
    VARIABLE_PARAM_TYPE, VOID_TYPE,
};
