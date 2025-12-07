//! Semantic analysis module for AngelScript.
//!
//! This module provides semantic analysis functionality following a 2-pass model:
//! - Pass 1: Registration (register all global names in ScriptRegistry)
//! - Pass 2: Compilation & Codegen (type compilation + function compilation)
//!   - Pass 2a: Type Compilation (fill in type details, resolve TypeExpr → DataType)
//!   - Pass 2b: Function Compilation (type check function bodies, emit bytecode)
//!
//! # Quick Start
//!
//! For most use cases, use the unified `Compiler` interface:
//!
//! ```ignore
//! use angelscript::{parse_lenient, Compiler, FfiRegistryBuilder};
//! use bumpalo::Bump;
//! use std::sync::Arc;
//!
//! let arena = Bump::new();
//! let (script, _) = parse_lenient(source, &arena);
//! let ffi = Arc::new(FfiRegistryBuilder::new().build().unwrap());
//! let compiled = Compiler::compile(&script, ffi);
//!
//! if compiled.is_success() {
//!     // Use compiled.module, compiled.context, etc.
//! }
//! ```

pub mod compilation_context;
pub mod compiler;
pub mod const_eval;
pub mod error;
pub mod local_scope;
pub mod passes;
pub mod template_instantiator;
pub mod types;

// Unified compiler interface (recommended for most users)
pub use compiler::{CompilationResult, Compiler, TypeCompilationResult};

// Individual pass results (for advanced use cases)
pub use passes::{
    CompiledFunction, FunctionCompiler, RegistrationData, RegistrationDataWithContext,
    Registrar, TypeCompilationData, TypeCompiler,
};

// Re-export CompiledModule from codegen (it's the output of FunctionCompiler)
pub use crate::codegen::CompiledModule;

// Re-export core types from types module
pub use const_eval::{eval_const_int, ConstEvaluator, ConstValue};
pub use error::{SemanticError, SemanticErrorKind, SemanticErrors};
pub use local_scope::{CapturedVar, LocalScope, LocalVar};
pub use types::{
    Conversion, ConversionKind, DataType, DataTypeExt, FieldDef, FunctionDef, FunctionTraits,
    GlobalVarDef, MethodSignature, OperatorBehavior, PrimitiveType, PropertyAccessors,
    RefModifier, ScriptRegistry, TypeDef, Visibility,
};

// Re-export TypeHash from crate::types for backward compatibility
pub use angelscript_core::TypeHash;

// Re-export CompilationContext and FunctionRef
pub use compilation_context::{CompilationContext, FunctionRef};
