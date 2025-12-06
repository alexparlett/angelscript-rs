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
//! │  │  │  - script_types: HashMap<TypeId, TypeDef>     │  │    │
//! │  │  │  - script_functions: HashMap<FunctionId, ...> │  │    │
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

use crate::ffi::{NativeFn, TemplateInstanceInfo, TemplateValidation};
use crate::semantic::types::behaviors::TypeBehaviors;
use crate::semantic::types::type_def::{
    FunctionId, MethodSignature, OperatorBehavior, PropertyAccessors, TypeDef, TypeId,
    BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE, INT8_TYPE,
    PrimitiveType, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE, UINT8_TYPE, VARIABLE_PARAM_TYPE, VOID_TYPE,
};
use crate::semantic::types::SELF_TYPE;
use crate::semantic::types::DataType;
use crate::types::{FfiFunctionDef, FfiResolutionError, ResolvedFfiFunctionDef};

/// Immutable FFI registry, shared across all Units in a Context.
///
/// This registry holds all FFI types, functions, and behaviors that have been
/// resolved and are ready for use during compilation. It is constructed once
/// during Context sealing and then shared via `Arc` across all compilation Units.
///
/// Template instances are NOT stored here - they are created per-Unit during
/// compilation and cached in the per-Unit Registry.
pub struct FfiRegistry {
    // === Type Storage ===
    /// All FFI types indexed by TypeId
    types: FxHashMap<TypeId, TypeDef>,
    /// Type name to TypeId mapping
    type_names: FxHashMap<String, TypeId>,

    // === Function Storage ===
    /// All FFI functions indexed by FunctionId (resolved, non-template)
    functions: FxHashMap<FunctionId, ResolvedFfiFunctionDef>,
    /// Function name to FunctionId mapping (supports overloads)
    function_names: FxHashMap<String, Vec<FunctionId>>,
    /// Native function implementations indexed by FunctionId
    native_fns: FxHashMap<FunctionId, NativeFn>,
    /// Unresolved template functions (resolved at instantiation time)
    unresolved_functions: FxHashMap<FunctionId, FfiFunctionDef>,

    // === Behavior Storage ===
    /// Type behaviors (constructors, factories, etc.) indexed by TypeId
    behaviors: FxHashMap<TypeId, TypeBehaviors>,

    // === Template Support ===
    /// Template validation callbacks indexed by template TypeId
    template_callbacks:
        FxHashMap<TypeId, Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

    // === Namespace Tracking ===
    /// All registered namespaces
    namespaces: FxHashSet<String>,
}

impl std::fmt::Debug for FfiRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfiRegistry")
            .field("types", &self.types)
            .field("type_names", &self.type_names)
            .field("functions", &self.functions)
            .field("function_names", &self.function_names)
            .field("native_fns", &format!("<{} native fns>", self.native_fns.len()))
            .field("unresolved_functions", &format!("<{} unresolved>", self.unresolved_functions.len()))
            .field("behaviors", &self.behaviors)
            .field("template_callbacks", &format!("<{} callbacks>", self.template_callbacks.len()))
            .field("namespaces", &self.namespaces)
            .finish()
    }
}

impl FfiRegistry {
    // =========================================================================
    // Type Lookups
    // =========================================================================

    /// Get a type definition by TypeId.
    pub fn get_type(&self, id: TypeId) -> Option<&TypeDef> {
        self.types.get(&id)
    }

    /// Look up a TypeId by type name.
    pub fn get_type_by_name(&self, name: &str) -> Option<TypeId> {
        self.type_names.get(name).copied()
    }

    /// Get access to the type name map for iteration.
    pub fn type_by_name(&self) -> &FxHashMap<String, TypeId> {
        &self.type_names
    }

    /// Get the number of registered types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    // =========================================================================
    // Function Lookups
    // =========================================================================

    /// Get a function definition by FunctionId.
    pub fn get_function(&self, id: FunctionId) -> Option<&ResolvedFfiFunctionDef> {
        self.functions.get(&id)
    }

