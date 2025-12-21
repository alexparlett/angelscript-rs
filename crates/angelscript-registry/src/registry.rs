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
//! # Thread Safety
//!
//! `SymbolRegistry` is **not thread-safe** by design. In the typical usage pattern:
//!
//! - **Registration phase**: The registry is populated single-threaded during
//!   context setup and script compilation. FFI types are registered first,
//!   then script types are added during compilation passes.
//!
//! - **Execution phase**: After compilation, the registry becomes effectively
//!   read-only. If multi-threaded execution is needed, the caller must wrap
//!   the registry in appropriate synchronization (e.g., `Arc<RwLock<_>>`).
//!
//! This design follows the pattern of most scripting engines where type
//! registration and script execution are distinct phases.
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
    PrimitiveEntry, PrimitiveKind, PropertyEntry, RegistrationError, TemplateParamEntry, TypeEntry,
    TypeHash,
};

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

    /// Global properties by hash (O(1) lookup).
    /// Hash is computed from qualified name via `TypeHash::from_name()`.
    globals: FxHashMap<TypeHash, GlobalPropertyEntry>,

    // === Namespace-Partitioned Indexes (for O(1) scope building) ===
    /// Types indexed by namespace: namespace -> (simple_name -> hash).
    types_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,

    /// Functions indexed by namespace: namespace -> (simple_name -> [hashes]).
    /// Multiple hashes because functions can have overloads.
    functions_by_namespace: FxHashMap<String, FxHashMap<String, Vec<TypeHash>>>,

    /// Globals indexed by namespace: namespace -> (simple_name -> hash).
    globals_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,

    /// Type aliases (typedef): alias_name -> target_type_hash.
    /// These are resolved during type lookup to their target types.
    type_aliases: FxHashMap<String, TypeHash>,

    /// Type aliases indexed by namespace: namespace -> (simple_name -> target_hash).
    type_aliases_by_namespace: FxHashMap<String, FxHashMap<String, TypeHash>>,
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

    /// Get a mutable reference to a class entry by hash.
    ///
    /// Returns `None` if the type doesn't exist or is not a class.
    pub fn get_class_mut(&mut self, hash: TypeHash) -> Option<&mut ClassEntry> {
        self.get_mut(hash)?.as_class_mut()
    }

    /// Get a mutable reference to an interface entry by hash.
    ///
    /// Returns `None` if the type doesn't exist or is not an interface.
    pub fn get_interface_mut(&mut self, hash: TypeHash) -> Option<&mut InterfaceEntry> {
        self.get_mut(hash)?.as_interface_mut()
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
    ///
    /// Note: FFI classes do not support inheritance. Script classes can only
    /// inherit from other script classes.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistrationError> {
        let hash = entry.type_hash();

        // Check for duplicates BEFORE allocating strings
        if self.types.contains_key(&hash) {
            return Err(RegistrationError::DuplicateType(
                entry.qualified_name().to_string(),
            ));
        }

        // Now allocate strings for successful registration
        let qualified_name = entry.qualified_name().to_string();
        let simple_name = entry.name().to_string();
        let namespace = entry.namespace().join("::");

        // Add to namespace index (skip template params - they belong to their owner)
        if !entry.is_template_param() {
            self.types_by_namespace
                .entry(namespace)
                .or_default()
                .insert(simple_name, hash);
        }

        self.type_by_name.insert(qualified_name, hash);
        self.types.insert(hash, entry);
        Ok(())
    }

    /// Register a function entry.
    ///
    /// Returns an error if a function with the same hash already exists.
    pub fn register_function(&mut self, entry: FunctionEntry) -> Result<(), RegistrationError> {
        let hash = entry.def.func_hash;

        // Check for duplicates BEFORE allocating strings
        if self.functions.contains_key(&hash) {
            return Err(RegistrationError::DuplicateRegistration {
                name: entry.def.qualified_name().to_string(),
                kind: "function".to_string(),
            });
        }

        // Now allocate strings for successful registration
        let qualified_name = entry.def.qualified_name();
        let simple_name = entry.def.name.clone();
        let namespace = entry.def.namespace.join("::");

        // Add to namespace index (only for global functions, not methods)
        if entry.def.object_type.is_none() {
            self.functions_by_namespace
                .entry(namespace)
                .or_default()
                .entry(simple_name)
                .or_default()
                .push(hash);
        }

        // If this is a method, add to the owning class's vtable
        if let Some(owner_hash) = entry.def.object_type {
            // Compute signature hash from params (including modifiers)
            let param_sig_hashes: Vec<u64> = entry
                .def
                .params
                .iter()
                .map(|p| p.data_type.signature_hash())
                .collect();
            let sig_hash =
                TypeHash::from_signature(&entry.def.name, &param_sig_hashes, entry.def.is_const())
                    .0;

            // Add to class vtable if the class exists
            if let Some(class) = self.get_class_mut(owner_hash) {
                class.vtable.add_method(&entry.def.name, sig_hash, hash);
            }
        }

        self.function_overloads
            .entry(qualified_name.to_string())
            .or_default()
            .push(hash);
        self.functions.insert(hash, entry);
        Ok(())
    }

    /// Register a primitive type.
    ///
    /// Primitives are always registered (no duplicate check).
    /// They are always in the global namespace (empty string key).
    pub fn register_primitive(&mut self, entry: PrimitiveEntry) {
        let hash = entry.type_hash;
        let name = entry.name().to_string();

        // Add to namespace index (global namespace = empty string)
        self.types_by_namespace
            .entry(String::new())
            .or_default()
            .insert(name.clone(), hash);

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
            for method_hash in class.all_methods() {
                if let Some(func) = self.functions.get(&method_hash) {
                    methods.push(func);
                }
            }
        }

        // Inherited methods
        for base in self.base_class_chain(class_hash) {
            for method_hash in base.all_methods() {
                if let Some(func) = self.functions.get(&method_hash) {
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
    ///
    /// Uses the namespace index for O(n) iteration where n is the number of types
    /// in the namespace, rather than O(N) iteration over all types.
    pub fn types_in_namespace<'a>(&'a self, ns: &'a str) -> impl Iterator<Item = &'a TypeEntry> {
        self.types_by_namespace
            .get(ns)
            .into_iter()
            .flat_map(|name_map| name_map.values())
            .filter_map(|hash| self.types.get(hash))
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

        // Check for duplicates BEFORE allocating strings
        if self.globals.contains_key(&hash) {
            return Err(RegistrationError::DuplicateRegistration {
                name: entry.qualified_name.clone(),
                kind: "global property".to_string(),
            });
        }

        // Now allocate strings for successful registration
        let simple_name = entry.name.clone();
        let namespace = entry.namespace.join("::");

        // Add to namespace index
        self.globals_by_namespace
            .entry(namespace)
            .or_default()
            .insert(simple_name, hash);

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
    // Namespace Index Access
    // ==========================================================================

    /// Get all types in a namespace.
    ///
    /// Returns a map of simple name -> TypeHash for all types in the namespace.
    /// Use empty string for the global namespace.
    pub fn get_namespace_types(&self, namespace: &str) -> Option<&FxHashMap<String, TypeHash>> {
        self.types_by_namespace.get(namespace)
    }

    /// Get all functions in a namespace.
    ///
    /// Returns a map of simple name -> Vec<TypeHash> for all functions in the namespace.
    /// Multiple hashes per name indicate overloads.
    /// Use empty string for the global namespace.
    pub fn get_namespace_functions(
        &self,
        namespace: &str,
    ) -> Option<&FxHashMap<String, Vec<TypeHash>>> {
        self.functions_by_namespace.get(namespace)
    }

    /// Get all globals in a namespace.
    ///
    /// Returns a map of simple name -> TypeHash for all global properties in the namespace.
    /// Use empty string for the global namespace.
    pub fn get_namespace_globals(&self, namespace: &str) -> Option<&FxHashMap<String, TypeHash>> {
        self.globals_by_namespace.get(namespace)
    }

    // ==========================================================================
    // Validation
    // ==========================================================================

    /// Validate all FFI types have correct behaviors for their type kinds.
    ///
    /// This should be called during module installation to ensure:
    /// - Standard reference types have AddRef and Release
    /// - Scoped types have Release
    /// - Non-POD value types have constructor and destructor
    /// - NoCount/NoHandle types don't have forbidden behaviors
    ///
    /// Only validates FFI types - script types have behaviors auto-generated.
    ///
    /// Returns a list of all validation errors found.
    pub fn validate_ffi_behaviors(&self) -> Vec<RegistrationError> {
        let mut errors = Vec::new();

        for entry in self.types.values() {
            if let TypeEntry::Class(class) = entry {
                // Skip non-FFI types (script types have auto-generated behaviors)
                if !class.source.is_ffi() {
                    continue;
                }

                // Skip template definitions (validated when instantiated)
                if class.is_template() {
                    continue;
                }

                let result = class.behaviors.validate(&class.type_kind);
                if !result.is_ok() {
                    errors.extend(result.into_errors(class.qualified_name.clone()));
                }
            }
        }

        errors
	}

    // ==========================================================================
    // Type Aliases (typedef)
    // ==========================================================================

    /// Register a type alias.
    ///
    /// Creates a mapping from alias_name to target_type_hash. The alias behaves
    /// as an alternative name for the target type (not a distinct type).
    ///
    /// # Arguments
    /// * `alias_name` - Simple name of the alias (e.g., "EntityId")
    /// * `namespace` - Namespace parts (empty for global namespace)
    /// * `target_hash` - Hash of the target type this alias refers to
    pub fn register_type_alias(
        &mut self,
        alias_name: &str,
        namespace: &[String],
        target_hash: TypeHash,
    ) -> Result<(), RegistrationError> {
        let qualified_name = if namespace.is_empty() {
            alias_name.to_string()
        } else {
            format!("{}::{}", namespace.join("::"), alias_name)
        };

        // Check for duplicate alias
        if self.type_aliases.contains_key(&qualified_name) {
            return Err(RegistrationError::DuplicateRegistration {
                name: qualified_name,
                kind: "type alias".to_string(),
            });
        }

        // Add to qualified name lookup
        self.type_aliases.insert(qualified_name, target_hash);

        // Add to namespace index
        let ns_key = namespace.join("::");
        self.type_aliases_by_namespace
            .entry(ns_key)
            .or_default()
            .insert(alias_name.to_string(), target_hash);

        Ok(())
    }

    /// Get a type alias by its qualified name.
    ///
    /// Returns the target TypeHash if the alias exists.
    pub fn get_type_alias(&self, qualified_name: &str) -> Option<TypeHash> {
        self.type_aliases.get(qualified_name).copied()
    }

    /// Get type aliases in a namespace.
    ///
    /// Returns a map of simple name -> target TypeHash for all aliases in the namespace.
    /// Use empty string for the global namespace.
    pub fn get_namespace_type_aliases(
        &self,
        namespace: &str,
    ) -> Option<&FxHashMap<String, TypeHash>> {
        self.type_aliases_by_namespace.get(namespace)
    }

    /// Check if a type alias exists by qualified name.
    pub fn contains_type_alias(&self, qualified_name: &str) -> bool {
        self.type_aliases.contains_key(qualified_name)
    }

    /// Get the number of registered type aliases.
    pub fn type_alias_count(&self) -> usize {
        self.type_aliases.len()
    }
}

impl std::fmt::Debug for SymbolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymbolRegistry")
            .field("types", &self.types.len())
            .field("functions", &self.functions.len())
            .field("globals", &self.globals.len())
            .field("namespaces", &self.namespaces.len())
            .field("type_aliases", &self.type_aliases.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        DataType, FunctionDef, FunctionTraits, Param, TypeKind, Visibility, primitives,
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

        registry
            .register_function(FunctionEntry::ffi(def1))
            .unwrap();
        registry
            .register_function(FunctionEntry::ffi(def2))
            .unwrap();

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

        let warrior = ClassEntry::ffi("Warrior", TypeKind::reference()).with_base(player_hash);
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

        let entry = GlobalPropertyEntry::constant("GRAVITY", ConstantValue::Double(9.81));
        registry.register_global(entry).unwrap();

        assert_eq!(registry.global_count(), 1);
        assert!(registry.contains_global(TypeHash::from_name("GRAVITY")));
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

        let entry1 = GlobalPropertyEntry::constant("SPEED", ConstantValue::Double(100.0));
        let entry2 = GlobalPropertyEntry::constant("SPEED", ConstantValue::Double(200.0));

        registry.register_global(entry1).unwrap();
        let result = registry.register_global(entry2);

        assert!(result.is_err());
    }

    #[test]
    fn iterate_globals() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        registry
            .register_global(GlobalPropertyEntry::constant(
                "GRAVITY",
                ConstantValue::Double(9.81),
            ))
            .unwrap();
        registry
            .register_global(GlobalPropertyEntry::constant(
                "SPEED",
                ConstantValue::Double(100.0),
            ))
            .unwrap();

        let globals: Vec<_> = registry.globals().collect();
        assert_eq!(globals.len(), 2);
    }

    // =========================================================================
    // Namespace Index Tests
    // =========================================================================

    #[test]
    fn namespace_index_types_global_namespace() {
        let mut registry = SymbolRegistry::new();

        // Register a class in the global namespace (empty namespace)
        let class = ClassEntry::ffi("Player", TypeKind::reference());
        let hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Should be indexed under empty string namespace
        let types = registry.get_namespace_types("").unwrap();
        assert_eq!(types.get("Player"), Some(&hash));
    }

    #[test]
    fn namespace_index_types_with_namespace() {
        use angelscript_core::TypeSource;

        let mut registry = SymbolRegistry::new();

        // Register a class in Game namespace
        let class = ClassEntry::new(
            "Player",
            vec!["Game".to_string()],
            "Game::Player",
            TypeHash::from_name("Game::Player"),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        let hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Should be indexed under "Game" namespace
        let types = registry.get_namespace_types("Game").unwrap();
        assert_eq!(types.get("Player"), Some(&hash));

        // Should NOT be in global namespace
        assert!(
            registry.get_namespace_types("").is_none()
                || registry
                    .get_namespace_types("")
                    .unwrap()
                    .get("Player")
                    .is_none()
        );
    }

    #[test]
    fn namespace_index_types_nested_namespace() {
        use angelscript_core::TypeSource;

        let mut registry = SymbolRegistry::new();

        // Register class in Game::Entities namespace
        let class = ClassEntry::new(
            "Enemy",
            vec!["Game".to_string(), "Entities".to_string()],
            "Game::Entities::Enemy",
            TypeHash::from_name("Game::Entities::Enemy"),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        let hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Should be indexed under "Game::Entities" namespace
        let types = registry.get_namespace_types("Game::Entities").unwrap();
        assert_eq!(types.get("Enemy"), Some(&hash));
    }

    #[test]
    fn namespace_index_functions_global_namespace() {
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
        let hash = def.func_hash;
        registry.register_function(FunctionEntry::ffi(def)).unwrap();

        // Should be indexed under empty string namespace
        let funcs = registry.get_namespace_functions("").unwrap();
        assert!(funcs.get("print").unwrap().contains(&hash));
    }

    #[test]
    fn namespace_index_functions_with_namespace() {
        let mut registry = SymbolRegistry::new();

        let mut def = FunctionDef::new(
            TypeHash::from_function("Game::log", &[primitives::INT32]),
            "log".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        def.namespace = vec!["Game".to_string()];
        let hash = def.func_hash;
        registry.register_function(FunctionEntry::ffi(def)).unwrap();

        // Should be indexed under "Game" namespace
        let funcs = registry.get_namespace_functions("Game").unwrap();
        assert!(funcs.get("log").unwrap().contains(&hash));
    }

    #[test]
    fn namespace_index_functions_overloads() {
        let mut registry = SymbolRegistry::new();

        // Register two overloads of the same function
        let mut def1 = FunctionDef::new(
            TypeHash::from_function("Game::log", &[primitives::INT32]),
            "log".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        def1.namespace = vec!["Game".to_string()];
        let hash1 = def1.func_hash;

        let mut def2 = FunctionDef::new(
            TypeHash::from_function("Game::log", &[primitives::STRING]),
            "log".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        def2.namespace = vec!["Game".to_string()];
        let hash2 = def2.func_hash;

        registry
            .register_function(FunctionEntry::ffi(def1))
            .unwrap();
        registry
            .register_function(FunctionEntry::ffi(def2))
            .unwrap();

        // Both overloads should be indexed under "Game" namespace
        let funcs = registry.get_namespace_functions("Game").unwrap();
        let log_overloads = funcs.get("log").unwrap();
        assert_eq!(log_overloads.len(), 2);
        assert!(log_overloads.contains(&hash1));
        assert!(log_overloads.contains(&hash2));
    }

    #[test]
    fn namespace_index_methods_not_indexed() {
        let mut registry = SymbolRegistry::new();

        // Register a method (has object_type)
        let def = FunctionDef::new(
            TypeHash::from_function("Player::update", &[]),
            "update".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(TypeHash::from_name("Player")), // object_type makes it a method
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        registry.register_function(FunctionEntry::ffi(def)).unwrap();

        // Methods should NOT be indexed by namespace (only global functions are)
        assert!(
            registry.get_namespace_functions("").is_none()
                || registry
                    .get_namespace_functions("")
                    .unwrap()
                    .get("update")
                    .is_none()
        );
    }

    #[test]
    fn namespace_index_globals_global_namespace() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        let entry = GlobalPropertyEntry::constant("GRAVITY", ConstantValue::Double(9.81));
        let hash = entry.type_hash;
        registry.register_global(entry).unwrap();

        // Should be indexed under empty string namespace
        let globals = registry.get_namespace_globals("").unwrap();
        assert_eq!(globals.get("GRAVITY"), Some(&hash));
    }

    #[test]
    fn namespace_index_globals_with_namespace() {
        use angelscript_core::ConstantValue;

        let mut registry = SymbolRegistry::new();

        let mut entry = GlobalPropertyEntry::constant("MAX_ENEMIES", ConstantValue::Int32(100));
        entry = entry.with_namespace(vec!["Game".to_string()]);
        let hash = entry.type_hash;
        registry.register_global(entry).unwrap();

        // Should be indexed under "Game" namespace
        let globals = registry.get_namespace_globals("Game").unwrap();
        assert_eq!(globals.get("MAX_ENEMIES"), Some(&hash));
    }

    #[test]
    fn namespace_index_empty_namespace_returns_none() {
        let registry = SymbolRegistry::new();

        // Empty registry should return None for any namespace
        assert!(registry.get_namespace_types("Game").is_none());
        assert!(registry.get_namespace_functions("Game").is_none());
        assert!(registry.get_namespace_globals("Game").is_none());
    }

    #[test]
    fn namespace_index_primitives_in_global_namespace() {
        let registry = SymbolRegistry::with_primitives();

        // Primitives should be indexed in global namespace
        let types = registry.get_namespace_types("").unwrap();
        assert!(types.get("int").is_some());
        assert!(types.get("float").is_some());
        assert!(types.get("bool").is_some());
    }

    // =========================================================================
    // Behavior Validation Tests
    // =========================================================================

    #[test]
    fn validate_standard_ref_missing_behaviors() {
        let mut registry = SymbolRegistry::new();

        // Standard reference type without AddRef/Release
        let class = ClassEntry::ffi("BadRefType", TypeKind::reference());
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, RegistrationError::MissingBehaviors { missing, .. } if missing.contains(&"AddRef"))));
    }

    #[test]
    fn validate_standard_ref_with_behaviors() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // Standard reference type with proper behaviors
        let mut class = ClassEntry::ffi("GoodRefType", TypeKind::reference());
        class.behaviors.set_addref(TypeHash::from_name("addref"));
        class.behaviors.set_release(TypeHash::from_name("release"));
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_nocount_forbids_addref_release() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // NoCount type with forbidden AddRef
        let mut class = ClassEntry::ffi("BadNoCount", TypeKind::no_count());
        class.behaviors.set_addref(TypeHash::from_name("addref"));
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(
            e,
            RegistrationError::ForbiddenBehavior {
                behavior: "AddRef",
                ..
            }
        )));
    }

    #[test]
    fn validate_nocount_without_forbidden_behaviors() {
        let mut registry = SymbolRegistry::new();

        // NoCount type without any behaviors (valid)
        let class = ClassEntry::ffi("GoodNoCount", TypeKind::no_count());
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_nohandle_forbids_factory() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // NoHandle type with forbidden factory
        let mut class = ClassEntry::ffi("BadNoHandle", TypeKind::no_handle());
        class.behaviors.add_factory(TypeHash::from_name("factory"));
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(
            e,
            RegistrationError::ForbiddenBehavior {
                behavior: "Factory",
                ..
            }
        )));
    }

    #[test]
    fn validate_scoped_needs_release() {
        let mut registry = SymbolRegistry::new();

        // Scoped type without Release
        let class = ClassEntry::ffi("BadScoped", TypeKind::scoped());
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(e, RegistrationError::MissingBehaviors { missing, .. } if missing.contains(&"Release"))));
    }

    #[test]
    fn validate_scoped_with_release() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // Scoped type with Release (valid)
        let mut class = ClassEntry::ffi("GoodScoped", TypeKind::scoped());
        class.behaviors.set_release(TypeHash::from_name("release"));
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_scoped_forbids_addref() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // Scoped type with forbidden AddRef
        let mut class = ClassEntry::ffi("BadScoped", TypeKind::scoped());
        class.behaviors.set_addref(TypeHash::from_name("addref"));
        class.behaviors.set_release(TypeHash::from_name("release"));
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| matches!(
            e,
            RegistrationError::ForbiddenBehavior {
                behavior: "AddRef",
                ..
            }
        )));
    }

    #[test]
    fn validate_skips_templates() {
        use angelscript_core::TypeHash;

        let mut registry = SymbolRegistry::new();

        // Template definition without behaviors (should be skipped)
        let mut class = ClassEntry::ffi("array", TypeKind::reference());
        class.template_params = vec![TypeHash::from_name("array::T")];
        registry.register_type(class.into()).unwrap();

        let errors = registry.validate_ffi_behaviors();
        // Templates are skipped, so no errors
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_behaviors_directly() {
        use angelscript_core::TypeHash;

        // Test validating behaviors directly on a class
        let mut class = ClassEntry::ffi("TestClass", TypeKind::reference());
        class.behaviors.set_addref(TypeHash::from_name("addref"));
        class.behaviors.set_release(TypeHash::from_name("release"));

        let result = class.behaviors.validate(&class.type_kind);
        assert!(result.is_ok());

        // Now test with missing behaviors
        let bad_class = ClassEntry::ffi("BadClass", TypeKind::reference());
        let result = bad_class.behaviors.validate(&bad_class.type_kind);
        assert!(!result.is_ok());
        assert!(result.missing.contains(&"AddRef"));
    }

    // =========================================================================
    // VTable Building Tests
    // =========================================================================

    #[test]
    fn register_function_adds_method_to_vtable() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a class first
        let class = ClassEntry::ffi("Entity", TypeKind::reference());
        let class_hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Register a method for that class
        let method_def = FunctionDef::new(
            TypeHash::from_method(class_hash, "update", &[]),
            "update".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        registry
            .register_function(FunctionEntry::ffi(method_def))
            .unwrap();

        // Verify vtable was populated
        let class = registry.get_class_mut(class_hash).unwrap();
        assert_eq!(class.vtable.len(), 1);
        assert!(!class.vtable.slots_for_name("update").is_empty());
    }

    #[test]
    fn register_function_multiple_methods_same_class() {
        let mut registry = SymbolRegistry::with_primitives();

        let class = ClassEntry::ffi("Entity", TypeKind::reference());
        let class_hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Register two methods
        let update_def = FunctionDef::new(
            TypeHash::from_method(class_hash, "update", &[]),
            "update".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        let render_def = FunctionDef::new(
            TypeHash::from_method(class_hash, "render", &[]),
            "render".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );

        registry
            .register_function(FunctionEntry::ffi(update_def))
            .unwrap();
        registry
            .register_function(FunctionEntry::ffi(render_def))
            .unwrap();

        let class = registry.get_class_mut(class_hash).unwrap();
        assert_eq!(class.vtable.len(), 2);
    }

    #[test]
    fn register_function_overloads_get_separate_slots() {
        let mut registry = SymbolRegistry::with_primitives();

        let class = ClassEntry::ffi("Entity", TypeKind::reference());
        let class_hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Register two overloads of "foo" with different signatures
        let foo_int = FunctionDef::new(
            TypeHash::from_method(class_hash, "foo", &[primitives::INT32]),
            "foo".to_string(),
            vec![],
            vec![Param::new("x", DataType::simple(primitives::INT32))],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );
        let foo_float = FunctionDef::new(
            TypeHash::from_method(class_hash, "foo", &[primitives::FLOAT]),
            "foo".to_string(),
            vec![],
            vec![Param::new("x", DataType::simple(primitives::FLOAT))],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );

        registry
            .register_function(FunctionEntry::ffi(foo_int))
            .unwrap();
        registry
            .register_function(FunctionEntry::ffi(foo_float))
            .unwrap();

        let class = registry.get_class_mut(class_hash).unwrap();
        assert_eq!(class.vtable.len(), 2);
        // Both should be under "foo" name
        assert_eq!(class.vtable.slots_for_name("foo").len(), 2);
    }

    #[test]
    fn register_function_const_and_nonconst_get_separate_slots() {
        let mut registry = SymbolRegistry::with_primitives();

        let class = ClassEntry::ffi("Entity", TypeKind::reference());
        let class_hash = class.type_hash;
        registry.register_type(class.into()).unwrap();

        // Register const and non-const versions of same method
        let foo_nonconst = FunctionDef::new(
            TypeHash::from_method(class_hash, "foo", &[]),
            "foo".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::default(), // not const
            true,
            Visibility::Public,
        );
        let foo_const = FunctionDef::new(
            TypeHash::from_method(class_hash, "foo_const", &[]),
            "foo".to_string(),
            vec![],
            vec![],
            DataType::void(),
            Some(class_hash),
            FunctionTraits::const_method(), // const
            true,
            Visibility::Public,
        );

        registry
            .register_function(FunctionEntry::ffi(foo_nonconst))
            .unwrap();
        registry
            .register_function(FunctionEntry::ffi(foo_const))
            .unwrap();

        let class = registry.get_class_mut(class_hash).unwrap();
        // const and non-const should be separate overloads
        assert_eq!(class.vtable.len(), 2);
        assert_eq!(class.vtable.slots_for_name("foo").len(), 2);
    }

    #[test]
    fn register_function_global_function_no_vtable() {
        let mut registry = SymbolRegistry::with_primitives();

        // Register a global function (no object_type)
        let func_def = FunctionDef::new(
            TypeHash::from_function("globalFunc", &[]),
            "globalFunc".to_string(),
            vec![],
            vec![],
            DataType::void(),
            None, // No object_type = global function
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );

        registry
            .register_function(FunctionEntry::ffi(func_def))
            .unwrap();

        // Should be registered as a function, not touch any vtable
        assert!(
            registry
                .get_function(TypeHash::from_function("globalFunc", &[]))
                .is_some()
        );
    }
}
