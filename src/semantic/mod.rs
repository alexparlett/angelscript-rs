//! Semantic analysis module for AngelScript.
//!
//! This module provides semantic analysis functionality following a 2-pass model:
//! - Pass 1: Registration (register all global names in Registry)
//! - Pass 2: Compilation & Codegen (type compilation + function compilation)
//!   - Pass 2a: Type Compilation (fill in type details, resolve TypeExpr → DataType)
//!   - Pass 2b: Function Compilation (type check function bodies, emit bytecode)

pub mod bytecode;
pub mod data_type;
pub mod error;
pub mod function_compiler;
pub mod local_scope;
pub mod registrar;
pub mod registry;
pub mod type_compiler;
pub mod type_def;

pub use bytecode::{BytecodeEmitter, CompiledBytecode, Instruction};
pub use data_type::DataType;
pub use error::{SemanticError, SemanticErrorKind, SemanticErrors};
pub use function_compiler::{CompiledFunction, FunctionCompiler};
pub use local_scope::{LocalScope, LocalVar};
pub use registrar::{RegistrationData, Registrar};
pub use registry::{FunctionDef, GlobalVarDef, Registry};
pub use type_compiler::{TypeCompilationData, TypeCompiler};
pub use type_def::{
    FieldDef, FunctionId, FunctionTraits, MethodSignature, PrimitiveType, TypeDef, TypeId,
    Visibility, ARRAY_TEMPLATE, BOOL_TYPE, DICT_TEMPLATE, DOUBLE_TYPE, FIRST_USER_TYPE_ID,
    FLOAT_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE, INT8_TYPE, STRING_TYPE, UINT16_TYPE,
    UINT32_TYPE, UINT64_TYPE, UINT8_TYPE, VOID_TYPE,
};
