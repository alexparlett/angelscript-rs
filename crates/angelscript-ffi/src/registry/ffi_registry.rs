//! Immutable FFI registry shared across all compilation Units.
//!
//! This module provides `FfiRegistry`, an immutable registry holding all resolved
//! FFI data (types, functions, behaviors) that can be shared via `Arc` across
//! multiple compilation Units.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Context                               │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │              Arc<FfiRegistry>                        │    │
//! │  │  (immutable, shared across all Units)                │    │
//! │  │  - FFI types (TypeDef)                               │    │
//! │  │  - FFI functions (ResolvedFfiFunctionDef)            │    │
//! │  │  - FFI behaviors (TypeBehaviors)                     │    │
//! │  │  - Template callbacks                                │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! │                           │                                  │
//! │                    Arc::clone()                              │
//! │                           ▼                                  │
//! │  ┌─────────────────────────────────────────────────────┐    │
//! │  │              Unit (per compilation)                  │    │
//! │  │  ┌───────────────────────────────────────────────┐  │    │
//! │  │  │            Registry<'ast>                      │  │    │
//! │  │  │  - ffi: Arc<FfiRegistry>  (shared, immutable) │  │    │
//! │  │  │  - script_types: HashMap<TypeHash, TypeDef>     │  │    │
//! │  │  │  - script_functions: HashMap<TypeHash, ...> │  │    │
//! │  │  │  - template_cache: HashMap<...>               │  │    │
//! │  │  └───────────────────────────────────────────────┘  │    │
//! │  └─────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Build the registry during Context sealing
//! let mut builder = FfiRegistryBuilder::new();
//! builder.register_type(type_def, Some("MyClass"));
//! builder.register_function(ffi_func_def);
//! let registry = Arc::new(builder.build()?);
//!
//! // Share across Units
//! let unit1_registry = Registry::with_ffi(Arc::clone(&registry));
//! let unit2_registry = Registry::with_ffi(Arc::clone(&registry));
//! ```

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{NativeFn, TemplateInstanceInfo, TemplateValidation};
use angelscript_core::TypeBehaviors;
use angelscript_core::{
    FunctionDef, MethodSignature, OperatorBehavior, Param, PropertyAccessors, TypeDef,
    PrimitiveKind, FunctionTraits, Visibility, RegistrationError,
};
use angelscript_core::primitives;
use angelscript_core::DataType;
use angelscript_core::TypeHash;

/// Immutable FFI registry, shared across all Units in a Context.
///
/// This registry holds all FFI types, functions, and behaviors that have been
/// resolved and are ready for use during compilation. It is constructed once
/// during Context sealing and then shared via `Arc` across all compilation Units.
///
/// Template instances are NOT stored here - they are created per-Unit during
/// compilation and cached in the per-Unit Registry.
///
/// # Type Identity
///
/// Types and functions are identified by `TypeHash` - a deterministic 64-bit hash
/// computed from the qualified name (for types) or name+signature (for functions).
/// This enables forward references and eliminates registration order dependencies.
pub struct FfiRegistry {
    // === Type Storage (TypeHash primary key) ===
    /// All FFI types indexed by TypeHash
    types: FxHashMap<TypeHash, TypeDef>,
    /// Cached type name → TypeHash mapping (built once during registry creation)
    type_by_name: FxHashMap<String, TypeHash>,

    // === Function Storage (TypeHash primary key) ===
    /// All FFI functions indexed by TypeHash (resolved, non-template)
    functions: FxHashMap<TypeHash, FunctionDef>,
    /// Function name to TypeHash mapping (supports overloads)
    function_overloads: FxHashMap<String, Vec<TypeHash>>,
    /// Native function implementations indexed by TypeHash
    native_fns: FxHashMap<TypeHash, NativeFn>,

    // === Behavior Storage ===
    /// Type behaviors (constructors, factories, etc.) indexed by TypeHash
    behaviors: FxHashMap<TypeHash, TypeBehaviors>,

    // === Template Support ===
    /// Template validation callbacks indexed by TypeHash
    template_callbacks:
        FxHashMap<TypeHash, Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

    // === Namespace Tracking ===
    /// All registered namespaces
    namespaces: FxHashSet<String>,
}

impl std::fmt::Debug for FfiRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiRegistry")
            .field("types", &format!("<{} types>", self.types.len()))
            .field("type_by_name", &format!("<{} names>", self.type_by_name.len()))
            .field("functions", &format!("<{} functions>", self.functions.len()))
            .field("function_overloads", &format!("<{} names>", self.function_overloads.len()))
            .field("native_fns", &format!("<{} native fns>", self.native_fns.len()))
            .field("behaviors", &format!("<{} behaviors>", self.behaviors.len()))
            .field("template_callbacks", &format!("<{} callbacks>", self.template_callbacks.len()))
            .field("namespaces", &self.namespaces)
            .finish()
    }
}

impl FfiRegistry {
    // =========================================================================
    // Type Lookups
    // =========================================================================

    /// Get a type definition by TypeHash.
    ///
    /// TypeHash is now the primary key for all lookups.
    pub fn get_type(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types.get(&hash)
    }

    /// Look up a TypeHash by type name.
    ///
    /// Computes the TypeHash from the name and returns it if the type exists.
    pub fn get_type_by_name(&self, name: &str) -> Option<TypeHash> {
        let hash = TypeHash::from_name(name);
        if self.types.contains_key(&hash) {
            Some(hash)
        } else {
            None
        }
    }

