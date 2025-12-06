//! Unified compilation context providing access to both FFI and Script types.
//!
//! `CompilationContext` is the unified facade for type/function lookups during compilation.
//! It holds an immutable `Arc<FfiRegistry>` (shared across all Units) and a mutable
//! `ScriptRegistry` (per-compilation), routing lookups based on the FFI_BIT in TypeId/FunctionId.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   CompilationContext                         │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  type_by_name: HashMap<String, TypeId>              │   │
//! │  │  func_by_name: HashMap<String, Vec<FunctionId>>     │   │
//! │  │  (unified name lookup - FFI + Script)               │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                              │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  ffi: Arc<FfiRegistry>                              │   │
//! │  │  (immutable, shared - primitives, FFI types)        │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                              │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  script: ScriptRegistry<'ast>                       │   │
//! │  │  (mutable - script-defined types)                   │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! │                                                              │
//! │  ┌─────────────────────────────────────────────────────┐   │
//! │  │  template_cache: HashMap<(TypeId, Vec<TypeId>), TypeId>│ │
//! │  │  (template instantiation cache)                      │   │
//! │  └─────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Lookup Behavior
//!
//! - **By ID** (`get_type`, `get_function`): Routes by `id.is_ffi()` bit
//! - **By Name** (`lookup_type`, `lookup_functions`): Single unified HashMap lookup
//! - **Mutable access** (`get_type_mut`): Only for script types (FFI is immutable)
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use angelscript::ffi::FfiRegistry;
//! use angelscript::semantic::CompilationContext;
//!
//! let ffi_registry = Arc::new(FfiRegistryBuilder::new().build().unwrap());
//! let mut ctx = CompilationContext::new(ffi_registry);
//!
//! // Lookup primitives (from FFI)
//! let int_type = ctx.lookup_type("int").unwrap();
//!
//! // Register script types
//! let player_id = ctx.register_type(player_typedef, Some("Player"));
//!
//! // Unified lookup works for both
//! assert!(ctx.lookup_type("int").is_some());
//! assert!(ctx.lookup_type("Player").is_some());
//! ```

use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::ffi::FfiRegistry;
use crate::semantic::error::SemanticError;
use crate::semantic::template_instantiator::TemplateInstantiator;
use crate::semantic::types::behaviors::TypeBehaviors;
use crate::semantic::types::registry::{FunctionDef, GlobalVarDef, MixinDef, ScriptParam, ScriptRegistry};
use crate::semantic::types::type_def::{
    FieldDef, FunctionId, FunctionTraits, MethodSignature, OperatorBehavior, PropertyAccessors,
    TypeDef, TypeId, Visibility,
};
use crate::semantic::types::DataType;
use crate::types::ResolvedFfiFunctionDef;

/// Unified reference to a function definition (either FFI or Script).
///
/// This enum provides a unified interface to access function metadata
/// regardless of whether the function is from FFI or script code.
#[derive(Debug, Clone, Copy)]
pub enum FunctionRef<'a, 'ast> {
    /// Reference to a script function
    Script(&'a FunctionDef<'ast>),
    /// Reference to an FFI function
    Ffi(&'a ResolvedFfiFunctionDef),
}

impl<'a, 'ast> FunctionRef<'a, 'ast> {
    /// Get the function's unique identifier.
    pub fn id(&self) -> FunctionId {
        match self {
            FunctionRef::Script(f) => f.id,
            FunctionRef::Ffi(f) => f.id,
        }
    }

    /// Get the function name.
    pub fn name(&self) -> &str {
        match self {
            FunctionRef::Script(f) => &f.name,
            FunctionRef::Ffi(f) => &f.name,
        }
    }

    /// Get the function's return type.
    pub fn return_type(&self) -> &DataType {
        match self {
            FunctionRef::Script(f) => &f.return_type,
            FunctionRef::Ffi(f) => &f.return_type,
        }
    }

    /// Get the function traits (const, virtual, etc.).
    pub fn traits(&self) -> &FunctionTraits {
        match self {
            FunctionRef::Script(f) => &f.traits,
            FunctionRef::Ffi(f) => &f.traits,
        }
    }

