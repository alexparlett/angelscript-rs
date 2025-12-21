//! TypeBehaviors - lifecycle and initialization functions for types.
//!
//! This module defines `TypeBehaviors`, which stores function hashes for various
//! lifecycle behaviors like construction, destruction, initialization lists,
//! and operator overloads. Behaviors are stored centrally in the Registry,
//! indexed by TypeHash.

use rustc_hash::FxHashMap;

use crate::{ListPattern, OperatorBehavior, ReferenceKind, RegistrationError, TypeHash, TypeKind};

/// Result of behavior validation for a type.
///
/// Contains both forbidden behaviors that were registered (but shouldn't be)
/// and required behaviors that are missing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BehaviorValidationResult {
    /// Behaviors that are registered but forbidden for this type kind.
    pub forbidden: Vec<ForbiddenBehavior>,
    /// Behaviors that are required but missing for this type kind.
    pub missing: Vec<&'static str>,
}

/// A forbidden behavior that was registered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForbiddenBehavior {
    /// The behavior name.
    pub behavior: &'static str,
    /// Why it's forbidden.
    pub reason: String,
}

impl BehaviorValidationResult {
    /// Returns true if there are no validation errors.
    pub fn is_ok(&self) -> bool {
        self.forbidden.is_empty() && self.missing.is_empty()
    }

    /// Convert to a list of RegistrationErrors for a specific type.
    pub fn into_errors(self, type_name: String) -> Vec<RegistrationError> {
        let mut errors = Vec::new();

        for f in self.forbidden {
            errors.push(RegistrationError::ForbiddenBehavior {
                type_name: type_name.clone(),
                behavior: f.behavior,
                reason: f.reason,
            });
        }

        if !self.missing.is_empty() {
            errors.push(RegistrationError::MissingBehaviors {
                type_name,
                missing: self.missing,
            });
        }

        errors
    }
}

/// A list initialization behavior (factory or construct).
///
/// Pairs a function hash with its list pattern, describing how initialization
/// list elements map to the function's parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBehavior {
    /// Function hash for this list behavior.
    pub func_hash: TypeHash,
    /// Pattern describing expected init list structure (required).
    pub pattern: ListPattern,
}

