//! Template instance cache.
//!
//! Caches template instances to avoid duplicate instantiation and
//! to respect FFI specializations.

use angelscript_core::TypeHash;
use rustc_hash::FxHashMap;

/// Cache for template instances.
///
/// Maps (template_hash, type_args) → instance_hash.
/// Pre-populated with FFI specializations during context setup.
#[derive(Debug, Default, Clone)]
pub struct TemplateInstanceCache {
    /// Type template instances: (template, args) → instance
    type_instances: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
    /// Function template instances: (template, args) → instance
    function_instances: FxHashMap<(TypeHash, Vec<TypeHash>), TypeHash>,
}

impl TemplateInstanceCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a type template instance.
    /// Called during FFI registration for specializations and during compilation.
    pub fn cache_type_instance(
        &mut self,
        template: TypeHash,
        args: Vec<TypeHash>,
        instance: TypeHash,
    ) {
        self.type_instances.insert((template, args), instance);
    }

    /// Look up a cached type instance.
    pub fn get_type_instance(&self, template: TypeHash, args: &[TypeHash]) -> Option<TypeHash> {
        self.type_instances.get(&(template, args.to_vec())).copied()
    }

    /// Register a function template instance.
    pub fn cache_function_instance(
        &mut self,
        template: TypeHash,
        args: Vec<TypeHash>,
        instance: TypeHash,
    ) {
        self.function_instances.insert((template, args), instance);
    }

    /// Look up a cached function instance.
    pub fn get_function_instance(&self, template: TypeHash, args: &[TypeHash]) -> Option<TypeHash> {
        self.function_instances
            .get(&(template, args.to_vec()))
            .copied()
    }

    /// Check if a type instance exists (FFI specialization or previous instantiation).
    pub fn has_type_instance(&self, template: TypeHash, args: &[TypeHash]) -> bool {
        self.type_instances.contains_key(&(template, args.to_vec()))
    }

    /// Check if a function instance exists.
    pub fn has_function_instance(&self, template: TypeHash, args: &[TypeHash]) -> bool {
        self.function_instances
            .contains_key(&(template, args.to_vec()))
    }

    /// Get the number of cached type instances.
    pub fn type_instance_count(&self) -> usize {
        self.type_instances.len()
    }

    /// Get the number of cached function instances.
    pub fn function_instance_count(&self) -> usize {
        self.function_instances.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::primitives;

    #[test]
    fn cache_new_is_empty() {
        let cache = TemplateInstanceCache::new();
        assert_eq!(cache.type_instance_count(), 0);
        assert_eq!(cache.function_instance_count(), 0);
    }

    #[test]
    fn cache_type_instance() {
        let mut cache = TemplateInstanceCache::new();

        let array_template = TypeHash::from_name("array");
        let array_int = TypeHash::from_template_instance(array_template, &[primitives::INT32]);

        cache.cache_type_instance(array_template, vec![primitives::INT32], array_int);

        assert!(cache.has_type_instance(array_template, &[primitives::INT32]));
        assert_eq!(
            cache.get_type_instance(array_template, &[primitives::INT32]),
            Some(array_int)
        );
        assert_eq!(cache.type_instance_count(), 1);
    }

    #[test]
    fn cache_type_instance_different_args() {
        let mut cache = TemplateInstanceCache::new();

        let array_template = TypeHash::from_name("array");
        let array_int = TypeHash::from_template_instance(array_template, &[primitives::INT32]);
        let array_float = TypeHash::from_template_instance(array_template, &[primitives::DOUBLE]);

        cache.cache_type_instance(array_template, vec![primitives::INT32], array_int);
        cache.cache_type_instance(array_template, vec![primitives::DOUBLE], array_float);

        assert_eq!(
            cache.get_type_instance(array_template, &[primitives::INT32]),
            Some(array_int)
        );
        assert_eq!(
            cache.get_type_instance(array_template, &[primitives::DOUBLE]),
            Some(array_float)
        );
        assert_eq!(cache.type_instance_count(), 2);
    }

    #[test]
    fn cache_function_instance() {
        let mut cache = TemplateInstanceCache::new();

        let identity_template = TypeHash::from_name("identity");
        let identity_int =
            TypeHash::from_template_instance(identity_template, &[primitives::INT32]);

        cache.cache_function_instance(identity_template, vec![primitives::INT32], identity_int);

        assert!(cache.has_function_instance(identity_template, &[primitives::INT32]));
        assert_eq!(
            cache.get_function_instance(identity_template, &[primitives::INT32]),
            Some(identity_int)
        );
        assert_eq!(cache.function_instance_count(), 1);
    }

    #[test]
    fn cache_miss_returns_none() {
        let cache = TemplateInstanceCache::new();

        let array_template = TypeHash::from_name("array");

        assert!(!cache.has_type_instance(array_template, &[primitives::INT32]));
        assert_eq!(
            cache.get_type_instance(array_template, &[primitives::INT32]),
            None
        );
    }

    #[test]
    fn cache_multi_arg_template() {
        let mut cache = TemplateInstanceCache::new();

        let dict_template = TypeHash::from_name("dict");
        let dict_string_int = TypeHash::from_template_instance(
            dict_template,
            &[primitives::STRING, primitives::INT32],
        );

        cache.cache_type_instance(
            dict_template,
            vec![primitives::STRING, primitives::INT32],
            dict_string_int,
        );

        assert!(cache.has_type_instance(dict_template, &[primitives::STRING, primitives::INT32]));
        // Different order should not match
        assert!(!cache.has_type_instance(dict_template, &[primitives::INT32, primitives::STRING]));
    }
}