    /// Get the parameter types as DataTypes.
    pub fn param_types(&self) -> Vec<DataType> {
        match self {
            FunctionRef::Script(f) => f.params.iter().map(|p| p.data_type.clone()).collect(),
            FunctionRef::Ffi(f) => f.params.iter().map(|p| p.data_type.clone()).collect(),
        }
    }

    /// Get a specific parameter's DataType by index.
    ///
    /// Panics if index is out of bounds.
    pub fn param_type(&self, index: usize) -> &DataType {
        match self {
            FunctionRef::Script(f) => &f.params[index].data_type,
            FunctionRef::Ffi(f) => &f.params[index].data_type,
        }
    }

    /// Get the number of parameters.
    pub fn param_count(&self) -> usize {
        match self {
            FunctionRef::Script(f) => f.params.len(),
            FunctionRef::Ffi(f) => f.params.len(),
        }
    }

    /// Get the owning type if this is a method.
    pub fn owner_type(&self) -> Option<TypeId> {
        match self {
            FunctionRef::Script(f) => f.object_type,
            FunctionRef::Ffi(f) => f.owner_type,
        }
    }

    /// Check if this is a method (has an owner type).
    pub fn is_method(&self) -> bool {
        self.owner_type().is_some()
    }

    /// Get the visibility of this function.
    pub fn visibility(&self) -> Visibility {
        match self {
            FunctionRef::Script(f) => f.visibility,
            FunctionRef::Ffi(f) => f.visibility,
        }
    }

    /// Get the number of required parameters (without defaults).
    pub fn required_param_count(&self) -> usize {
        match self {
            FunctionRef::Script(f) => f.params.iter().filter(|p| p.default.is_none()).count(),
            FunctionRef::Ffi(f) => f.params.iter().filter(|p| p.default_value.is_none()).count(),
        }
    }

    /// Check if this function has default arguments.
    pub fn has_defaults(&self) -> bool {
        self.required_param_count() < self.param_count()
    }

    /// Get as a script function reference, if this is a script function.
    pub fn as_script(&self) -> Option<&'a FunctionDef<'ast>> {
        match self {
            FunctionRef::Script(f) => Some(f),
            FunctionRef::Ffi(_) => None,
        }
    }

    /// Get as an FFI function reference, if this is an FFI function.
    pub fn as_ffi(&self) -> Option<&'a ResolvedFfiFunctionDef> {
        match self {
            FunctionRef::Script(_) => None,
            FunctionRef::Ffi(f) => Some(f),
        }
    }

    /// Check if this is a script function.
    pub fn is_script(&self) -> bool {
        matches!(self, FunctionRef::Script(_))
    }

    /// Check if this is an FFI function.
    pub fn is_ffi(&self) -> bool {
        matches!(self, FunctionRef::Ffi(_))
    }
}

/// Routes a lookup by TypeId to the appropriate registry (FFI or Script).
///
/// This macro reduces boilerplate for methods that have the same signature on both registries.
/// It routes based on `type_id.is_ffi()`.
macro_rules! route_type_lookup {
    ($self:expr, $type_id:expr, $method:ident $(, $args:expr)*) => {
        if $type_id.is_ffi() {
            $self.ffi.$method($type_id $(, $args)*)
        } else {
            $self.script.$method($type_id $(, $args)*)
        }
    };
}

/// Unified compilation context providing access to both FFI and Script registries.
///
/// This is the primary interface for type and function lookups during compilation.
/// It maintains unified name→ID maps for fast lookup, and routes ID-based queries
/// to the appropriate registry based on the FFI_BIT.
pub struct CompilationContext<'ast> {
    /// Immutable FFI registry (shared across all Units)
    ffi: Arc<FfiRegistry>,

    /// Mutable script registry (per-compilation)
    script: ScriptRegistry<'ast>,

    /// Unified type name → TypeId map (FFI + Script)
    type_by_name: FxHashMap<String, TypeId>,

    /// Unified function name → FunctionIds map (FFI + Script)
    func_by_name: FxHashMap<String, Vec<FunctionId>>,

    /// Template instantiator with cache
    template_instantiator: TemplateInstantiator,
}

impl<'ast> std::fmt::Debug for CompilationContext<'ast> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilationContext")
            .field("ffi", &self.ffi)
            .field("script", &self.script)
            .field("type_by_name", &format!("<{} entries>", self.type_by_name.len()))
            .field("func_by_name", &format!("<{} entries>", self.func_by_name.len()))
            .field("template_instantiator", &self.template_instantiator)
            .finish()
    }
}

