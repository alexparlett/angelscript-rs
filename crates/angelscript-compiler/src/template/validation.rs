//! Template validation callbacks.
//!
//! Provides support for template validation callbacks that can reject
//! invalid template instantiations (e.g., dict requires hashable keys).

use angelscript_core::{
    CompilationError, DataType, Span, TemplateInstanceInfo, TemplateValidation, TypeHash,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// A template validation callback function.
///
/// Returns `TemplateValidation` indicating whether the instantiation is valid.
pub type TemplateCallbackFn =
    Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>;

/// Registry of template validation callbacks.
#[derive(Default, Clone)]
pub struct TemplateCallbackRegistry {
    callbacks: FxHashMap<TypeHash, TemplateCallbackFn>,
}

impl TemplateCallbackRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a validation callback for a template.
    pub fn register(&mut self, template_hash: TypeHash, callback: TemplateCallbackFn) {
        self.callbacks.insert(template_hash, callback);
    }

    /// Check if a callback exists for a template.
    pub fn has_callback(&self, template_hash: TypeHash) -> bool {
        self.callbacks.contains_key(&template_hash)
    }

    /// Get the callback for a template.
    pub fn get(&self, template_hash: TypeHash) -> Option<&TemplateCallbackFn> {
        self.callbacks.get(&template_hash)
    }

    /// Validate a template instance.
    pub fn validate(
        &self,
        template_hash: TypeHash,
        info: &TemplateInstanceInfo,
    ) -> TemplateValidation {
        if let Some(callback) = self.callbacks.get(&template_hash) {
            callback(info)
        } else {
            TemplateValidation::valid()
        }
    }
}

impl std::fmt::Debug for TemplateCallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateCallbackRegistry")
            .field("callbacks", &format!("{} registered", self.callbacks.len()))
            .finish()
    }
}

/// Trait for types that can provide template callbacks.
pub trait TemplateCallback {
    /// Check if this has a callback for a template.
    fn has_template_callback(&self, template_hash: TypeHash) -> bool;

    /// Validate a template instance.
    fn validate_template_instance(
        &self,
        template_hash: TypeHash,
        info: &TemplateInstanceInfo,
    ) -> TemplateValidation;
}

/// Validate template instantiation via registered callback.
///
/// Returns Ok if no callback is registered or if validation passes.
/// Returns Err with TemplateValidationFailed if validation fails.
pub fn validate_template_instance<T: TemplateCallback>(
    callbacks: &T,
    template_hash: TypeHash,
    template_name: &str,
    type_args: &[DataType],
    span: Span,
) -> Result<(), CompilationError> {
    if !callbacks.has_template_callback(template_hash) {
        return Ok(());
    }

    let info = TemplateInstanceInfo::new(template_name.to_string(), type_args.to_vec());

    let validation = callbacks.validate_template_instance(template_hash, &info);

    if !validation.is_valid {
        return Err(CompilationError::TemplateValidationFailed {
            template: template_name.to_string(),
            message: validation.error.unwrap_or_else(|| "validation failed".to_string()),
            span,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn callback_registry_empty() {
        let registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("array");
        assert!(!registry.has_callback(template_hash));
    }

    #[test]
    fn callback_registry_register_and_get() {
        let mut registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("array");

        let callback: TemplateCallbackFn = Arc::new(|_| TemplateValidation::valid());
        registry.register(template_hash, callback);

        assert!(registry.has_callback(template_hash));
        assert!(registry.get(template_hash).is_some());
    }

    #[test]
    fn callback_registry_validate_no_callback() {
        let registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("array");
        let info = TemplateInstanceInfo::new("array", vec![DataType::simple(primitives::INT32)]);

        let result = registry.validate(template_hash, &info);
        assert!(result.is_valid);
    }

    #[test]
    fn callback_registry_validate_with_callback() {
        let mut registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("dict");

        // Callback that rejects void type arguments
        let callback: TemplateCallbackFn = Arc::new(|info| {
            for sub_type in &info.sub_types {
                if sub_type.type_hash == primitives::VOID {
                    return TemplateValidation::invalid("cannot use void as type argument");
                }
            }
            TemplateValidation::valid()
        });
        registry.register(template_hash, callback);

        // Valid instantiation
        let info_valid =
            TemplateInstanceInfo::new("dict", vec![DataType::simple(primitives::STRING)]);
        let result = registry.validate(template_hash, &info_valid);
        assert!(result.is_valid);

        // Invalid instantiation
        let info_invalid =
            TemplateInstanceInfo::new("dict", vec![DataType::simple(primitives::VOID)]);
        let result = registry.validate(template_hash, &info_invalid);
        assert!(!result.is_valid);
        assert!(result.error.unwrap().contains("void"));
    }

    struct MockCallbackProvider {
        registry: TemplateCallbackRegistry,
    }

    impl TemplateCallback for MockCallbackProvider {
        fn has_template_callback(&self, template_hash: TypeHash) -> bool {
            self.registry.has_callback(template_hash)
        }

        fn validate_template_instance(
            &self,
            template_hash: TypeHash,
            info: &TemplateInstanceInfo,
        ) -> TemplateValidation {
            self.registry.validate(template_hash, info)
        }
    }

    #[test]
    fn validate_template_instance_no_callback() {
        let provider = MockCallbackProvider {
            registry: TemplateCallbackRegistry::new(),
        };

        let result = validate_template_instance(
            &provider,
            TypeHash::from_name("array"),
            "array",
            &[DataType::simple(primitives::INT32)],
            Span::default(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn validate_template_instance_passes() {
        let mut registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("array");
        registry.register(template_hash, Arc::new(|_| TemplateValidation::valid()));

        let provider = MockCallbackProvider { registry };

        let result = validate_template_instance(
            &provider,
            template_hash,
            "array",
            &[DataType::simple(primitives::INT32)],
            Span::default(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn validate_template_instance_fails() {
        let mut registry = TemplateCallbackRegistry::new();
        let template_hash = TypeHash::from_name("array");
        registry.register(
            template_hash,
            Arc::new(|_| TemplateValidation::invalid("test failure")),
        );

        let provider = MockCallbackProvider { registry };

        let result = validate_template_instance(
            &provider,
            template_hash,
            "array",
            &[DataType::simple(primitives::INT32)],
            Span::default(),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TemplateValidationFailed {
                template, message, ..
            } => {
                assert_eq!(template, "array");
                assert!(message.contains("test failure"));
            }
            e => panic!("Expected TemplateValidationFailed, got {:?}", e),
        }
    }
}
