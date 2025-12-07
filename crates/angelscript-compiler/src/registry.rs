//! Registry - type and function storage with consistent API.
//!
//! This module provides:
//! - [`Registry`]: Trait defining the common API for type/function registries
//! - [`ScriptRegistry`]: Clean implementation for script-defined types and functions
//!
//! # Architecture
//!
//! The registry uses `TypeHash` as the primary key for all lookups:
//! - Types: `TypeHash -> TypeDef`
//! - Functions: `TypeHash (func_hash) -> FunctionDef`
//! - Behaviors: `TypeHash -> TypeBehaviors`
//!
//! Name-based lookups are secondary indexes that return `TypeHash` values.
//!
//! # Registry Trait
//!
//! The `Registry` trait provides a consistent API that both `ScriptRegistry` and
//! `FfiRegistry` can implement, enabling unified lookup across both registries.
//!
//! ```
//! use angelscript_compiler::registry::{Registry, ScriptRegistry};
//! use angelscript_compiler::types::TypeHash;
//!
//! fn lookup_in_registry(registry: &impl Registry, name: &str) -> Option<TypeHash> {
//!     registry.lookup_type(name)
//! }
//! ```

use rustc_hash::FxHashMap;

use crate::types::{
    DataType, FunctionDef, OperatorBehavior, PropertyAccessors, TypeBehaviors, TypeDef, TypeHash,
};

// ============================================================================
// Registry Trait
// ============================================================================

/// Common interface for type and function registries.
///
/// This trait provides a consistent API for looking up types, functions, and behaviors.
/// Both `ScriptRegistry` and `FfiRegistry` implement this trait, enabling unified
/// lookup across script-defined and FFI-registered items.
///
/// # Naming Conventions
///
/// - `get_*`: Lookup by TypeHash (primary key), returns `Option<&T>`
/// - `lookup_*`: Lookup by name (secondary index), returns `Option<TypeHash>` or `&[TypeHash]`
/// - `has_*`: Check existence by TypeHash, returns `bool`
/// - `find_*`: Complex lookup with multiple criteria
///
/// # Example
///
/// ```
/// use angelscript_compiler::registry::{Registry, ScriptRegistry};
/// use angelscript_compiler::types::TypeHash;
///
/// fn find_type(registry: &impl Registry, name: &str) -> Option<TypeHash> {
///     registry.lookup_type(name)
/// }
/// ```
pub trait Registry {
    // =========================================================================
    // Type Lookups
    // =========================================================================

    /// Get a type definition by TypeHash.
    fn get_type(&self, hash: TypeHash) -> Option<&TypeDef>;

    /// Look up a TypeHash by type name.
    fn lookup_type(&self, name: &str) -> Option<TypeHash>;

    /// Check if a type exists by TypeHash.
    fn has_type(&self, hash: TypeHash) -> bool {
        self.get_type(hash).is_some()
    }

    // =========================================================================
    // Function Lookups
    // =========================================================================

    /// Get a function definition by func_hash.
    fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef>;

    /// Look up all function hashes with the given name (for overload resolution).
    fn lookup_functions(&self, name: &str) -> &[TypeHash];

    /// Check if a function exists by func_hash.
    fn has_function(&self, hash: TypeHash) -> bool {
        self.get_function(hash).is_some()
    }

    // =========================================================================
    // Behavior Lookups
    // =========================================================================

    /// Get behaviors for a type.
    fn get_behaviors(&self, type_hash: TypeHash) -> Option<&TypeBehaviors>;

    /// Find all constructors for a type (value types).
    fn find_constructors(&self, type_hash: TypeHash) -> &[TypeHash];

    /// Find all factories for a type (reference types).
    fn find_factories(&self, type_hash: TypeHash) -> &[TypeHash];

    // =========================================================================
    // Method Lookups
    // =========================================================================

    /// Get all method func_hashes for a type.
    fn get_methods(&self, type_hash: TypeHash) -> Vec<TypeHash>;

