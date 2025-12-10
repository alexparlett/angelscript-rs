//! Registry entry types.
//!
//! This module provides the entry types used in the unified type registry:
//!
//! - [`TypeEntry`] - Unified enum wrapping all type entries
//! - [`ClassEntry`] - Class types (including templates)
//! - [`EnumEntry`] - Enumeration types
//! - [`InterfaceEntry`] - Interface types
//! - [`FuncdefEntry`] - Function pointer types
//! - [`PrimitiveEntry`] - Built-in primitive types
//! - [`TemplateParamEntry`] - Template type parameters
//! - [`FunctionEntry`] - Function entries with implementation
//!
//! Supporting types:
//! - [`TypeSource`], [`FunctionSource`] - Origin tracking (FFI vs script)
//! - [`PropertyEntry`], [`EnumValue`] - Member types

mod class;
mod common;
mod enum_entry;
mod funcdef;
mod function;
mod global_property;
mod interface;
mod primitive;
mod source;
mod template_param;
mod type_entry;

// Source tracking
pub use source::{FunctionSource, TypeSource};

// Common member types
pub use common::{EnumValue, PropertyEntry};

// Individual entry types
pub use class::ClassEntry;
pub use enum_entry::EnumEntry;
pub use funcdef::FuncdefEntry;
pub use function::{FunctionEntry, FunctionImpl};
pub use interface::InterfaceEntry;
pub use primitive::PrimitiveEntry;
pub use template_param::TemplateParamEntry;

// Unified type entry
pub use type_entry::TypeEntry;

// Global property types
pub use global_property::{
    ConstantValue, GlobalPropertyAccessor, GlobalPropertyEntry, GlobalPropertyImpl,
    IntoGlobalProperty, PropertyError,
};
