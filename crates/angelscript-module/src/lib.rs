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

// Re-export local builders
pub use class_builder::ClassBuilder;
pub use enum_builder::EnumBuilder;
pub use function_builder::FunctionBuilder;
pub use global_property::GlobalPropertyBuilder;
pub use interface_builder::InterfaceBuilder;
pub use module::{Module, ModuleError};