    /// Find all methods with the given name on a type.
    fn find_methods_by_name(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash>;

    /// Find a method by name on a type (first match).
    fn find_method(&self, type_hash: TypeHash, name: &str) -> Option<TypeHash> {
        self.find_methods_by_name(type_hash, name).first().copied()
    }

    // =========================================================================
    // Operator Lookups
    // =========================================================================

    /// Find all overloads of an operator method for a type.
    fn find_operator_methods(&self, type_hash: TypeHash, operator: OperatorBehavior) -> &[TypeHash];

    /// Find an operator method on a type (first match).
    fn find_operator_method(&self, type_hash: TypeHash, operator: OperatorBehavior) -> Option<TypeHash> {
        self.find_operator_methods(type_hash, operator).first().copied()
    }

    // =========================================================================
    // Property Lookups
    // =========================================================================

    /// Find a property by name on a type.
    fn find_property(&self, type_hash: TypeHash, name: &str) -> Option<&PropertyAccessors>;

    // =========================================================================
    // Inheritance
    // =========================================================================

    /// Get the base class of a type (if any).
    fn get_base_class(&self, type_hash: TypeHash) -> Option<TypeHash>;

    /// Check if `derived` is a subclass of `base`.
    fn is_subclass_of(&self, derived: TypeHash, base: TypeHash) -> bool {
        if derived == base {
            return true;
        }

        let mut current = self.get_base_class(derived);
        while let Some(parent) = current {
            if parent == base {
                return true;
            }
            current = self.get_base_class(parent);
        }

        false
    }

    /// Get all interfaces implemented by a class.
    fn get_interfaces(&self, type_hash: TypeHash) -> &[TypeHash];

    // =========================================================================
    // Enum Support
    // =========================================================================

    /// Look up an enum value by enum type hash and value name.
    fn lookup_enum_value(&self, type_hash: TypeHash, value_name: &str) -> Option<i64>;

    // =========================================================================
    // Template Support
    // =========================================================================

    /// Check if a type is a template (has template parameters).
    fn is_template(&self, type_hash: TypeHash) -> bool;

    /// Check if a type is a template instance.
    fn is_template_instance(&self, type_hash: TypeHash) -> bool;
}

// ============================================================================
// ScriptRegistry
// ============================================================================

/// Registry for script-defined types and functions.
///
/// This is a clean implementation with no redundant maps:
/// - Single `types` map keyed by `TypeHash`
/// - Single `functions` map keyed by `TypeHash` (func_hash)
/// - Name indexes are secondary lookups that return `TypeHash`
///
/// Functions are always registered with complete signatures - there are no
/// `update_*` methods because functions are never in an incomplete state.
#[derive(Debug)]
pub struct ScriptRegistry {
    // === Type Storage ===
    /// Types indexed by TypeHash (primary key).
    types: FxHashMap<TypeHash, TypeDef>,
    /// Type name to TypeHash mapping (secondary index).
    type_by_name: FxHashMap<String, TypeHash>,

    // === Function Storage ===
    /// Functions indexed by func_hash (primary key).
    functions: FxHashMap<TypeHash, FunctionDef>,
    /// Function name to func_hashes mapping (secondary index, supports overloads).
    func_by_name: FxHashMap<String, Vec<TypeHash>>,

