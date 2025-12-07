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
//! - [`PropertyEntry`], [`FieldEntry`], [`EnumValue`] - Member types

mod source;
mod common;
mod primitive;
mod template_param;
mod enum_entry;
mod interface;
mod funcdef;
mod class;
mod function;
mod type_entry;

// Source tracking
pub use source::{FunctionSource, TypeSource};

// Common member types
pub use common::{EnumValue, FieldEntry, PropertyEntry};

// Individual entry types
pub use primitive::PrimitiveEntry;
pub use template_param::TemplateParamEntry;
pub use enum_entry::EnumEntry;
pub use interface::InterfaceEntry;
pub use funcdef::FuncdefEntry;
pub use class::ClassEntry;
pub use function::{FunctionEntry, FunctionImpl};

// Unified type entry
pub use type_entry::TypeEntry;
