//! SymbolRegistry - unified type and function registry.
//!
//! This module provides [`SymbolRegistry`], the central storage for all types and
//! functions in the AngelScript runtime. It provides O(1) lookup by hash and
//! supports FFI, shared script, and local script entities.
//!
//! # Storage Model
//!
//! - **Types**: All type entries (`TypeEntry`) stored in a single map by `TypeHash`
//! - **Functions**: All functions (global, methods, operators, behaviors) stored in
//!   a single `functions` map. Other structures reference them by `TypeHash`.
//! - **Template Callbacks**: Stored separately because they have a specific signature
//!   (`Fn(&TemplateInstanceInfo) -> TemplateValidation`) different from normal native
//!   functions that use `CallContext`.
//!
//! # Example
//!
//! ```
//! use angelscript_registry::SymbolRegistry;
//! use angelscript_core::{TypeEntry, PrimitiveEntry, PrimitiveKind, primitives};
//!
//! let mut registry = SymbolRegistry::new();
//!
//! // Register primitives
//! registry.register_all_primitives();
//!
//! // Lookup by hash
//! let int_type = registry.get(primitives::INT32);
//! assert!(int_type.is_some());
//! ```

use rustc_hash::{FxHashMap, FxHashSet};

use angelscript_core::{
    ClassEntry, EnumEntry, FuncdefEntry, FunctionEntry, GlobalPropertyEntry, InterfaceEntry,
    PrimitiveEntry, PrimitiveKind, PropertyEntry, RegistrationError, TemplateInstanceInfo,
    TemplateParamEntry, TemplateValidation, TypeEntry, TypeHash,
};

/// Type-erased template validation callback.
///
/// Called at compile-time to validate template instantiation.
/// Returns validation result indicating if the instantiation is valid.
pub type TemplateCallback =
    Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>;

/// Unified type and function registry.
///
/// Provides central storage for all types and functions in the AngelScript runtime.
/// All lookups are O(1) by `TypeHash`.
#[derive(Default)]
pub struct SymbolRegistry {
    /// All types by hash (O(1) lookup).
    types: FxHashMap<TypeHash, TypeEntry>,

    /// Qualified name to hash lookup.
    type_by_name: FxHashMap<String, TypeHash>,

    /// ALL functions (methods + globals) - single source of truth.
    functions: FxHashMap<TypeHash, FunctionEntry>,

    /// Overload resolution by qualified name.
    function_overloads: FxHashMap<String, Vec<TypeHash>>,

    /// Registered namespaces.
    namespaces: FxHashSet<String>,

    /// Template validation callbacks.
    /// Stored separately because they have a specific signature different from
    /// normal native functions that use `CallContext`.
    template_callbacks: FxHashMap<TypeHash, TemplateCallback>,

    /// Global properties by hash (O(1) lookup).
    /// Hash is computed from qualified name via `TypeHash::from_name()`.
    globals: FxHashMap<TypeHash, GlobalPropertyEntry>,
}