impl<'ast> CompilationContext<'ast> {
    /// Create a new compilation context with the given FFI registry.
    ///
    /// The unified name maps are initialized from the FFI registry.
    pub fn new(ffi: Arc<FfiRegistry>) -> Self {
        Self {
            type_by_name: ffi.type_by_name().clone(),
            func_by_name: ffi.func_by_name().clone(),
            ffi,
            script: ScriptRegistry::new(),
            template_instantiator: TemplateInstantiator::new(),
        }
    }

    /// Get a reference to the underlying FFI registry.
    pub fn ffi(&self) -> &FfiRegistry {
        &self.ffi
    }

    /// Get a reference to the underlying script registry.
    pub fn script(&self) -> &ScriptRegistry<'ast> {
        &self.script
    }

    /// Get a mutable reference to the underlying script registry.
    pub fn script_mut(&mut self) -> &mut ScriptRegistry<'ast> {
        &mut self.script
    }

    // =========================================================================
    // Type Lookups
    // =========================================================================

    /// Look up a type by name.
    ///
    /// This uses the unified name map which includes both FFI and script types.
    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.type_by_name.get(name).copied()
    }

    /// Get a type definition by TypeId.
    ///
    /// Routes to FFI or Script registry based on the FFI_BIT.
    /// Panics if the TypeId is not found.
    pub fn get_type(&self, type_id: TypeId) -> &TypeDef {
        if type_id.is_ffi() {
            self.ffi
                .get_type(type_id)
                .expect("FFI TypeId not found in FfiRegistry")
        } else {
            self.script.get_type(type_id)
        }
    }

    /// Try to get a type definition by TypeId.
    ///
    /// Returns None if the TypeId is not found.
    pub fn try_get_type(&self, type_id: TypeId) -> Option<&TypeDef> {
        if type_id.is_ffi() {
            self.ffi.get_type(type_id)
        } else {
            // ScriptRegistry::get_type panics, so we need to check first
            if self.script.type_by_name().values().any(|&id| id == type_id) {
                Some(self.script.get_type(type_id))
            } else {
                // Check if it's in the types map directly
                // This is a workaround since ScriptRegistry doesn't have try_get_type
                None
            }
        }
    }

    /// Get access to the unified type name map.
    pub fn type_by_name(&self) -> &FxHashMap<String, TypeId> {
        &self.type_by_name
    }

    /// Get the total count of registered types (FFI + Script).
    pub fn type_count(&self) -> usize {
        self.ffi.type_count() + self.script.type_count()
    }

    // =========================================================================
    // Type Registration (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a new script type and return its TypeId.
    ///
    /// The type is added to both the ScriptRegistry and the unified name map.
    pub fn register_type(&mut self, typedef: TypeDef, name: Option<&str>) -> TypeId {
        let type_id = self.script.register_type(typedef, name);

        if let Some(name) = name {
            self.type_by_name.insert(name.to_string(), type_id);
        }

        type_id
    }

    /// Register a type alias.
    ///
    /// Creates an alias name that points to an existing type.
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeId) {
        self.script.register_type_alias(alias_name, target_type);
        self.type_by_name.insert(alias_name.to_string(), target_type);
    }

    // =========================================================================
    // Function Lookups
    // =========================================================================

    /// Look up all functions with the given name (for overload resolution).
    ///
    /// Uses the unified name map.
    pub fn lookup_functions(&self, name: &str) -> &[FunctionId] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a function definition by FunctionId (unified interface).
    ///
    /// Returns a `FunctionRef` that provides unified access to function metadata
    /// for both FFI and script functions.
    /// Panics if the function is not found.
    pub fn get_function(&self, func_id: FunctionId) -> FunctionRef<'_, 'ast> {
        if func_id.is_ffi() {
            FunctionRef::Ffi(
                self.ffi
                    .get_function(func_id)
                    .expect("FFI function not found"),
            )
        } else {
            FunctionRef::Script(self.script.get_function(func_id))
        }
    }

    /// Get the total count of registered functions (FFI + Script).
    pub fn function_count(&self) -> usize {
        self.ffi.function_count() + self.script.function_count()
    }

    /// Generate the next script FunctionId.
    ///
    /// This allocates a new FunctionId for a script-defined function.
    /// FFI functions get their IDs from FfiRegistry.
    pub fn next_function_id(&self) -> FunctionId {
        FunctionId::next()
    }

    // =========================================================================
    // Function Registration (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a script function and return its FunctionId.
    ///
    /// The function is added to both the ScriptRegistry and the unified name map.
    pub fn register_function(&mut self, def: FunctionDef<'ast>) -> FunctionId {
        let func_id = def.id;
        let qualified_name = def.qualified_name();

        self.script.register_function(def);

        // Add to unified name map
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_id);

        func_id
    }

    /// Update a function's signature.
    pub fn update_function_signature(
        &mut self,
        qualified_name: &str,
        params: Vec<ScriptParam<'ast>>,
        return_type: DataType,
        object_type: Option<TypeId>,
        traits: FunctionTraits,
    ) {
        self.script.update_function_signature(
            qualified_name,
            params,
            return_type,
            object_type,
            traits,
        );
    }

    /// Update a function's parameters directly by FunctionId.
    pub fn update_function_params(&mut self, func_id: FunctionId, params: Vec<ScriptParam<'ast>>) {
        self.script.update_function_params(func_id, params);
    }

    /// Update a function's return type directly by FunctionId.
    pub fn update_function_return_type(&mut self, func_id: FunctionId, return_type: DataType) {
        self.script.update_function_return_type(func_id, return_type);
    }

    // =========================================================================
    // Behavior Lookups
    // =========================================================================

    /// Get the behaviors for a type, if any are registered.
    ///
    /// Routes to FFI or Script registry based on the FFI_BIT.
    pub fn get_behaviors(&self, type_id: TypeId) -> Option<&TypeBehaviors> {
        route_type_lookup!(self, type_id, get_behaviors)
    }

    /// Find all constructors for a given type (value types).
    pub fn find_constructors(&self, type_id: TypeId) -> Vec<FunctionId> {
        route_type_lookup!(self, type_id, find_constructors)
    }

    /// Find all factories for a given type (reference types).
    pub fn find_factories(&self, type_id: TypeId) -> Vec<FunctionId> {
        route_type_lookup!(self, type_id, find_factories)
    }

    /// Find a constructor for a type with specific argument types.
    pub fn find_constructor(&self, type_id: TypeId, arg_types: &[DataType]) -> Option<FunctionId> {
        route_type_lookup!(self, type_id, find_constructor, arg_types)
    }

    /// Find the copy constructor for a type.
    /// Copy constructor has signature: ClassName(const ClassName&in) or ClassName(const ClassName&inout)
    pub fn find_copy_constructor(&self, type_id: TypeId) -> Option<FunctionId> {
        route_type_lookup!(self, type_id, find_copy_constructor)
    }

    /// Check if a constructor is marked as explicit.
    pub fn is_constructor_explicit(&self, func_id: FunctionId) -> bool {
        route_type_lookup!(self, func_id, is_constructor_explicit)
    }

    // =========================================================================
    // Method Lookups
    // =========================================================================

    /// Get all methods for a given type.
    pub fn get_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        route_type_lookup!(self, type_id, get_methods)
    }

    /// Find a method by name on a type (first match, with inheritance).
    pub fn find_method(&self, type_id: TypeId, name: &str) -> Option<FunctionId> {
        route_type_lookup!(self, type_id, find_method, name)
    }

    /// Find all methods with the given name on a type (for overload resolution).
    pub fn find_methods_by_name(&self, type_id: TypeId, name: &str) -> Vec<FunctionId> {
        route_type_lookup!(self, type_id, find_methods_by_name, name)
    }

    /// Get all methods for a class, including inherited methods.
    pub fn get_all_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        if type_id.is_ffi() {
            // FfiRegistry doesn't have get_all_methods, use get_methods
            self.ffi.get_methods(type_id)
        } else {
            self.script.get_all_methods(type_id)
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
        route_type_lookup!(self, type_id, find_operator_method, operator)
    }

    /// Find all overloads of an operator method for a type.
    pub fn find_operator_methods(
        &self,
        type_id: TypeId,
        operator: OperatorBehavior,
    ) -> &[FunctionId] {
        route_type_lookup!(self, type_id, find_operator_methods, operator)
    }

    /// Find the best operator method based on desired mutability.
    ///
    /// This method uses unified function lookup because operator methods
    /// on script types (like template instances) may be FFI functions.
    pub fn find_operator_method_with_mutability(
        &self,
        type_id: TypeId,
        operator: OperatorBehavior,
        prefer_mutable: bool,
    ) -> Option<FunctionId> {
        let overloads = self.find_operator_methods(type_id, operator);
        if overloads.is_empty() {
            return None;
        }

        // If only one overload, return it
        if overloads.len() == 1 {
            return Some(overloads[0]);
        }

        // Multiple overloads - find the one matching our preference
        // For mutable access, we want the one with non-const return type
        // For const access, we want the const one
        let mut mutable_method = None;
        let mut const_method = None;

        for &func_id in overloads {
            // Use unified lookup since operator methods could be FFI or Script
            let func = self.get_function(func_id);
            if func.return_type().is_const {
                const_method = Some(func_id);
            } else {
                mutable_method = Some(func_id);
            }
        }

        if prefer_mutable {
            mutable_method.or(const_method)
        } else {
            const_method.or(mutable_method)
        }
    }

    // =========================================================================
    // Property Lookups
    // =========================================================================

    /// Find a property by name on a type.
    pub fn find_property(&self, type_id: TypeId, property_name: &str) -> Option<PropertyAccessors> {
        route_type_lookup!(self, type_id, find_property, property_name)
    }

    /// Get all properties for a type (including inherited).
    pub fn get_all_properties(&self, type_id: TypeId) -> FxHashMap<String, PropertyAccessors> {
        route_type_lookup!(self, type_id, get_all_properties)
    }

    // =========================================================================
    // Inheritance Support
    // =========================================================================

    /// Get the base class of a type (if any).
    pub fn get_base_class(&self, type_id: TypeId) -> Option<TypeId> {
        route_type_lookup!(self, type_id, get_base_class)
    }

    /// Check if `derived_class` is a subclass of `base_class`.
    pub fn is_subclass_of(&self, derived_class: TypeId, base_class: TypeId) -> bool {
        if derived_class == base_class {
            return true;
        }

        // Walk the inheritance chain
        let mut current = self.get_base_class(derived_class);
        while let Some(parent_id) = current {
            if parent_id == base_class {
                return true;
            }
            current = self.get_base_class(parent_id);
        }

        false
    }

    /// Check if a class is marked as 'final'.
    pub fn is_class_final(&self, type_id: TypeId) -> bool {
        if type_id.is_ffi() {
            match self.ffi.get_type(type_id) {
                Some(TypeDef::Class { is_final, .. }) => *is_final,
                _ => false,
            }
        } else {
            self.script.is_class_final(type_id)
        }
    }

    // =========================================================================
    // Interface Support
    // =========================================================================

    /// Get all method signatures for an interface type.
    pub fn get_interface_methods(&self, type_id: TypeId) -> Option<&[MethodSignature]> {
        route_type_lookup!(self, type_id, get_interface_methods)
    }

    /// Get all interfaces implemented by a class.
    pub fn get_all_interfaces(&self, type_id: TypeId) -> Vec<TypeId> {
        route_type_lookup!(self, type_id, get_all_interfaces)
    }

    /// Check if a class has a method matching an interface method signature.
    pub fn has_method_matching_interface(
        &self,
        class_type_id: TypeId,
        interface_method: &MethodSignature,
    ) -> bool {
        if class_type_id.is_ffi() {
            self.ffi.has_method_matching_interface(class_type_id, interface_method)
        } else {
            self.script
                .has_method_matching_interface(class_type_id, interface_method)
        }
    }

    // =========================================================================
    // Enum Support
    // =========================================================================

    /// Look up an enum value by enum type ID and value name.
    pub fn lookup_enum_value(&self, type_id: TypeId, value_name: &str) -> Option<i64> {
        route_type_lookup!(self, type_id, lookup_enum_value, value_name)
    }

    // =========================================================================
    // Funcdef Support
    // =========================================================================

    /// Get the signature of a funcdef type.
    pub fn get_funcdef_signature(&self, type_id: TypeId) -> Option<(&[DataType], &DataType)> {
        route_type_lookup!(self, type_id, get_funcdef_signature)
    }

    /// Check if a function is compatible with a funcdef type.
    pub fn is_function_compatible_with_funcdef(
        &self,
        func_id: FunctionId,
        funcdef_type_id: TypeId,
    ) -> bool {
        if func_id.is_ffi() {
            self.ffi.is_function_compatible_with_funcdef(func_id, funcdef_type_id)
        } else {
            self.script
                .is_function_compatible_with_funcdef(func_id, funcdef_type_id)
        }
    }

    /// Find a function by name that is compatible with a funcdef type.
    pub fn find_compatible_function(
        &self,
        name: &str,
        funcdef_type_id: TypeId,
    ) -> Option<FunctionId> {
        // Try script functions first
        if let Some(func_id) = self.script.find_compatible_function(name, funcdef_type_id) {
            return Some(func_id);
        }
        // Try FFI functions
        self.ffi.find_compatible_function(name, funcdef_type_id)
    }

    // =========================================================================
    // Class Field Support
    // =========================================================================

    /// Get the fields of a class (does not include inherited fields).
    pub fn get_class_fields(&self, type_id: TypeId) -> &[FieldDef] {
        if type_id.is_ffi() {
            match self.ffi.get_type(type_id) {
                Some(TypeDef::Class { fields, .. }) => fields,
                _ => &[],
            }
        } else {
            self.script.get_class_fields(type_id)
        }
    }

    // =========================================================================
    // Global Variable Support (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a global variable.
    pub fn register_global_var(
        &mut self,
        name: String,
        namespace: Vec<String>,
        data_type: DataType,
    ) {
        self.script.register_global_var(name, namespace, data_type);
    }

    /// Look up a global variable by qualified name.
    pub fn lookup_global_var(&self, name: &str) -> Option<&GlobalVarDef> {
        self.script.lookup_global_var(name)
    }

    // =========================================================================
    // Mixin Support (delegates to ScriptRegistry)
    // =========================================================================

    /// Register a mixin class.
    pub fn register_mixin(&mut self, mixin: MixinDef<'ast>) {
        self.script.register_mixin(mixin);
    }

    /// Look up a mixin by qualified name.
    pub fn lookup_mixin(&self, name: &str) -> Option<&MixinDef<'ast>> {
        self.script.lookup_mixin(name)
    }

    /// Check if a name refers to a mixin.
    pub fn is_mixin(&self, name: &str) -> bool {
        self.script.is_mixin(name)
    }

    // =========================================================================
    // Template Instantiation
    // =========================================================================

    /// Instantiate a template type with the given type arguments.
    ///
    /// This creates a new concrete type from a template (e.g., `array<int>` from `array<T>`).
    /// Template instances are cached to avoid duplicate instantiations.
    ///
    /// # Arguments
    /// - `template_id`: The TypeId of the template type (must have template_params)
    /// - `args`: The concrete type arguments to substitute for template parameters
    ///
    /// # Returns
    /// - `Ok(TypeId)` - The TypeId of the instantiated type
    /// - `Err(SemanticError)` - If the template is invalid or validation fails
    pub fn instantiate_template(
        &mut self,
        template_id: TypeId,
        args: Vec<DataType>,
    ) -> Result<TypeId, SemanticError> {
        self.template_instantiator.instantiate(
            template_id,
            args,
            &self.ffi,
            &mut self.script,
            &mut self.type_by_name,
        )
    }

    /// Check if a type is a template (has template parameters).
    ///
    /// Templates are always FFI types - script types can only be template *instances*.
    pub fn is_template(&self, type_id: TypeId) -> bool {
        type_id.is_ffi() && self.ffi.is_template(type_id)
    }
}

