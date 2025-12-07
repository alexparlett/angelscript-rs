//! TypeBehaviors - lifecycle and initialization functions for types.
//!
//! This module defines `TypeBehaviors`, which stores function hashes for various
//! lifecycle behaviors like construction, destruction, and initialization lists.
//! Behaviors are stored centrally in the Registry, indexed by TypeHash.

use crate::TypeHash;

/// Lifecycle and initialization behaviors for a type.
///
/// Stored centrally in `ScriptRegistry::behaviors`, indexed by `TypeHash`.
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
/// ```
/// use angelscript_core::{TypeBehaviors, TypeHash};
///
/// let mut behaviors = TypeBehaviors::new();
/// behaviors.add_constructor(TypeHash::from_name("MyClass::MyClass"));
/// behaviors.add_constructor(TypeHash::from_name("MyClass::MyClass(int)"));
///
/// assert!(behaviors.has_constructors());
/// assert_eq!(behaviors.constructors.len(), 2);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeBehaviors {
    // === Object Lifecycle (multiple overloads allowed) ===
    /// Constructors - initialize in pre-allocated memory (for value types).
    /// Multiple overloads supported: default(), from int, from string, etc.
    /// Corresponds to asBEHAVE_CONSTRUCT
    pub constructors: Vec<TypeHash>,

    /// Factories - allocate and return new instance (for reference types).
    /// Multiple overloads supported: default(), from int, with capacity, etc.
    /// Corresponds to asBEHAVE_FACTORY
    pub factories: Vec<TypeHash>,

    // === Single behaviors (no overloads) ===
    /// Destructor - cleans up before deallocation.
    /// Corresponds to asBEHAVE_DESTRUCT
    pub destructor: Option<TypeHash>,

    // === Reference Counting ===
    /// AddRef - increments reference count (for reference types).
    /// Corresponds to asBEHAVE_ADDREF
    pub addref: Option<TypeHash>,

    /// Release - decrements reference count (for reference types).
    /// Corresponds to asBEHAVE_RELEASE
    pub release: Option<TypeHash>,

    // === List Initialization ===
    /// List construct - for value types with init list syntax.
    /// Used for types like `MyStruct s = {1, 2, 3}` where the value is constructed in place.
    /// Corresponds to asBEHAVE_LIST_CONSTRUCT
    pub list_construct: Option<TypeHash>,

    /// List factory - for reference types with init list syntax.
    /// Used for types like `array<int> a = {1, 2, 3}` where a handle is returned.
    /// Corresponds to asBEHAVE_LIST_FACTORY
    pub list_factory: Option<TypeHash>,

    // === Weak References ===
    /// Get weak reference flag - returns a shared weak ref flag object.
    /// Corresponds to asBEHAVE_GET_WEAKREF_FLAG
    pub get_weakref_flag: Option<TypeHash>,

    // === Template Support ===
    /// Template callback - validates template instantiation.
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
            && self.destructor.is_none()
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

    /// Check if this type has a destructor.
    pub fn has_destructor(&self) -> bool {
        self.destructor.is_some()
    }

    /// Check if this type has addref behavior.
    pub fn has_addref(&self) -> bool {
        self.addref.is_some()
    }

    /// Check if this type has release behavior.
    pub fn has_release(&self) -> bool {
        self.release.is_some()
    }

    /// Check if this type has reference counting (both addref and release).
    pub fn has_ref_counting(&self) -> bool {
        self.addref.is_some() && self.release.is_some()
    }

    /// Check if this type has list_factory behavior.
    pub fn has_list_factory(&self) -> bool {
        self.list_factory.is_some()
    }

    /// Check if this type has list_construct behavior.
    pub fn has_list_construct(&self) -> bool {
        self.list_construct.is_some()
    }

    /// Get the list initialization function, preferring list_factory over list_construct.
    /// This is the function to call when encountering an init list for this type.
    pub fn list_init_func(&self) -> Option<TypeHash> {
        self.list_factory.or(self.list_construct)
    }

    /// Add a constructor to this type's behaviors.
    pub fn add_constructor(&mut self, func_hash: TypeHash) {
        self.constructors.push(func_hash);
    }

    /// Add a factory to this type's behaviors.
    pub fn add_factory(&mut self, func_hash: TypeHash) {
        self.factories.push(func_hash);
    }

    /// Set the destructor for this type.
    pub fn set_destructor(&mut self, func_hash: TypeHash) {
        self.destructor = Some(func_hash);
    }

    /// Set the addref behavior for this type.
    pub fn set_addref(&mut self, func_hash: TypeHash) {
        self.addref = Some(func_hash);
    }

    /// Set the release behavior for this type.
    pub fn set_release(&mut self, func_hash: TypeHash) {
        self.release = Some(func_hash);
    }

    /// Set both addref and release behaviors for reference counting.
    pub fn set_ref_counting(&mut self, addref: TypeHash, release: TypeHash) {
        self.addref = Some(addref);
        self.release = Some(release);
    }

    /// Set the list construct behavior for this type.
    pub fn set_list_construct(&mut self, func_hash: TypeHash) {
        self.list_construct = Some(func_hash);
    }

    /// Set the list factory behavior for this type.
    pub fn set_list_factory(&mut self, func_hash: TypeHash) {
        self.list_factory = Some(func_hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_behaviors_is_empty() {
        let behaviors = TypeBehaviors::new();
        assert!(behaviors.is_empty());
        assert!(!behaviors.has_constructors());
        assert!(!behaviors.has_factories());
        assert!(!behaviors.has_destructor());
        assert!(!behaviors.has_list_init());
        assert!(!behaviors.has_ref_counting());
        assert!(behaviors.list_init_func().is_none());
    }

    #[test]
    fn default_behaviors_is_empty() {
        let behaviors = TypeBehaviors::default();
        assert!(behaviors.is_empty());
    }

    #[test]
    fn add_constructors() {
        let mut behaviors = TypeBehaviors::new();
        let ctor1 = TypeHash::from_name("MyClass::MyClass");
        let ctor2 = TypeHash::from_name("MyClass::MyClass(int)");

        behaviors.add_constructor(ctor1);
        behaviors.add_constructor(ctor2);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_constructors());
        assert_eq!(behaviors.constructors.len(), 2);
        assert_eq!(behaviors.constructors[0], ctor1);
        assert_eq!(behaviors.constructors[1], ctor2);
    }

    #[test]
    fn add_factories() {
        let mut behaviors = TypeBehaviors::new();
        let factory1 = TypeHash::from_name("MyClass::Create");
        let factory2 = TypeHash::from_name("MyClass::Create(int)");

        behaviors.add_factory(factory1);
        behaviors.add_factory(factory2);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_factories());
        assert_eq!(behaviors.factories.len(), 2);
    }

    #[test]
    fn set_destructor() {
        let mut behaviors = TypeBehaviors::new();
        let dtor = TypeHash::from_name("MyClass::~MyClass");

        behaviors.set_destructor(dtor);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_destructor());
        assert_eq!(behaviors.destructor, Some(dtor));
    }

    #[test]
    fn set_ref_counting() {
        let mut behaviors = TypeBehaviors::new();
        let addref = TypeHash::from_name("MyClass::AddRef");
        let release = TypeHash::from_name("MyClass::Release");

        behaviors.set_ref_counting(addref, release);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_addref());
        assert!(behaviors.has_release());
        assert!(behaviors.has_ref_counting());
        assert_eq!(behaviors.addref, Some(addref));
        assert_eq!(behaviors.release, Some(release));
    }

    #[test]
    fn ref_counting_requires_both() {
        let mut behaviors = TypeBehaviors::new();
        let addref = TypeHash::from_name("MyClass::AddRef");

        behaviors.set_addref(addref);

        assert!(behaviors.has_addref());
        assert!(!behaviors.has_release());
        assert!(!behaviors.has_ref_counting()); // Needs both!
    }

    #[test]
    fn list_factory_behavior() {
        let mut behaviors = TypeBehaviors::new();
        let factory = TypeHash::from_name("array::ListFactory");

        behaviors.set_list_factory(factory);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert!(behaviors.has_list_factory());
        assert!(!behaviors.has_list_construct());
        assert_eq!(behaviors.list_init_func(), Some(factory));
    }

    #[test]
    fn list_construct_behavior() {
        let mut behaviors = TypeBehaviors::new();
        let construct = TypeHash::from_name("Vec3::ListConstruct");

        behaviors.set_list_construct(construct);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert!(behaviors.has_list_construct());
        assert!(!behaviors.has_list_factory());
        assert_eq!(behaviors.list_init_func(), Some(construct));
    }

    #[test]
    fn list_factory_preferred_over_construct() {
        let mut behaviors = TypeBehaviors::new();
        let construct = TypeHash::from_name("ListConstruct");
        let factory = TypeHash::from_name("ListFactory");

        behaviors.set_list_construct(construct);
        behaviors.set_list_factory(factory);

        // Factory should be preferred
        assert_eq!(behaviors.list_init_func(), Some(factory));
    }

    #[test]
    fn full_lifecycle() {
        let mut behaviors = TypeBehaviors::new();

        behaviors.add_constructor(TypeHash::from_name("ctor"));
        behaviors.set_destructor(TypeHash::from_name("dtor"));
        behaviors.set_ref_counting(
            TypeHash::from_name("addref"),
            TypeHash::from_name("release"),
        );

        assert!(behaviors.has_constructors());
        assert!(behaviors.has_destructor());
        assert!(behaviors.has_ref_counting());
        assert!(!behaviors.is_empty());
    }

    #[test]
    fn clone_and_eq() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_constructor(TypeHash::from_name("ctor"));
        behaviors.set_destructor(TypeHash::from_name("dtor"));

        let cloned = behaviors.clone();
        assert_eq!(behaviors, cloned);
    }
}
