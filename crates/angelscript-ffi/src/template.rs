//! Core types for FFI registration.
//!
//! These types are used during registration to specify type information.
//! Type specifications use AST primitives parsed from declaration strings.
//!
//! IDs are assigned at registration time using the global atomic counters
//! (`TypeHash::from_name("test_type")` and `TypeHash::from_name("test_func")` for FFI types/functions).

use angelscript_core::DataType;

use super::list_buffer::ListPattern;
use super::native_fn::NativeFn;

/// List construction behavior with its pattern.
///
/// Used by `list_construct` and `list_factory` to define how initialization
/// lists are processed.
#[derive(Debug)]
pub struct ListBehavior {
    /// Native function to call with the list data
    pub native_fn: NativeFn,
    /// Expected list pattern (repeat, fixed, or repeat-tuple)
    pub pattern: ListPattern,
}

/// Information about a template instantiation for validation callback.
#[derive(Debug, Clone)]
pub struct TemplateInstanceInfo {
    /// The template name (e.g., "array")
    pub template_name: String,
    /// The type arguments (e.g., [int] for array<int>)
    pub sub_types: Vec<DataType>,
}

impl TemplateInstanceInfo {
    /// Create a new template instance info.
    pub fn new(template_name: impl Into<String>, sub_types: Vec<DataType>) -> Self {
        Self {
            template_name: template_name.into(),
            sub_types,
        }
    }
}

/// Result of template validation callback.
#[derive(Debug, Clone)]
pub struct TemplateValidation {
    /// Is this instantiation valid?
    pub is_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Should this instance use garbage collection?
    pub needs_gc: bool,
}

impl TemplateValidation {
    /// Create a valid template validation result.
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error: None,
            needs_gc: false,
        }
    }

    /// Create an invalid template validation result with an error message.
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error: Some(msg.into()),
            needs_gc: false,
        }
    }

    /// Create a valid result that needs garbage collection.
    pub fn with_gc() -> Self {
        Self {
            is_valid: true,
            error: None,
            needs_gc: true,
        }
    }
}

impl Default for TemplateValidation {
    fn default() -> Self {
        Self::valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_instance_info_new() {
        let info = TemplateInstanceInfo::new("array", vec![]);
        assert_eq!(info.template_name, "array");
        assert!(info.sub_types.is_empty());
    }

    #[test]
    fn template_validation_valid() {
        let v = TemplateValidation::valid();
        assert!(v.is_valid);
        assert!(v.error.is_none());
        assert!(!v.needs_gc);
    }

    #[test]
    fn template_validation_invalid() {
        let v = TemplateValidation::invalid("Key must be hashable");
        assert!(!v.is_valid);
        assert_eq!(v.error, Some("Key must be hashable".to_string()));
        assert!(!v.needs_gc);
    }

    #[test]
    fn template_validation_with_gc() {
        let v = TemplateValidation::with_gc();
        assert!(v.is_valid);
        assert!(v.error.is_none());
        assert!(v.needs_gc);
    }

    #[test]
    fn template_validation_default() {
        let v = TemplateValidation::default();
        assert!(v.is_valid);
    }
}
