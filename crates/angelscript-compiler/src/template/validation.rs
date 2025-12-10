//! Template validation support.
//!
//! Template validation uses the existing `template_callback` behavior on `TypeBehaviors`.
//! The callback is a function registered in the registry that validates instantiation.

use angelscript_core::{
    CompilationError, DataType, Span, TemplateInstanceInfo, TemplateValidation, TypeHash,
};

/// Trait for types that can provide template validation.
///
/// This is implemented by registries/contexts that can look up template callbacks
/// and invoke them.
pub trait TemplateCallback {
    /// Check if a template has a validation callback registered.
    fn has_template_callback(&self, template_hash: TypeHash) -> bool;

    /// Validate a template instance via the registered callback.
    ///
    /// Returns `TemplateValidation::valid()` if no callback is registered.
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

    struct MockCallbackProvider {
        has_callback: bool,
        validation_result: TemplateValidation,
    }

    impl TemplateCallback for MockCallbackProvider {
        fn has_template_callback(&self, _: TypeHash) -> bool {
            self.has_callback
        }

        fn validate_template_instance(
            &self,
            _: TypeHash,
            _: &TemplateInstanceInfo,
        ) -> TemplateValidation {
            self.validation_result.clone()
        }
    }

    #[test]
    fn validate_template_instance_no_callback() {
        let provider = MockCallbackProvider {
            has_callback: false,
            validation_result: TemplateValidation::valid(),
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
        let provider = MockCallbackProvider {
            has_callback: true,
            validation_result: TemplateValidation::valid(),
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
    fn validate_template_instance_fails() {
        let provider = MockCallbackProvider {
            has_callback: true,
            validation_result: TemplateValidation::invalid("test failure"),
        };

        let result = validate_template_instance(
            &provider,
            TypeHash::from_name("array"),
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