    /// Get access to the type name → TypeHash map for iteration.
    ///
    /// This map is cached at registry build time for O(1) access.
    /// Used by CompilationContext for initializing its unified name maps.
    pub fn type_by_name(&self) -> &FxHashMap<String, TypeHash> {
        &self.type_by_name
    }

    /// Get the number of registered types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get a type definition by TypeHash.
    ///
    /// This is the primary lookup method in the TypeHash-based architecture.
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types.get(&hash)
    }

    /// Check if a type exists by TypeHash.
    pub fn has_type(&self, hash: TypeHash) -> bool {
        self.types.contains_key(&hash)
    }

    // =========================================================================
    // Function Lookups
    // =========================================================================

    /// Get a function definition by TypeHash (func_hash).
    ///
    /// TypeHash is now the primary key for function lookups.
    pub fn get_function(&self, hash: TypeHash) -> Option<&FunctionDef> {
        self.functions.get(&hash)
    }

    /// Look up all function hashes with the given name (for overload resolution).
    ///
    /// Returns TypeHashes which can be used with `get_function_by_hash`.
    pub fn lookup_function_hashes(&self, name: &str) -> &[TypeHash] {
        self.function_overloads
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up all functions with the given name (for overload resolution).
    ///
    /// Returns TypeHashes (func_hash) which are now the primary key.
    pub fn lookup_functions(&self, name: &str) -> Vec<TypeHash> {
        self.function_overloads
            .get(name)
            .map(|hashes| {
                hashes
                    .iter()
                    .filter_map(|hash| self.functions.get(hash).map(|f| f.func_hash))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the native function implementation by TypeHash (func_hash).
    pub fn get_native_fn(&self, hash: TypeHash) -> Option<&NativeFn> {
        self.native_fns.get(&hash)
    }

    /// Get the native function implementation by TypeHash.
    pub fn get_native_fn_by_hash(&self, hash: TypeHash) -> Option<&NativeFn> {
        self.native_fns.get(&hash)
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get access to the function name → func_hashes map for iteration.
    ///
    /// Returns a reference to the internal map of function names to TypeHashes.
    pub fn func_by_name(&self) -> &FxHashMap<String, Vec<TypeHash>> {
        &self.function_overloads
    }

    /// Get a function definition by TypeHash.
    ///
    /// This is the primary lookup method in the TypeHash-based architecture.
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionDef> {
        self.functions.get(&hash)
    }

    /// Get the TypeHash for a TypeHash.
    pub fn get_function_id_by_hash(&self, hash: TypeHash) -> Option<TypeHash> {
        self.functions.get(&hash).map(|f| f.func_hash)
    }

    /// Check if a function exists by TypeHash.
    pub fn has_function(&self, hash: TypeHash) -> bool {
        self.functions.contains_key(&hash)
    }

    // =========================================================================
    // Behavior Lookups
    // =========================================================================

    /// Get the behaviors for a type by TypeHash.
    pub fn get_behaviors(&self, hash: TypeHash) -> Option<&TypeBehaviors> {
        self.behaviors.get(&hash)
    }

    /// Get the behaviors for a type by TypeHash.
    pub fn get_behaviors_by_hash(&self, hash: TypeHash) -> Option<&TypeBehaviors> {
        self.behaviors.get(&hash)
    }

    /// Find all constructors for a given type (value types).
    pub fn find_constructors(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        self.behaviors.get(&type_hash)
            .map(|b| b.constructors.clone())
            .unwrap_or_default()
    }

    /// Find all factories for a given type (reference types).
    pub fn find_factories(&self, type_hash: TypeHash) -> Vec<TypeHash> {
        self.behaviors.get(&type_hash)
            .map(|b| b.factories.clone())
            .unwrap_or_default()
    }

    /// Find a constructor for a type with specific argument types.
    pub fn find_constructor(&self, type_id: TypeHash, arg_types: &[DataType]) -> Option<TypeHash> {
        let constructors = self.find_constructors(type_id);
        for ctor_id in constructors {
            if let Some(func) = self.get_function(ctor_id)
                && func.params.len() == arg_types.len() {
                    let all_match = func
                        .params
                        .iter()
                        .zip(arg_types.iter())
                        .all(|(param, arg_type)| &param.data_type == arg_type);
                    if all_match {
                        return Some(ctor_id);
                    }
                }
        }
        None
    }

    /// Find the copy constructor for a type.
    /// Copy constructor has signature: ClassName(const ClassName&in) or ClassName(const ClassName&inout)
    pub fn find_copy_constructor(&self, type_id: TypeHash) -> Option<TypeHash> {
        use angelscript_core::RefModifier;

        let constructors = self.find_constructors(type_id);
        for ctor_id in constructors {
            if let Some(func) = self.get_function(ctor_id) {
                // Copy constructor must have exactly one parameter
                if func.params.len() != 1 {
                    continue;
                }
                let param = &func.params[0];
                // Parameter must be a reference (&in or &inout)
                if !matches!(
                    param.data_type.ref_modifier,
                    RefModifier::In | RefModifier::InOut
                ) {
                    continue;
                }
                // Parameter type must match the class type
                // Get the expected type hash from the TypeDef
                if let Some(type_def) = self.get_type(type_id) {
                    if param.data_type.type_hash == type_def.type_hash() {
                        return Some(ctor_id);
                    }
                }
            }
        }
        None
    }

    /// Check if a constructor is marked as explicit.
    pub fn is_constructor_explicit(&self, func_id: TypeHash) -> bool {
        self.get_function(func_id)
            .map(|f| f.traits.is_explicit)
            .unwrap_or(false)
    }

    // =========================================================================
    // Method Lookups
    // =========================================================================

    /// Get all method FunctionIds for a type.
    pub fn get_methods(&self, type_id: TypeHash) -> Vec<TypeHash> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { methods, .. }) => methods.clone(),
            _ => Vec::new(),
        }
    }

    /// Find a method by name on a type (first match, no inheritance).
    pub fn find_method(&self, type_id: TypeHash, name: &str) -> Option<TypeHash> {
        self.find_methods_by_name(type_id, name).first().copied()
    }

    /// Find all methods with the given name on a type.
    pub fn find_methods_by_name(&self, type_id: TypeHash, name: &str) -> Vec<TypeHash> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { methods, .. }) => methods
                .iter()
                .filter(|&&id| {
                    self.get_function(id)
                        .map(|f| f.name == name)
                        .unwrap_or(false)
                })
                .copied()
                .collect(),
            _ => Vec::new(),
        }
    }

    // =========================================================================
    // Operator Lookups
    // =========================================================================

    /// Find an operator method on a type.
    pub fn find_operator_method(
        &self,
        type_id: TypeHash,
        operator: OperatorBehavior,
    ) -> Option<TypeHash> {
        self.find_operator_methods(type_id, operator).first().copied()
    }

    /// Find all overloads of an operator method for a type.
    pub fn find_operator_methods(
        &self,
        type_id: TypeHash,
        operator: OperatorBehavior,
    ) -> &[TypeHash] {
        match self.get_type(type_id) {
            Some(TypeDef::Class { operator_methods, .. }) => {
                operator_methods.get(&operator).map(|v| v.as_slice()).unwrap_or(&[])
            }
            _ => &[],
        }
    }

    // =========================================================================
    // Property Lookups
    // =========================================================================

    /// Find a property by name on a type.
    pub fn find_property(&self, type_id: TypeHash, name: &str) -> Option<PropertyAccessors> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { properties, .. }) => properties.get(name).cloned(),
            _ => None,
        }
    }

    /// Get all properties for a type.
    pub fn get_all_properties(&self, type_id: TypeHash) -> FxHashMap<String, PropertyAccessors> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { properties, .. }) => properties.clone(),
            _ => FxHashMap::default(),
        }
    }

    // =========================================================================
    // Interface Support
    // =========================================================================

    /// Get all method signatures for an interface type.
    pub fn get_interface_methods(&self, type_id: TypeHash) -> Option<&[MethodSignature]> {
        match self.get_type(type_id) {
            Some(TypeDef::Interface { methods, .. }) => Some(methods.as_slice()),
            _ => None,
        }
    }

    /// Get all interfaces implemented by a class.
    pub fn get_all_interfaces(&self, type_id: TypeHash) -> Vec<TypeHash> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { interfaces, .. }) => interfaces.clone(),
            _ => Vec::new(),
        }
    }

    /// Check if a class has a method matching an interface method signature.
    pub fn has_method_matching_interface(
        &self,
        class_type_id: TypeHash,
        interface_method: &MethodSignature,
    ) -> bool {
        // Get all methods with this name on the FFI class
        let methods = self.find_methods_by_name(class_type_id, &interface_method.name);
        for method_id in methods {
            if let Some(func) = self.get_function(method_id) {
                // Check return type matches
                if func.return_type.type_hash != interface_method.return_type.type_hash {
                    continue;
                }
                // Check parameter count matches
                if func.params.len() != interface_method.params.len() {
                    continue;
                }
                // Check parameter types match
                let params_match = func.params.iter().zip(interface_method.params.iter()).all(
                    |(func_param, iface_param)| {
                        func_param.data_type.type_hash == iface_param.type_hash
                            && func_param.data_type.ref_modifier == iface_param.ref_modifier
                            && func_param.data_type.is_handle == iface_param.is_handle
                    },
                );
                if params_match {
                    return true;
                }
            }
        }
        false
    }

    // =========================================================================
    // Inheritance Support
    // =========================================================================

    /// Get the base class of a type (if any).
    pub fn get_base_class(&self, type_id: TypeHash) -> Option<TypeHash> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { base_class, .. }) => *base_class,
            _ => None,
        }
    }

    /// Check if `derived_class` is a subclass of `base_class`.
    pub fn is_subclass_of(&self, derived_class: TypeHash, base_class: TypeHash) -> bool {
        if derived_class == base_class {
            return true;
        }

        let mut current = self.get_base_class(derived_class);
        while let Some(parent_id) = current {
            if parent_id == base_class {
                return true;
            }
            current = self.get_base_class(parent_id);
        }

        false
    }

    // =========================================================================
    // Enum Support
    // =========================================================================

    /// Look up an enum value by enum type ID and value name.
    pub fn lookup_enum_value(&self, type_id: TypeHash, value_name: &str) -> Option<i64> {
        match self.get_type(type_id) {
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
    pub fn get_funcdef_signature(&self, type_id: TypeHash) -> Option<(&[DataType], &DataType)> {
        match self.get_type(type_id) {
            Some(TypeDef::Funcdef {
                params,
                return_type,
                ..
            }) => Some((params.as_slice(), return_type)),
            _ => None,
        }
    }

    /// Check if a function is compatible with a funcdef type.
    pub fn is_function_compatible_with_funcdef(
        &self,
        func_id: TypeHash,
        funcdef_type_id: TypeHash,
    ) -> bool {
        let Some(func) = self.get_function(func_id) else {
            return false;
        };
        let Some((params, return_type)) = self.get_funcdef_signature(funcdef_type_id) else {
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
        func.params.iter().zip(params.iter()).all(|(func_param, funcdef_param)| {
            func_param.data_type.type_hash == funcdef_param.type_hash
                && func_param.data_type.ref_modifier == funcdef_param.ref_modifier
        })
    }

    /// Find a function by name that is compatible with a funcdef type.
    pub fn find_compatible_function(
        &self,
        name: &str,
        funcdef_type_id: TypeHash,
    ) -> Option<TypeHash> {
        // Search through all FFI functions for a match
        for func in self.functions.values() {
            if func.name == name && self.is_function_compatible_with_funcdef(func.func_hash, funcdef_type_id) {
                return Some(func.func_hash);
            }
        }
        None
    }

    // =========================================================================
    // Template Support
    // =========================================================================

    /// Get the template callback for a template type by TypeHash.
    ///
    /// This method provides backward compatibility during the TypeHash migration.
    pub fn get_template_callback(
        &self,
        type_id: TypeHash,
    ) -> Option<&Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>> {
        self.get_type(type_id)
            .map(|def| def.type_hash())
            .and_then(|hash| self.template_callbacks.get(&hash))
    }

    /// Get the template callback for a template type by TypeHash.
    pub fn get_template_callback_by_hash(
        &self,
        hash: TypeHash,
    ) -> Option<&Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>> {
        self.template_callbacks.get(&hash)
    }

    /// Check if a type is a template (has template parameters).
    pub fn is_template(&self, type_id: TypeHash) -> bool {
        match self.get_type(type_id) {
            Some(TypeDef::Class { template_params, .. }) => !template_params.is_empty(),
            _ => false,
        }
    }

    /// Check if a type is a template by TypeHash.
    pub fn is_template_by_hash(&self, hash: TypeHash) -> bool {
        match self.get_type_by_hash(hash) {
            Some(TypeDef::Class { template_params, .. }) => !template_params.is_empty(),
            _ => false,
        }
    }

    // =========================================================================
    // Namespace Support
    // =========================================================================

    /// Check if a namespace exists.
    pub fn has_namespace(&self, namespace: &str) -> bool {
        self.namespaces.contains(namespace)
    }

    /// Get all registered namespaces.
    pub fn namespaces(&self) -> &FxHashSet<String> {
        &self.namespaces
    }

}

// ============================================================================
// FfiRegistryBuilder
// ============================================================================

/// Builder for constructing an immutable `FfiRegistry`.
///
/// Used during the Context registration phase to accumulate FFI types,
/// functions, and behaviors. When complete, call `build()` to resolve
/// all types and produce an immutable `FfiRegistry`.
///
/// # Example
///
/// ```ignore
/// let mut builder = FfiRegistryBuilder::new();
///
/// // Register types
/// builder.register_type(my_type_def, Some("MyClass"));
///
/// // Register functions (with unresolved types)
/// builder.register_function(my_ffi_func);
///
/// // Build the immutable registry (resolves all types)
/// let registry = builder.build()?;
/// ```
pub struct FfiRegistryBuilder {
    // === Type Storage ===
    types: FxHashMap<TypeHash, TypeDef>,
    type_names: FxHashMap<String, TypeHash>,

    // === Function Storage ===
    /// Functions (resolved during build)
    functions: Vec<(FunctionDef, Option<NativeFn>)>,

    // === Interface Storage ===
    /// Interfaces (resolved during build)
    interfaces: Vec<(TypeHash, String, crate::types::FfiInterfaceDef)>,

    // === Funcdef Storage ===
    /// Funcdefs (resolved during build)
    funcdefs: Vec<(TypeHash, String, crate::types::FfiFuncdefDef)>,

    // === Behavior Storage ===
    behaviors: FxHashMap<TypeHash, TypeBehaviors>,

    // === Template Support ===
    template_callbacks:
        FxHashMap<TypeHash, Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

    // === Namespace Tracking ===
    namespaces: FxHashSet<String>,
}

impl std::fmt::Debug for FfiRegistryBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiRegistryBuilder")
            .field("types", &self.types)
            .field("type_names", &self.type_names)
            .field("functions", &self.functions.len())
            .field("interfaces", &self.interfaces.len())
            .field("funcdefs", &self.funcdefs.len())
            .field("behaviors", &self.behaviors)
            .field("template_callbacks", &self.template_callbacks.len())
            .field("namespaces", &self.namespaces)
            .finish()
    }
}