impl SymbolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all primitives pre-registered.
    pub fn with_primitives() -> Self {
        let mut registry = Self::new();
        registry.register_all_primitives();
        registry
    }

    // ==========================================================================
    // Type Lookup
    // ==========================================================================

    /// Get a type by its hash.
    pub fn get(&self, hash: TypeHash) -> Option<&TypeEntry> {
        self.types.get(&hash)
    }

    /// Get a mutable type by its hash.
    pub fn get_mut(&mut self, hash: TypeHash) -> Option<&mut TypeEntry> {
        self.types.get_mut(&hash)
    }

    /// Get a type by its qualified name.
    pub fn get_by_name(&self, name: &str) -> Option<&TypeEntry> {
        self.type_by_name
            .get(name)
            .and_then(|hash| self.types.get(hash))
    }

    /// Check if a type exists by hash.
    pub fn contains_type(&self, hash: TypeHash) -> bool {
        self.types.contains_key(&hash)
    }

    /// Check if a type exists by name.
    pub fn contains_type_name(&self, name: &str) -> bool {
        self.type_by_name.contains_key(name)
    }

    // ==========================================================================
    // Function Lookup
    // ==========================================================================

    /// Get a function by its hash.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionEntry> {
        self.functions.get(&hash)
    }

    /// Get a mutable function by its hash.
    pub fn get_function_mut(&mut self, hash: TypeHash) -> Option<&mut FunctionEntry> {
        self.functions.get_mut(&hash)
    }

    /// Get all overloads for a function by qualified name.
    pub fn get_function_overloads(&self, name: &str) -> Option<&[TypeHash]> {
        self.function_overloads.get(name).map(|v| v.as_slice())
    }

    /// Check if a function exists by hash.
    pub fn contains_function(&self, hash: TypeHash) -> bool {
        self.functions.contains_key(&hash)
    }

    // ==========================================================================
    // Registration
    // ==========================================================================

    /// Register a type entry.
    ///
    /// Returns an error if a type with the same hash already exists.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let hash = entry.type_hash();
        let name = entry.qualified_name().to_string();

        if self.types.contains_key(&hash) {
            return Err(RegistrationError::DuplicateType(name));
        }

        self.type_by_name.insert(name, hash);
        self.types.insert(hash, entry);
        Ok(())
    }

    /// Register a function entry.
    ///
    /// Returns an error if a function with the same hash already exists.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        let hash = entry.def.func_hash;
        let name = entry.def.qualified_name();

        if self.functions.contains_key(&hash) {
            return Err(RegistrationError::DuplicateRegistration {
                name: name.to_string(),
                kind: "function".to_string(),
            });
        }

        self.function_overloads
            .entry(name.to_string())
            .or_default()
            .push(hash);
        self.functions.insert(hash, entry);
        Ok(())
    }

    /// Register a primitive type.
    ///
    /// Primitives are always registered (no duplicate check).
    pub fn register_primitive(&mut self, entry: PrimitiveEntry) {
        let hash = entry.type_hash;
        let name = entry.name().to_string();
        self.type_by_name.insert(name, hash);
        self.types.insert(hash, TypeEntry::Primitive(entry));
    }

    /// Register all primitive types.
    pub fn register_all_primitives(&mut self) {
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Void));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Bool));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Int8));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Int16));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Int32));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Int64));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Uint8));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Uint16));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Uint32));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Uint64));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Float));
        self.register_primitive(PrimitiveEntry::new(PrimitiveKind::Double));
    }

    /// Register a namespace.
    pub fn register_namespace(&mut self, ns: impl Into<String>) {
        self.namespaces.insert(ns.into());
    }

    /// Register a template validation callback.
    pub fn register_template_callback(&mut self, template: TypeHash, callback: TemplateCallback) {
        self.template_callbacks.insert(template, callback);
    }

    // ==========================================================================
    // Iteration
    // ==========================================================================

    /// Iterate over all types.
    pub fn types(&self) -> impl Iterator<Item = &TypeEntry> {
        self.types.values()
    }

    /// Iterate over all class entries.
    pub fn classes(&self) -> impl Iterator<Item = &ClassEntry> {
        self.types.values().filter_map(|t| t.as_class())
    }

    /// Iterate over all enum entries.
    pub fn enums(&self) -> impl Iterator<Item = &EnumEntry> {
        self.types.values().filter_map(|t| t.as_enum())
    }

    /// Iterate over all interface entries.
    pub fn interfaces(&self) -> impl Iterator<Item = &InterfaceEntry> {
        self.types.values().filter_map(|t| t.as_interface())
    }

    /// Iterate over all funcdef entries.
    pub fn funcdefs(&self) -> impl Iterator<Item = &FuncdefEntry> {
        self.types.values().filter_map(|t| t.as_funcdef())
    }

    /// Iterate over all template parameter entries.
    pub fn template_params(&self) -> impl Iterator<Item = &TemplateParamEntry> {
        self.types.values().filter_map(|t| t.as_template_param())
    }

    /// Iterate over all functions.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionEntry> {
        self.functions.values()
    }

    /// Get the number of registered types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    // ==========================================================================
    // Inheritance Helpers
    // ==========================================================================

    /// Get the inheritance chain for a class (excluding the class itself).
    ///
    /// Returns base classes from immediate parent to root.
    pub fn base_class_chain(&self, hash: TypeHash) -> Vec<&ClassEntry> {
        let mut chain = Vec::new();
        let mut current = hash;

        while let Some(entry) = self.types.get(&current)
            && let Some(class) = entry.as_class()
            && let Some(base) = class.base_class
            && let Some(base_entry) = self.types.get(&base)
            && let Some(base_class) = base_entry.as_class()
        {
            chain.push(base_class);
            current = base;
        }

        chain
    }

    /// Get all methods for a class, including inherited methods.
    ///
    /// Methods are returned in order: own methods first, then inherited.
    pub fn all_methods(&self, class_hash: TypeHash) -> Vec<&FunctionEntry> {
        let mut methods = Vec::new();

        // Own methods
        if let Some(entry) = self.types.get(&class_hash)
            && let Some(class) = entry.as_class()
        {
            for method_hash in &class.methods {
                if let Some(func) = self.functions.get(method_hash) {
                    methods.push(func);
                }
            }
        }

        // Inherited methods
        for base in self.base_class_chain(class_hash) {
            for method_hash in &base.methods {
                if let Some(func) = self.functions.get(method_hash) {
                    methods.push(func);
                }
            }
        }

        methods
    }

    /// Get all properties for a class, including inherited properties.
    ///
    /// Properties are returned in order: own properties first, then inherited.
    pub fn all_properties(&self, class_hash: TypeHash) -> Vec<&PropertyEntry> {
        let mut properties = Vec::new();

        // Own properties
        if let Some(entry) = self.types.get(&class_hash)
            && let Some(class) = entry.as_class()
        {
            properties.extend(class.properties.iter());
        }

        // Inherited properties
        for base in self.base_class_chain(class_hash) {
            properties.extend(base.properties.iter());
        }

        properties
    }

    // ==========================================================================
    // Namespace Helpers
    // ==========================================================================

    /// Iterate over types in a specific namespace.
    pub fn types_in_namespace<'a>(&'a self, ns: &'a str) -> impl Iterator<Item = &'a TypeEntry> {
        let prefix = if ns.is_empty() {
            String::new()
        } else {
            format!("{}::", ns)
        };

        self.types.values().filter(move |t| {
            let qname = t.qualified_name();
            if ns.is_empty() {
                // Root namespace: no :: in name
                !qname.contains("::")
            } else {
                qname.starts_with(&prefix)
            }
        })
    }

    /// Iterate over all registered namespaces.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        self.namespaces.iter().map(|s| s.as_str())
    }

    /// Check if a namespace is registered.
    pub fn has_namespace(&self, ns: &str) -> bool {
        self.namespaces.contains(ns)
    }

    // ==========================================================================
    // Global Properties
    // ==========================================================================

    /// Register a global property.
    ///
    /// Returns an error if a global with the same qualified name already exists.
    pub fn register_global(&mut self, entry: GlobalPropertyEntry) -> Result<(), RegistrationError> {
        let hash = entry.type_hash;

        if self.globals.contains_key(&hash) {
            return Err(RegistrationError::DuplicateRegistration {
                name: entry.qualified_name.clone(),
                kind: "global property".to_string(),
            });
        }

        self.globals.insert(hash, entry);
        Ok(())
    }

    /// Get a global property by its hash.
    pub fn get_global(&self, hash: TypeHash) -> Option<&GlobalPropertyEntry> {
        self.globals.get(&hash)
    }

    /// Get a global property by its qualified name.
    pub fn get_global_by_name(&self, name: &str) -> Option<&GlobalPropertyEntry> {
        self.globals.get(&TypeHash::from_name(name))
    }

    /// Check if a global property exists by hash.
    pub fn contains_global(&self, hash: TypeHash) -> bool {
        self.globals.contains_key(&hash)
    }

    /// Iterate over all global properties.
    pub fn globals(&self) -> impl Iterator<Item = &GlobalPropertyEntry> {
        self.globals.values()
    }

    /// Get the number of registered global properties.
    pub fn global_count(&self) -> usize {
        self.globals.len()
    }

    // ==========================================================================
    // Template Support
    // ==========================================================================

    /// Validate a template instantiation using the registered callback.
    ///
    /// Returns `TemplateValidation::valid()` if no callback is registered.
    pub fn validate_template_instance(&self, info: &TemplateInstanceInfo) -> TemplateValidation {
        let template_hash = TypeHash::from_name(&info.template_name);
        if let Some(callback) = self.template_callbacks.get(&template_hash) {
            callback(info)
        } else {
            TemplateValidation::valid()
        }
    }

    /// Check if a template has a validation callback registered.
    pub fn has_template_callback(&self, template: TypeHash) -> bool {
        self.template_callbacks.contains_key(&template)
    }
}

