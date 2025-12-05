//! Semantic analysis module for AngelScript.
//!
//! This module provides semantic analysis functionality following a 2-pass model:
//! - Pass 1: Registration (register all global names in Registry)
//! - Pass 2: Compilation & Codegen (type compilation + function compilation)
//!   - Pass 2a: Type Compilation (fill in type details, resolve TypeExpr → DataType)
//!   - Pass 2b: Function Compilation (type check function bodies, emit bytecode)
//!
//! # Quick Start
//!
//! For most use cases, use the unified `Compiler` interface:
//!
//! ```ignore
//! use angelscript::{parse_lenient, Compiler};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let (script, _) = parse_lenient(source, &arena);
//! let compiled = Compiler::compile(&script);
//!
//! if compiled.is_success() {
//!     // Use compiled.module, compiled.registry, etc.
//! }
//! ```

pub mod compiler;
pub mod const_eval;
pub mod error;
pub mod local_scope;
pub mod passes;
pub mod types;

// Unified compiler interface (recommended for most users)
pub use compiler::{CompilationResult, Compiler, TypeCompilationResult};

// Individual pass results (for advanced use cases)
pub use passes::{
    CompiledFunction, FunctionCompiler, RegistrationData, Registrar, TypeCompilationData,
    TypeCompiler,
};

// Re-export CompiledModule from codegen (it's the output of FunctionCompiler)
pub use crate::codegen::CompiledModule;

// Re-export core types from types module
pub use const_eval::{eval_const_int, ConstEvaluator, ConstValue};
pub use error::{SemanticError, SemanticErrorKind, SemanticErrors};
pub use local_scope::{CapturedVar, LocalScope, LocalVar};
pub use types::{
    Conversion, ConversionKind, DataType, FieldDef, FunctionDef, FunctionId, FunctionTraits,
    GlobalVarDef, MethodSignature, OperatorBehavior, PrimitiveType, PropertyAccessors,
    RefModifier, ScriptRegistry, TypeDef, TypeId, Visibility, BOOL_TYPE,
    DOUBLE_TYPE, FIRST_USER_TYPE_ID, FLOAT_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE, INT8_TYPE,
    NULL_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE, UINT8_TYPE, VOID_TYPE,
};

// Re-export Registry as alias for backwards compatibility during transition
pub use types::ScriptRegistry as Registry;