impl Default for FfiRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FfiRegistryBuilder {
    /// Create a new builder with primitive types pre-registered.
    pub fn new() -> Self {
        let mut builder = Self {
            types: FxHashMap::default(),
            type_names: FxHashMap::default(),
            functions: Vec::new(),
            interfaces: Vec::new(),
            funcdefs: Vec::new(),
            behaviors: FxHashMap::default(),
            template_callbacks: FxHashMap::default(),
            namespaces: FxHashSet::default(),
        };

        // Pre-register primitive types (both TypeDef and name lookup)
        builder.register_primitive(PrimitiveKind::Void, primitives::VOID);
        builder.register_primitive(PrimitiveKind::Bool, primitives::BOOL);
        builder.register_primitive(PrimitiveKind::Int8, primitives::INT8);
        builder.register_primitive(PrimitiveKind::Int16, primitives::INT16);
        builder.register_primitive(PrimitiveKind::Int32, primitives::INT32);
        builder.register_primitive(PrimitiveKind::Int64, primitives::INT64);
        builder.register_primitive(PrimitiveKind::Uint8, primitives::UINT8);
        builder.register_primitive(PrimitiveKind::Uint16, primitives::UINT16);
        builder.register_primitive(PrimitiveKind::Uint32, primitives::UINT32);
        builder.register_primitive(PrimitiveKind::Uint64, primitives::UINT64);
        builder.register_primitive(PrimitiveKind::Float, primitives::FLOAT);
        builder.register_primitive(PrimitiveKind::Double, primitives::DOUBLE);

        // Register type aliases for primitives
        builder.type_names.insert("int".to_string(), primitives::INT32);
        builder.type_names.insert("uint".to_string(), primitives::UINT32);

        // Register special types
        // "auto" and "?" are used for variable parameter types in FFI
        builder.type_names.insert("auto".to_string(), primitives::VARIABLE_PARAM);
        builder.type_names.insert("?".to_string(), primitives::VARIABLE_PARAM);

        builder
    }

