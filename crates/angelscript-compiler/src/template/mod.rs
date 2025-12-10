//! Template instantiation system.
//!
//! Provides template instantiation for types and functions, with caching
//! to avoid duplicate instantiation and support for FFI specializations.
//!
//! ## Components
//!
//! - [`TemplateInstanceCache`]: Cache for template instances
//! - [`SubstitutionMap`]: Maps template parameters to concrete types
//! - [`instantiate_template_type`]: Instantiate a template type
//! - [`instantiate_template_function`]: Instantiate a template function
//! - [`validate_template_instance`]: Validate via registered callback

mod cache;
mod instantiation;
mod substitution;
mod validation;

pub use cache::TemplateInstanceCache;
pub use instantiation::{
    format_template_instance_name, format_type_args, instantiate_child_funcdef,
    instantiate_template_function, instantiate_template_type,
};
pub use substitution::{
    build_substitution_map, substitute_params, substitute_type, SubstitutionMap,
};
pub use validation::{validate_template_instance, TemplateCallback};