impl<'ast> Default for CompilationContext<'ast> {
    fn default() -> Self {
        Self::new(Arc::new(
            crate::ffi::FfiRegistryBuilder::new().build().unwrap(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::FfiRegistryBuilder;
    use crate::semantic::types::type_def::{BOOL_TYPE, INT32_TYPE, VOID_TYPE};
    use crate::types::TypeKind;

    fn create_test_context() -> CompilationContext<'static> {
        let ffi = Arc::new(FfiRegistryBuilder::new().build().unwrap());
        CompilationContext::new(ffi)
    }

    #[test]
    fn new_context_has_primitives() {
        let ctx = create_test_context();

        // Primitives should be accessible via unified lookup
        // Note: "int" is the name for int32, "uint" is the name for uint32
        assert!(ctx.lookup_type("void").is_some());
        assert!(ctx.lookup_type("bool").is_some());
        assert!(ctx.lookup_type("int").is_some());
        assert!(ctx.lookup_type("int8").is_some());
        assert!(ctx.lookup_type("int16").is_some());
        assert!(ctx.lookup_type("int64").is_some());
        assert!(ctx.lookup_type("float").is_some());
        assert!(ctx.lookup_type("double").is_some());

        // Check TypeIds match constants
        assert_eq!(ctx.lookup_type("void"), Some(VOID_TYPE));
        assert_eq!(ctx.lookup_type("int"), Some(INT32_TYPE));
        assert_eq!(ctx.lookup_type("bool"), Some(BOOL_TYPE));
    }

    #[test]
    fn register_script_type() {
        let mut ctx = create_test_context();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
            type_kind: TypeKind::reference(),
        };

        let type_id = ctx.register_type(typedef, Some("Player"));

        // Should be findable via unified lookup
        assert_eq!(ctx.lookup_type("Player"), Some(type_id));

        // Should be a script type (no FFI bit)
        assert!(type_id.is_script());

        // Should be retrievable
        assert!(ctx.get_type(type_id).is_class());
    }

    #[test]
    fn get_type_routes_correctly() {
        let ctx = create_test_context();

        // FFI type (primitive)
        let void_type = ctx.get_type(VOID_TYPE);
        assert!(void_type.is_primitive());

        // Verify routing works for FFI types
        assert!(VOID_TYPE.is_ffi());
    }

    #[test]
    fn type_alias_works() {
        let mut ctx = create_test_context();

        ctx.register_type_alias("integer", INT32_TYPE);

        assert_eq!(ctx.lookup_type("integer"), Some(INT32_TYPE));
        assert_eq!(ctx.lookup_type("int"), Some(INT32_TYPE));
    }

    #[test]
    fn instantiate_template_basic() {
        let mut builder = FfiRegistryBuilder::new();

        // Register a template type
        let t_param = TypeId::next_ffi();
        builder.register_type_with_id(
            t_param,
            TypeDef::TemplateParam {
                name: "T".to_string(),
                index: 0,
                owner: TypeId::next_ffi(), // Will be updated
            },
            None,
        );

        let template_def = TypeDef::Class {
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
            template_params: vec![t_param],
            template: None,
            type_args: Vec::new(),
            type_kind: TypeKind::reference(),
        };

        let template_id = builder.register_type(template_def, Some("array"));

        let ffi = Arc::new(builder.build().unwrap());
        let mut ctx = CompilationContext::new(ffi);

        // Instantiate array<int>
        let instance_id = ctx
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        // Should be a script type
        assert!(instance_id.is_script());

        // Should be cached
        let instance_id2 = ctx
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();
        assert_eq!(instance_id, instance_id2);

        // Should be findable by name (int32's name is "int")
        assert_eq!(ctx.lookup_type("array<int>"), Some(instance_id));
    }

    #[test]
    fn instantiate_template_wrong_arg_count() {
        let mut builder = FfiRegistryBuilder::new();

        let t_param = TypeId::next_ffi();
        builder.register_type_with_id(
            t_param,
            TypeDef::TemplateParam {
                name: "T".to_string(),
                index: 0,
                owner: TypeId::next_ffi(),
            },
            None,
        );

        let template_def = TypeDef::Class {
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
            template_params: vec![t_param],
            template: None,
            type_args: Vec::new(),
            type_kind: TypeKind::reference(),
        };

        let template_id = builder.register_type(template_def, Some("array"));

        let ffi = Arc::new(builder.build().unwrap());
        let mut ctx = CompilationContext::new(ffi);

        // Try to instantiate with wrong number of args
        let result = ctx.instantiate_template(template_id, vec![]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("expects 1 type arguments"));
    }

    #[test]
    fn instantiate_non_template_fails() {
        let mut ctx = create_test_context();

        // INT32_TYPE is a primitive, not a template
        let result = ctx.instantiate_template(INT32_TYPE, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn context_debug() {
        let ctx = create_test_context();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("CompilationContext"));
    }

    #[test]
    fn context_default() {
        let ctx = CompilationContext::default();
        assert!(ctx.lookup_type("int").is_some());
    }
}