impl std::fmt::Debug for SymbolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolRegistry")
            .field("types", &self.types.len())
            .field("functions", &self.functions.len())
            .field("globals", &self.globals.len())
            .field("namespaces", &self.namespaces.len())
            .field("template_callbacks", &self.template_callbacks.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        primitives, DataType, FunctionDef, FunctionTraits, TypeKind, Visibility,
    };

    #[test]
    fn new_registry_is_empty() {
        let registry = SymbolRegistry::new();
        assert_eq!(registry.type_count(), 0);
        assert_eq!(registry.function_count(), 0);
    }

    #[test]
    fn register_all_primitives() {
        let registry = SymbolRegistry::with_primitives();
        assert_eq!(registry.type_count(), 12); // void, bool, int8..int64, uint8..uint64, float, double

        assert!(registry.get(primitives::INT32).is_some());
        assert!(registry.get(primitives::FLOAT).is_some());
        assert!(registry.get(primitives::VOID).is_some());
    }

    #[test]
    fn lookup_by_name() {
        let registry = SymbolRegistry::with_primitives();

        let int_type = registry.get_by_name("int");
        assert!(int_type.is_some());
        assert!(int_type.unwrap().is_primitive());
    }

    #[test]
    fn register_class() {
        let mut registry = SymbolRegistry::new();

        let class = ClassEntry::ffi("Player", TypeKind::reference());
        registry.register_type(class.into()).unwrap();

        assert!(registry.contains_type(TypeHash::from_name("Player")));
        assert!(registry.contains_type_name("Player"));
    }

    #[test]
    fn duplicate_type_error() {
        let mut registry = SymbolRegistry::new();

        let class1 = ClassEntry::ffi("Player", TypeKind::reference());
        let class2 = ClassEntry::ffi("Player", TypeKind::reference());

        registry.register_type(class1.into()).unwrap();
        let result = registry.register_type(class2.into());

        assert!(result.is_err());
    }

    #[test]
    fn register_function() {
        let mut registry = SymbolRegistry::new();

        let def = FunctionDef::new(
            TypeHash::from_function("print", &[primitives::INT32]),
            "print".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let entry = FunctionEntry::ffi(def);
        registry.register_function(entry).unwrap();

        assert_eq!(registry.function_count(), 1);
    }

    #[test]
    fn function_overloads() {
        let mut registry = SymbolRegistry::new();

        let def1 = FunctionDef::new(
            TypeHash::from_function("print", &[primitives::INT32]),
            "print".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let def2 = FunctionDef::new(
            TypeHash::from_function("print", &[primitives::STRING]),
            "print".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );

        registry.register_function(FunctionEntry::ffi(def1)).unwrap();
        registry.register_function(FunctionEntry::ffi(def2)).unwrap();

        let overloads = registry.get_function_overloads("print").unwrap();
        assert_eq!(overloads.len(), 2);
    }

    #[test]
    fn iterate_classes() {
        let mut registry = SymbolRegistry::with_primitives();

        registry
            .register_type(ClassEntry::ffi("Player", TypeKind::reference()).into())
            .unwrap();
        registry
            .register_type(ClassEntry::ffi("Enemy", TypeKind::reference()).into())
            .unwrap();
        registry
            .register_type(
                EnumEntry::ffi("Color")
                    .with_value("Red", 0)
                    .with_value("Green", 1)
                    .into(),
            )
            .unwrap();

        let classes: Vec<_> = registry.classes().collect();
        assert_eq!(classes.len(), 2);

        let enums: Vec<_> = registry.enums().collect();
        assert_eq!(enums.len(), 1);
    }

    #[test]
    fn inheritance_chain() {
        let mut registry = SymbolRegistry::new();

        let entity = ClassEntry::ffi("Entity", TypeKind::reference());
        let entity_hash = entity.type_hash;
        registry.register_type(entity.into()).unwrap();

        let player = ClassEntry::ffi("Player", TypeKind::reference()).with_base(entity_hash);
        let player_hash = player.type_hash;
        registry.register_type(player.into()).unwrap();

        let warrior =
            ClassEntry::ffi("Warrior", TypeKind::reference()).with_base(player_hash);
        let warrior_hash = warrior.type_hash;
        registry.register_type(warrior.into()).unwrap();

        let chain = registry.base_class_chain(warrior_hash);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].name, "Player");
        assert_eq!(chain[1].name, "Entity");
    }

    #[test]
    fn namespace_registration() {
        let mut registry = SymbolRegistry::new();

        registry.register_namespace("Game");
        registry.register_namespace("Game::Entities");

        assert!(registry.has_namespace("Game"));
        assert!(registry.has_namespace("Game::Entities"));
        assert!(!registry.has_namespace("Unknown"));
    }

    #[test]
    fn template_callback() {
        let mut registry = SymbolRegistry::new();

        let array_hash = TypeHash::from_name("array");
        registry.register_template_callback(
            array_hash,
            Box::new(|info| {
                if info.sub_types.is_empty() {
                    TemplateValidation::invalid("array requires a type argument")
                } else if info.sub_types[0].is_void() {
                    TemplateValidation::invalid("array cannot hold void")
                } else {
                    TemplateValidation::valid()
                }
            }),
        );

        assert!(registry.has_template_callback(array_hash));

        // Valid instantiation
        let valid_info = TemplateInstanceInfo::new("array", vec![DataType::simple(primitives::INT32)]);
        let result = registry.validate_template_instance(&valid_info);
        assert!(result.is_valid);

        // Invalid instantiation (void)
        let void_info = TemplateInstanceInfo::new("array", vec![DataType::void()]);
        let result = registry.validate_template_instance(&void_info);
        assert!(!result.is_valid);

        // No callback registered - should return valid
        let dict_info = TemplateInstanceInfo::new("dictionary", vec![]);
        let result = registry.validate_template_instance(&dict_info);
        assert!(result.is_valid);
    }

    #[test]
    fn debug_impl() {
        let registry = SymbolRegistry::with_primitives();
        let debug_str = format!("{:?}", registry);
        assert!(debug_str.contains("SymbolRegistry"));
        assert!(debug_str.contains("types"));
    }

    #[test]
    fn register_global_property() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("PI", ConstantValue::Double(3.14159));
        registry.register_global(entry).unwrap();

        assert_eq!(registry.global_count(), 1);
        assert!(registry.contains_global(TypeHash::from_name("PI")));
    }

    #[test]
    fn get_global_by_name() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("MAX_PLAYERS", ConstantValue::Int32(64));
        registry.register_global(entry).unwrap();

        let global = registry.get_global_by_name("MAX_PLAYERS").unwrap();
        assert_eq!(global.name, "MAX_PLAYERS");
        assert!(global.is_const);
    }

    #[test]
    fn duplicate_global_error() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        let entry1 = GlobalPropertyEntry::constant("PI", ConstantValue::Double(3.14));
        let entry2 = GlobalPropertyEntry::constant("PI", ConstantValue::Double(3.14159));

        registry.register_global(entry1).unwrap();
        let result = registry.register_global(entry2);

        assert!(result.is_err());
    }

    #[test]
    fn iterate_globals() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        registry
            .register_global(GlobalPropertyEntry::constant("PI", ConstantValue::Double(3.14)))
            .unwrap();
        registry
            .register_global(GlobalPropertyEntry::constant("E", ConstantValue::Double(2.71)))
            .unwrap();

        let globals: Vec<_> = registry.globals().collect();
        assert_eq!(globals.len(), 2);
    }
}