    /// Look up all functions with the given name (for overload resolution).
    pub fn lookup_functions(&self, name: &str) -> &[FunctionId] {
        self.function_names
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the native function implementation for a FunctionId.
    pub fn get_native_fn(&self, id: FunctionId) -> Option<&NativeFn> {
        self.native_fns.get(&id)
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get access to the function name map for iteration.
    pub fn func_by_name(&self) -> &FxHashMap<String, Vec<FunctionId>> {
        &self.function_names
    }

    // =========================================================================
    // Behavior Lookups
    // =========================================================================

    /// Get the behaviors for a type, if any are registered.
    pub fn get_behaviors(&self, type_id: TypeId) -> Option<&TypeBehaviors> {
        self.behaviors.get(&type_id)
    }

    /// Find all constructors for a given type (value types).
    pub fn find_constructors(&self, type_id: TypeId) -> Vec<FunctionId> {
        self.behaviors
            .get(&type_id)
            .map(|b| b.constructors.clone())
            .unwrap_or_default()
    }

    /// Find all factories for a given type (reference types).
    pub fn find_factories(&self, type_id: TypeId) -> Vec<FunctionId> {
        self.behaviors
            .get(&type_id)
            .map(|b| b.factories.clone())
            .unwrap_or_default()
    }

    /// Find a constructor for a type with specific argument types.
    pub fn find_constructor(&self, type_id: TypeId, arg_types: &[DataType]) -> Option<FunctionId> {
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
    pub fn find_copy_constructor(&self, type_id: TypeId) -> Option<FunctionId> {
        use crate::semantic::RefModifier;

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
                if param.data_type.type_id == type_id {
                    return Some(ctor_id);
                }
            }
        }
        None
    }

    /// Check if a constructor is marked as explicit.
    pub fn is_constructor_explicit(&self, func_id: FunctionId) -> bool {
        self.get_function(func_id)
            .map(|f| f.traits.is_explicit)
            .unwrap_or(false)
    }

    // =========================================================================
    // Method Lookups
    // =========================================================================

    /// Get all method FunctionIds for a type.
    pub fn get_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { methods, .. }) => methods.clone(),
            _ => Vec::new(),
        }
    }

    /// Find a method by name on a type (first match, no inheritance).
    pub fn find_method(&self, type_id: TypeId, name: &str) -> Option<FunctionId> {
        self.find_methods_by_name(type_id, name).first().copied()
    }

    /// Find all methods with the given name on a type.
    pub fn find_methods_by_name(&self, type_id: TypeId, name: &str) -> Vec<FunctionId> {
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
        type_id: TypeId,
        operator: OperatorBehavior,
    ) -> Option<FunctionId> {
        self.find_operator_methods(type_id, operator).first().copied()
    }

    /// Find all overloads of an operator method for a type.
    pub fn find_operator_methods(
        &self,
        type_id: TypeId,
        operator: OperatorBehavior,
    ) -> &[FunctionId] {
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
    pub fn find_property(&self, type_id: TypeId, name: &str) -> Option<PropertyAccessors> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { properties, .. }) => properties.get(name).cloned(),
            _ => None,
        }
    }

    /// Get all properties for a type.
    pub fn get_all_properties(&self, type_id: TypeId) -> FxHashMap<String, PropertyAccessors> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { properties, .. }) => properties.clone(),
            _ => FxHashMap::default(),
        }
    }

    // =========================================================================
    // Interface Support
    // =========================================================================

    /// Get all method signatures for an interface type.
    pub fn get_interface_methods(&self, type_id: TypeId) -> Option<&[MethodSignature]> {
        match self.get_type(type_id) {
            Some(TypeDef::Interface { methods, .. }) => Some(methods.as_slice()),
            _ => None,
        }
    }

    /// Get all interfaces implemented by a class.
    pub fn get_all_interfaces(&self, type_id: TypeId) -> Vec<TypeId> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { interfaces, .. }) => interfaces.clone(),
            _ => Vec::new(),
        }
    }

    /// Check if a class has a method matching an interface method signature.
    pub fn has_method_matching_interface(
        &self,
        class_type_id: TypeId,
        interface_method: &MethodSignature,
    ) -> bool {
        // Get all methods with this name on the FFI class
        let methods = self.find_methods_by_name(class_type_id, &interface_method.name);
        for method_id in methods {
            if let Some(func) = self.get_function(method_id) {
                // Check return type matches
                if func.return_type.type_id != interface_method.return_type.type_id {
                    continue;
                }
                // Check parameter count matches
                if func.params.len() != interface_method.params.len() {
                    continue;
                }
                // Check parameter types match
                let params_match = func.params.iter().zip(interface_method.params.iter()).all(
                    |(func_param, iface_param)| {
                        func_param.data_type.type_id == iface_param.type_id
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
    pub fn get_base_class(&self, type_id: TypeId) -> Option<TypeId> {
        match self.get_type(type_id) {
            Some(TypeDef::Class { base_class, .. }) => *base_class,
            _ => None,
        }
    }

    /// Check if `derived_class` is a subclass of `base_class`.
    pub fn is_subclass_of(&self, derived_class: TypeId, base_class: TypeId) -> bool {
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
    pub fn lookup_enum_value(&self, type_id: TypeId, value_name: &str) -> Option<i64> {
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
    pub fn get_funcdef_signature(&self, type_id: TypeId) -> Option<(&[DataType], &DataType)> {
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
        func_id: FunctionId,
        funcdef_type_id: TypeId,
    ) -> bool {
        let Some(func) = self.get_function(func_id) else {
            return false;
        };
        let Some((params, return_type)) = self.get_funcdef_signature(funcdef_type_id) else {
            return false;
        };

        // Check return type matches
        if func.return_type.type_id != return_type.type_id {
            return false;
        }

        // Check parameter count matches
        if func.params.len() != params.len() {
            return false;
        }

        // Check parameter types match
        func.params.iter().zip(params.iter()).all(|(func_param, funcdef_param)| {
            func_param.data_type.type_id == funcdef_param.type_id
                && func_param.data_type.ref_modifier == funcdef_param.ref_modifier
        })
    }

    /// Find a function by name that is compatible with a funcdef type.
    pub fn find_compatible_function(
        &self,
        name: &str,
        funcdef_type_id: TypeId,
    ) -> Option<FunctionId> {
        // Search through all FFI functions for a match
        for (&func_id, func) in &self.functions {
            if func.name == name && self.is_function_compatible_with_funcdef(func_id, funcdef_type_id) {
                return Some(func_id);
            }
        }
        None
    }

    // =========================================================================
    // Template Support
    // =========================================================================

    /// Get the template callback for a template type.
    pub fn get_template_callback(
        &self,
        type_id: TypeId,
    ) -> Option<&Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>> {
        self.template_callbacks.get(&type_id)
    }

    /// Check if a type is a template (has template parameters).
    pub fn is_template(&self, type_id: TypeId) -> bool {
        match self.get_type(type_id) {
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

    /// Get an unresolved function by ID (for template instantiation).
    pub fn get_unresolved_function(&self, id: FunctionId) -> Option<&FfiFunctionDef> {
        self.unresolved_functions.get(&id)
    }

    /// Get all unresolved functions.
    pub fn unresolved_functions(&self) -> &FxHashMap<FunctionId, FfiFunctionDef> {
        &self.unresolved_functions
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
    types: FxHashMap<TypeId, TypeDef>,
    type_names: FxHashMap<String, TypeId>,

    // === Function Storage ===
    /// Functions (resolved during build)
    functions: Vec<(FfiFunctionDef, Option<NativeFn>)>,

    // === Interface Storage ===
    /// Interfaces (resolved during build)
    interfaces: Vec<(TypeId, String, crate::types::FfiInterfaceDef)>,

    // === Funcdef Storage ===
    /// Funcdefs (resolved during build)
    funcdefs: Vec<(TypeId, String, crate::types::FfiFuncdefDef)>,

    // === Behavior Storage ===
    behaviors: FxHashMap<TypeId, TypeBehaviors>,

    // === Template Support ===
    template_callbacks:
        FxHashMap<TypeId, Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

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
        builder.register_primitive(PrimitiveType::Void, VOID_TYPE);
        builder.register_primitive(PrimitiveType::Bool, BOOL_TYPE);
        builder.register_primitive(PrimitiveType::Int8, INT8_TYPE);
        builder.register_primitive(PrimitiveType::Int16, INT16_TYPE);
        builder.register_primitive(PrimitiveType::Int32, INT32_TYPE);
        builder.register_primitive(PrimitiveType::Int64, INT64_TYPE);
        builder.register_primitive(PrimitiveType::Uint8, UINT8_TYPE);
        builder.register_primitive(PrimitiveType::Uint16, UINT16_TYPE);
        builder.register_primitive(PrimitiveType::Uint32, UINT32_TYPE);
        builder.register_primitive(PrimitiveType::Uint64, UINT64_TYPE);
        builder.register_primitive(PrimitiveType::Float, FLOAT_TYPE);
        builder.register_primitive(PrimitiveType::Double, DOUBLE_TYPE);

        // Register type aliases for primitives
        builder.type_names.insert("int".to_string(), INT32_TYPE);
        builder.type_names.insert("uint".to_string(), UINT32_TYPE);

        // Register special types
        // "auto" and "?" are used for variable parameter types in FFI
        builder.type_names.insert("auto".to_string(), VARIABLE_PARAM_TYPE);
        builder.type_names.insert("?".to_string(), VARIABLE_PARAM_TYPE);

        builder
    }

    /// Register a primitive type with its TypeDef and name lookup.
    fn register_primitive(&mut self, kind: PrimitiveType, type_id: TypeId) {
        self.types.insert(type_id, TypeDef::Primitive { kind });
        self.type_names.insert(kind.name().to_string(), type_id);
    }

    // =========================================================================
    // Type Registration
    // =========================================================================

    /// Register a type and return its TypeId.
    ///
    /// If `name` is provided, the type will be registered in the name lookup map.
    /// Uses `TypeId::next_ffi()` to ensure the FFI bit is set.
    pub fn register_type(&mut self, type_def: TypeDef, name: Option<&str>) -> TypeId {
        let type_id = TypeId::next_ffi();
        self.types.insert(type_id, type_def);

        if let Some(name) = name {
            self.type_names.insert(name.to_string(), type_id);
        }

        type_id
    }

    /// Register a type with a specific TypeId.
    ///
    /// This is used when the TypeId has already been assigned (e.g., during module import).
    pub fn register_type_with_id(
        &mut self,
        type_id: TypeId,
        type_def: TypeDef,
        name: Option<&str>,
    ) {
        self.types.insert(type_id, type_def);

        if let Some(name) = name {
            self.type_names.insert(name.to_string(), type_id);
        }
    }

    /// Register a type alias (typedef).
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeId) {
        self.type_names.insert(alias_name.to_string(), target_type);
    }

    /// Look up a type by name (useful during registration).
    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.type_names.get(name).copied()
    }

    /// Get a type definition by TypeId (useful during registration).
    pub fn get_type(&self, type_id: TypeId) -> Option<&TypeDef> {
        self.types.get(&type_id)
    }

    /// Get a mutable type definition by TypeId.
    pub fn get_type_mut(&mut self, type_id: TypeId) -> Option<&mut TypeDef> {
        self.types.get_mut(&type_id)
    }

    // =========================================================================
    // Function Registration
    // =========================================================================

    /// Register an FFI function.
    ///
    /// The function's types may be unresolved; they will be resolved during `build()`.
    pub fn register_function(&mut self, func: FfiFunctionDef, native_fn: Option<NativeFn>) {
        self.functions.push((func, native_fn));
    }

    // =========================================================================
    // Behavior Registration
    // =========================================================================

    /// Set behaviors for a type. Overwrites any existing behaviors.
    pub fn set_behaviors(&mut self, type_id: TypeId, behaviors: TypeBehaviors) {
        self.behaviors.insert(type_id, behaviors);
    }

    /// Get or create behaviors for a type (for incremental registration).
    pub fn behaviors_mut(&mut self, type_id: TypeId) -> &mut TypeBehaviors {
        self.behaviors.entry(type_id).or_default()
    }

    /// Get behaviors for a type.
    pub fn get_behaviors(&self, type_id: TypeId) -> Option<&TypeBehaviors> {
        self.behaviors.get(&type_id)
    }

    // =========================================================================
    // Template Registration
    // =========================================================================

    /// Register a template callback for a template type.
    pub fn register_template_callback<F>(&mut self, type_id: TypeId, callback: F)
    where
        F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static,
    {
        self.template_callbacks.insert(type_id, Arc::new(callback));
    }

    /// Register a template callback using an Arc (for shared callbacks).
    pub fn register_template_callback_arc(
        &mut self,
        type_id: TypeId,
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
    pub fn build(mut self) -> Result<FfiRegistry, Vec<FfiRegistryError>> {
        let mut errors = Vec::new();
        let mut resolved_functions = FxHashMap::default();
        let mut function_names: FxHashMap<String, Vec<FunctionId>> = FxHashMap::default();
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

        // Resolve all functions
        let mut unresolved_functions = FxHashMap::default();

        for (ffi_func, native_fn_opt) in functions {
            let func_id = ffi_func.id;

            // Type lookup closure
            let type_lookup = |name: &str| -> Option<TypeId> { self.type_names.get(name).copied() };

            // Get the owner type's template parameters (if this is a method on a template)
            let owner_template_params: Option<Vec<TypeId>> = ffi_func.owner_type.and_then(|owner_id| {
                self.types.get(&owner_id).and_then(|typedef| {
                    match typedef {
                        TypeDef::Class { template_params, .. } if !template_params.is_empty() => {
                            Some(template_params.clone())
                        }
                        _ => None,
                    }
                })
            });
            let owner_type_id = ffi_func.owner_type;

            // Create instantiate function that handles self-referential template types
            let mut instantiate = |template_id: TypeId, args: Vec<DataType>| -> Result<TypeId, String> {
                // Check if this is a self-reference: instantiating owner type with its own template params
                if let (Some(owner_id), Some(params)) = (owner_type_id, &owner_template_params)
                    && template_id == owner_id && args.len() == params.len() {
                        // Check if all args are the template params in order
                        let is_self_ref = args.iter().zip(params.iter()).all(|(arg, &param_id)| {
                            arg.type_id == param_id && !arg.is_const && !arg.is_handle
                        });
                        if is_self_ref {
                            return Ok(SELF_TYPE);
                        }
                    }

                // Also check if the template_id's params match the args exactly
                // This handles factory functions that return the template type itself
                if let Some(template_def) = self.types.get(&template_id)
                    && let TypeDef::Class { template_params, .. } = template_def
                        && !template_params.is_empty() && template_params.len() == args.len() {
                            // Check if all args are the template's own params
                            let is_self_ref = args.iter().zip(template_params.iter()).all(|(arg, &param_id)| {
                                arg.type_id == param_id && !arg.is_const && !arg.is_handle
                            });
                            if is_self_ref {
                                return Ok(SELF_TYPE);
                            }
                        }

                // Not a self-reference - template instantiation not supported during build
                Err("Template instantiation not supported during FfiRegistry build".to_string())
            };

            match ffi_func.resolve(&type_lookup, &mut instantiate) {
                Ok(resolved) => {
                    // Add to function name map
                    let qualified_name = resolved.qualified_name();
                    function_names
                        .entry(qualified_name)
                        .or_default()
                        .push(func_id);

                    resolved_functions.insert(func_id, resolved);

                    // Store native function if provided
                    if let Some(native_fn) = native_fn_opt {
                        native_fns.insert(func_id, native_fn);
                    }
                }
                Err(_e) => {
                    // Failed to resolve - likely a template method with unresolved type params
                    // Store unresolved for later resolution at template instantiation time
                    unresolved_functions.insert(func_id, ffi_func);

                    // Store native function if provided
                    if let Some(native_fn) = native_fn_opt {
                        native_fns.insert(func_id, native_fn);
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(FfiRegistry {
            types: self.types,
            type_names: self.type_names,
            functions: resolved_functions,
            function_names,
            native_fns,
            unresolved_functions,
            behaviors: self.behaviors,
            template_callbacks: self.template_callbacks,
            namespaces: self.namespaces,
        })
    }

    /// Resolve an interface definition's method types.
    fn resolve_interface(
        type_names: &FxHashMap<String, TypeId>,
        interface_def: &crate::types::FfiInterfaceDef,
        qualified_name: &str,
    ) -> Result<TypeDef, FfiRegistryError> {
        let methods: Result<Vec<MethodSignature>, FfiRegistryError> = interface_def
            .methods()
            .iter()
            .map(|m| {
                // Resolve params
                let params: Result<Vec<DataType>, FfiRegistryError> = m
                    .params
                    .iter()
                    .map(|p| Self::resolve_ffi_data_type(type_names, &p.data_type))
                    .collect();

                // Resolve return type
                let return_type = Self::resolve_ffi_data_type(type_names, &m.return_type)?;

                Ok(MethodSignature {
                    name: m.name.clone(),
                    params: params?,
                    return_type,
                    is_const: m.is_const,
                })
            })
            .collect();

        Ok(TypeDef::Interface {
            name: interface_def.name().to_string(),
            qualified_name: qualified_name.to_string(),
            methods: methods?,
        })
    }

    /// Resolve a funcdef definition's types.
    fn resolve_funcdef(
        type_names: &FxHashMap<String, TypeId>,
        funcdef_def: &crate::types::FfiFuncdefDef,
        qualified_name: &str,
    ) -> Result<TypeDef, FfiRegistryError> {
        // Resolve params
        let params: Result<Vec<DataType>, FfiRegistryError> = funcdef_def
            .params
            .iter()
            .map(|p| Self::resolve_ffi_data_type(type_names, &p.data_type))
            .collect();

        // Resolve return type
        let return_type = Self::resolve_ffi_data_type(type_names, &funcdef_def.return_type)?;

        Ok(TypeDef::Funcdef {
            name: funcdef_def.name.clone(),
            qualified_name: qualified_name.to_string(),
            params: params?,
            return_type,
        })
    }

    /// Resolve an FfiDataType to a DataType.
    fn resolve_ffi_data_type(
        type_names: &FxHashMap<String, TypeId>,
        ffi_type: &crate::types::FfiDataType,
    ) -> Result<DataType, FfiRegistryError> {
        use crate::types::{FfiDataType, UnresolvedBaseType};

        match ffi_type {
            FfiDataType::Resolved(dt) => Ok(dt.clone()),
            FfiDataType::Unresolved {
                base,
                is_const,
                is_handle,
                is_handle_to_const,
                ref_modifier,
            } => {
                let type_id = match base {
                    UnresolvedBaseType::Simple(name) => type_names
                        .get(name)
                        .copied()
                        .ok_or_else(|| FfiRegistryError::TypeNotFound(name.clone()))?,
                    UnresolvedBaseType::Template { name, args: _ } => {
                        // For templates, just look up the base template type
                        // (full instantiation happens elsewhere)
                        type_names
                            .get(name)
                            .copied()
                            .ok_or_else(|| FfiRegistryError::TypeNotFound(name.clone()))?
                    }
                };

                Ok(DataType {
                    type_id,
                    is_const: *is_const,
                    is_handle: *is_handle,
                    is_handle_to_const: *is_handle_to_const,
                    ref_modifier: *ref_modifier,
                })
            }
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur when building an FfiRegistry.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FfiRegistryError {
    /// Failed to resolve a function's types.
    #[error("function resolution failed: {0}")]
    FunctionResolution(FfiResolutionError),

    /// A referenced type was not found.
    #[error("type not found: {0}")]
    TypeNotFound(String),

    /// Duplicate type registration.
    #[error("duplicate type: {0}")]
    DuplicateType(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FfiDataType, FfiParam};

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
        assert_eq!(builder.lookup_type("void"), Some(VOID_TYPE));
        assert_eq!(builder.lookup_type("int"), Some(INT32_TYPE));
        assert_eq!(builder.lookup_type("float"), Some(FLOAT_TYPE));
    }

    #[test]
    fn builder_register_type() {
        let mut builder = FfiRegistryBuilder::new();

        let type_id = builder.register_type(
            TypeDef::Enum {
                name: "Color".to_string(),
                qualified_name: "Color".to_string(),
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

        builder.register_type_alias("integer", INT32_TYPE);

        assert_eq!(builder.lookup_type("integer"), Some(INT32_TYPE));
        assert_eq!(builder.lookup_type("int"), Some(INT32_TYPE));
    }

    #[test]
    fn builder_build_empty() {
        let builder = FfiRegistryBuilder::new();
        let registry = builder.build().unwrap();

        // Should have primitive names registered for lookup
        // (TypeDef::Primitive is handled by Registry, not FfiRegistry)
        assert!(registry.get_type_by_name("void").is_some());
        assert!(registry.get_type_by_name("int").is_some());
        assert_eq!(registry.get_type_by_name("void"), Some(VOID_TYPE));
        assert_eq!(registry.get_type_by_name("int"), Some(INT32_TYPE));
    }

    #[test]
    fn builder_build_with_resolved_function() {
        let mut builder = FfiRegistryBuilder::new();

        let func = FfiFunctionDef::new(FunctionId::next_ffi(), "add")
            .with_params(vec![
                FfiParam::new("a", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
                FfiParam::new("b", FfiDataType::resolved(DataType::simple(INT32_TYPE))),
            ])
            .with_return_type(FfiDataType::resolved(DataType::simple(INT32_TYPE)));

        builder.register_function(func, None);

        let registry = builder.build().unwrap();

        assert_eq!(registry.function_count(), 1);
        let func_ids = registry.lookup_functions("add");
        assert_eq!(func_ids.len(), 1);

        let resolved = registry.get_function(func_ids[0]).unwrap();
        assert_eq!(resolved.name, "add");
        assert_eq!(resolved.params.len(), 2);
        assert_eq!(resolved.return_type.type_id, INT32_TYPE);
    }

    #[test]
    fn builder_build_with_unresolved_function() {
        let mut builder = FfiRegistryBuilder::new();

        // Register a custom type first
        let my_class_id = builder.register_type(
            TypeDef::Class {
                name: "MyClass".to_string(),
                qualified_name: "MyClass".to_string(),
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
                type_kind: crate::types::TypeKind::reference(),
            },
            Some("MyClass"),
        );

        // Register function with unresolved type
        let func = FfiFunctionDef::new(FunctionId::next_ffi(), "process")
            .with_params(vec![FfiParam::new(
                "obj",
                FfiDataType::unresolved_handle("MyClass", false),
            )])
            .with_return_type(FfiDataType::resolved(DataType::simple(VOID_TYPE)));

        builder.register_function(func, None);

        let registry = builder.build().unwrap();

        let func_ids = registry.lookup_functions("process");
        assert_eq!(func_ids.len(), 1);

        let resolved = registry.get_function(func_ids[0]).unwrap();
        assert_eq!(resolved.params[0].data_type.type_id, my_class_id);
        assert!(resolved.params[0].data_type.is_handle);
    }

    #[test]
    fn builder_build_with_unknown_type_stores_unresolved() {
        let mut builder = FfiRegistryBuilder::new();

        // Register function referencing unknown type
        let func_id = FunctionId::next_ffi();
        let func = FfiFunctionDef::new(func_id, "process")
            .with_params(vec![FfiParam::new(
                "obj",
                FfiDataType::unresolved_simple("UnknownType"),
            )]);

        builder.register_function(func, None);

        // Build succeeds - unknown types are stored as unresolved for later resolution
        let result = builder.build();
        assert!(result.is_ok());

        let registry = result.unwrap();
        // Function should be in unresolved_functions, not functions
        assert!(registry.get_function(func_id).is_none());
        assert!(registry.get_unresolved_function(func_id).is_some());
    }

    #[test]
    fn registry_lookup_enum_value() {
        let mut builder = FfiRegistryBuilder::new();

        let type_id = builder.register_type(
            TypeDef::Enum {
                name: "Color".to_string(),
                qualified_name: "Color".to_string(),
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
                type_kind: crate::types::TypeKind::reference(),
            },
            Some("MyClass"),
        );

        let ctor_id = FunctionId::next_ffi();
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
                type_kind: crate::types::TypeKind::reference(),
            },
            Some("Base"),
        );

        // Register derived class
        let derived_id = builder.register_type(
            TypeDef::Class {
                name: "Derived".to_string(),
                qualified_name: "Derived".to_string(),
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
                type_kind: crate::types::TypeKind::reference(),
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
                fields: Vec::new(),
                methods: Vec::new(),
                base_class: None,
                interfaces: Vec::new(),
                operator_methods: FxHashMap::default(),
                properties: FxHashMap::default(),
                is_final: false,
                is_abstract: false,
                template_params: vec![TypeId::next_ffi()], // One template param
                template: None,
                type_args: Vec::new(),
                type_kind: crate::types::TypeKind::reference(),
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
        let err = FfiRegistryError::TypeNotFound("MyClass".to_string());
        assert!(err.to_string().contains("type not found"));
        assert!(err.to_string().contains("MyClass"));

        let err = FfiRegistryError::DuplicateType("MyClass".to_string());
        assert!(err.to_string().contains("duplicate type"));
    }
}