    /// Register a primitive type with its TypeDef and name lookup.
    fn register_primitive(&mut self, kind: PrimitiveKind, type_id: TypeHash) {
        let type_hash = angelscript_core::TypeHash::from_name(kind.name());
        self.types.insert(type_id, TypeDef::Primitive { kind, type_hash });
        self.type_names.insert(kind.name().to_string(), type_id);
    }

    // =========================================================================
    // Type Registration
    // =========================================================================

    /// Register a type and return its TypeHash.
    ///
    /// The TypeHash is extracted from the TypeDef's type_hash field.
    /// If `name` is provided, the type will be registered in the name lookup map.
    pub fn register_type(&mut self, type_def: TypeDef, name: Option<&str>) -> TypeHash {
        let type_hash = type_def.type_hash();
        self.types.insert(type_hash, type_def);

        if let Some(name) = name {
            self.type_names.insert(name.to_string(), type_hash);
        }

        type_hash
    }

    /// Register a type with a specific TypeHash.
    ///
    /// This is used when the TypeHash has already been assigned (e.g., during module import).
    pub fn register_type_with_id(
        &mut self,
        type_id: TypeHash,
        type_def: TypeDef,
        name: Option<&str>,
    ) {
        self.types.insert(type_id, type_def);

        if let Some(name) = name {
            self.type_names.insert(name.to_string(), type_id);
        }
    }

