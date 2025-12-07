//! Script-only registry for types, functions, and variables.
//!
//! The ScriptRegistry is storage for script-defined declarations in an AngelScript
//! program. It stores script-defined types (classes, interfaces, enums), functions
//! (with overloading support), and global variables.
//!
//! FFI types (including primitives) are stored in `FfiRegistry` and accessed via
//! `CompilationContext` which provides a unified lookup interface.
//!
//! # Architecture
//!
//! - **Types**: Stored in HashMap with TypeHash as key (script TypeIds don't have FFI_BIT)
//! - **Functions**: Stored in HashMap with TypeHash as key, plus a name→[TypeHash] map for overloading
//! - **Behaviors**: Stored separately from TypeDef for lifecycle callbacks
//!
//! # Example
//!
//! ```ignore
//! use angelscript::semantic::ScriptRegistry;
//!
//! let registry = ScriptRegistry::new();
//! // Script registry starts empty - no primitives
//! // Primitives are in FfiRegistry, accessed via CompilationContext
//! ```

use super::{
    DataType, FunctionTraits, OperatorBehavior, PropertyAccessors, TypeDef, Visibility,
};
use angelscript_core::TypeHash;
use angelscript_parser::ast::expr::Expr;
use rustc_hash::FxHashMap;

/// A script function parameter with type and optional default value.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptParam<'ast> {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub data_type: DataType,
    /// Default value expression (if any)
    pub default: Option<&'ast Expr<'ast>>,
}

impl<'ast> ScriptParam<'ast> {
    /// Create a new parameter with no default.
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            default: None,
        }
    }

    /// Create a new parameter with a default value.
    pub fn with_default(name: impl Into<String>, data_type: DataType, default: &'ast Expr<'ast>) -> Self {
        Self {
            name: name.into(),
            data_type,
            default: Some(default),
        }
    }
}

/// Function definition with complete signature
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef<'ast> {
    /// Deterministic hash for identity - computed from qualified name and parameter types
    /// This is the primary identity used for lookups in both FFI and script registries
    pub func_hash: TypeHash,
    /// Function name (unqualified)
    pub name: String,
    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,
    /// Parameters with types and optional defaults
    pub params: Vec<ScriptParam<'ast>>,
    /// Return type
    pub return_type: DataType,
    /// Object type if this is a method
    pub object_type: Option<TypeHash>,
    /// Function traits (virtual, const, etc.)
    pub traits: FunctionTraits,
    /// True if this is a native (FFI) function
    pub is_native: bool,
    /// Visibility (public, private, protected) - only meaningful for methods
    pub visibility: Visibility,
    /// Whether the function signature has been filled in by Pass 2a
    /// Functions are registered with empty signatures in Pass 1, then filled in Pass 2a
    pub signature_filled: bool,
}

impl<'ast> FunctionDef<'ast> {
    /// Get the qualified name of this function
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }
}

/// A global variable definition
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalVarDef {
    /// Variable name (unqualified)
    pub name: String,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Variable type
    pub data_type: DataType,
}

impl GlobalVarDef {
    /// Get the qualified name of this variable
    pub fn qualified_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }
}

/// A mixin class definition
///
/// Mixin classes are partial class structures that can be included into regular classes.
/// They are not real types and cannot be instantiated. When a class includes a mixin,
/// the mixin's methods and properties are copied into the class.
#[derive(Debug, Clone)]
pub struct MixinDef<'ast> {
    /// Mixin name (unqualified)
    pub name: String,
    /// Qualified name with namespace
    pub qualified_name: String,
    /// Namespace path
    pub namespace: Vec<String>,
    /// Interfaces that the mixin requires (classes including this mixin must implement these)
    pub required_interfaces: Vec<String>,
    /// Members of the mixin class (methods and fields)
    /// This is a slice into arena-allocated memory
    pub members: &'ast [angelscript_parser::ast::decl::ClassMember<'ast>],
}

impl<'ast> MixinDef<'ast> {
    /// Get the qualified name of this mixin
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }
}

/// Script-only registry for all script-defined types, functions, and variables.
///
/// This registry only holds script-defined items. FFI types (including primitives)
/// are stored in `FfiRegistry`. Use `CompilationContext` for unified access to both.
#[derive(Clone)]
pub struct ScriptRegistry<'ast> {
    // Type storage - HashMap for O(1) lookup by TypeHash (script TypeIds don't have FFI_BIT)
    types: FxHashMap<TypeHash, TypeDef>,
    type_by_name: FxHashMap<String, TypeHash>,

    // Type behaviors (lifecycle, initialization) - stored separately from TypeDef
    // This follows the C++ AngelScript pattern where behaviors are registered separately
    behaviors: FxHashMap<TypeHash, super::behaviors::TypeBehaviors>,

    // Function storage
    functions: FxHashMap<TypeHash, FunctionDef<'ast>>,
    func_by_name: FxHashMap<String, Vec<TypeHash>>,

    // Global variable storage
    global_vars: FxHashMap<String, GlobalVarDef>,

    // Mixin storage (mixins are not types, stored separately)
    mixins: FxHashMap<String, MixinDef<'ast>>,

    // === Hash-Based Lookups (Phase 2 TypeHash Migration) ===
    /// Types indexed by TypeHash (secondary index)
    types_by_hash: FxHashMap<TypeHash, TypeHash>,
}

impl<'ast> std::fmt::Debug for ScriptRegistry<'ast> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptRegistry")
            .field("types", &self.types)
            .field("type_by_name", &self.type_by_name)
            .field("behaviors", &self.behaviors)
            .field("functions", &self.functions)
            .field("func_by_name", &self.func_by_name)
            .field("global_vars", &self.global_vars)
            .field("mixins", &self.mixins)
            .field("types_by_hash", &format!("<{} entries>", self.types_by_hash.len()))
            .finish()
    }
}

impl<'ast> ScriptRegistry<'ast> {
    /// Create a new empty script registry.
    ///
    /// The registry starts empty - primitives and FFI types are stored in `FfiRegistry`
    /// and accessed via `CompilationContext`.
    pub fn new() -> Self {
        Self {
            types: FxHashMap::default(),
            type_by_name: FxHashMap::default(),
            behaviors: FxHashMap::default(),
            functions: FxHashMap::default(),
            func_by_name: FxHashMap::default(),
            global_vars: FxHashMap::default(),
            mixins: FxHashMap::default(),
            types_by_hash: FxHashMap::default(),
        }
    }

    /// Register a new type and return its TypeHash.
    /// Uses the typedef's own type_hash as the primary key.
    pub fn register_type(&mut self, typedef: TypeDef, name: Option<&str>) -> TypeHash {
        let type_hash = typedef.type_hash();
        self.types.insert(type_hash, typedef);
        self.types_by_hash.insert(type_hash, type_hash);

        if let Some(name) = name {
            self.type_by_name.insert(name.to_string(), type_hash);
        }

        type_hash
    }