    // === Behavior Storage ===
    /// Type behaviors indexed by TypeHash.
    behaviors: FxHashMap<TypeHash, TypeBehaviors>,
}

impl ScriptRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            types: FxHashMap::default(),
            type_by_name: FxHashMap::default(),
            functions: FxHashMap::default(),
            func_by_name: FxHashMap::default(),
            behaviors: FxHashMap::default(),
        }
    }

    // =========================================================================
    // Type Registration
    // =========================================================================

    /// Register a type definition.
    ///
    /// The type's `type_hash` is used as the primary key. The qualified name
    /// is added to the name index for lookup.
    ///
    /// Returns the TypeHash of the registered type.
    pub fn register_type(&mut self, typedef: TypeDef) -> TypeHash {
        let type_hash = typedef.type_hash();
        let qualified_name = typedef.qualified_name().to_string();

        self.type_by_name.insert(qualified_name, type_hash);
        self.types.insert(type_hash, typedef);

        type_hash
    }

    /// Register a type with an additional name alias.
    ///
    /// Useful for registering both qualified and unqualified names.
    pub fn register_type_with_alias(&mut self, typedef: TypeDef, alias: &str) -> TypeHash {
        let type_hash = self.register_type(typedef);
        self.type_by_name.insert(alias.to_string(), type_hash);
        type_hash
    }

    // =========================================================================
    // Type Lookups
    // =========================================================================

    /// Get a type definition by TypeHash.
    ///
    /// This is the primary lookup method - O(1) hash map access.
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types.get(&hash)
    }

    /// Get a mutable type definition by TypeHash.
    pub fn get_type_mut(&mut self, hash: TypeHash) -> Option<&mut TypeDef> {
        self.types.get_mut(&hash)
    }

    /// Look up a TypeHash by type name.
    ///
    /// This is a secondary lookup - first finds the hash, then use `get_type`.
    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_by_name.get(name).copied()
    }

    /// Check if a type exists by TypeHash.
    pub fn has_type(&self, hash: TypeHash) -> bool {
        self.types.contains_key(&hash)
    }

    /// Get the number of registered types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Iterate over all types.
    pub fn types(&self) -> impl Iterator<Item = (&TypeHash, &TypeDef)> {
        self.types.iter()
    }

    // =========================================================================
    // Function Registration
    // =========================================================================

    /// Register a function definition.
    ///
    /// The function's `func_hash` is used as the primary key. The qualified name
    /// is added to the name index for overload resolution.
    ///
    /// Returns the func_hash of the registered function.
    pub fn register_function(&mut self, func: FunctionDef) -> TypeHash {
        let func_hash = func.func_hash;
        let qualified_name = func.qualified_name();

        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_hash);
        self.functions.insert(func_hash, func);

        func_hash
    }

    // =========================================================================
    // Function Lookups
    // =========================================================================

    /// Get a function definition by func_hash.
    ///
    /// This is the primary lookup method - O(1) hash map access.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef> {
        self.functions.get(&hash)
    }

    /// Get a mutable function definition by func_hash.
    pub fn get_function_mut(&mut self, hash: TypeHash) -> Option<&mut FunctionDef> {
        self.functions.get_mut(&hash)
    }

    /// Look up all function hashes with the given name (for overload resolution).
    ///
    /// Returns a slice of func_hashes which can be used with `get_function`.
    pub fn lookup_functions(&self, name: &str) -> &[TypeHash] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a function exists by func_hash.
    pub fn has_function(&self, hash: TypeHash) -> bool {
        self.functions.contains_key(&hash)
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Iterate over all functions.
    pub fn functions(&self) -> impl Iterator<Item = (&TypeHash, &FunctionDef)> {
        self.functions.iter()
    }

    // =========================================================================
    // Behavior Registration and Lookup
    // =========================================================================

    /// Register behaviors for a type.
    ///
    /// Overwrites any existing behaviors for this type.
    pub fn register_behaviors(&mut self, type_hash: TypeHash, behaviors: TypeBehaviors) {
        self.behaviors.insert(type_hash, behaviors);
    }

    /// Get or create behaviors for a type (for incremental registration).
    pub fn behaviors_mut(&mut self, type_hash: TypeHash) -> &mut TypeBehaviors {
        self.behaviors.entry(type_hash).or_default()
    }

    /// Get behaviors for a type.
    pub fn get_behaviors(&self, type_hash: TypeHash) -> Option<&TypeBehaviors> {
        self.behaviors.get(&type_hash)
    }

    /// Find all constructors for a type (value types).
    pub fn find_constructors(&self, type_hash: TypeHash) -> &[TypeHash] {
        self.behaviors
            .get(&type_hash)
            .map(|b| b.constructors.as_slice())
            .unwrap_or(&[])
    }

    /// Find all factories for a type (reference types).
    pub fn find_factories(&self, type_hash: TypeHash) -> &[TypeHash] {
        self.behaviors
            .get(&type_hash)
            .map(|b| b.factories.as_slice())
            .unwrap_or(&[])
    }

    /// Find a constructor matching specific argument types.
    pub fn find_constructor(&self, type_hash: TypeHash, arg_types: &[DataType]) -> Option<TypeHash> {
        let constructors = self.find_constructors(type_hash);
        for &ctor_hash in constructors {
            if let Some(func) = self.get_function(ctor_hash) {
                if func.params.len() == arg_types.len() {
                    let all_match = func
                        .params
                        .iter()
                        .zip(arg_types.iter())
                        .all(|(param, arg)| param.data_type == *arg);
                    if all_match {
                        return Some(ctor_hash);
                    }
                }
            }
        }
        None
    }

    /// Find the copy constructor for a type.
    ///
    /// Copy constructor has signature: `ClassName(const ClassName &in)` or `ClassName(const ClassName &inout)`
    pub fn find_copy_constructor(&self, type_hash: TypeHash) -> Option<TypeHash> {
        use crate::types::RefModifier;

        let constructors = self.find_constructors(type_hash);
        for &ctor_hash in constructors {
            if let Some(func) = self.get_function(ctor_hash) {
                // Copy constructor must have exactly one parameter
                if func.params.len() != 1 {
                    continue;
                }
                let param = &func.params[0];
                // Parameter must be a reference (&in or &inout)
                if !matches!(param.data_type.ref_modifier, RefModifier::In | RefModifier::InOut) {
                    continue;
                }
                // Parameter type must match the class type
                if param.data_type.type_hash == type_hash {
                    return Some(ctor_hash);
                }
            }
        }
        None
    }

    // =========================================================================
    // Method Lookups
    // =========================================================================

    /// Get all method func_hashes for a type.
    pub fn get_methods(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { methods, .. }) => methods.clone(),
            _ => Vec::new(),
        }
    }

    /// Find all methods with the given name on a type.
    pub fn find_methods_by_name(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { methods, .. }) => methods
                .iter()
                .filter(|&&hash| {
                    self.get_function(hash)
                        .map(|f| f.name == name)
                        .unwrap_or(false)
                })
                .copied()
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Find a method by name on a type (first match).
    pub fn find_method(&self, type_hash: TypeHash, name: &str) -> Option<TypeHash> {
        self.find_methods_by_name(type_hash, name).first().copied()
    }

    /// Add a method to a class's method list.
    pub fn add_method_to_class(&mut self, type_hash: TypeHash, method_hash: TypeHash) {
        if let Some(TypeDef::Class { methods, .. }) = self.get_type_mut(type_hash) {
            methods.push(method_hash);
        }
    }

    // =========================================================================
    // Operator Lookups
    // =========================================================================

    /// Find all overloads of an operator method for a type.
    pub fn find_operator_methods(
        &self,
        type_hash: TypeHash,
        operator: OperatorBehavior,
    ) -> &[TypeHash] {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { operator_methods, .. }) => operator_methods
                .get(&operator)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    /// Find an operator method on a type (first match).
    pub fn find_operator_method(
        &self,
        type_hash: TypeHash,
        operator: OperatorBehavior,
    ) -> Option<TypeHash> {
        self.find_operator_methods(type_hash, operator).first().copied()
    }

    // =========================================================================
    // Property Lookups
    // =========================================================================

    /// Find a property by name on a type.
    pub fn find_property(&self, type_hash: TypeHash, name: &str) -> Option<&PropertyAccessors> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { properties, .. }) => properties.get(name),
            _ => None,
        }
    }

    /// Get all properties for a type.
    pub fn get_properties(&self, type_hash: TypeHash) -> Option<&FxHashMap<String, PropertyAccessors>> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { properties, .. }) => Some(properties),
            _ => None,
        }
    }

    // =========================================================================
    // Inheritance
    // =========================================================================

    /// Get the base class of a type (if any).
    pub fn get_base_class(&self, type_hash: TypeHash) -> Option<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { base_class, .. }) => *base_class,
            _ => None,
        }
    }

    /// Check if `derived` is a subclass of `base`.
    pub fn is_subclass_of(&self, derived: TypeHash, base: TypeHash) -> bool {
        if derived == base {
            return true;
        }

        let mut current = self.get_base_class(derived);
        while let Some(parent) = current {
            if parent == base {
                return true;
            }
            current = self.get_base_class(parent);
        }

        false
    }

    /// Get all interfaces implemented by a class.
    pub fn get_interfaces(&self, type_hash: TypeHash) -> &[TypeHash] {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { interfaces, .. }) => interfaces.as_slice(),
            _ => &[],
        }
    }

    // =========================================================================
    // Enum Support
    // =========================================================================

    /// Look up an enum value by enum type hash and value name.
    pub fn lookup_enum_value(&self, type_hash: TypeHash, value_name: &str) -> Option<i64> {
        match self.get_type(type_hash) {
            Some(TypeDef::Enum { values, .. }) => values
                .iter()
                .find(|(name, _)| name == value_name)
                .map(|(_, val)| *val),
            _ => None,
        }
    }

    // =========================================================================
    // Funcdef Support
    // =========================================================================

    /// Get the signature of a funcdef type.
    pub fn get_funcdef_signature(&self, type_hash: TypeHash) -> Option<(&[DataType], &DataType)> {
        match self.get_type(type_hash) {
            Some(TypeDef::Funcdef { params, return_type, .. }) => {
                Some((params.as_slice(), return_type))
            }
            _ => None,
        }
    }

    /// Check if a function is compatible with a funcdef type.
    pub fn is_function_compatible_with_funcdef(
        &self,
        func_hash: TypeHash,
        funcdef_hash: TypeHash,
    ) -> bool {
        let Some(func) = self.get_function(func_hash) else {
            return false;
        };
        let Some((params, return_type)) = self.get_funcdef_signature(funcdef_hash) else {
            return false;
        };

        // Check return type matches
        if func.return_type.type_hash != return_type.type_hash {
            return false;
        }

        // Check parameter count matches
        if func.params.len() != params.len() {
            return false;
        }

        // Check parameter types match
        func.params
            .iter()
            .zip(params.iter())
            .all(|(func_param, funcdef_param)| {
                func_param.data_type.type_hash == funcdef_param.type_hash
                    && func_param.data_type.ref_modifier == funcdef_param.ref_modifier
            })
    }

    // =========================================================================
    // Template Support
    // =========================================================================

    /// Check if a type is a template (has template parameters).
    pub fn is_template(&self, type_hash: TypeHash) -> bool {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { template_params, .. }) => !template_params.is_empty(),
            _ => false,
        }
    }

    /// Check if a type is a template instance.
    pub fn is_template_instance(&self, type_hash: TypeHash) -> bool {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { template, .. }) => template.is_some(),
            _ => false,
        }
    }

    /// Get the template parameters for a template type.
    pub fn get_template_params(&self, type_hash: TypeHash) -> Option<&[TypeHash]> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { template_params, .. }) if !template_params.is_empty() => {
                Some(template_params.as_slice())
            }
            _ => None,
        }
    }
}