    /// Register a type alias (typedef).
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeHash) {
        self.type_names.insert(alias_name.to_string(), target_type);
    }

    /// Look up a type by name (useful during registration).
    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_names.get(name).copied()
    }

    /// Get a type definition by TypeHash (useful during registration).
    pub fn get_type(&self, type_id: TypeHash) -> Option<&TypeDef> {
        self.types.get(&type_id)
    }

    /// Get a mutable type definition by TypeHash.
    pub fn get_type_mut(&mut self, type_id: TypeHash) -> Option<&mut TypeDef> {
        self.types.get_mut(&type_id)
    }

    // =========================================================================
    // Function Registration
    // =========================================================================

    /// Register an FFI function.
    ///
    /// The function's types may be unresolved; they will be resolved during `build()`.
    pub fn register_function(&mut self, func: FunctionDef, native_fn: Option<NativeFn>) {
        self.functions.push((func, native_fn));
    }

    // =========================================================================
    // Behavior Registration
    // =========================================================================

    /// Set behaviors for a type. Overwrites any existing behaviors.
    pub fn set_behaviors(&mut self, type_id: TypeHash, behaviors: TypeBehaviors) {
        self.behaviors.insert(type_id, behaviors);
    }

    /// Get or create behaviors for a type (for incremental registration).
    pub fn behaviors_mut(&mut self, type_id: TypeHash) -> &mut TypeBehaviors {
        self.behaviors.entry(type_id).or_default()
    }

    /// Get behaviors for a type.
    pub fn get_behaviors(&self, type_id: TypeHash) -> Option<&TypeBehaviors> {
        self.behaviors.get(&type_id)
    }

    // =========================================================================
    // Template Registration
    // =========================================================================

    /// Register a template callback for a template type.
    pub fn register_template_callback<F>(&mut self, type_id: TypeHash, callback: F)
    where
        F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static,
    {
        self.template_callbacks.insert(type_id, Arc::new(callback));
    }

    /// Register a template callback using an Arc (for shared callbacks).
    pub fn register_template_callback_arc(
        &mut self,
        type_id: TypeHash,
        callback: Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>,
    ) {
        self.template_callbacks.insert(type_id, callback);
    }

    // =========================================================================
    // Namespace Registration
    // =========================================================================

    /// Register a namespace.
    pub fn register_namespace(&mut self, namespace: &str) {
        if !namespace.is_empty() {
            self.namespaces.insert(namespace.to_string());
        }
    }

    // =========================================================================
    // Interface Registration
    // =========================================================================

    /// Register an interface with potentially unresolved method types.
    ///
    /// The interface's method parameter and return types may be unresolved;
    /// they will be resolved during `build()`.
    pub fn register_interface(
        &mut self,
        interface_def: crate::types::FfiInterfaceDef,
        qualified_name: &str,
    ) {
        let type_id = interface_def.id;
        self.type_names
            .insert(qualified_name.to_string(), type_id);
        self.interfaces.push((
            type_id,
            qualified_name.to_string(),
            interface_def,
        ));
    }

    // =========================================================================
    // Funcdef Registration
    // =========================================================================

    /// Register a funcdef with potentially unresolved types.
    ///
    /// The funcdef's parameter and return types may be unresolved;
    /// they will be resolved during `build()`.
    pub fn register_funcdef(
        &mut self,
        funcdef_def: crate::types::FfiFuncdefDef,
        qualified_name: &str,
    ) {
        let type_id = funcdef_def.id;
        self.type_names
            .insert(qualified_name.to_string(), type_id);
        self.funcdefs.push((
            type_id,
            qualified_name.to_string(),
            funcdef_def,
        ));
    }

    // =========================================================================
    // Build
    // =========================================================================

    /// Build the immutable FfiRegistry.
    ///
    /// This resolves all unresolved types in functions, interfaces, and funcdefs,
    /// then validates the registry.
    ///
    /// # Errors
    ///
    /// Returns a vector of resolution errors if any types cannot be resolved.
    pub fn build(mut self) -> Result<FfiRegistry, Vec<RegistrationError>> {
        let mut errors = Vec::new();
        let mut resolved_functions = FxHashMap::default();
        let mut function_names: FxHashMap<String, Vec<TypeHash>> = FxHashMap::default();
        let mut native_fns = FxHashMap::default();

        // Extract items to avoid borrow conflicts
        let interfaces = std::mem::take(&mut self.interfaces);
        let funcdefs = std::mem::take(&mut self.funcdefs);
        let functions = std::mem::take(&mut self.functions);

        // Resolve all interfaces
        for (type_id, qualified_name, interface_def) in interfaces {
            match Self::resolve_interface(&self.type_names, &interface_def, &qualified_name) {
                Ok(typedef) => {
                    self.types.insert(type_id, typedef);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        // Resolve all funcdefs
        for (type_id, qualified_name, funcdef_def) in funcdefs {
            match Self::resolve_funcdef(&self.type_names, &funcdef_def, &qualified_name) {
                Ok(typedef) => {
                    self.types.insert(type_id, typedef);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        // Process all functions - types are already resolved (DataType used directly)
        for (func_def, native_fn_opt) in functions {
            let func_hash = func_def.func_hash;

            // Add to function name map
            function_names
                .entry(func_def.qualified_name().to_string())
                .or_default()
                .push(func_hash);

            resolved_functions.insert(func_hash, func_def);

            // Store native function if provided
            if let Some(native_fn) = native_fn_opt {
                native_fns.insert(func_hash, native_fn);
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // Types are already keyed by type_hash in self.types
        let types: FxHashMap<TypeHash, TypeDef> = self.types
            .into_iter()
            .map(|(_, def)| (def.type_hash(), def))
            .collect();

        // Build type name → TypeHash cache (O(1) lookup instead of O(n) iteration)
        let type_by_name: FxHashMap<String, TypeHash> = types
            .iter()
            .map(|(hash, def)| (def.qualified_name().to_string(), *hash))
            .collect();

        // Functions are keyed by func_hash
        let functions: FxHashMap<TypeHash, FunctionDef> = resolved_functions
            .into_iter()
            .map(|(_, def)| (def.func_hash, def))
            .collect();

        // Build function name → func_hash mapping (for overload resolution)
        let function_overloads: FxHashMap<String, Vec<TypeHash>> = {
            let mut map: FxHashMap<String, Vec<TypeHash>> = FxHashMap::default();
            for func in functions.values() {
                map.entry(func.qualified_name().to_string())
                    .or_default()
                    .push(func.func_hash);
            }
            map
        };

        // Native functions are already keyed by func_hash from the resolution loop above

        // Behaviors are keyed by type_hash - O(1) lookup instead of O(n) scan
        let behaviors: FxHashMap<TypeHash, TypeBehaviors> = self.behaviors
            .into_iter()
            .filter(|(type_id, _)| types.contains_key(type_id))
            .collect();

        // Template callbacks are keyed by type_hash - O(1) lookup instead of O(n) scan
        let template_callbacks: FxHashMap<TypeHash, Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>> = self.template_callbacks
            .into_iter()
            .filter(|(type_id, _)| types.contains_key(type_id))
            .collect();

        Ok(FfiRegistry {
            types,
            type_by_name,
            functions,
            function_overloads,
            native_fns,
            behaviors,
            template_callbacks,
            namespaces: self.namespaces,
        })
    }

    /// Convert an interface definition to a TypeDef.
    ///
    /// Types are already resolved (DataType used directly), so this is just a conversion.
    fn resolve_interface(
        _type_names: &FxHashMap<String, TypeHash>,
        interface_def: &crate::types::FfiInterfaceDef,
        qualified_name: &str,
    ) -> Result<TypeDef, RegistrationError> {
        let methods: Vec<MethodSignature> = interface_def
            .methods()
            .iter()
            .map(|m| {
                MethodSignature {
                    name: m.name.clone(),
                    params: m.params.iter().map(|p| p.data_type.clone()).collect(),
                    return_type: m.return_type.clone(),
                    is_const: m.is_const,
                }
            })
            .collect();

        Ok(TypeDef::Interface {
            name: interface_def.name().to_string(),
            qualified_name: qualified_name.to_string(),
            type_hash: angelscript_core::TypeHash::from_name(qualified_name),
            methods,
        })
    }

    /// Convert a funcdef definition to a TypeDef.
    ///
    /// Types are already resolved (DataType used directly), so this is just a conversion.
    fn resolve_funcdef(
        _type_names: &FxHashMap<String, TypeHash>,
        funcdef_def: &crate::types::FfiFuncdefDef,
        qualified_name: &str,
    ) -> Result<TypeDef, RegistrationError> {
        Ok(TypeDef::Funcdef {
            name: funcdef_def.name.clone(),
            qualified_name: qualified_name.to_string(),
            type_hash: angelscript_core::TypeHash::from_name(qualified_name),
            params: funcdef_def.params.iter().map(|p| p.data_type.clone()).collect(),
            return_type: funcdef_def.return_type.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_new_has_primitives() {
        let builder = FfiRegistryBuilder::new();

        // Check primitives are registered
        assert!(builder.lookup_type("void").is_some());
        assert!(builder.lookup_type("bool").is_some());
        assert!(builder.lookup_type("int").is_some());
        assert!(builder.lookup_type("int8").is_some());
        assert!(builder.lookup_type("int16").is_some());
        assert!(builder.lookup_type("int64").is_some());
        assert!(builder.lookup_type("uint").is_some());
        assert!(builder.lookup_type("uint8").is_some());
        assert!(builder.lookup_type("uint16").is_some());
        assert!(builder.lookup_type("uint64").is_some());
        assert!(builder.lookup_type("float").is_some());
        assert!(builder.lookup_type("double").is_some());

        // Check TypeIds match constants
        assert_eq!(builder.lookup_type("void"), Some(primitives::VOID));
        assert_eq!(builder.lookup_type("int"), Some(primitives::INT32));
        assert_eq!(builder.lookup_type("float"), Some(primitives::FLOAT));
    }

    #[test]
    fn builder_register_type() {
        let mut builder = FfiRegistryBuilder::new();

        let type_id = builder.register_type(
            TypeDef::Enum {
                name: "Color".to_string(),
                qualified_name: "Color".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("Color"),
                values: vec![
                    ("Red".to_string(), 0),
                    ("Green".to_string(), 1),
                    ("Blue".to_string(), 2),
                ],
            },
            Some("Color"),
        );

        assert!(builder.lookup_type("Color").is_some());
        assert_eq!(builder.lookup_type("Color"), Some(type_id));

        let type_def = builder.get_type(type_id);
        assert!(type_def.is_some());
        assert!(matches!(type_def, Some(TypeDef::Enum { name, .. }) if name == "Color"));
    }

    #[test]
    fn builder_register_type_alias() {
        let mut builder = FfiRegistryBuilder::new();

        builder.register_type_alias("integer", primitives::INT32);

        assert_eq!(builder.lookup_type("integer"), Some(primitives::INT32));
        assert_eq!(builder.lookup_type("int"), Some(primitives::INT32));
    }

    #[test]
    fn builder_build_empty() {
        let builder = FfiRegistryBuilder::new();
        let registry = builder.build().unwrap();

        // Should have primitive names registered for lookup
        // (TypeDef::Primitive is handled by Registry, not FfiRegistry)
        assert!(registry.get_type_by_name("void").is_some());
        assert!(registry.get_type_by_name("int").is_some());
        assert_eq!(registry.get_type_by_name("void"), Some(primitives::VOID));
        assert_eq!(registry.get_type_by_name("int"), Some(primitives::INT32));
    }

    #[test]
    fn builder_build_with_function() {
        let mut builder = FfiRegistryBuilder::new();

        let func = FunctionDef::new(
            TypeHash::from_function("add", &[primitives::INT32, primitives::INT32]),
            "add".to_string(),
            vec![],
            vec![
                Param::new("a", DataType::simple(primitives::INT32)),
                Param::new("b", DataType::simple(primitives::INT32)),
            ],
            DataType::simple(primitives::INT32),
            None,
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );

        builder.register_function(func, None);

        let registry = builder.build().unwrap();

        assert_eq!(registry.function_count(), 1);
        let func_ids = registry.lookup_functions("add");
        assert_eq!(func_ids.len(), 1);

        let resolved = registry.get_function(func_ids[0]).unwrap();
        assert_eq!(resolved.name, "add");
        assert_eq!(resolved.params.len(), 2);
        assert_eq!(resolved.return_type.type_hash, primitives::INT32);
    }

    #[test]
    fn builder_build_with_user_type_function() {
        let mut builder = FfiRegistryBuilder::new();

        // Register a custom type first
        let my_class_hash = TypeHash::from_name("MyClass");
        let _my_class_id = builder.register_type(
            TypeDef::Class {
                name: "MyClass".to_string(),
                qualified_name: "MyClass".to_string(),
                type_hash: my_class_hash,
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: None,
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
                type_kind: angelscript_core::TypeKind::reference(),
            },
            Some("MyClass"),
        );

        // Register function with user type (using TypeHash::from_name directly)
        let func = FunctionDef::new(
            TypeHash::from_function("process", &[my_class_hash]),
            "process".to_string(),
            vec![],
            vec![Param::new(
                "obj",
                DataType::with_handle(my_class_hash, false),
            )],
            DataType::simple(primitives::VOID),
            None,
            FunctionTraits::default(),
            true,
            Visibility::Public,
        );

        builder.register_function(func, None);

        let registry = builder.build().unwrap();

        let func_ids = registry.lookup_functions("process");
        assert_eq!(func_ids.len(), 1);

        let resolved = registry.get_function(func_ids[0]).unwrap();
        assert_eq!(resolved.params[0].data_type.type_hash, my_class_hash);
        assert!(resolved.params[0].data_type.is_handle);
    }

    #[test]
    fn registry_lookup_enum_value() {
        let mut builder = FfiRegistryBuilder::new();

        let type_id = builder.register_type(
            TypeDef::Enum {
                name: "Color".to_string(),
                qualified_name: "Color".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("Color"),
                values: vec![
                    ("Red".to_string(), 0),
                    ("Green".to_string(), 1),
                    ("Blue".to_string(), 2),
                ],
            },
            Some("Color"),
        );

        let registry = builder.build().unwrap();

        assert_eq!(registry.lookup_enum_value(type_id, "Red"), Some(0));
        assert_eq!(registry.lookup_enum_value(type_id, "Green"), Some(1));
        assert_eq!(registry.lookup_enum_value(type_id, "Blue"), Some(2));
        assert_eq!(registry.lookup_enum_value(type_id, "Unknown"), None);
    }

    #[test]
    fn registry_behaviors() {
        let mut builder = FfiRegistryBuilder::new();

        let type_id = builder.register_type(
            TypeDef::Class {
                name: "MyClass".to_string(),
                qualified_name: "MyClass".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("MyClass"),
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: None,
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
                type_kind: angelscript_core::TypeKind::reference(),
            },
            Some("MyClass"),
        );

        let ctor_id = TypeHash::from_name("test_func");
        let mut behaviors = TypeBehaviors::default();
        behaviors.constructors.push(ctor_id);
        builder.set_behaviors(type_id, behaviors);

        let registry = builder.build().unwrap();

        let constructors = registry.find_constructors(type_id);
        assert_eq!(constructors.len(), 1);
        assert_eq!(constructors[0], ctor_id);
    }

    #[test]
    fn registry_namespaces() {
        let mut builder = FfiRegistryBuilder::new();

        builder.register_namespace("Game");
        builder.register_namespace("Game::Player");
        builder.register_namespace(""); // Empty should be ignored

        let registry = builder.build().unwrap();

        assert!(registry.has_namespace("Game"));
        assert!(registry.has_namespace("Game::Player"));
        assert!(!registry.has_namespace(""));
        assert!(!registry.has_namespace("Unknown"));
    }

    #[test]
    fn registry_is_subclass_of() {
        let mut builder = FfiRegistryBuilder::new();

        // Register base class
        let base_id = builder.register_type(
            TypeDef::Class {
                name: "Base".to_string(),
                qualified_name: "Base".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("Base"),
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: None,
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
                type_kind: angelscript_core::TypeKind::reference(),
            },
            Some("Base"),
        );

        // Register derived class
        let derived_id = builder.register_type(
            TypeDef::Class {
                name: "Derived".to_string(),
                qualified_name: "Derived".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("Derived"),
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: Some(base_id),
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: Vec::new(),
                template: None,
                type_args: Vec::new(),
                type_kind: angelscript_core::TypeKind::reference(),
            },
            Some("Derived"),
        );

        let registry = builder.build().unwrap();

        assert!(registry.is_subclass_of(derived_id, base_id));
        assert!(registry.is_subclass_of(derived_id, derived_id)); // Same class
        assert!(registry.is_subclass_of(base_id, base_id)); // Same class
        assert!(!registry.is_subclass_of(base_id, derived_id)); // Not the other way
    }

    #[test]
    fn registry_template_callback() {
        let mut builder = FfiRegistryBuilder::new();

        let template_id = builder.register_type(
            TypeDef::Class {
                name: "array".to_string(),
                qualified_name: "array".to_string(),
                type_hash: angelscript_core::TypeHash::from_name("array"),
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: None,
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: vec![TypeHash::from_name("test_type")], // One template param
                template: None,
                type_args: Vec::new(),
                type_kind: angelscript_core::TypeKind::reference(),
            },
            Some("array"),
        );

        builder.register_template_callback(template_id, |_info| TemplateValidation::valid());

        let registry = builder.build().unwrap();

        assert!(registry.is_template(template_id));
        assert!(registry.get_template_callback(template_id).is_some());
    }

    #[test]
    fn builder_default() {
        let builder = FfiRegistryBuilder::default();
        assert!(builder.lookup_type("int").is_some());
    }

    #[test]
    fn registry_debug() {
        let builder = FfiRegistryBuilder::new();
        let registry = builder.build().unwrap();
        let debug = format!("{:?}", registry);
        assert!(debug.contains("FfiRegistry"));
    }

    #[test]
    fn builder_debug() {
        let builder = FfiRegistryBuilder::new();
        let debug = format!("{:?}", builder);
        assert!(debug.contains("FfiRegistryBuilder"));
    }

    #[test]
    fn error_display() {
        let err = RegistrationError::TypeNotFound("MyClass".to_string());
        assert!(err.to_string().contains("type not found"));
        assert!(err.to_string().contains("MyClass"));

        let err = RegistrationError::DuplicateType("MyClass".to_string());
        assert!(err.to_string().contains("duplicate type"));
    }
}