    /// Register a type alias (typedef)
    ///
    /// This creates an alias name that points to an existing type.
    /// For example, `typedef float real;` would call `register_type_alias("real", primitives::FLOAT)`.
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeHash) {
        self.type_by_name
            .insert(alias_name.to_string(), target_type);
    }

    /// Look up a type by name (returns None if not found)
    pub fn lookup_type(&self, name: &str) -> Option<TypeHash> {
        self.type_by_name.get(name).copied()
    }

    /// Get access to the type name map for iteration
    pub fn type_by_name(&self) -> &FxHashMap<String, TypeHash> {
        &self.type_by_name
    }

    /// Get a type definition by TypeHash
    pub fn get_type(&self, type_id: TypeHash) -> &TypeDef {
        self.types.get(&type_id).expect("TypeHash not found in registry")
    }

    /// Try to get a type definition by TypeHash, returns None if not found.
    pub fn try_get_type(&self, type_id: TypeHash) -> Option<&TypeDef> {
        self.types.get(&type_id)
    }

    /// Get a mutable type definition by TypeHash.
    /// Panics if the TypeHash is not found - use for internal code where missing = bug.
    pub fn get_type_mut(&mut self, type_id: TypeHash) -> &mut TypeDef {
        self.types.get_mut(&type_id).expect("TypeHash not found in registry")
    }

    /// Try to get a mutable type definition by TypeHash, returns None if not found.
    pub fn try_get_type_mut(&mut self, type_id: TypeHash) -> Option<&mut TypeDef> {
        self.types.get_mut(&type_id)
    }

    /// Get a type definition by TypeHash.
    pub fn get_type_by_hash(&self, hash: TypeHash) -> Option<&TypeDef> {
        self.types_by_hash
            .get(&hash)
            .and_then(|id| self.types.get(id))
    }

    /// Get the behaviors for a type, if any are registered.
    pub fn get_behaviors(&self, type_id: TypeHash) -> Option<&super::behaviors::TypeBehaviors> {
        self.behaviors.get(&type_id)
    }

    /// Get mutable behaviors for a type, if any are registered.
    pub fn get_behaviors_mut(
        &mut self,
        type_id: TypeHash,
    ) -> Option<&mut super::behaviors::TypeBehaviors> {
        self.behaviors.get_mut(&type_id)
    }

    /// Set behaviors for a type. Overwrites any existing behaviors.
    pub fn set_behaviors(&mut self, type_id: TypeHash, behaviors: super::behaviors::TypeBehaviors) {
        self.behaviors.insert(type_id, behaviors);
    }

    /// Get or create behaviors for a type (for incremental registration).
    pub fn behaviors_mut(&mut self, type_id: TypeHash) -> &mut super::behaviors::TypeBehaviors {
        self.behaviors.entry(type_id).or_default()
    }

    /// Look up an enum value by enum type ID and value name
    /// Returns the numeric value if found, None otherwise
    pub fn lookup_enum_value(&self, type_id: TypeHash, value_name: &str) -> Option<i64> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Enum { values, .. } = typedef {
            values
                .iter()
                .find(|(name, _)| name == value_name)
                .map(|(_, val)| *val)
        } else {
            None
        }
    }

    /// Stub: Template instantiation has moved to CompilationContext.
    ///
    /// TODO(Phase 6.4): This stub exists only to allow tests to compile during transition.
    /// Once CompilationContext is implemented, tests should be updated to use it instead,
    /// and this method should be removed.
    #[deprecated(note = "Use CompilationContext::instantiate_template instead - Phase 6.4")]
    pub fn instantiate_template(
        &mut self,
        _template_id: TypeHash,
        _args: Vec<DataType>,
    ) -> Result<TypeHash, crate::semantic::error::SemanticError> {
        Err(crate::semantic::error::SemanticError::new(
            crate::semantic::error::SemanticErrorKind::NotATemplate,
            angelscript_parser::lexer::Span::default(),
            "instantiate_template() has moved to CompilationContext (Phase 6.4)".to_string(),
        ))
    }

    /// Register a function and return its TypeHash (func_hash)
    pub fn register_function(&mut self, def: FunctionDef<'ast>) -> TypeHash {
        let func_hash = def.func_hash;
        let qualified_name = def.qualified_name();

        self.functions.insert(func_hash, def);

        // Add to overload map
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_hash);

        func_hash
    }

    /// Look up all functions with the given name (for overload resolution)
    pub fn lookup_functions(&self, name: &str) -> &[TypeHash] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a function definition by TypeHash
    pub fn get_function(&self, func_id: TypeHash) -> &FunctionDef<'ast> {
        self.functions
            .get(&func_id)
            .expect("TypeHash not found in registry")
    }

    /// Get a mutable function definition by TypeHash
    pub fn get_function_mut(&mut self, func_id: TypeHash) -> &mut FunctionDef<'ast> {
        self.functions
            .get_mut(&func_id)
            .expect("TypeHash not found in registry")
    }

    /// Get the count of registered functions
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get a function definition by func_hash (same as get_function since func_hash is the key)
    pub fn get_function_by_hash(&self, hash: TypeHash) -> Option<&FunctionDef<'ast>> {
        self.functions.get(&hash)
    }

    /// Get the count of registered types
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get all methods for a given type
    pub fn get_methods(&self, type_id: TypeHash) -> Vec<TypeHash> {
        self.functions
            .values()
            .filter(|f| f.object_type == Some(type_id))
            .map(|f| f.func_hash)
            .collect()
    }

    /// Register a global variable
    pub fn register_global_var(
        &mut self,
        name: String,
        namespace: Vec<String>,
        data_type: DataType,
    ) {
        let qualified_name = if namespace.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", namespace.join("::"), name)
        };

        self.global_vars.insert(
            qualified_name,
            GlobalVarDef {
                name,
                namespace,
                data_type,
            },
        );
    }

    /// Look up a global variable by qualified name
    pub fn lookup_global_var(&self, name: &str) -> Option<&GlobalVarDef> {
        self.global_vars.get(name)
    }

    /// Register a mixin class
    pub fn register_mixin(&mut self, mixin: MixinDef<'ast>) {
        let qualified_name = mixin.qualified_name.clone();
        self.mixins.insert(qualified_name, mixin);
    }

    /// Look up a mixin by qualified name
    pub fn lookup_mixin(&self, name: &str) -> Option<&MixinDef<'ast>> {
        self.mixins.get(name)
    }

    /// Check if a name refers to a mixin (not a type)
    pub fn is_mixin(&self, name: &str) -> bool {
        self.mixins.contains_key(name)
    }

    /// Update a class with complete details (fields, methods, inheritance)
    pub fn update_class_details(
        &mut self,
        type_id: TypeHash,
        fields: Vec<super::type_def::FieldDef>,
        methods: Vec<TypeHash>,
        base_class: Option<TypeHash>,
        interfaces: Vec<TypeHash>,
        operator_methods: FxHashMap<super::type_def::OperatorBehavior, Vec<TypeHash>>,
        properties: FxHashMap<String, super::type_def::PropertyAccessors>,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Class {
            fields: class_fields,
            methods: class_methods,
            base_class: class_base,
            interfaces: class_interfaces,
            operator_methods: class_operator_methods,
            properties: class_properties,
            ..
        } = typedef
        {
            *class_fields = fields;
            *class_methods = methods;
            *class_base = base_class;
            *class_interfaces = interfaces;
            *class_operator_methods = operator_methods;
            *class_properties = properties;
        }
    }

    /// Update an interface with complete method signatures
    pub fn update_interface_details(
        &mut self,
        type_id: TypeHash,
        methods: Vec<super::type_def::MethodSignature>,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Interface {
            methods: interface_methods,
            ..
        } = typedef
        {
            *interface_methods = methods;
        }
    }

    /// Update a funcdef with complete signature
    pub fn update_funcdef_signature(
        &mut self,
        type_id: TypeHash,
        params: Vec<DataType>,
        return_type: DataType,
    ) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Funcdef {
            params: funcdef_params,
            return_type: funcdef_return,
            ..
        } = typedef
        {
            *funcdef_params = params;
            *funcdef_return = return_type;
        }
    }

    /// Get the signature of a funcdef type
    /// Returns (params, return_type) or None if not a funcdef
    pub fn get_funcdef_signature(&self, type_id: TypeHash) -> Option<(&[DataType], &DataType)> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Funcdef {
            params,
            return_type,
            ..
        } = typedef
        {
            Some((params.as_slice(), return_type))
        } else {
            None
        }
    }

    /// Check if a function is compatible with a funcdef type
    ///
    /// A function is compatible if:
    /// - Return types match (or are implicitly convertible)
    /// - Parameter count matches
    /// - Parameter types match (or are implicitly convertible)
    /// - Reference modifiers match exactly
    pub fn is_function_compatible_with_funcdef(
        &self,
        func_id: TypeHash,
        funcdef_type_id: TypeHash,
    ) -> bool {
        let (funcdef_params, funcdef_return) = match self.get_funcdef_signature(funcdef_type_id) {
            Some(sig) => sig,
            None => return false,
        };

        let func = self.get_function(func_id);

        // Check return type matches
        if func.return_type.type_hash != funcdef_return.type_hash {
            return false;
        }

        // Check parameter count matches
        if func.params.len() != funcdef_params.len() {
            return false;
        }

        // Check each parameter type matches
        for (func_param, funcdef_param) in func.params.iter().zip(funcdef_params.iter()) {
            // Base type must match
            if func_param.data_type.type_hash != funcdef_param.type_hash {
                return false;
            }
            // Reference modifier must match
            if func_param.data_type.ref_modifier != funcdef_param.ref_modifier {
                return false;
            }
            // Handle modifier must match
            if func_param.data_type.is_handle != funcdef_param.is_handle {
                return false;
            }
        }

        true
    }

    /// Find a function by name that is compatible with a funcdef type
    /// Returns the TypeHash if found and compatible, None otherwise
    pub fn find_compatible_function(
        &self,
        name: &str,
        funcdef_type_id: TypeHash,
    ) -> Option<TypeHash> {
        let func_ids = self.func_by_name.get(name)?;

        // Find the first function that matches the funcdef signature
        func_ids
            .iter()
            .find(|&&func_id| self.is_function_compatible_with_funcdef(func_id, funcdef_type_id))
            .copied()
    }

    /// Update a function's signature
    ///
    /// Only updates the first function with this name that still has empty params.
    /// This handles overloaded functions correctly - each call from type_compilation
    /// fills in the next overload.
    pub fn update_function_signature(
        &mut self,
        qualified_name: &str,
        params: Vec<ScriptParam<'ast>>,
        return_type: DataType,
        object_type: Option<TypeHash>,
        traits: FunctionTraits,
    ) {
        // Find the function(s) with this name
        if let Some(func_ids) = self.func_by_name.get(qualified_name).cloned() {
            // Find the first function that hasn't been filled in yet
            // Match by object_type to ensure we update the right method
            for func_id in func_ids {
                if let Some(func) = self.functions.get(&func_id) {
                    // Match on object_type to ensure we update the right method
                    // For methods, object_type must match; for free functions, both should be None
                    let object_type_matches = func.object_type == object_type;
                    // A function hasn't been filled in yet if signature_filled is false.
                    // Pass 1 registers all functions with signature_filled: false,
                    // and Pass 2a sets it to true when filling the signature.
                    if object_type_matches && !func.signature_filled {
                        if let Some(func_mut) = self.functions.get_mut(&func_id) {
                            func_mut.params = params;
                            func_mut.return_type = return_type;
                            func_mut.traits = traits;
                            func_mut.signature_filled = true;
                        }
                        return; // Only update one function
                    }
                }
            }
        }
    }

    /// Update a function's parameters directly by TypeHash
    /// Used to fill in params for auto-generated constructors
    pub fn update_function_params(&mut self, func_id: TypeHash, params: Vec<ScriptParam<'ast>>) {
        if let Some(func) = self.functions.get_mut(&func_id) {
            func.params = params;
        }
    }

    /// Update a function's return type directly by TypeHash
    /// Used to fill in return type for auto-generated operators like opAssign
    pub fn update_function_return_type(&mut self, func_id: TypeHash, return_type: DataType) {
        if let Some(func) = self.functions.get_mut(&func_id) {
            func.return_type = return_type;
        }
    }

    /// Find a constructor for a given type with specific argument types.
    /// Returns the TypeHash of the best matching constructor, if any.
    ///
    /// Returns None if the type doesn't exist in this registry (allowing
    /// CompilationContext to try other registries via or_else).
    pub fn find_constructor(&self, type_id: TypeHash, arg_types: &[DataType]) -> Option<TypeHash> {
        // Get the type definition - return None if not in this registry
        let typedef = self.try_get_type(type_id)?;

        // Only classes have constructors - get the methods list
        let method_ids = match typedef {
            TypeDef::Class { methods, .. } => methods,
            _ => return None,
        };

        // Filter to only constructors and find exact match
        for &method_id in method_ids {
            let func = self.get_function(method_id);

            // Only consider constructors
            if !func.traits.is_constructor {
                continue;
            }

            // Check if parameter count and types match exactly
            if func.params.len() == arg_types.len() {
                let all_match = func
                    .params
                    .iter()
                    .zip(arg_types.iter())
                    .all(|(param, arg_type)| &param.data_type == arg_type);

                if all_match {
                    return Some(method_id);
                }
            }
        }

        None
    }

    /// Find all constructors for a given type (value types)
    /// Returns a vector of FunctionIds for all constructors
    pub fn find_constructors(&self, type_id: TypeHash) -> Vec<TypeHash> {
        // Look up constructors from behaviors registry
        self.behaviors
            .get(&type_id)
            .map(|b| b.constructors.clone())
            .unwrap_or_default()
    }

    /// Find all factories for a given type (reference types)
    /// Returns a vector of FunctionIds for all factories
    pub fn find_factories(&self, type_id: TypeHash) -> Vec<TypeHash> {
        // Look up factories from behaviors registry
        self.behaviors
            .get(&type_id)
            .map(|b| b.factories.clone())
            .unwrap_or_default()
    }

    /// Check if a constructor is marked as explicit
    /// Explicit constructors cannot be used for implicit conversions
    pub fn is_constructor_explicit(&self, func_id: TypeHash) -> bool {
        let func = self.get_function(func_id);
        func.traits.is_explicit
    }

    /// Find the copy constructor for a given type
    /// Copy constructor has signature: ClassName(const ClassName&in) or ClassName(const ClassName&inout)
    /// Returns None if no copy constructor exists or if it was deleted
    pub fn find_copy_constructor(&self, type_id: TypeHash) -> Option<TypeHash> {
        let constructors = self.find_constructors(type_id);

        // Look for copy constructor signature: single parameter of same type with &in or &inout
        for &ctor_id in &constructors {
            let func = self.get_function(ctor_id);

            // Copy constructor must have exactly one parameter
            if func.params.len() != 1 {
                continue;
            }

            let param = &func.params[0];

            // Parameter must be a reference (&in or &inout)
            if !matches!(
                param.data_type.ref_modifier,
                crate::semantic::RefModifier::In | crate::semantic::RefModifier::InOut
            ) {
                continue;
            }

            // Parameter type must match the class type (ignoring const/ref modifiers)
            if param.data_type.type_hash == type_id {
                return Some(ctor_id);
            }
        }

        None
    }

    /// Add a method to a class's methods list
    /// This is used when auto-generating constructors in Pass 1
    /// Works for both regular classes and template instances (which are also Classes)
    pub fn add_method_to_class(&mut self, type_id: TypeHash, func_id: TypeHash) {
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Class { methods, .. } = typedef { methods.push(func_id) }
    }

    /// Find an operator method on a type.
    ///
    /// **IMPORTANT**: Operators are MEMBER METHODS ONLY (not global functions).
    /// This searches the type's operator_methods map.
    ///
    /// Returns None if:
    /// - The type is not a class
    /// - The operator is not implemented for this type
    ///
    /// # Example
    /// ```ignore
    /// // Find opAdd on Vector3: vec1 + vec2 → vec1.opAdd(vec2)
    /// if let Some(func_id) = registry.find_operator_method(vec3_type, OperatorBehavior::OpAdd) {
    ///     // Call the operator method
    /// }
    /// ```
    pub fn find_operator_method(
        &self,
        type_id: TypeHash,
        operator: OperatorBehavior,
    ) -> Option<TypeHash> {
        self.find_operator_methods(type_id, operator).first().copied()
    }

    /// Find all overloads of an operator method for a type.
    ///
    /// Returns all registered operator methods for the given behavior.
    /// Use this when you need to do overload resolution based on const-ness
    /// or parameter types.
    pub fn find_operator_methods(
        &self,
        type_id: TypeHash,
        operator: OperatorBehavior,
    ) -> &[TypeHash] {
        let Some(typedef) = self.try_get_type(type_id) else {
            return &[];
        };
        match typedef {
            TypeDef::Class {
                operator_methods, ..
            } => operator_methods.get(&operator).map(|v| v.as_slice()).unwrap_or(&[]),
            _ => &[],
        }
    }

    /// Find the best operator method for a type based on desired mutability.
    ///
    /// When `prefer_mutable` is true, prefers non-const methods (for assignment targets).
    /// When `prefer_mutable` is false, prefers const methods (for read-only access).
    /// Falls back to any available method if the preferred type isn't available.
    pub fn find_operator_method_with_mutability(
        &self,
        type_id: TypeHash,
        operator: OperatorBehavior,
        prefer_mutable: bool,
    ) -> Option<TypeHash> {
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
            let func = self.get_function(func_id);
            if func.return_type.is_const {
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

    /// Get the base class of a type (if any)
    pub fn get_base_class(&self, type_id: TypeHash) -> Option<TypeHash> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { base_class, .. } = typedef {
            *base_class
        } else {
            None
        }
    }

    /// Check if a class is marked as 'final' (cannot be inherited from)
    pub fn is_class_final(&self, type_id: TypeHash) -> bool {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { is_final, .. } = typedef {
            *is_final
        } else {
            false
        }
    }

    /// Check if `derived_class` is a subclass of `base_class` (directly or indirectly).
    ///
    /// Returns true if `derived_class` inherits from `base_class` at any level
    /// in the inheritance hierarchy, or if they are the same class.
    pub fn is_subclass_of(&self, derived_class: TypeHash, base_class: TypeHash) -> bool {
        // Same class counts as "is subclass of"
        if derived_class == base_class {
            return true;
        }

        // Walk up the inheritance chain
        let mut current = self.get_base_class(derived_class);
        while let Some(parent_id) = current {
            if parent_id == base_class {
                return true;
            }
            current = self.get_base_class(parent_id);
        }

        false
    }

    /// Get the fields of a class (does not include inherited fields)
    pub fn get_class_fields(&self, type_id: TypeHash) -> &[super::type_def::FieldDef] {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { fields, .. } = typedef {
            fields
        } else {
            &[]
        }
    }

    /// Find a method directly on this class (not in base classes)
    fn find_direct_method(&self, type_id: TypeHash, name: &str) -> Option<TypeHash> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { methods, .. } = typedef {
            methods
                .iter()
                .copied()
                .find(|&id| self.get_function(id).name == name)
        } else {
            None
        }
    }

    /// Find method in class or base classes using virtual dispatch (most derived wins)
    ///
    /// This walks the inheritance chain starting from the most derived class,
    /// returning the first method found with the given name. This implements
    /// proper virtual method dispatch where derived methods override base methods.
    ///
    /// For overloaded methods, this returns the first match only.
    /// Use `get_all_methods()` if you need to see all overloads.
    ///
    /// # Example
    /// ```ignore
    /// class Base { void foo() {} }
    /// class Derived : Base { void foo() override {} }
    ///
    /// registry.find_method(derived_id, "foo")  // Returns Derived::foo
    /// ```
    pub fn find_method(&self, type_id: TypeHash, name: &str) -> Option<TypeHash> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.find_method_impl(type_id, name, &mut visited)
    }

    fn find_method_impl(
        &self,
        type_id: TypeHash,
        name: &str,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> Option<TypeHash> {
        // Cycle protection
        if self.has_visited_in_chain(type_id, visited) {
            return None;
        }

        // Check this class first (most derived)
        if let Some(method) = self.find_direct_method(type_id, name) {
            return Some(method);
        }

        // Walk base class chain
        if let Some(base_id) = self.get_base_class(type_id) {
            return self.find_method_impl(base_id, name, visited);
        }

        None
    }

    /// Find all methods with the given name in a class and its base classes.
    ///
    /// This is used for overload resolution - returns all method overloads
    /// so the caller can select the best match based on argument types.
    ///
    /// Returns methods in order: derived class methods first, then base class methods.
    pub fn find_methods_by_name(&self, type_id: TypeHash, name: &str) -> Vec<TypeHash> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.find_methods_by_name_impl(type_id, name, &mut visited)
    }

    fn find_methods_by_name_impl(
        &self,
        type_id: TypeHash,
        name: &str,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> Vec<TypeHash> {
        // Cycle protection
        if self.has_visited_in_chain(type_id, visited) {
            return Vec::new();
        }

        let typedef = self.get_type(type_id);

        match typedef {
            TypeDef::Class {
                methods,
                base_class,
                ..
            } => {
                // Get all methods with matching name from this class (or template instance)
                let mut matching_methods: Vec<TypeHash> = methods
                    .iter()
                    .copied()
                    .filter(|&id| {
                        self.get_function(id).name == name
                    })
                    .collect();

                // Recursively add matching methods from base class
                if let Some(base_id) = base_class {
                    let base_methods = self.find_methods_by_name_impl(*base_id, name, visited);
                    matching_methods.extend(base_methods);
                }

                matching_methods
            }
            _ => Vec::new(),
        }
    }

    /// Get all methods for a class, including inherited methods from base class
    ///
    /// Returns methods in order: derived class methods first, then base class methods.
    /// This is useful for analysis, debugging, and IDE features.
    ///
    /// For actual method dispatch, use `find_method()` which implements proper
    /// virtual dispatch semantics.
    pub fn get_all_methods(&self, type_id: TypeHash) -> Vec<TypeHash> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_methods_impl(type_id, &mut visited)
    }

    fn get_all_methods_impl(
        &self,
        type_id: TypeHash,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> Vec<TypeHash> {
        // Cycle protection
        if self.has_visited_in_chain(type_id, visited) {
            return Vec::new();
        }

        let typedef = self.get_type(type_id);

        match typedef {
            TypeDef::Class {
                methods,
                base_class,
                ..
            } => {
                let mut all_methods = methods.clone();

                // Recursively add base class methods
                if let Some(base_id) = base_class {
                    let base_methods = self.get_all_methods_impl(*base_id, visited);
                    all_methods.extend(base_methods);
                }

                all_methods
            }
            _ => Vec::new(),
        }
    }

    /// Get all properties for a class, including inherited properties from base class
    ///
    /// Returns a map of all accessible properties. Derived class properties shadow base class
    /// properties with the same name.
    pub fn get_all_properties(&self, type_id: TypeHash) -> FxHashMap<String, PropertyAccessors> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_properties_impl(type_id, &mut visited)
    }

    fn get_all_properties_impl(
        &self,
        type_id: TypeHash,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> FxHashMap<String, PropertyAccessors> {
        // Cycle protection
        if self.has_visited_in_chain(type_id, visited) {
            return FxHashMap::default();
        }

        let typedef = self.get_type(type_id);

        match typedef {
            TypeDef::Class {
                properties,
                base_class,
                ..
            } => {
                let mut all_properties = FxHashMap::default();

                // First, add base class properties (if any)
                // Base properties are added first so derived properties can override them
                if let Some(base_id) = base_class {
                    let base_properties = self.get_all_properties_impl(*base_id, visited);
                    all_properties.extend(base_properties);
                }

                // Then add/override with this class's properties
                all_properties.extend(properties.clone());

                all_properties
            }
            _ => FxHashMap::default(),
        }
    }

    /// Look up a property by name in a class (including inherited properties)
    ///
    /// Returns None if the type is not a class or doesn't have the property
    pub fn find_property(&self, type_id: TypeHash, property_name: &str) -> Option<PropertyAccessors> {
        let all_properties = self.get_all_properties(type_id);
        all_properties.get(property_name).cloned()
    }

    /// Look up a method by name in a class (including inherited methods)
    ///
    /// Returns the first matching method using virtual dispatch (derived class methods take precedence).
    /// For overloaded methods, returns the first match only - use get_all_methods for full list.
    ///
    /// This is an alias for `find_method()` for backwards compatibility.
    pub fn find_method_by_name(&self, type_id: TypeHash, method_name: &str) -> Option<TypeHash> {
        self.find_method(type_id, method_name)
    }

    /// Get all method signatures for an interface type
    ///
    /// Returns the list of MethodSignature for an interface, or None if not an interface.
    /// Used for validating that classes implement all interface methods.
    pub fn get_interface_methods(
        &self,
        type_id: TypeHash,
    ) -> Option<&[super::type_def::MethodSignature]> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Interface { methods, .. } = typedef {
            Some(methods.as_slice())
        } else {
            None
        }
    }

    /// Get all interfaces implemented by a class (including inherited interfaces)
    ///
    /// Returns a list of interface TypeIds. Interfaces inherited from base classes are included.
    pub fn get_all_interfaces(&self, type_id: TypeHash) -> Vec<TypeHash> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_interfaces_impl(type_id, &mut visited)
    }

    fn get_all_interfaces_impl(
        &self,
        type_id: TypeHash,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> Vec<TypeHash> {
        // Cycle protection
        if self.has_visited_in_chain(type_id, visited) {
            return Vec::new();
        }

        let typedef = self.get_type(type_id);

        match typedef {
            TypeDef::Class {
                interfaces,
                base_class,
                ..
            } => {
                let mut all_interfaces = interfaces.clone();

                // Add interfaces from base class
                if let Some(base_id) = base_class {
                    let base_interfaces = self.get_all_interfaces_impl(*base_id, visited);
                    // Add only interfaces not already in the list
                    for iface_id in base_interfaces {
                        if !all_interfaces.contains(&iface_id) {
                            all_interfaces.push(iface_id);
                        }
                    }
                }

                all_interfaces
            }
            _ => Vec::new(),
        }
    }

    /// Find a method in the base class chain (not in the derived class itself)
    ///
    /// This is used to validate the `override` keyword - checks if there's a method
    /// in the base class hierarchy that the derived method is overriding.
    ///
    /// Returns the TypeHash of the base method if found, None otherwise.
    pub fn find_base_method(&self, type_id: TypeHash, method_name: &str) -> Option<TypeHash> {
        // Get base class
        let base_id = self.get_base_class(type_id)?;

        // Search in base class and its ancestors
        self.find_method(base_id, method_name)
    }

    /// Find a method in the base class chain with matching signature
    ///
    /// This is used to validate the `override` keyword with signature matching.
    /// Checks parameter types and return type for compatibility.
    pub fn find_base_method_with_signature(
        &self,
        type_id: TypeHash,
        method_name: &str,
        params: &[DataType],
        return_type: &DataType,
    ) -> Option<TypeHash> {
        // Get base class
        let base_id = self.get_base_class(type_id)?;

        // Get all methods with this name in base class chain
        let base_methods = self.find_methods_by_name(base_id, method_name);

        // Find one with matching signature
        for &method_id in &base_methods {
            let func = self.get_function(method_id);

            // Check return type
            if func.return_type.type_hash != return_type.type_hash {
                continue;
            }

            // Check parameter count
            if func.params.len() != params.len() {
                continue;
            }

            // Check parameter types
            let params_match = func
                .params
                .iter()
                .zip(params.iter())
                .all(|(a, b)| a.data_type.type_hash == b.type_hash && a.data_type.ref_modifier == b.ref_modifier);

            if params_match {
                return Some(method_id);
            }
        }

        None
    }

    /// Check if a class has a method matching an interface method signature
    ///
    /// Searches the class and its base classes for a method with matching
    /// name, parameter types, and return type.
    pub fn has_method_matching_interface(
        &self,
        class_type_id: TypeHash,
        interface_method: &super::type_def::MethodSignature,
    ) -> bool {
        // Get all methods with this name in the class hierarchy
        let methods = self.find_methods_by_name(class_type_id, &interface_method.name);

        for &method_id in &methods {
            let func = self.get_function(method_id);

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

        false
    }

    /// Check if a base class method is marked as final
    ///
    /// Used to validate that derived classes don't override final methods.
    pub fn is_base_method_final(&self, type_id: TypeHash, method_name: &str) -> Option<TypeHash> {
        // Find the method in base class chain
        let base_method_id = self.find_base_method(type_id, method_name)?;
        let base_func = self.get_function(base_method_id);

        if base_func.traits.is_final {
            Some(base_method_id)
        } else {
            None
        }
    }

    /// Detect if setting `base_id` as the base class of `type_id` would create a circular inheritance chain.
    ///
    /// Returns true if a cycle would be created. This checks if `type_id` appears anywhere
    /// in the inheritance chain of `base_id`.
    ///
    /// # Example
    /// ```ignore
    /// // Direct cycle: class A : A
    /// registry.would_create_circular_inheritance(type_a, type_a) // true
    ///
    /// // Indirect cycle: class A : B, class B : A
    /// // When processing class A and B already exists with base class A:
    /// registry.would_create_circular_inheritance(type_a, type_b) // true
    /// ```
    pub fn would_create_circular_inheritance(
        &self,
        type_id: TypeHash,
        proposed_base_id: TypeHash,
    ) -> bool {
        // Direct self-inheritance
        if type_id == proposed_base_id {
            return true;
        }

        // Check if type_id appears anywhere in the inheritance chain of proposed_base_id
        let mut visited = rustc_hash::FxHashSet::default();
        let mut current = Some(proposed_base_id);

        while let Some(curr_id) = current {
            if visited.contains(&curr_id) {
                // We hit a cycle in the existing chain (shouldn't happen, but be safe)
                return true;
            }
            visited.insert(curr_id);

            // Check if we reached the type we're trying to set as derived
            if curr_id == type_id {
                return true;
            }

            // Move to next base class
            current = self.get_base_class(curr_id);
        }

        false
    }

    /// Check if a type has circular inheritance (for defensive checks).
    ///
    /// This is used by recursive methods to protect against infinite loops
    /// if circular inheritance somehow exists in the registry.
    fn has_visited_in_chain(
        &self,
        type_id: TypeHash,
        visited: &mut rustc_hash::FxHashSet<TypeHash>,
    ) -> bool {
        if visited.contains(&type_id) {
            return true;
        }
        visited.insert(type_id);
        false
    }

}

impl<'ast> Default for ScriptRegistry<'ast> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::data_type::RefModifier;
    use crate::semantic::types::type_def::Visibility;
    use angelscript_core::{primitives, TypeHash};

    /// Test helper to create a ScriptParam from a DataType with an auto-generated name
    fn param(data_type: DataType) -> ScriptParam<'static> {
        ScriptParam {
            name: String::new(),
            data_type,
            default: None,
        }
    }

    /// Test helper to create a placeholder func_hash for test FunctionDefs
    fn test_func_hash() -> TypeHash {
        TypeHash::EMPTY
    }

    #[test]
    fn script_registry_new_is_empty() {
        let registry = ScriptRegistry::new();
        // ScriptRegistry starts empty - primitives are in FfiRegistry
        assert_eq!(registry.types.len(), 0);
    }

    #[test]
    fn lookup_nonexistent_type() {
        let registry = ScriptRegistry::new();
        assert_eq!(registry.lookup_type("NonExistent"), None);
    }

    #[test]
    fn register_simple_class() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };

        let type_id = registry.register_type(typedef, Some("Player"));
        assert_eq!(registry.lookup_type("Player"), Some(type_id));
        assert!(registry.get_type(type_id).is_class());
    }

    #[test]
    fn register_qualified_class() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Game::Player"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };

        let type_id = registry.register_type(typedef, Some("Game::Player"));
        assert_eq!(registry.lookup_type("Game::Player"), Some(type_id));
    }

    #[test]
    fn register_interface() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Interface {
            name: "IDrawable".to_string(),
            qualified_name: "IDrawable".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("IDrawable"),
            methods: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("IDrawable"));
        assert_eq!(registry.lookup_type("IDrawable"), Some(type_id));
        assert!(registry.get_type(type_id).is_interface());
    }

    #[test]
    fn register_enum() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Color"),
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        };

        let type_id = registry.register_type(typedef, Some("Color"));
        assert_eq!(registry.lookup_type("Color"), Some(type_id));
        assert!(registry.get_type(type_id).is_enum());
    }

    #[test]
    fn register_funcdef() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Funcdef {
            name: "Callback".to_string(),
            qualified_name: "Callback".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Callback"),
            params: vec![DataType::simple(primitives::INT32)],
            return_type: DataType::simple(primitives::VOID),
        };

        let type_id = registry.register_type(typedef, Some("Callback"));
        assert_eq!(registry.lookup_type("Callback"), Some(type_id));
        assert!(registry.get_type(type_id).is_funcdef());
    }

    #[test]
    fn get_type_mut() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };

        let type_id = registry.register_type(typedef, Some("Player"));

        // Modify the type
        if let TypeDef::Class { fields, .. } = registry.get_type_mut(type_id) {
            fields.push(super::super::type_def::FieldDef::new(
                "health".to_string(),
                DataType::simple(primitives::INT32),
                Visibility::Public,
            ));
        }

        // Verify modification
        if let TypeDef::Class { fields, .. } = registry.get_type(type_id) {
            assert_eq!(fields.len(), 1);
        } else {
            panic!("Expected Class");
        }
    }


    #[test]
    fn find_operator_method_with_mutability_prefers_non_const() {
        let mut registry = ScriptRegistry::new();

        // Create two opIndex methods - one const, one non-const
        let class_id = TypeHash::from_name("TestClass");
        let const_method_hash = TypeHash::from_method(class_id, "opIndex", &[primitives::INT32], true, true);
        let const_method = FunctionDef {
            func_hash: const_method_hash,
            name: "opIndex".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType {
                type_hash: primitives::INT32,
                is_const: true, // const return
                is_handle: false,
                is_handle_to_const: false,
                ref_modifier: RefModifier::In,
            },
            object_type: None,
            traits: FunctionTraits { is_const: true, ..FunctionTraits::new() },
            is_native: true,
            visibility: Visibility::Public,
            signature_filled: true,
        };
        registry.register_function(const_method);

        let mutable_method_hash = TypeHash::from_method(class_id, "opIndex", &[primitives::INT32], false, false);
        let mutable_method = FunctionDef {
            func_hash: mutable_method_hash,
            name: "opIndex".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType {
                type_hash: primitives::INT32,
                is_const: false, // non-const return
                is_handle: false,
                is_handle_to_const: false,
                ref_modifier: RefModifier::In,
            },
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: true,
            visibility: Visibility::Public,
            signature_filled: true,
        };
        registry.register_function(mutable_method);

        // Create a class with both opIndex methods
        let mut operator_methods: FxHashMap<OperatorBehavior, Vec<TypeHash>> = FxHashMap::default();
        operator_methods.insert(
            OperatorBehavior::OpIndex,
            vec![const_method_hash, mutable_method_hash],
        );

        registry.types.insert(class_id, TypeDef::Class {
            name: "TestClass".to_string(),
            qualified_name: "TestClass".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("TestClass"),
            fields: Vec::new(),
            methods: vec![const_method_hash, mutable_method_hash],
            base_class: None,
            interfaces: Vec::new(),
            operator_methods,
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            type_kind: angelscript_core::TypeKind::reference(),
        });
        registry.type_by_name.insert("TestClass".to_string(), class_id);

        // When prefer_mutable=true, should return the non-const method
        let result = registry.find_operator_method_with_mutability(
            class_id,
            OperatorBehavior::OpIndex,
            true,
        );
        assert_eq!(result, Some(mutable_method_hash), "Should prefer mutable method for assignment");

        // When prefer_mutable=false, should return the const method
        let result = registry.find_operator_method_with_mutability(
            class_id,
            OperatorBehavior::OpIndex,
            false,
        );
        assert_eq!(result, Some(const_method_hash), "Should prefer const method for read");
    }

    #[test]
    fn register_function() {
        let mut registry = ScriptRegistry::new();

        let func = FunctionDef {
            func_hash: test_func_hash(),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let func_id = registry.register_function(func);
        assert_eq!(func_id, test_func_hash());
    }

    #[test]
    fn lookup_function_by_name() {
        let mut registry = ScriptRegistry::new();

        let func = FunctionDef {
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        registry.register_function(func);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0], TypeHash(0));
    }

    #[test]
    fn lookup_nonexistent_function() {
        let registry = ScriptRegistry::new();
        let functions = registry.lookup_functions("nonexistent");
        assert_eq!(functions.len(), 0);
    }

    #[test]
    fn function_overloading() {
        let mut registry = ScriptRegistry::new();

        // foo(int)
        let func1 = FunctionDef {
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        // foo(float)
        let func2 = FunctionDef {
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::FLOAT))],
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        registry.register_function(func1);
        registry.register_function(func2);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn qualified_function_name() {
        let func = FunctionDef {
            name: "update".to_string(),
            namespace: vec!["Game".to_string(), "Player".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        assert_eq!(func.qualified_name(), "Game::Player::update");
    }

    #[test]
    fn unqualified_function_name() {
        let func = FunctionDef {
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        assert_eq!(func.qualified_name(), "foo");
    }

    #[test]
    fn get_function() {
        let mut registry = ScriptRegistry::new();

        let func = FunctionDef {
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![param(DataType::simple(primitives::INT32))],
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let func_id = registry.register_function(func.clone());
        let retrieved = registry.get_function(func_id);
        assert_eq!(retrieved.name, "foo");
    }

    #[test]
    fn get_methods_for_type() {
        let mut registry = ScriptRegistry::new();

        let player_type = TypeHash(100);

        // Method for Player
        let method1_hash = TypeHash::from_method(player_type, "update", &[], false, false);
        let method1 = FunctionDef {
            name: "update".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: method1_hash,
        };

        // Another method for Player
        let method2_hash = TypeHash::from_method(player_type, "draw", &[], false, false);
        let method2 = FunctionDef {
            name: "draw".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: method2_hash,
        };

        // Global function (not a method)
        let global_func_hash = TypeHash::from_function("main", &[]);
        let global_func = FunctionDef {
            name: "main".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: global_func_hash,
        };

        registry.register_function(method1);
        registry.register_function(method2);
        registry.register_function(global_func);

        let methods = registry.get_methods(player_type);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&method1_hash));
        assert!(methods.contains(&method2_hash));
        assert!(!methods.contains(&global_func_hash));
    }

    #[test]
    fn registry_default() {
        let registry = ScriptRegistry::default();
        // ScriptRegistry no longer contains primitives (they're in FfiRegistry)
        // ScriptRegistry::default() starts empty
        assert_eq!(registry.types.len(), 0);
    }

    #[test]
    fn registry_clone() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };

        let type_id = registry.register_type(typedef, Some("Player"));

        let cloned = registry.clone();
        assert_eq!(cloned.lookup_type("Player"), Some(type_id));
    }

    // ============================================================================
    // Constructor Lookup Tests (Task 7)
    // ============================================================================

    #[test]
    fn find_constructor_exact_match() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Vector3"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register a constructor: Vector3(int, int, int)
        let int_type = DataType::simple(primitives::INT32);
        let func_def = FunctionDef {
            func_hash: test_func_hash(),
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![param(int_type.clone()), param(int_type.clone()), param(int_type.clone())],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let func_id = registry.register_function(func_def);

        // Add the constructor to the class's methods list
        registry.add_method_to_class(type_id, func_id);

        // Find constructor with matching args
        let found = registry.find_constructor(
            type_id,
            &[int_type.clone(), int_type.clone(), int_type.clone()],
        );

        assert!(found.is_some());
        assert_eq!(found.unwrap(), func_id);
    }

    #[test]
    fn find_constructor_no_match() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Vector3"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register constructor: Vector3(int, int, int)
        let int_type = DataType::simple(primitives::INT32);
        let func_def = FunctionDef {
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![param(int_type.clone()), param(int_type.clone()), param(int_type.clone())],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        registry.register_function(func_def);

        // Try to find constructor with different args (float, float, float)
        let float_type = DataType::simple(primitives::FLOAT);
        let found = registry.find_constructor(
            type_id,
            &[float_type.clone(), float_type.clone(), float_type.clone()],
        );

        assert!(found.is_none());
    }

    #[test]
    fn find_constructor_type_not_in_registry() {
        // Test that find_constructor returns None (doesn't panic) when the type
        // doesn't exist in this registry. This is important for the or_else chain
        // in CompilationContext::find_constructor.
        let registry = ScriptRegistry::new();

        // Try to find constructor for a type that was never registered
        let nonexistent_type = angelscript_core::TypeHash::from_name("NonExistent");
        let int_type = DataType::simple(primitives::INT32);

        let found = registry.find_constructor(nonexistent_type, &[int_type]);

        // Should return None, not panic
        assert!(found.is_none());
    }

    #[test]
    fn is_constructor_explicit() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Vector3"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register explicit constructor: Vector3(int) explicit
        let int_type = DataType::simple(primitives::INT32);
        let func_def = FunctionDef {
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![param(int_type.clone())],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: true, // Explicit!
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let func_id = registry.register_function(func_def);

        // Check if constructor is explicit
        assert!(registry.is_constructor_explicit(func_id));
    }

    #[test]
    fn find_all_constructors() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Vector3"),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        let int_type = DataType::simple(primitives::INT32);

        // Register default constructor
        let func_def1 = FunctionDef {
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        // Register single-param constructor
        let func_def2 = FunctionDef {
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![param(int_type.clone())],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let func_id1 = registry.register_function(func_def1);
        let func_id2 = registry.register_function(func_def2);

        // Add the constructors to the class's methods list
        registry.add_method_to_class(type_id, func_id1);
        registry.add_method_to_class(type_id, func_id2);

        // Add constructors to behaviors
        registry.behaviors_mut(type_id).add_constructor(func_id1);
        registry.behaviors_mut(type_id).add_constructor(func_id2);

        // Find all constructors
        let constructors = registry.find_constructors(type_id);

        assert_eq!(constructors.len(), 2);
        assert!(constructors.contains(&func_id1));
        assert!(constructors.contains(&func_id2));
    }

    #[test]
    fn find_copy_constructor_with_in_ref() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create copy constructor with &in: Player(const Player&in)
        let copy_ctor_param = DataType::with_ref_in(type_id);
        let copy_ctor = FunctionDef {
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param(copy_ctor_param)],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: Some(
                    crate::semantic::types::type_def::AutoGeneratedMethod::CopyConstructor,
                ),
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let copy_ctor_id = registry.register_function(copy_ctor);
        registry.add_method_to_class(type_id, copy_ctor_id);
        registry
            .behaviors_mut(type_id)
            .add_constructor(copy_ctor_id);

        // Should find the copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, Some(copy_ctor_id));
    }

    #[test]
    fn find_copy_constructor_with_inout_ref() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create copy constructor with &inout: Player(const Player&inout)
        let copy_ctor_param = DataType::with_ref_inout(type_id);
        let copy_ctor = FunctionDef {
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param(copy_ctor_param)],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: Some(
                    crate::semantic::types::type_def::AutoGeneratedMethod::CopyConstructor,
                ),
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let copy_ctor_id = registry.register_function(copy_ctor);
        registry.add_method_to_class(type_id, copy_ctor_id);
        registry
            .behaviors_mut(type_id)
            .add_constructor(copy_ctor_id);

        // Should find the copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, Some(copy_ctor_id));
    }

    #[test]
    fn find_copy_constructor_not_found_wrong_param_count() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with two parameters (not a copy constructor)
        let param1 = DataType::with_ref_in(type_id);
        let param2 = DataType::simple(primitives::INT32);
        let ctor = FunctionDef {
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param(param1), param(param2)],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_not_found_wrong_ref_modifier() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with &out (wrong for copy constructor)
        let param_type = DataType::with_ref_out(type_id);
        let ctor = FunctionDef {
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param(param_type)],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_not_found_wrong_type() {
        let mut registry = ScriptRegistry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with different type parameter (not same class)
        let param_type = DataType::with_ref_in(primitives::INT32);
        let ctor = FunctionDef {
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param(param_type)],
            return_type: DataType::simple(primitives::VOID),
            object_type: Some(type_id),
            traits: FunctionTraits {
                is_constructor: true,
                is_destructor: false,
                is_final: false,
                is_virtual: false,
                is_abstract: false,
                is_const: false,
                is_explicit: false,
                auto_generated: None,
            },
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_no_constructors() {
        let mut registry = ScriptRegistry::new();

        // Register a class with no constructors
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Player"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn get_all_methods_with_inheritance() {
        let mut registry = ScriptRegistry::new();

        // Create base class with a method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![TypeHash(100)], // base method
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class with a method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Derived"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![TypeHash(200)], // derived method
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Get all methods for derived class
        let all_methods = registry.get_all_methods(derived_id);

        // Should have both derived and base methods
        assert_eq!(all_methods.len(), 2);
        assert!(all_methods.contains(&TypeHash(200))); // derived
        assert!(all_methods.contains(&TypeHash(100))); // base
    }

    #[test]
    fn get_all_properties_with_inheritance() {
        let mut registry = ScriptRegistry::new();

        // Create base class with a property
        let mut base_props = rustc_hash::FxHashMap::default();
        base_props.insert(
            "health".to_string(),
            PropertyAccessors::read_only(TypeHash(100)),
        );

        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: base_props,
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class with a property
        let mut derived_props = rustc_hash::FxHashMap::default();
        derived_props.insert(
            "score".to_string(),
            PropertyAccessors::read_write(TypeHash(200), TypeHash(201)),
        );

        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Derived"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: derived_props,
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Get all properties for derived class
        let all_props = registry.get_all_properties(derived_id);

        // Should have both derived and base properties
        assert_eq!(all_props.len(), 2);
        assert!(all_props.contains_key("health")); // base
        assert!(all_props.contains_key("score")); // derived
    }

    #[test]
    fn find_method_walks_inheritance_chain() {
        let mut registry = ScriptRegistry::new();

        // Register the base method first (gets TypeHash(0))
        let base_method = FunctionDef {
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None, // Set later
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with the method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![base_method_id],
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class WITHOUT overriding the method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Derived"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(), // No methods in derived
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Should find base class method
        let found = registry.find_method(derived_id, "foo");
        assert_eq!(found, Some(base_method_id));
    }

    #[test]
    fn find_method_returns_most_derived() {
        let mut registry = ScriptRegistry::new();

        // Register the base method
        let base_method = FunctionDef {
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with the method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![base_method_id],
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Register the derived method (same name, overrides base)
        let derived_method = FunctionDef {
            name: "foo".to_string(),
            namespace: vec!["Derived".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };
        let derived_method_id = registry.register_function(derived_method);

        // Create derived class that OVERRIDES the method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Derived"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![derived_method_id], // Override
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Should find derived class method (most derived wins)
        let found = registry.find_method(derived_id, "foo");
        assert_eq!(found, Some(derived_method_id));

        // Base class should still find its own method
        let base_found = registry.find_method(base_id, "foo");
        assert_eq!(base_found, Some(base_method_id));
    }

    #[test]
    fn find_method_multi_level_inheritance() {
        let mut registry = ScriptRegistry::new();

        // Register the base method
        let base_method = FunctionDef {
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(primitives::VOID),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,

            visibility: Visibility::Public,
            signature_filled: true,
            func_hash: test_func_hash(),
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![base_method_id],
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create middle class (no override)
        let middle_typedef = TypeDef::Class {
            name: "Middle".to_string(),
            qualified_name: "Middle".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Middle"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let middle_id = registry.register_type(middle_typedef, Some("Middle"));

        // Create most derived class (no override)
        let most_derived_typedef = TypeDef::Class {
            name: "MostDerived".to_string(),
            qualified_name: "MostDerived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("MostDerived"),
            base_class: Some(middle_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let most_derived_id = registry.register_type(most_derived_typedef, Some("MostDerived"));

        // Should walk through Middle to Base and find method
        let found = registry.find_method(most_derived_id, "foo");
        assert_eq!(found, Some(base_method_id));
    }

    #[test]
    fn find_method_not_found_returns_none() {
        let mut registry = ScriptRegistry::new();

        let typedef = TypeDef::Class {
            name: "MyClass".to_string(),
            qualified_name: "MyClass".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("MyClass"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let type_id = registry.register_type(typedef, Some("MyClass"));

        // Should return None for non-existent method
        let found = registry.find_method(type_id, "nonexistent");
        assert_eq!(found, None);
    }

    #[test]
    fn find_property_in_base_class() {
        let mut registry = ScriptRegistry::new();

        // Create base class with a property
        let mut base_props = rustc_hash::FxHashMap::default();
        base_props.insert(
            "health".to_string(),
            PropertyAccessors::read_only(TypeHash(100)),
        );

        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Base"),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: base_props,
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class without that property
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            type_hash: angelscript_core::TypeHash::from_name("Derived"),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        type_kind: angelscript_core::TypeKind::reference(),
            };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Should find property from base class
        let found = registry.find_property(derived_id, "health");
        assert!(found.is_some());
        assert!(found.unwrap().is_read_only());
    }

}