impl ListBehavior {
    /// Create a new list behavior.
    pub fn new(func_hash: TypeHash, pattern: ListPattern) -> Self {
        Self { func_hash, pattern }
    }
}

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
    /// List construct behaviors - for value types with init list syntax.
    /// Used for types like `MyStruct s = {1, 2, 3}` where the value is constructed in place.
    /// Multiple overloads supported (e.g., different patterns).
    /// Corresponds to asBEHAVE_LIST_CONSTRUCT
    pub list_constructs: Vec<ListBehavior>,

    /// List factory behaviors - for reference types with init list syntax.
    /// Used for types like `array<int> a = {1, 2, 3}` where a handle is returned.
    /// Multiple overloads supported (e.g., different patterns).
    /// Corresponds to asBEHAVE_LIST_FACTORY
    pub list_factories: Vec<ListBehavior>,

    // === Weak References ===
    /// Get weak reference flag - returns a shared weak ref flag object.
    /// Corresponds to asBEHAVE_GET_WEAKREF_FLAG
    pub get_weakref_flag: Option<TypeHash>,

    // === Template Support ===
    /// Template callback - validates template instantiation.
    /// Corresponds to asBEHAVE_TEMPLATE_CALLBACK
    pub template_callback: Option<TypeHash>,

    // === Operators ===
    /// Operator overloads for this type.
    /// Maps operator behavior to function hashes (multiple overloads per operator).
    /// The actual functions are stored in the registry's `functions` map.
    pub operators: FxHashMap<OperatorBehavior, Vec<TypeHash>>,
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
            && self.list_constructs.is_empty()
            && self.list_factories.is_empty()
            && self.get_weakref_flag.is_none()
            && self.template_callback.is_none()
            && self.operators.is_empty()
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
        !self.list_constructs.is_empty() || !self.list_factories.is_empty()
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
        !self.list_factories.is_empty()
    }

    /// Check if this type has list_construct behavior.
    pub fn has_list_construct(&self) -> bool {
        !self.list_constructs.is_empty()
    }

    /// Get all list behaviors, preferring factories over constructs.
    ///
    /// Returns factories if any exist, otherwise returns constructs.
    /// This preference exists because:
    /// - List factories are for reference types (handles), which are more common
    ///   in init list scenarios (e.g., `array<T>`, `dictionary<K,V>`)
    /// - List constructs are for value types constructed in place
    /// - A type should typically have one or the other, not both
    ///
    /// If you need access to both, use `list_factories` and `list_constructs` directly.
    pub fn list_behaviors(&self) -> &[ListBehavior] {
        if !self.list_factories.is_empty() {
            &self.list_factories
        } else {
            &self.list_constructs
        }
    }

    /// Get the first list pattern (for simple single-pattern cases).
    ///
    /// Returns the pattern from the first list behavior (factory preferred over construct).
    pub fn list_pattern(&self) -> Option<&ListPattern> {
        self.list_behaviors().first().map(|b| &b.pattern)
    }

    /// Get the first list initialization function hash (for compatibility).
    ///
    /// Prefers list_factory over list_construct.
    pub fn list_init_func(&self) -> Option<TypeHash> {
        self.list_behaviors().first().map(|b| b.func_hash)
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

    /// Add a list construct behavior for this type.
    pub fn add_list_construct(&mut self, behavior: ListBehavior) {
        self.list_constructs.push(behavior);
    }

    /// Add a list factory behavior for this type.
    pub fn add_list_factory(&mut self, behavior: ListBehavior) {
        self.list_factories.push(behavior);
    }

    // === Operator Methods ===

    /// Check if this type has any operators defined.
    pub fn has_operators(&self) -> bool {
        !self.operators.is_empty()
    }

    /// Check if this type has a specific operator.
    pub fn has_operator(&self, op: OperatorBehavior) -> bool {
        self.operators.contains_key(&op)
    }

    /// Add an operator overload for this type.
    pub fn add_operator(&mut self, op: OperatorBehavior, func_hash: TypeHash) {
        self.operators.entry(op).or_default().push(func_hash);
    }

    /// Get all overloads for a specific operator.
    pub fn get_operator(&self, op: OperatorBehavior) -> Option<&[TypeHash]> {
        self.operators.get(&op).map(|v| v.as_slice())
    }

    /// Get all operators defined for this type.
    pub fn operators(&self) -> impl Iterator<Item = (&OperatorBehavior, &Vec<TypeHash>)> {
        self.operators.iter()
    }

    // === Validation Methods ===

    /// Validate all behaviors for an FFI type.
    ///
    /// Checks both:
    /// - Forbidden behaviors (registered but not allowed for this type kind)
    /// - Required behaviors (must be present for this type kind)
    ///
    /// This is only called for FFI types during module installation.
    /// Script types have behaviors auto-generated by the VM.
    pub fn validate(&self, type_kind: &TypeKind) -> BehaviorValidationResult {
        let mut result = BehaviorValidationResult::default();

        self.check_forbidden_behaviors(type_kind, &mut result.forbidden);
        self.check_required_behaviors(type_kind, &mut result.missing);

        result
    }

    /// Check for forbidden behaviors based on type kind.
    fn check_forbidden_behaviors(&self, type_kind: &TypeKind, errors: &mut Vec<ForbiddenBehavior>) {
        if let TypeKind::Reference { kind } = type_kind {
            // Check AddRef
            if self.addref.is_some() && !kind.allows_addref() {
                errors.push(ForbiddenBehavior {
                    behavior: "AddRef",
                    reason: format!(
                        "{} reference types cannot have AddRef behavior",
                        kind.name()
                    ),
                });
            }

            // Check Release
            if self.release.is_some() && !kind.allows_release() {
                errors.push(ForbiddenBehavior {
                    behavior: "Release",
                    reason: format!(
                        "{} reference types cannot have Release behavior",
                        kind.name()
                    ),
                });
            }

            // Check Factories
            if (!self.factories.is_empty() || !self.list_factories.is_empty())
                && !kind.allows_factories()
            {
                errors.push(ForbiddenBehavior {
                    behavior: "Factory",
                    reason: format!(
                        "{} reference types cannot have factories (factories return handles)",
                        kind.name()
                    ),
                });
            }
        }
    }

    /// Check for required behaviors based on type kind.
    fn check_required_behaviors(&self, type_kind: &TypeKind, missing: &mut Vec<&'static str>) {
        match type_kind {
            TypeKind::Reference { kind } => {
                // Standard reference types need AddRef + Release
                if kind.requires_ref_counting() {
                    if !self.has_addref() {
                        missing.push("AddRef");
                    }
                    if !self.has_release() {
                        missing.push("Release");
                    }
                }

                // Scoped types need Release (for cleanup at scope exit)
                if matches!(kind, ReferenceKind::Scoped) && !self.has_release() {
                    missing.push("Release");
                }
            }

            TypeKind::Value { is_pod: false, .. } => {
                // Non-POD value types need constructor + destructor
                if !self.has_constructors() {
                    missing.push("Constructor");
                }
                if !self.has_destructor() {
                    missing.push("Destructor");
                }
            }

            // ScriptObject - VM handles lifecycle
            // POD value types - no requirements
            _ => {}
        }
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
        use crate::primitives;
        let mut behaviors = TypeBehaviors::new();
        let factory = TypeHash::from_name("array::ListFactory");
        let pattern = ListPattern::Repeat(primitives::INT32);

        behaviors.add_list_factory(ListBehavior::new(factory, pattern.clone()));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert!(behaviors.has_list_factory());
        assert!(!behaviors.has_list_construct());
        assert_eq!(behaviors.list_init_func(), Some(factory));
        assert_eq!(behaviors.list_pattern(), Some(&pattern));
    }

    #[test]
    fn list_construct_behavior() {
        use crate::primitives;
        let mut behaviors = TypeBehaviors::new();
        let construct = TypeHash::from_name("Vec3::ListConstruct");
        let pattern = ListPattern::Fixed(vec![
            primitives::FLOAT,
            primitives::FLOAT,
            primitives::FLOAT,
        ]);

        behaviors.add_list_construct(ListBehavior::new(construct, pattern.clone()));

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_list_init());
        assert!(behaviors.has_list_construct());
        assert!(!behaviors.has_list_factory());
        assert_eq!(behaviors.list_init_func(), Some(construct));
        assert_eq!(behaviors.list_pattern(), Some(&pattern));
    }

    #[test]
    fn list_factory_preferred_over_construct() {
        use crate::primitives;
        let mut behaviors = TypeBehaviors::new();
        let construct = TypeHash::from_name("ListConstruct");
        let factory = TypeHash::from_name("ListFactory");
        let construct_pattern = ListPattern::Repeat(primitives::INT32);
        let factory_pattern = ListPattern::Repeat(primitives::FLOAT);

        behaviors.add_list_construct(ListBehavior::new(construct, construct_pattern));
        behaviors.add_list_factory(ListBehavior::new(factory, factory_pattern.clone()));

        // Factory should be preferred
        assert_eq!(behaviors.list_init_func(), Some(factory));
        assert_eq!(behaviors.list_pattern(), Some(&factory_pattern));
    }

    #[test]
    fn multiple_list_factories() {
        use crate::primitives;
        let mut behaviors = TypeBehaviors::new();
        let factory1 = TypeHash::from_name("f1");
        let factory2 = TypeHash::from_name("f2");
        let pattern1 = ListPattern::Repeat(primitives::INT32);
        let pattern2 = ListPattern::Fixed(vec![primitives::INT32, primitives::STRING]);

        behaviors.add_list_factory(ListBehavior::new(factory1, pattern1.clone()));
        behaviors.add_list_factory(ListBehavior::new(factory2, pattern2));

        assert_eq!(behaviors.list_factories.len(), 2);
        assert_eq!(behaviors.list_behaviors().len(), 2);
        // First factory is preferred
        assert_eq!(behaviors.list_init_func(), Some(factory1));
        assert_eq!(behaviors.list_pattern(), Some(&pattern1));
    }

    #[test]
    fn list_behavior_creation() {
        use crate::primitives;
        let pattern = ListPattern::Repeat(primitives::INT32);
        let func_hash = TypeHash::from_name("factory");
        let behavior = ListBehavior::new(func_hash, pattern.clone());

        assert_eq!(behavior.func_hash, func_hash);
        assert_eq!(behavior.pattern, pattern);
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

    #[test]
    fn add_operators() {
        let mut behaviors = TypeBehaviors::new();
        let op_add = TypeHash::from_name("MyClass::opAdd");
        let op_add_r = TypeHash::from_name("MyClass::opAdd_r");

        behaviors.add_operator(OperatorBehavior::OpAdd, op_add);
        behaviors.add_operator(OperatorBehavior::OpAddR, op_add_r);

        assert!(!behaviors.is_empty());
        assert!(behaviors.has_operators());
        assert!(behaviors.has_operator(OperatorBehavior::OpAdd));
        assert!(behaviors.has_operator(OperatorBehavior::OpAddR));
        assert!(!behaviors.has_operator(OperatorBehavior::OpSub));
    }

    #[test]
    fn operator_overloads() {
        let mut behaviors = TypeBehaviors::new();
        let op_add_int = TypeHash::from_name("MyClass::opAdd(int)");
        let op_add_float = TypeHash::from_name("MyClass::opAdd(float)");

        behaviors.add_operator(OperatorBehavior::OpAdd, op_add_int);
        behaviors.add_operator(OperatorBehavior::OpAdd, op_add_float);

        let overloads = behaviors.get_operator(OperatorBehavior::OpAdd).unwrap();
        assert_eq!(overloads.len(), 2);
        assert_eq!(overloads[0], op_add_int);
        assert_eq!(overloads[1], op_add_float);
    }

    #[test]
    fn get_nonexistent_operator() {
        let behaviors = TypeBehaviors::new();
        assert!(behaviors.get_operator(OperatorBehavior::OpAdd).is_none());
    }

    #[test]
    fn operators_iterator() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_operator(OperatorBehavior::OpAdd, TypeHash::from_name("add"));
        behaviors.add_operator(OperatorBehavior::OpSub, TypeHash::from_name("sub"));

        let ops: Vec<_> = behaviors.operators().collect();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn operators_affects_is_empty() {
        let mut behaviors = TypeBehaviors::new();
        assert!(behaviors.is_empty());

        behaviors.add_operator(OperatorBehavior::OpNeg, TypeHash::from_name("neg"));
        assert!(!behaviors.is_empty());
    }

    // =========================================================================
    // Behavior Validation Tests
    // =========================================================================

    #[test]
    fn validate_standard_ref_ok_with_addref_release() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_addref(TypeHash::from_name("addref"));
        behaviors.set_release(TypeHash::from_name("release"));

        let result = behaviors.validate(&TypeKind::reference());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_standard_ref_missing_addref_release() {
        let behaviors = TypeBehaviors::new();

        let result = behaviors.validate(&TypeKind::reference());
        assert!(!result.is_ok());
        assert!(result.missing.contains(&"AddRef"));
        assert!(result.missing.contains(&"Release"));
    }

    #[test]
    fn validate_nocount_forbids_addref() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_addref(TypeHash::from_name("addref"));

        let result = behaviors.validate(&TypeKind::no_count());
        assert!(!result.is_ok());
        assert_eq!(result.forbidden.len(), 1);
        assert_eq!(result.forbidden[0].behavior, "AddRef");
        assert!(result.forbidden[0].reason.contains("NoCount"));
    }

    #[test]
    fn validate_nocount_forbids_release() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_release(TypeHash::from_name("release"));

        let result = behaviors.validate(&TypeKind::no_count());
        assert!(!result.is_ok());
        assert_eq!(result.forbidden.len(), 1);
        assert_eq!(result.forbidden[0].behavior, "Release");
    }

    #[test]
    fn validate_nocount_allows_factory() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_factory(TypeHash::from_name("factory"));

        // NoCount types allow factories (handles work)
        let result = behaviors.validate(&TypeKind::no_count());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_nohandle_forbids_all() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_addref(TypeHash::from_name("addref"));
        behaviors.set_release(TypeHash::from_name("release"));
        behaviors.add_factory(TypeHash::from_name("factory"));

        // NoHandle forbids AddRef, Release, and Factory
        let result = behaviors.validate(&TypeKind::no_handle());
        assert!(!result.is_ok());
        assert_eq!(result.forbidden.len(), 3);
    }

    #[test]
    fn validate_nohandle_empty_is_ok() {
        let behaviors = TypeBehaviors::new();

        // NoHandle types don't need anything
        let result = behaviors.validate(&TypeKind::no_handle());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_scoped_needs_release() {
        let behaviors = TypeBehaviors::new();

        let result = behaviors.validate(&TypeKind::scoped());
        assert!(!result.is_ok());
        assert!(result.missing.contains(&"Release"));
    }

    #[test]
    fn validate_scoped_ok_with_release() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_release(TypeHash::from_name("release"));

        let result = behaviors.validate(&TypeKind::scoped());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_scoped_forbids_addref() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_addref(TypeHash::from_name("addref"));
        behaviors.set_release(TypeHash::from_name("release"));

        let result = behaviors.validate(&TypeKind::scoped());
        assert!(!result.is_ok());
        assert_eq!(result.forbidden.len(), 1);
        assert_eq!(result.forbidden[0].behavior, "AddRef");
    }

    #[test]
    fn validate_list_factory_forbidden_for_nohandle() {
        use crate::primitives;

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_list_factory(ListBehavior::new(
            TypeHash::from_name("list_factory"),
            ListPattern::Repeat(primitives::INT32),
        ));

        let result = behaviors.validate(&TypeKind::no_handle());
        assert!(!result.is_ok());
        assert_eq!(result.forbidden[0].behavior, "Factory");
    }

    #[test]
    fn validate_non_pod_value_needs_ctor_dtor() {
        let behaviors = TypeBehaviors::new();

        let result = behaviors.validate(&TypeKind::value::<u32>());
        assert!(!result.is_ok());
        assert!(result.missing.contains(&"Constructor"));
        assert!(result.missing.contains(&"Destructor"));
    }

    #[test]
    fn validate_non_pod_value_ok_with_both() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.add_constructor(TypeHash::from_name("ctor"));
        behaviors.set_destructor(TypeHash::from_name("dtor"));

        let result = behaviors.validate(&TypeKind::value::<u32>());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_pod_value_needs_nothing() {
        let behaviors = TypeBehaviors::new();

        let result = behaviors.validate(&TypeKind::pod::<i32>());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_script_object_needs_nothing() {
        let behaviors = TypeBehaviors::new();

        // Script objects are managed by VM - but validate() is only for FFI
        // This tests that the validation doesn't incorrectly flag anything
        let result = behaviors.validate(&TypeKind::script_object());
        assert!(result.is_ok());
    }

    #[test]
    fn validate_result_into_errors() {
        let mut behaviors = TypeBehaviors::new();
        behaviors.set_addref(TypeHash::from_name("addref"));

        let result = behaviors.validate(&TypeKind::no_count());
        let errors = result.into_errors("MyType".to_string());

        assert_eq!(errors.len(), 1);
        match &errors[0] {
            RegistrationError::ForbiddenBehavior {
                type_name,
                behavior,
                ..
            } => {
                assert_eq!(type_name, "MyType");
                assert_eq!(*behavior, "AddRef");
            }
            _ => panic!("expected ForbiddenBehavior"),
        }
    }

    #[test]
    fn validate_result_into_errors_missing() {
        let behaviors = TypeBehaviors::new();

        let result = behaviors.validate(&TypeKind::reference());
        let errors = result.into_errors("MyRef".to_string());

        assert_eq!(errors.len(), 1);
        match &errors[0] {
            RegistrationError::MissingBehaviors { type_name, missing } => {
                assert_eq!(type_name, "MyRef");
                assert!(missing.contains(&"AddRef"));
                assert!(missing.contains(&"Release"));
            }
            _ => panic!("expected MissingBehaviors"),
        }
    }
}
