//! Compiler passes for the two-pass compilation model.
//!
//! ## Architecture
//!
//! - **Pass 1 (Registration)**: Walk AST and register all type and function declarations
//!   into the registry. Collects signatures without compiling function bodies.
//!   Inheritance references are collected but not resolved (enabling forward references).
//! - **Pass 1b (Type Completion)**: Resolve pending inheritance references, then copy
//!   inherited members from base classes to enable O(1) lookups.
//! - **Pass 2 (Compilation)**: Type check function bodies and generate bytecode.
//!
//! ## Module Structure
//!
//! - [`registration`]: Pass 1 implementation
//! - [`completion`]: Pass 1b implementation
//! - [`compilation`]: Pass 2 implementation
//!
//! ## Usage (Orchestration)
//!
//! ```ignore
//! use angelscript_compiler::{
//!     CompilationContext, RegistrationPass, TypeCompletionPass, CompilationPass
//! };
//!
//! // Phase 1: Create context
//! let mut ctx = CompilationContext::new(&global_registry);
//!
//! // Phase 2: Run registration pass (collects types, doesn't resolve inheritance)
//! let reg_pass = RegistrationPass::new(&mut ctx, unit_id);
//! let reg_output = reg_pass.run(&script);
//!
//! // Phase 3: Run completion pass (resolves inheritance, copies members)
//! // Uses try-combinations resolution - no scope rebuilding needed
//! let unit_registry = ctx.take_unit_registry();
//! let comp_pass = TypeCompletionPass::new(&mut unit_registry, reg_output.pending_resolutions);
//! let comp_output = comp_pass.run();
//!
//! // Phase 4: Compile function bodies to bytecode
//! let compile_pass = CompilationPass::new(&mut ctx, unit_id);
//! let compile_output = compile_pass.run(&script);
//! ```

pub mod compilation;
pub mod completion;
pub mod registration;

use angelscript_core::{Span, TypeHash};
use rustc_hash::FxHashMap;

pub use compilation::{CompilationOutput, CompilationPass, CompiledFunctionEntry, GlobalInitEntry};
pub use completion::{CompletionOutput, TypeCompletionPass};
pub use registration::{RegistrationOutput, RegistrationPass};

// Re-export PendingResolutions for orchestration (pass output from Pass 1 to Pass 1b)

/// Unresolved inheritance reference from AST.
///
/// Collected during Pass 1 and resolved in Pass 1b. This enables forward references
/// where a class can inherit from a type declared later in the source.
#[derive(Debug, Clone)]
pub struct PendingInheritance {
    /// Raw name from source (e.g., "Base", "Foo::Bar")
    pub name: String,
    /// Source location for error reporting
    pub span: Span,
    /// Current namespace when parsed (e.g., ["Game", "Entities"])
    pub namespace_context: Vec<String>,
    /// Active imports when parsed (e.g., ["Utils", "Math"])
    pub imports: Vec<String>,
}

/// Pending resolutions collected during Pass 1, consumed by Pass 1b.
///
/// This struct holds all unresolved type references that need namespace-aware
/// resolution after all types have been registered.
#[derive(Debug, Default)]
pub struct PendingResolutions {
    /// Class inheritance: class_hash -> pending bases/mixins/interfaces
    pub class_inheritance: FxHashMap<TypeHash, Vec<PendingInheritance>>,
    /// Interface inheritance: interface_hash -> pending base interfaces
    pub interface_bases: FxHashMap<TypeHash, Vec<PendingInheritance>>,
}
