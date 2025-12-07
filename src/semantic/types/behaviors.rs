//! Type behaviors - lifecycle and initialization functions for types.
//!
//! This module defines `TypeBehaviors`, which stores FunctionIds for various
//! lifecycle behaviors like construction, destruction, and initialization lists.
//! Behaviors are stored centrally in the Registry, indexed by TypeHash.

use crate::types::TypeHash;

/// Lifecycle and initialization behaviors for a type.
///
/// Stored centrally in `Registry::behaviors`, indexed by `TypeHash`.
/// This follows the C++ AngelScript pattern where behaviors are registered
/// separately from the type definition itself.
///
/// # List Initialization
///
/// Types can support initialization list syntax in two ways:
/// - `list_construct`: For value types, constructs in place (e.g., `MyVec3 v = {1.0, 2.0, 3.0}`)
/// - `list_factory`: For reference types, returns a new handle (e.g., `array<int> a = {1, 2, 3}`)
///
/// # Example
///
/// ```ignore
/// // In the compiler, look up behaviors for a type:
/// if let Some(behaviors) = registry.get_behaviors(array_type_id) {
///     if let Some(factory_id) = behaviors.list_factory {
///         // Use the list factory to construct from init list
///     }
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeBehaviors {
    // === Object Lifecycle (multiple overloads allowed) ===
    /// Constructors - initialize in pre-allocated memory (for value types)
    /// Multiple overloads supported: default(), from int, from string, etc.
    /// Corresponds to asBEHAVE_CONSTRUCT
    pub constructors: Vec<TypeHash>,

    /// Factories - allocate and return new instance (for reference types)
    /// Multiple overloads supported: default(), from int, with capacity, etc.
    /// Corresponds to asBEHAVE_FACTORY
    pub factories: Vec<TypeHash>,

    // === Single behaviors (no overloads) ===
    /// Destructor - cleans up before deallocation
    /// Corresponds to asBEHAVE_DESTRUCT
    pub destruct: Option<TypeHash>,

    // === Reference Counting ===
    /// AddRef - increments reference count (for reference types)
    /// Corresponds to asBEHAVE_ADDREF
    pub addref: Option<TypeHash>,

    /// Release - decrements reference count (for reference types)
    /// Corresponds to asBEHAVE_RELEASE
    pub release: Option<TypeHash>,

    // === List Initialization ===
    /// List construct - for value types with init list syntax
    /// Used for types like `MyStruct s = {1, 2, 3}` where the value is constructed in place
    /// Corresponds to asBEHAVE_LIST_CONSTRUCT
    pub list_construct: Option<TypeHash>,

    /// List factory - for reference types with init list syntax
    /// Used for types like `array<int> a = {1, 2, 3}` where a handle is returned
    /// Corresponds to asBEHAVE_LIST_FACTORY
    pub list_factory: Option<TypeHash>,

    // === Weak References ===
    /// Get weak reference flag - returns a shared weak ref flag object
    /// Corresponds to asBEHAVE_GET_WEAKREF_FLAG
    pub get_weakref_flag: Option<TypeHash>,

    // === Template Support ===
    /// Template callback - validates template instantiation
    /// Corresponds to asBEHAVE_TEMPLATE_CALLBACK
    pub template_callback: Option<TypeHash>,
}

impl TypeBehaviors {
    /// Create a new empty TypeBehaviors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any behaviors are defined.
    pub fn is_empty(&self) -> bool {
        self.constructors.is_empty()
            && self.factories.is_empty()
            && self.destruct.is_none()
            && self.addref.is_none()
            && self.release.is_none()
            && self.list_construct.is_none()
            && self.list_factory.is_none()
            && self.get_weakref_flag.is_none()
            && self.template_callback.is_none()
    }

    /// Check if this type has any constructors.
    pub fn has_constructors(&self) -> bool {
        !self.constructors.is_empty()
    }

    /// Check if this type has any factories.
    pub fn has_factories(&self) -> bool {
        !self.factories.is_empty()
    }

    /// Check if this type has list initialization support.
    pub fn has_list_init(&self) -> bool {
        self.list_construct.is_some() || self.list_factory.is_some()
    }

    /// Check if this type has addref behavior.
    pub fn has_addref(&self) -> bool {
        self.addref.is_some()
    }

    /// Check if this type has release behavior.
    pub fn has_release(&self) -> bool {
        self.release.is_some()
    }

    /// Check if this type has list_factory behavior.
    pub fn has_list_factory(&self) -> bool {
        self.list_factory.is_some()
    }

    /// Get the list initialization function, preferring list_factory over list_construct.
    /// This is the function to call when encountering an init list for this type.
    pub fn list_init_func(&self) -> Option<TypeHash> {
        self.list_factory.or(self.list_construct)
    }

    /// Add a constructor to this type's behaviors.
    pub fn add_constructor(&mut self, func_id: TypeHash) {
        self.constructors.push(func_id);
    }

    /// Add a factory to this type's behaviors.
    pub fn add_factory(&mut self, func_id: TypeHash) {
        self.factories.push(func_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_behaviors_default_is_empty() {
        let behaviors = TypeBehaviors::new();
        assert!(behaviors.is_empty());
        assert!(!behaviors.has_list_init());
        assert!(!behaviors.has_constructors());
        assert!(!behaviors.has_factories());
        assert!(behaviors.list_init_func().is_none());
    }

    #[test]
    fn type_behaviors_with_list_factory() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.list_factory = Some(TypeHash(42));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert!(behaviors.has_list_factory());
        assert_eq!(behaviors.list_init_func(), Some(TypeHash(42)));
    }

    #[test]
    fn type_behaviors_with_list_construct() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.list_construct = Some(TypeHash(100));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert_eq!(behaviors.list_init_func(), Some(TypeHash(100)));
    }

    #[test]
    fn type_behaviors_list_factory_preferred_over_construct() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.list_construct = Some(TypeHash(100));
        behaviors.list_factory = Some(TypeHash(42));

        // list_factory should be preferred
        assert_eq!(behaviors.list_init_func(), Some(TypeHash(42)));
    }

    #[test]
    fn type_behaviors_with_constructors() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_constructor(TypeHash(1));
        behaviors.add_constructor(TypeHash(2));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_constructors());
        assert_eq!(behaviors.constructors.len(), 2);
    }

    #[test]
    fn type_behaviors_with_factories() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_factory(TypeHash(10));
        behaviors.add_factory(TypeHash(20));
        behaviors.add_factory(TypeHash(30));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_factories());
        assert_eq!(behaviors.factories.len(), 3);
    }

    #[test]
    fn type_behaviors_with_lifecycle() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_constructor(TypeHash(1));
        behaviors.destruct = Some(TypeHash(2));
        behaviors.addref = Some(TypeHash(3));
        behaviors.release = Some(TypeHash(4));

        assert!(!behaviors.is_empty());
        assert!(!behaviors.has_list_init());
        assert!(behaviors.has_constructors());
        assert!(behaviors.has_addref());
        assert!(behaviors.has_release());
    }
}