impl Default for ScriptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Registry Trait Implementation
// ============================================================================

impl Registry for ScriptRegistry {
    fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types.get(&hash)
    }

    fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_by_name.get(name).copied()
    }

    fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef> {
        self.functions.get(&hash)
    }

    fn lookup_functions(&self, name: &str) -> &[TypeHash] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    fn get_behaviors(&self, type_hash: TypeHash) -> Option<&TypeBehaviors> {
        self.behaviors.get(&type_hash)
    }

    fn find_constructors(&self, type_hash: TypeHash) -> &[TypeHash] {
        self.behaviors
            .get(&type_hash)
            .map(|b| b.constructors.as_slice())
            .unwrap_or(&[])
    }

    fn find_factories(&self, type_hash: TypeHash) -> &[TypeHash] {
        self.behaviors
            .get(&type_hash)
            .map(|b| b.factories.as_slice())
            .unwrap_or(&[])
    }

    fn get_methods(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { methods, .. }) => methods.clone(),
            _ => Vec::new(),
        }
    }

    fn find_methods_by_name(&self, type_hash: TypeHash, name: &str) -> Vec<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { methods, .. }) => methods
                .iter()
                .filter(|&&hash| {
                    self.get_function(hash)
                        .map(|f| f.name == name)
                        .unwrap_or(false)
                })
                .copied()
                .collect(),
            _ => Vec::new(),
        }
    }

    fn find_operator_methods(&self, type_hash: TypeHash, operator: OperatorBehavior) -> &[TypeHash] {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { operator_methods, .. }) => operator_methods
                .get(&operator)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            _ => &[],
        }
    }

    fn find_property(&self, type_hash: TypeHash, name: &str) -> Option<&PropertyAccessors> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { properties, .. }) => properties.get(name),
            _ => None,
        }
    }

    fn get_base_class(&self, type_hash: TypeHash) -> Option<TypeHash> {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { base_class, .. }) => *base_class,
            _ => None,
        }
    }

    fn get_interfaces(&self, type_hash: TypeHash) -> &[TypeHash] {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { interfaces, .. }) => interfaces.as_slice(),
            _ => &[],
        }
    }

    fn lookup_enum_value(&self, type_hash: TypeHash, value_name: &str) -> Option<i64> {
        match self.get_type(type_hash) {
            Some(TypeDef::Enum { values, .. }) => values
                .iter()
                .find(|(name, _)| name == value_name)
                .map(|(_, val)| *val),
            _ => None,
        }
    }

    fn is_template(&self, type_hash: TypeHash) -> bool {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { template_params, .. }) => !template_params.is_empty(),
            _ => false,
        }
    }

    fn is_template_instance(&self, type_hash: TypeHash) -> bool {
        match self.get_type(type_hash) {
            Some(TypeDef::Class { template, .. }) => template.is_some(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{primitives, DataType, FunctionTraits, Param, TypeKind, Visibility};

    fn make_class(name: &str) -> TypeDef {
        let type_hash = TypeHash::from_name(name);
        TypeDef::Class {
            name: name.to_string(),
            qualified_name: name.to_string(),
            type_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        }
    }

    fn make_function(name: &str, params: Vec<Param>, return_type: DataType) -> FunctionDef {
        let param_hashes: Vec<TypeHash> = params.iter().map(|p| p.data_type.type_hash).collect();
        FunctionDef {
            func_hash: TypeHash::from_function(name, &param_hashes),
            name: name.to_string(),
            namespace: vec![],
            params,
            return_type,
            object_type: None,
            traits: FunctionTraits::default(),
            is_native: false,
            visibility: Visibility::Public,
        }
    }

    // =========================================================================
    // Type Tests
    // =========================================================================

    #[test]
    fn new_registry_is_empty() {
        let registry = ScriptRegistry::new();
        assert_eq!(registry.type_count(), 0);
        assert_eq!(registry.function_count(), 0);
    }

    #[test]
    fn register_and_get_type() {
        let mut registry = ScriptRegistry::new();
        let typedef = make_class("Player");
        let type_hash = typedef.type_hash();

        let returned_hash = registry.register_type(typedef);

        assert_eq!(returned_hash, type_hash);
        assert!(registry.has_type(type_hash));
        assert_eq!(registry.type_count(), 1);

        let retrieved = registry.get_type(type_hash).unwrap();
        assert_eq!(retrieved.name(), "Player");
    }

    #[test]
    fn lookup_type_by_name() {
        let mut registry = ScriptRegistry::new();
        let typedef = make_class("Enemy");
        let type_hash = typedef.type_hash();

        registry.register_type(typedef);

        assert_eq!(registry.lookup_type("Enemy"), Some(type_hash));
        assert_eq!(registry.lookup_type("Unknown"), None);
    }

    #[test]
    fn register_type_with_alias() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("Game::Player");
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            type_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };

        registry.register_type_with_alias(typedef, "Player");

        // Can lookup by qualified name
        assert_eq!(registry.lookup_type("Game::Player"), Some(type_hash));
        // Can also lookup by alias
        assert_eq!(registry.lookup_type("Player"), Some(type_hash));
    }

    // =========================================================================
    // Function Tests
    // =========================================================================

    #[test]
    fn register_and_get_function() {
        let mut registry = ScriptRegistry::new();
        let func = make_function(
            "add",
            vec![
                Param::new("a", DataType::simple(primitives::INT32)),
                Param::new("b", DataType::simple(primitives::INT32)),
            ],
            DataType::simple(primitives::INT32),
        );
        let func_hash = func.func_hash;

        let returned_hash = registry.register_function(func);

        assert_eq!(returned_hash, func_hash);
        assert!(registry.has_function(func_hash));
        assert_eq!(registry.function_count(), 1);

        let retrieved = registry.get_function(func_hash).unwrap();
        assert_eq!(retrieved.name, "add");
        assert_eq!(retrieved.params.len(), 2);
    }

    #[test]
    fn lookup_function_overloads() {
        let mut registry = ScriptRegistry::new();

        // Register two overloads of "print"
        let print_int = make_function(
            "print",
            vec![Param::new("val", DataType::simple(primitives::INT32))],
            DataType::void(),
        );
        let print_float = make_function(
            "print",
            vec![Param::new("val", DataType::simple(primitives::FLOAT))],
            DataType::void(),
        );

        let hash1 = registry.register_function(print_int);
        let hash2 = registry.register_function(print_float);

        let overloads = registry.lookup_functions("print");
        assert_eq!(overloads.len(), 2);
        assert!(overloads.contains(&hash1));
        assert!(overloads.contains(&hash2));
    }

    #[test]
    fn lookup_nonexistent_function() {
        let registry = ScriptRegistry::new();
        assert!(registry.lookup_functions("unknown").is_empty());
    }

    // =========================================================================
    // Behavior Tests
    // =========================================================================

    #[test]
    fn register_and_get_behaviors() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("MyClass");

        let mut behaviors = TypeBehaviors::new();
        behaviors.add_constructor(TypeHash::from_name("ctor1"));
        behaviors.add_constructor(TypeHash::from_name("ctor2"));

        registry.register_behaviors(type_hash, behaviors);

        let retrieved = registry.get_behaviors(type_hash).unwrap();
        assert_eq!(retrieved.constructors.len(), 2);
    }

    #[test]
    fn behaviors_mut_creates_default() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("MyClass");

        // First access creates default
        let behaviors = registry.behaviors_mut(type_hash);
        behaviors.add_constructor(TypeHash::from_name("ctor"));

        assert_eq!(registry.find_constructors(type_hash).len(), 1);
    }

    #[test]
    fn find_constructors_and_factories() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("MyClass");

        let behaviors = registry.behaviors_mut(type_hash);
        behaviors.add_constructor(TypeHash::from_name("ctor"));
        behaviors.add_factory(TypeHash::from_name("factory1"));
        behaviors.add_factory(TypeHash::from_name("factory2"));

        assert_eq!(registry.find_constructors(type_hash).len(), 1);
        assert_eq!(registry.find_factories(type_hash).len(), 2);
    }

    // =========================================================================
    // Method Tests
    // =========================================================================

    #[test]
    fn get_methods_from_class() {
        let mut registry = ScriptRegistry::new();

        let method1 = TypeHash::from_name("method1");
        let method2 = TypeHash::from_name("method2");
        let type_hash = TypeHash::from_name("MyClass");

        let typedef = TypeDef::Class {
            name: "MyClass".to_string(),
            qualified_name: "MyClass".to_string(),
            type_hash,
            fields: vec![],
            methods: vec![method1, method2],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };

        registry.register_type(typedef);

        let methods = registry.get_methods(type_hash);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&method1));
        assert!(methods.contains(&method2));
    }

    #[test]
    fn find_methods_by_name() {
        let mut registry = ScriptRegistry::new();

        // Register methods
        let update_func = FunctionDef {
            func_hash: TypeHash::from_name("update"),
            name: "update".to_string(),
            namespace: vec![],
            params: vec![],
            return_type: DataType::void(),
            object_type: Some(TypeHash::from_name("Player")),
            traits: FunctionTraits::default(),
            is_native: false,
            visibility: Visibility::Public,
        };
        let draw_func = FunctionDef {
            func_hash: TypeHash::from_name("draw"),
            name: "draw".to_string(),
            namespace: vec![],
            params: vec![],
            return_type: DataType::void(),
            object_type: Some(TypeHash::from_name("Player")),
            traits: FunctionTraits::default(),
            is_native: false,
            visibility: Visibility::Public,
        };

        registry.register_function(update_func.clone());
        registry.register_function(draw_func.clone());

        // Register class with methods
        let type_hash = TypeHash::from_name("Player");
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash,
            fields: vec![],
            methods: vec![update_func.func_hash, draw_func.func_hash],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        registry.register_type(typedef);

        let update_methods = registry.find_methods_by_name(type_hash, "update");
        assert_eq!(update_methods.len(), 1);

        let unknown_methods = registry.find_methods_by_name(type_hash, "unknown");
        assert!(unknown_methods.is_empty());
    }

    #[test]
    fn add_method_to_class() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("MyClass");
        registry.register_type(make_class("MyClass"));

        let method_hash = TypeHash::from_name("newMethod");
        registry.add_method_to_class(type_hash, method_hash);

        let methods = registry.get_methods(type_hash);
        assert!(methods.contains(&method_hash));
    }

    // =========================================================================
    // Inheritance Tests
    // =========================================================================

    #[test]
    fn get_base_class() {
        let mut registry = ScriptRegistry::new();

        let base_hash = TypeHash::from_name("Base");
        registry.register_type(make_class("Base"));

        let derived_hash = TypeHash::from_name("Derived");
        let derived = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: derived_hash,
            fields: vec![],
            methods: vec![],
            base_class: Some(base_hash),
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        registry.register_type(derived);

        assert_eq!(registry.get_base_class(derived_hash), Some(base_hash));
        assert_eq!(registry.get_base_class(base_hash), None);
    }

    #[test]
    fn is_subclass_of() {
        let mut registry = ScriptRegistry::new();

        // Create inheritance chain: GrandChild -> Child -> Parent
        let parent_hash = TypeHash::from_name("Parent");
        registry.register_type(make_class("Parent"));

        let child_hash = TypeHash::from_name("Child");
        let child = TypeDef::Class {
            name: "Child".to_string(),
            qualified_name: "Child".to_string(),
            type_hash: child_hash,
            fields: vec![],
            methods: vec![],
            base_class: Some(parent_hash),
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        registry.register_type(child);

        let grandchild_hash = TypeHash::from_name("GrandChild");
        let grandchild = TypeDef::Class {
            name: "GrandChild".to_string(),
            qualified_name: "GrandChild".to_string(),
            type_hash: grandchild_hash,
            fields: vec![],
            methods: vec![],
            base_class: Some(child_hash),
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        registry.register_type(grandchild);

        // Same class
        assert!(registry.is_subclass_of(parent_hash, parent_hash));

        // Direct inheritance
        assert!(registry.is_subclass_of(child_hash, parent_hash));

        // Transitive inheritance
        assert!(registry.is_subclass_of(grandchild_hash, parent_hash));

        // Not the other way
        assert!(!registry.is_subclass_of(parent_hash, child_hash));
    }

    // =========================================================================
    // Enum Tests
    // =========================================================================

    #[test]
    fn lookup_enum_value() {
        let mut registry = ScriptRegistry::new();
        let type_hash = TypeHash::from_name("Color");

        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
            type_hash,
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        };
        registry.register_type(typedef);

        assert_eq!(registry.lookup_enum_value(type_hash, "Red"), Some(0));
        assert_eq!(registry.lookup_enum_value(type_hash, "Green"), Some(1));
        assert_eq!(registry.lookup_enum_value(type_hash, "Blue"), Some(2));
        assert_eq!(registry.lookup_enum_value(type_hash, "Unknown"), None);
    }

    // =========================================================================
    // Template Tests
    // =========================================================================

    #[test]
    fn is_template() {
        let mut registry = ScriptRegistry::new();

        // Regular class
        let regular_hash = TypeHash::from_name("Regular");
        registry.register_type(make_class("Regular"));

        // Template class
        let template_hash = TypeHash::from_name("array");
        let t_param = TypeHash::from_name("array::T");
        let template = TypeDef::Class {
            name: "array".to_string(),
            qualified_name: "array".to_string(),
            type_hash: template_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![t_param],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::reference(),
        };
        registry.register_type(template);

        assert!(!registry.is_template(regular_hash));
        assert!(registry.is_template(template_hash));
    }

    #[test]
    fn is_template_instance() {
        let mut registry = ScriptRegistry::new();

        let template_hash = TypeHash::from_name("array");
        let instance_hash = TypeHash::from_name("array<int>");

        let instance = TypeDef::Class {
            name: "array<int>".to_string(),
            qualified_name: "array<int>".to_string(),
            type_hash: instance_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: Some(template_hash),
            type_args: vec![DataType::simple(primitives::INT32)],
            type_kind: TypeKind::reference(),
        };
        registry.register_type(instance);

        assert!(registry.is_template_instance(instance_hash));
        assert!(!registry.is_template(instance_hash));
    }

    // =========================================================================
    // Default Trait
    // =========================================================================

    #[test]
    fn default_creates_empty() {
        let registry = ScriptRegistry::default();
        assert_eq!(registry.type_count(), 0);
        assert_eq!(registry.function_count(), 0);
    }
}
