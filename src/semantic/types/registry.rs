//! Global registry for types, functions, and variables.
//!
//! The Registry is the central storage for all global declarations in an AngelScript
//! program. It stores types (primitives, classes, interfaces, enums), functions
//! (with overloading support), and global variables.
//!
//! # Architecture
//!
//! - **Types**: Stored in a Vec with TypeId as index, plus a name→TypeId map
//! - **Functions**: Stored in a Vec with FunctionId as index, plus a name→[FunctionId] map for overloading
//! - **Template Cache**: Memoizes template instantiations to avoid duplicates
//!
//! # Example
//!
//! ```
//! use angelscript::semantic::{Registry, TypeDef, PrimitiveType, INT32_TYPE};
//!
//! let registry = Registry::new();
//!
//! // Built-in types are pre-registered
//! let int_type = registry.get_type(INT32_TYPE);
//! assert!(int_type.is_primitive());
//! ```

use super::data_type::{DataType, RefModifier};
use super::type_def::{
    BOOL_TYPE, DOUBLE_TYPE, FIRST_USER_TYPE_ID, FLOAT_TYPE, FunctionId, FunctionTraits, INT8_TYPE,
    INT16_TYPE, INT32_TYPE, INT64_TYPE, MethodSignature, OperatorBehavior, PrimitiveType,
    PropertyAccessors, SELF_TYPE, TypeDef, TypeId, UINT8_TYPE, UINT16_TYPE, UINT32_TYPE,
    UINT64_TYPE, VOID_TYPE, Visibility,
};
use crate::ast::RefKind;
use crate::ast::expr::Expr;
use crate::ast::types::{
    ParamType, PrimitiveType as AstPrimitiveType, TypeBase, TypeExpr, TypeSuffix,
};
use crate::ffi::{
    NativeFuncdefDef, NativeFunctionDef, NativeInterfaceDef, NativeInterfaceMethod,
    NativeMethodDef, NativePropertyDef, NativeTypeDef,
};
use crate::lexer::Span;
use crate::module::Module;
use crate::module::NativeEnumDef;
use crate::semantic::error::{SemanticError, SemanticErrorKind};
use rustc_hash::FxHashMap;
use thiserror::Error;

/// Function definition with complete signature
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef<'ast> {
    /// Function identifier
    pub id: FunctionId,
    /// Function name (unqualified)
    pub name: String,
    /// Namespace path (e.g., ["Game", "Player"])
    pub namespace: Vec<String>,
    /// Parameter types
    pub params: Vec<DataType>,
    /// Return type
    pub return_type: DataType,
    /// Object type if this is a method
    pub object_type: Option<TypeId>,
    /// Function traits (virtual, const, etc.)
    pub traits: FunctionTraits,
    /// True if this is a native (FFI) function
    pub is_native: bool,
    /// Default argument expressions (one per parameter, None if no default)
    pub default_args: Vec<Option<&'ast Expr<'ast>>>,
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

/// Errors that can occur when importing FFI modules into the registry.
#[derive(Debug, Clone, Error)]
pub enum ImportError {
    /// A referenced type was not found in the registry
    #[error("type not found: {0}")]
    TypeNotFound(String),

    /// A type with this name already exists
    #[error("duplicate type: {0}")]
    DuplicateType(String),

    /// Failed to resolve a type expression
    #[error("type resolution failed for '{type_name}': {reason}")]
    TypeResolutionFailed {
        /// The type name that failed to resolve
        type_name: String,
        /// The reason for the failure
        reason: String,
    },

    /// Template instantiation failed
    #[error("template instantiation failed: {0}")]
    TemplateInstantiationFailed(String),
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
    pub members: &'ast [crate::ast::decl::ClassMember<'ast>],
}

impl<'ast> MixinDef<'ast> {
    /// Get the qualified name of this mixin
    pub fn qualified_name(&self) -> &str {
        &self.qualified_name
    }
}

/// Global registry for all types, functions, and variables
#[derive(Clone)]
pub struct Registry<'ast> {
    // Type storage
    types: Vec<TypeDef>,
    type_by_name: FxHashMap<String, TypeId>,

    // Type behaviors (lifecycle, initialization) - stored separately from TypeDef
    // This follows the C++ AngelScript pattern where behaviors are registered separately
    behaviors: FxHashMap<TypeId, super::behaviors::TypeBehaviors>,

    // Function storage
    functions: FxHashMap<FunctionId, FunctionDef<'ast>>,
    func_by_name: FxHashMap<String, Vec<FunctionId>>,

    // Global variable storage
    global_vars: FxHashMap<String, GlobalVarDef>,

    // Mixin storage (mixins are not types, stored separately)
    mixins: FxHashMap<String, MixinDef<'ast>>,

    // Template instantiation cache (Template TypeId + arg TypeIds → Instance TypeId)
    // Note: Uses TypeId not DataType so array<int> and array<const int> share the same instance
    template_cache: FxHashMap<(TypeId, Vec<TypeId>), TypeId>,

    // Template callbacks for validating template instantiation
    // Stored separately because these are native Rust closures, not script functions
    template_callbacks: FxHashMap<
        TypeId,
        std::sync::Arc<dyn Fn(&crate::ffi::TemplateInstanceInfo) -> crate::ffi::TemplateValidation + Send + Sync>,
    >,

    // Fixed TypeIds for quick access (primitives only - FFI types use name lookup)
    pub void_type: TypeId,
    pub bool_type: TypeId,
    pub int8_type: TypeId,
    pub int16_type: TypeId,
    pub int32_type: TypeId,
    pub int64_type: TypeId,
    pub uint8_type: TypeId,
    pub uint16_type: TypeId,
    pub uint32_type: TypeId,
    pub uint64_type: TypeId,
    pub float_type: TypeId,
    pub double_type: TypeId,
}

impl<'ast> std::fmt::Debug for Registry<'ast> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("types", &self.types)
            .field("type_by_name", &self.type_by_name)
            .field("behaviors", &self.behaviors)
            .field("functions", &self.functions)
            .field("func_by_name", &self.func_by_name)
            .field("global_vars", &self.global_vars)
            .field("mixins", &self.mixins)
            .field("template_cache", &self.template_cache)
            .field("template_callbacks", &format!("<{} callbacks>", self.template_callbacks.len()))
            .field("void_type", &self.void_type)
            .field("bool_type", &self.bool_type)
            .field("int8_type", &self.int8_type)
            .field("int16_type", &self.int16_type)
            .field("int32_type", &self.int32_type)
            .field("int64_type", &self.int64_type)
            .field("uint8_type", &self.uint8_type)
            .field("uint16_type", &self.uint16_type)
            .field("uint32_type", &self.uint32_type)
            .field("uint64_type", &self.uint64_type)
            .field("float_type", &self.float_type)
            .field("double_type", &self.double_type)
            .finish()
    }
}

impl<'ast> Registry<'ast> {
    /// Create a new registry with all built-in types pre-registered
    pub fn new() -> Self {
        let mut registry = Self {
            types: Vec::with_capacity(32),
            type_by_name: FxHashMap::default(),
            behaviors: FxHashMap::default(),
            functions: FxHashMap::default(),
            func_by_name: FxHashMap::default(),
            global_vars: FxHashMap::default(),
            mixins: FxHashMap::default(),
            template_cache: FxHashMap::default(),
            template_callbacks: FxHashMap::default(),
            void_type: VOID_TYPE,
            bool_type: BOOL_TYPE,
            int8_type: INT8_TYPE,
            int16_type: INT16_TYPE,
            int32_type: INT32_TYPE,
            int64_type: INT64_TYPE,
            uint8_type: UINT8_TYPE,
            uint16_type: UINT16_TYPE,
            uint32_type: UINT32_TYPE,
            uint64_type: UINT64_TYPE,
            float_type: FLOAT_TYPE,
            double_type: DOUBLE_TYPE,
        };

        // Pre-register primitives at fixed indices (0-11)
        registry.register_primitive(PrimitiveType::Void, VOID_TYPE);
        registry.register_primitive(PrimitiveType::Bool, BOOL_TYPE);
        registry.register_primitive(PrimitiveType::Int8, INT8_TYPE);
        registry.register_primitive(PrimitiveType::Int16, INT16_TYPE);
        registry.register_primitive(PrimitiveType::Int32, INT32_TYPE);
        registry.register_primitive(PrimitiveType::Int64, INT64_TYPE);
        registry.register_primitive(PrimitiveType::Uint8, UINT8_TYPE);
        registry.register_primitive(PrimitiveType::Uint16, UINT16_TYPE);
        registry.register_primitive(PrimitiveType::Uint32, UINT32_TYPE);
        registry.register_primitive(PrimitiveType::Uint64, UINT64_TYPE);
        registry.register_primitive(PrimitiveType::Float, FLOAT_TYPE);
        registry.register_primitive(PrimitiveType::Double, DOUBLE_TYPE);

        // Fill gap 12-15 with placeholders
        while registry.types.len() < 16 {
            registry.types.push(TypeDef::Primitive {
                kind: PrimitiveType::Void,
            });
        }

        // Fill gap 19-31 with placeholders
        while registry.types.len() < FIRST_USER_TYPE_ID as usize {
            registry.types.push(TypeDef::Primitive {
                kind: PrimitiveType::Void,
            });
        }

        registry
    }

    /// Register a primitive type at a fixed index
    fn register_primitive(&mut self, kind: PrimitiveType, type_id: TypeId) {
        let index = type_id.as_u32() as usize;

        // Ensure vector is large enough
        while self.types.len() <= index {
            self.types.push(TypeDef::Primitive {
                kind: PrimitiveType::Void,
            });
        }

        self.types[index] = TypeDef::Primitive { kind };
        self.type_by_name.insert(kind.name().to_string(), type_id);
    }

    /// Register a new type and return its TypeId
    pub fn register_type(&mut self, typedef: TypeDef, name: Option<&str>) -> TypeId {
        let type_id = TypeId::new(self.types.len() as u32);
        self.types.push(typedef);

        if let Some(name) = name {
            self.type_by_name.insert(name.to_string(), type_id);
        }

        type_id
    }

    /// Register a type alias (typedef)
    ///
    /// This creates an alias name that points to an existing type.
    /// For example, `typedef float real;` would call `register_type_alias("real", FLOAT_TYPE)`.
    pub fn register_type_alias(&mut self, alias_name: &str, target_type: TypeId) {
        self.type_by_name
            .insert(alias_name.to_string(), target_type);
    }

    /// Look up a type by name (returns None if not found)
    pub fn lookup_type(&self, name: &str) -> Option<TypeId> {
        self.type_by_name.get(name).copied()
    }

    /// Get access to the type name map for iteration
    pub fn type_by_name(&self) -> &FxHashMap<String, TypeId> {
        &self.type_by_name
    }

    /// Get a type definition by TypeId
    pub fn get_type(&self, type_id: TypeId) -> &TypeDef {
        &self.types[type_id.as_u32() as usize]
    }

    /// Get a mutable type definition by TypeId
    pub fn get_type_mut(&mut self, type_id: TypeId) -> &mut TypeDef {
        &mut self.types[type_id.as_u32() as usize]
    }

    /// Get the behaviors for a type, if any are registered.
    pub fn get_behaviors(&self, type_id: TypeId) -> Option<&super::behaviors::TypeBehaviors> {
        self.behaviors.get(&type_id)
    }

    /// Get mutable behaviors for a type, if any are registered.
    pub fn get_behaviors_mut(
        &mut self,
        type_id: TypeId,
    ) -> Option<&mut super::behaviors::TypeBehaviors> {
        self.behaviors.get_mut(&type_id)
    }

    /// Set behaviors for a type. Overwrites any existing behaviors.
    pub fn set_behaviors(&mut self, type_id: TypeId, behaviors: super::behaviors::TypeBehaviors) {
        self.behaviors.insert(type_id, behaviors);
    }

    /// Get or create behaviors for a type (for incremental registration).
    pub fn behaviors_mut(&mut self, type_id: TypeId) -> &mut super::behaviors::TypeBehaviors {
        self.behaviors.entry(type_id).or_default()
    }

    /// Look up an enum value by enum type ID and value name
    /// Returns the numeric value if found, None otherwise
    pub fn lookup_enum_value(&self, type_id: TypeId, value_name: &str) -> Option<i64> {
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

    /// Instantiate a template with the given arguments
    pub fn instantiate_template(
        &mut self,
        template_id: TypeId,
        args: Vec<DataType>,
    ) -> Result<TypeId, SemanticError> {
        // Check if this is actually a template (a Class with non-empty template_params)
        let template_def = self.get_type(template_id);
        let (
            template_name,
            template_params,
            template_methods,
            template_operators,
            template_properties,
        ) = match template_def {
            TypeDef::Class {
                name,
                template_params,
                methods,
                operator_methods,
                properties,
                ..
            } if !template_params.is_empty() => (
                name.clone(),
                template_params.clone(),
                methods.clone(),
                operator_methods.clone(),
                properties.clone(),
            ),
            _ => {
                return Err(SemanticError::new(
                    SemanticErrorKind::NotATemplate,
                    Span::default(),
                    format!("Type {:?} is not a template", template_id),
                ));
            }
        };

        // Check argument count
        if args.len() != template_params.len() {
            return Err(SemanticError::new(
                SemanticErrorKind::WrongTemplateArgCount,
                Span::default(),
                format!(
                    "Template expects {} arguments, got {}",
                    template_params.len(),
                    args.len()
                ),
            ));
        }

        // Check cache - use only TypeIds for cache key so const qualifiers don't cause misses
        let cache_key_type_ids: Vec<TypeId> = args.iter().map(|dt| dt.type_id).collect();
        let cache_key = (template_id, cache_key_type_ids.clone());
        if let Some(&cached_id) = self.template_cache.get(&cache_key) {
            return Ok(cached_id);
        }

        // Invoke template_callback to validate type arguments (if registered)
        if let Some(callback) = self.template_callbacks.get(&template_id) {
            let info = crate::ffi::TemplateInstanceInfo {
                template_name: template_name.clone(),
                sub_types: args.clone(),
            };
            let validation = callback(&info);
            if !validation.is_valid {
                return Err(SemanticError::new(
                    SemanticErrorKind::InvalidTemplateInstantiation,
                    Span::default(),
                    validation
                        .error
                        .unwrap_or_else(|| "Template instantiation rejected by callback".to_string()),
                ));
            }
        }

        // Build instance name like "array<int32>" or "dict<string, int32>"
        let arg_names: Vec<String> = args
            .iter()
            .map(|arg| self.get_type(arg.type_id).name().to_string())
            .collect();
        let instance_name = format!("{}<{}>", template_name, arg_names.join(", "));

        // Build substitution map: template_param_type_id -> actual_type
        let subst_map: FxHashMap<TypeId, DataType> = template_params
            .iter()
            .zip(args.iter())
            .map(|(&param_id, arg)| (param_id, arg.clone()))
            .collect();

        // Reserve instance ID before specializing methods (they need to reference the new object type)
        let instance_id = TypeId::new(self.types.len() as u32);

        // Specialize methods
        let mut instance_methods = Vec::new();
        for &method_id in &template_methods {
            let specialized_id = self.specialize_function(method_id, &subst_map, instance_id);
            instance_methods.push(specialized_id);
        }

        // Specialize operators
        let mut instance_operators: FxHashMap<OperatorBehavior, Vec<FunctionId>> = FxHashMap::default();
        for (&behavior, op_ids) in &template_operators {
            let specialized_ids: Vec<FunctionId> = op_ids
                .iter()
                .map(|&op_id| self.specialize_function(op_id, &subst_map, instance_id))
                .collect();
            instance_operators.insert(behavior, specialized_ids);
        }

        // Create new instance as a Class with specialized methods
        let instance = TypeDef::Class {
            name: instance_name.clone(),
            qualified_name: instance_name.clone(),
            fields: Vec::new(),
            methods: instance_methods,
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: instance_operators,
            properties: template_properties, // TODO: specialize property functions
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(), // Instance is not a template
            template: Some(template_id),
            type_args: args.clone(),
        };

        // Actually register the instance
        self.types.push(instance);
        self.type_by_name.insert(instance_name.clone(), instance_id);

        // Copy and specialize behaviors from template to instance
        if let Some(template_behaviors) = self.behaviors.get(&template_id).cloned() {
            let mut instance_behaviors = template_behaviors.clone();

            // Specialize constructors
            instance_behaviors.constructors = template_behaviors
                .constructors
                .iter()
                .map(|&func_id| self.specialize_function(func_id, &subst_map, instance_id))
                .collect();

            // Specialize factories
            instance_behaviors.factories = template_behaviors
                .factories
                .iter()
                .map(|&func_id| self.specialize_function(func_id, &subst_map, instance_id))
                .collect();

            // Specialize single behaviors that involve template params
            if let Some(func_id) = template_behaviors.list_factory {
                instance_behaviors.list_factory =
                    Some(self.specialize_function(func_id, &subst_map, instance_id));
            }
            if let Some(func_id) = template_behaviors.list_construct {
                instance_behaviors.list_construct =
                    Some(self.specialize_function(func_id, &subst_map, instance_id));
            }

            self.behaviors.insert(instance_id, instance_behaviors);
        }

        // Cache the instance
        self.template_cache
            .insert((template_id, cache_key_type_ids), instance_id);

        Ok(instance_id)
    }

    /// Specialize a function by substituting template parameters with actual types.
    /// Returns a new FunctionId for the specialized function.
    /// If the function doesn't exist (e.g., a placeholder), returns the original ID unchanged.
    fn specialize_function(
        &mut self,
        orig_func_id: FunctionId,
        subst_map: &FxHashMap<TypeId, DataType>,
        new_object_type: TypeId,
    ) -> FunctionId {
        // If the function doesn't exist (placeholder ID), return it unchanged
        let Some(orig) = self.functions.get(&orig_func_id).cloned() else {
            return orig_func_id;
        };
        let new_id = FunctionId::next();

        // Substitute types in parameters
        // Pass new_object_type as self_type to replace SELF_TYPE placeholders
        let new_params: Vec<DataType> = orig
            .params
            .iter()
            .map(|p| self.substitute_type(p, subst_map, new_object_type))
            .collect();

        // Substitute return type
        let new_return = self.substitute_type(&orig.return_type, subst_map, new_object_type);

        // Create specialized function
        let new_func = FunctionDef {
            id: new_id,
            name: orig.name.clone(),
            namespace: orig.namespace.clone(),
            params: new_params,
            return_type: new_return,
            object_type: Some(new_object_type),
            traits: orig.traits,
            is_native: orig.is_native,
            default_args: orig.default_args.clone(),
            visibility: orig.visibility,
            signature_filled: true, // Specialized functions are always complete
        };

        self.functions.insert(new_id, new_func);
        new_id
    }

    /// Substitute template parameters in a DataType with actual types.
    /// `self_type` is the TypeId of the template instance being created, used to replace SELF_TYPE.
    fn substitute_type(
        &self,
        dt: &DataType,
        subst_map: &FxHashMap<TypeId, DataType>,
        self_type: TypeId,
    ) -> DataType {
        if dt.type_id == SELF_TYPE {
            // SELF_TYPE: replace with the instantiated type (e.g., array<int>)
            DataType {
                type_id: self_type,
                is_const: dt.is_const,
                is_handle: dt.is_handle,
                is_handle_to_const: dt.is_handle_to_const,
                ref_modifier: dt.ref_modifier.clone(),
            }
        } else if let Some(replacement) = subst_map.get(&dt.type_id) {
            // Found a template parameter to substitute
            let mut result = replacement.clone();
            // Merge qualifiers from original
            result.is_handle = dt.is_handle || result.is_handle;
            result.is_const = dt.is_const || result.is_const;
            if dt.ref_modifier != RefModifier::None {
                result.ref_modifier = dt.ref_modifier.clone();
            }
            result
        } else {
            // Not a template parameter, return as-is
            dt.clone()
        }
    }

    /// Register a function and return its FunctionId
    pub fn register_function(&mut self, def: FunctionDef<'ast>) -> FunctionId {
        let func_id = def.id;
        let qualified_name = def.qualified_name();

        self.functions.insert(func_id, def);

        // Add to overload map
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_id);

        func_id
    }

    /// Look up all functions with the given name (for overload resolution)
    pub fn lookup_functions(&self, name: &str) -> &[FunctionId] {
        self.func_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get a function definition by FunctionId
    pub fn get_function(&self, func_id: FunctionId) -> &FunctionDef<'ast> {
        self.functions
            .get(&func_id)
            .expect("FunctionId not found in registry")
    }

    /// Get a mutable function definition by FunctionId
    pub fn get_function_mut(&mut self, func_id: FunctionId) -> &mut FunctionDef<'ast> {
        self.functions
            .get_mut(&func_id)
            .expect("FunctionId not found in registry")
    }

    /// Get the count of registered functions
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Get the next available function ID
    pub fn next_function_id(&self) -> FunctionId {
        FunctionId::next()
    }

    /// Get the count of registered types
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get all methods for a given type
    pub fn get_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        self.functions
            .values()
            .filter(|f| f.object_type == Some(type_id))
            .map(|f| f.id)
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
        type_id: TypeId,
        fields: Vec<super::type_def::FieldDef>,
        methods: Vec<FunctionId>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
        operator_methods: FxHashMap<super::type_def::OperatorBehavior, Vec<FunctionId>>,
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
        type_id: TypeId,
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
        type_id: TypeId,
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
    pub fn get_funcdef_signature(&self, type_id: TypeId) -> Option<(&[DataType], &DataType)> {
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
        func_id: FunctionId,
        funcdef_type_id: TypeId,
    ) -> bool {
        let (funcdef_params, funcdef_return) = match self.get_funcdef_signature(funcdef_type_id) {
            Some(sig) => sig,
            None => return false,
        };

        let func = self.get_function(func_id);

        // Check return type matches
        if func.return_type.type_id != funcdef_return.type_id {
            return false;
        }

        // Check parameter count matches
        if func.params.len() != funcdef_params.len() {
            return false;
        }

        // Check each parameter type matches
        for (func_param, funcdef_param) in func.params.iter().zip(funcdef_params.iter()) {
            // Base type must match
            if func_param.type_id != funcdef_param.type_id {
                return false;
            }
            // Reference modifier must match
            if func_param.ref_modifier != funcdef_param.ref_modifier {
                return false;
            }
            // Handle modifier must match
            if func_param.is_handle != funcdef_param.is_handle {
                return false;
            }
        }

        true
    }

    /// Find a function by name that is compatible with a funcdef type
    /// Returns the FunctionId if found and compatible, None otherwise
    pub fn find_compatible_function(
        &self,
        name: &str,
        funcdef_type_id: TypeId,
    ) -> Option<FunctionId> {
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
        params: Vec<DataType>,
        return_type: DataType,
        object_type: Option<TypeId>,
        traits: FunctionTraits,
        default_args: Vec<Option<&'ast Expr<'ast>>>,
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
                            func_mut.default_args = default_args;
                            func_mut.signature_filled = true;
                        }
                        return; // Only update one function
                    }
                }
            }
        }
    }

    /// Update a function's parameters directly by FunctionId
    /// Used to fill in params for auto-generated constructors
    pub fn update_function_params(&mut self, func_id: FunctionId, params: Vec<DataType>) {
        if let Some(func) = self.functions.get_mut(&func_id) {
            func.params = params;
        }
    }

    /// Update a function's return type directly by FunctionId
    /// Used to fill in return type for auto-generated operators like opAssign
    pub fn update_function_return_type(&mut self, func_id: FunctionId, return_type: DataType) {
        if let Some(func) = self.functions.get_mut(&func_id) {
            func.return_type = return_type;
        }
    }

    /// Find a constructor for a given type with specific argument types
    /// Returns the FunctionId of the best matching constructor, if any
    pub fn find_constructor(&self, type_id: TypeId, arg_types: &[DataType]) -> Option<FunctionId> {
        // Get the type definition
        let typedef = self.get_type(type_id);

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
                    .all(|(param_type, arg_type)| param_type == arg_type);

                if all_match {
                    return Some(method_id);
                }
            }
        }

        None
    }

    /// Find all constructors for a given type
    /// Returns a vector of FunctionIds for all constructors
    pub fn find_constructors(&self, type_id: TypeId) -> Vec<FunctionId> {
        // Look up constructors from behaviors registry
        self.behaviors
            .get(&type_id)
            .map(|b| b.constructors.clone())
            .unwrap_or_default()
    }

    /// Check if a constructor is marked as explicit
    /// Explicit constructors cannot be used for implicit conversions
    pub fn is_constructor_explicit(&self, func_id: FunctionId) -> bool {
        let func = self.get_function(func_id);
        func.traits.is_explicit
    }

    /// Find the copy constructor for a given type
    /// Copy constructor has signature: ClassName(const ClassName&in) or ClassName(const ClassName&inout)
    /// Returns None if no copy constructor exists or if it was deleted
    pub fn find_copy_constructor(&self, type_id: TypeId) -> Option<FunctionId> {
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
                param.ref_modifier,
                crate::semantic::RefModifier::In | crate::semantic::RefModifier::InOut
            ) {
                continue;
            }

            // Parameter type must match the class type (ignoring const/ref modifiers)
            if param.type_id == type_id {
                return Some(ctor_id);
            }
        }

        None
    }

    /// Add a method to a class's methods list
    /// This is used when auto-generating constructors in Pass 1
    /// Works for both regular classes and template instances (which are also Classes)
    pub fn add_method_to_class(&mut self, type_id: TypeId, func_id: FunctionId) {
        let typedef = self.get_type_mut(type_id);
        match typedef {
            TypeDef::Class { methods, .. } => methods.push(func_id),
            _ => {}
        }
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
        type_id: TypeId,
        operator: OperatorBehavior,
    ) -> Option<FunctionId> {
        self.find_operator_methods(type_id, operator).first().copied()
    }

    /// Find all overloads of an operator method for a type.
    ///
    /// Returns all registered operator methods for the given behavior.
    /// Use this when you need to do overload resolution based on const-ness
    /// or parameter types.
    pub fn find_operator_methods(
        &self,
        type_id: TypeId,
        operator: OperatorBehavior,
    ) -> &[FunctionId] {
        let typedef = self.get_type(type_id);
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
    fn get_base_class(&self, type_id: TypeId) -> Option<TypeId> {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { base_class, .. } = typedef {
            *base_class
        } else {
            None
        }
    }

    /// Check if a class is marked as 'final' (cannot be inherited from)
    pub fn is_class_final(&self, type_id: TypeId) -> bool {
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
    pub fn is_subclass_of(&self, derived_class: TypeId, base_class: TypeId) -> bool {
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
    pub fn get_class_fields(&self, type_id: TypeId) -> &[super::type_def::FieldDef] {
        let typedef = self.get_type(type_id);
        if let TypeDef::Class { fields, .. } = typedef {
            fields
        } else {
            &[]
        }
    }

    /// Find a method directly on this class (not in base classes)
    fn find_direct_method(&self, type_id: TypeId, name: &str) -> Option<FunctionId> {
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
    pub fn find_method(&self, type_id: TypeId, name: &str) -> Option<FunctionId> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.find_method_impl(type_id, name, &mut visited)
    }

    fn find_method_impl(
        &self,
        type_id: TypeId,
        name: &str,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
    ) -> Option<FunctionId> {
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
    pub fn find_methods_by_name(&self, type_id: TypeId, name: &str) -> Vec<FunctionId> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.find_methods_by_name_impl(type_id, name, &mut visited)
    }

    fn find_methods_by_name_impl(
        &self,
        type_id: TypeId,
        name: &str,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
    ) -> Vec<FunctionId> {
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
                let mut matching_methods: Vec<FunctionId> = methods
                    .iter()
                    .copied()
                    .filter(|&id| self.get_function(id).name == name)
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
    pub fn get_all_methods(&self, type_id: TypeId) -> Vec<FunctionId> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_methods_impl(type_id, &mut visited)
    }

    fn get_all_methods_impl(
        &self,
        type_id: TypeId,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
    ) -> Vec<FunctionId> {
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
    pub fn get_all_properties(&self, type_id: TypeId) -> FxHashMap<String, PropertyAccessors> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_properties_impl(type_id, &mut visited)
    }

    fn get_all_properties_impl(
        &self,
        type_id: TypeId,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
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
    pub fn find_property(&self, type_id: TypeId, property_name: &str) -> Option<PropertyAccessors> {
        let all_properties = self.get_all_properties(type_id);
        all_properties.get(property_name).cloned()
    }

    /// Look up a method by name in a class (including inherited methods)
    ///
    /// Returns the first matching method using virtual dispatch (derived class methods take precedence).
    /// For overloaded methods, returns the first match only - use get_all_methods for full list.
    ///
    /// This is an alias for `find_method()` for backwards compatibility.
    pub fn find_method_by_name(&self, type_id: TypeId, method_name: &str) -> Option<FunctionId> {
        self.find_method(type_id, method_name)
    }

    /// Get all method signatures for an interface type
    ///
    /// Returns the list of MethodSignature for an interface, or None if not an interface.
    /// Used for validating that classes implement all interface methods.
    pub fn get_interface_methods(
        &self,
        type_id: TypeId,
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
    pub fn get_all_interfaces(&self, type_id: TypeId) -> Vec<TypeId> {
        let mut visited = rustc_hash::FxHashSet::default();
        self.get_all_interfaces_impl(type_id, &mut visited)
    }

    fn get_all_interfaces_impl(
        &self,
        type_id: TypeId,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
    ) -> Vec<TypeId> {
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
    /// Returns the FunctionId of the base method if found, None otherwise.
    pub fn find_base_method(&self, type_id: TypeId, method_name: &str) -> Option<FunctionId> {
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
        type_id: TypeId,
        method_name: &str,
        params: &[DataType],
        return_type: &DataType,
    ) -> Option<FunctionId> {
        // Get base class
        let base_id = self.get_base_class(type_id)?;

        // Get all methods with this name in base class chain
        let base_methods = self.find_methods_by_name(base_id, method_name);

        // Find one with matching signature
        for &method_id in &base_methods {
            let func = self.get_function(method_id);

            // Check return type
            if func.return_type.type_id != return_type.type_id {
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
                .all(|(a, b)| a.type_id == b.type_id && a.ref_modifier == b.ref_modifier);

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
        class_type_id: TypeId,
        interface_method: &super::type_def::MethodSignature,
    ) -> bool {
        // Get all methods with this name in the class hierarchy
        let methods = self.find_methods_by_name(class_type_id, &interface_method.name);

        for &method_id in &methods {
            let func = self.get_function(method_id);

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
                    func_param.type_id == iface_param.type_id
                        && func_param.ref_modifier == iface_param.ref_modifier
                        && func_param.is_handle == iface_param.is_handle
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
    pub fn is_base_method_final(&self, type_id: TypeId, method_name: &str) -> Option<FunctionId> {
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
        type_id: TypeId,
        proposed_base_id: TypeId,
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
        type_id: TypeId,
        visited: &mut rustc_hash::FxHashSet<TypeId>,
    ) -> bool {
        if visited.contains(&type_id) {
            return true;
        }
        visited.insert(type_id);
        false
    }

    // =========================================================================
    // FFI Module Import
    // =========================================================================

    /// Import native FFI modules into the registry.
    ///
    /// Converts all FFI registrations (types, functions, enums, interfaces,
    /// funcdefs, global properties) into Registry entries.
    ///
    /// # Processing Order
    ///
    /// Items are processed in dependency order:
    /// 1. Enums - No dependencies
    /// 2. Interfaces - Abstract method signatures only
    /// 3. Funcdefs - Function pointer types
    /// 4. Types/Classes - May reference enums, interfaces; includes methods
    /// 5. Functions - Global functions, may reference any type
    /// 6. Global Properties - May reference any type
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut registry = Registry::new();
    /// registry.import_modules(context.modules())?;
    /// ```
    pub fn import_modules(&mut self, modules: &[Module<'_>]) -> Result<(), ImportError> {
        // Phase 1: Register all enums (no dependencies)
        for module in modules {
            for enum_def in module.enums() {
                self.import_enum(enum_def, module.namespace())?;
            }
        }

        // Phase 2: Register all interfaces (no dependencies on other user types)
        for module in modules {
            for interface_def in module.interfaces() {
                self.import_interface(interface_def, module.namespace())?;
            }
        }

        // Phase 3: Register all funcdefs
        for module in modules {
            for funcdef_def in module.funcdefs() {
                self.import_funcdef(funcdef_def, module.namespace())?;
            }
        }

        // Phase 4: Register all types/classes (register type shells first)
        for module in modules {
            for type_def in module.types() {
                self.import_type_shell(type_def, module.namespace())?;
            }
        }

        // Phase 5: Fill in type details (methods, operators, properties)
        for module in modules {
            for type_def in module.types() {
                self.import_type_details(type_def, module.namespace())?;
            }
        }

        // Phase 6: Register all global functions
        for module in modules {
            for func_def in module.functions() {
                self.import_function(func_def, module.namespace())?;
            }
        }

        // Phase 7: Register all global properties
        for module in modules {
            for prop_def in module.global_properties() {
                self.import_global_property(prop_def, module.namespace())?;
            }
        }

        Ok(())
    }

    /// Import a native enum definition.
    fn import_enum(
        &mut self,
        enum_def: &NativeEnumDef,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let qualified_name = self.build_qualified_name(&enum_def.name, namespace);

        // Check for duplicates
        if self.type_by_name.contains_key(&qualified_name) {
            return Err(ImportError::DuplicateType(qualified_name));
        }

        // Create the TypeDef::Enum
        let typedef = TypeDef::Enum {
            name: enum_def.name.clone(),
            qualified_name: qualified_name.clone(),
            values: enum_def.values.clone(),
        };

        // Register at the pre-assigned TypeId
        self.register_type_at_id(enum_def.id, typedef, &qualified_name);

        Ok(())
    }

    /// Import a native interface definition.
    fn import_interface(
        &mut self,
        interface_def: &NativeInterfaceDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let qualified_name = self.build_qualified_name(&interface_def.name, namespace);

        // Check for duplicates
        if self.type_by_name.contains_key(&qualified_name) {
            return Err(ImportError::DuplicateType(qualified_name));
        }

        // Convert method signatures
        let mut methods = Vec::with_capacity(interface_def.methods.len());
        for method in &interface_def.methods {
            let method_sig = self.convert_interface_method(method, namespace)?;
            methods.push(method_sig);
        }

        // Create the TypeDef::Interface
        let typedef = TypeDef::Interface {
            name: interface_def.name.clone(),
            qualified_name: qualified_name.clone(),
            methods,
        };

        // Register at the pre-assigned TypeId
        self.register_type_at_id(interface_def.id, typedef, &qualified_name);

        Ok(())
    }

    /// Import a native funcdef definition.
    fn import_funcdef(
        &mut self,
        funcdef_def: &NativeFuncdefDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let name = funcdef_def.name.name.to_string();
        let qualified_name = self.build_qualified_name(&name, namespace);

        // Check for duplicates
        if self.type_by_name.contains_key(&qualified_name) {
            return Err(ImportError::DuplicateType(qualified_name));
        }

        // Resolve parameter types
        let mut params = Vec::with_capacity(funcdef_def.params.len());
        for param in funcdef_def.params {
            let data_type = self.resolve_ffi_param_type(&param.ty, namespace)?;
            params.push(data_type);
        }

        // Resolve return type
        let return_type = self.resolve_ffi_return_type(&funcdef_def.return_type, namespace)?;

        // Create the TypeDef::Funcdef
        let typedef = TypeDef::Funcdef {
            name: name.clone(),
            qualified_name: qualified_name.clone(),
            params,
            return_type,
        };

        // Register at the pre-assigned TypeId
        self.register_type_at_id(funcdef_def.id, typedef, &qualified_name);

        Ok(())
    }

    /// Import a native type definition (shell only - no methods yet).
    fn import_type_shell(
        &mut self,
        type_def: &NativeTypeDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let qualified_name = self.build_qualified_name(&type_def.name, namespace);

        // Get the actual type_id - either existing or new
        let actual_type_id = self.type_by_name.get(&qualified_name).copied();

        // Register template parameter types first (if this is a template)
        // This must happen even for pre-existing types (like builtin array)
        let template_params = if let Some(params) = &type_def.template_params {
            params
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let param_id = TypeId::next();
                    // Use the actual type_id as owner (if pre-existing) or the def's id
                    let owner_id = actual_type_id.unwrap_or(type_def.id);
                    let param_def = TypeDef::TemplateParam {
                        name: name.to_string(),
                        index: i,
                        owner: owner_id,
                    };
                    // Register at the specific index matching param_id
                    // (using register_type_at_id logic to ensure alignment)
                    let index = param_id.as_u32() as usize;
                    while self.types.len() <= index {
                        self.types.push(TypeDef::Primitive {
                            kind: PrimitiveType::Void,
                        });
                    }
                    self.types[index] = param_def;
                    // Register with a special internal name for lookup
                    self.type_by_name
                        .insert(format!("{}::${}", qualified_name, name), param_id);
                    param_id
                })
                .collect()
        } else {
            Vec::new()
        };

        // If type already exists, update its template_params and skip shell registration
        if let Some(existing_type_id) = actual_type_id {
            // Update the existing type's template_params if this is a template
            if !template_params.is_empty() {
                if let TypeDef::Class {
                    template_params: existing_params,
                    ..
                } = self.get_type_mut(existing_type_id)
                {
                    *existing_params = template_params;
                }
            }
            return Ok(());
        }

        // Register as Class type (shell with empty methods, populated in import_type_details)
        let typedef = TypeDef::Class {
            name: type_def.name.clone(),
            qualified_name: qualified_name.clone(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params,
            template: None,
            type_args: Vec::new(),
        };
        self.register_type_at_id(type_def.id, typedef, &qualified_name);

        // Register behaviors for the class/template
        self.import_behaviors(type_def, namespace)?;

        Ok(())
    }

    /// Import behaviors for a type and store them in Registry::behaviors.
    /// Behaviors are stored directly on NativeTypeDef.
    /// FunctionIds are taken from the NativeFn (assigned at FFI registration time).
    fn import_behaviors(
        &mut self,
        type_def: &NativeTypeDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let type_id = type_def.id;
        let mut type_behaviors = super::behaviors::TypeBehaviors::new();

        // Import list_factory behavior
        if let Some(ref list_factory) = type_def.list_factory {
            let func_id = list_factory.native_fn.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$list_factory".to_string(),
                namespace: namespace.to_vec(),
                params: vec![], // List factory takes list buffer via special mechanism
                return_type: DataType::simple(type_id),
                object_type: Some(type_id),
                traits: FunctionTraits {
                    is_constructor: true,
                    ..FunctionTraits::default()
                },
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.list_factory = Some(func_id);
        }

        // Import list_construct behavior
        if let Some(ref list_construct) = type_def.list_construct {
            let func_id = list_construct.native_fn.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$list_construct".to_string(),
                namespace: namespace.to_vec(),
                params: vec![], // List construct takes list buffer via special mechanism
                return_type: DataType::simple(VOID_TYPE),
                object_type: Some(type_id),
                traits: FunctionTraits {
                    is_constructor: true,
                    ..FunctionTraits::default()
                },
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.list_construct = Some(func_id);
        }

        // Import addref behavior
        if let Some(ref addref) = type_def.addref {
            let func_id = addref.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$addref".to_string(),
                namespace: namespace.to_vec(),
                params: vec![],
                return_type: DataType::simple(VOID_TYPE),
                object_type: Some(type_id),
                traits: FunctionTraits::default(),
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.addref = Some(func_id);
        }

        // Import release behavior
        if let Some(ref release) = type_def.release {
            let func_id = release.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$release".to_string(),
                namespace: namespace.to_vec(),
                params: vec![],
                return_type: DataType::simple(VOID_TYPE),
                object_type: Some(type_id),
                traits: FunctionTraits::default(),
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.release = Some(func_id);
        }

        // Import destruct behavior
        if let Some(ref destruct) = type_def.destruct {
            let func_id = destruct.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$destruct".to_string(),
                namespace: namespace.to_vec(),
                params: vec![],
                return_type: DataType::simple(VOID_TYPE),
                object_type: Some(type_id),
                traits: FunctionTraits {
                    is_destructor: true,
                    ..FunctionTraits::default()
                },
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.destruct = Some(func_id);
        }

        // Import get_weakref_flag behavior
        if let Some(ref get_weakref_flag) = type_def.get_weakref_flag {
            let func_id = get_weakref_flag.id;
            let func_def = FunctionDef {
                id: func_id,
                name: "$get_weakref_flag".to_string(),
                namespace: namespace.to_vec(),
                params: vec![],
                return_type: DataType::simple(VOID_TYPE), // Actually returns weak ref flag, but we use void placeholder
                object_type: Some(type_id),
                traits: FunctionTraits::default(),
                is_native: true,
                default_args: Vec::new(),
                visibility: Visibility::Public,
                signature_filled: true,
            };
            self.functions.insert(func_id, func_def);
            type_behaviors.get_weakref_flag = Some(func_id);
        }

        // Import template_callback - stored separately since it's a native closure, not a script function
        if let Some(ref callback) = type_def.template_callback {
            self.template_callbacks.insert(type_id, callback.clone());
        }

        // Only store if we have any behaviors (or always store to allow adding constructors later)
        // Always store so import_type_details can add constructors/factories
        self.behaviors.insert(type_id, type_behaviors);

        Ok(())
    }

    /// Import type details (methods, operators, properties).
    /// For templates, methods are imported with template param TypeIds which get
    /// substituted during instantiation.
    fn import_type_details(
        &mut self,
        type_def: &NativeTypeDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        // Look up the type ID by name - this handles builtin types (like array)
        // which have pre-assigned IDs different from the NativeTypeDef's ID
        let qualified_name = self.build_qualified_name(&type_def.name, namespace);
        let type_id = self
            .type_by_name
            .get(&qualified_name)
            .copied()
            .unwrap_or(type_def.id);
        let mut method_ids = Vec::new();
        let mut constructor_ids = Vec::new();
        let mut factory_ids = Vec::new();
        let mut operator_methods: FxHashMap<OperatorBehavior, Vec<FunctionId>> = FxHashMap::default();
        let mut properties = FxHashMap::default();

        // Determine template context for resolving template parameters like T
        // If this type has template_params, use qualified_name as the context
        let template_context: Option<&str> = if type_def.template_params.is_some() {
            Some(&qualified_name)
        } else {
            None
        };

        // Import constructors
        for ctor in &type_def.constructors {
            let func_id =
                self.import_method(ctor, type_id, namespace, true, false, template_context)?;
            method_ids.push(func_id);
            constructor_ids.push(func_id);
        }

        // Import factories (treated as constructors for reference types)
        for factory in &type_def.factories {
            let func_id =
                self.import_method(factory, type_id, namespace, true, false, template_context)?;
            method_ids.push(func_id);
            factory_ids.push(func_id);
        }

        // Import regular methods
        for method in &type_def.methods {
            let func_id =
                self.import_method(method, type_id, namespace, false, false, template_context)?;
            method_ids.push(func_id);
        }

        // Import operators
        for operator in &type_def.operators {
            let func_id =
                self.import_method(operator, type_id, namespace, false, true, template_context)?;
            method_ids.push(func_id);

            // Map operator name to OperatorBehavior
            let method_name = operator.name.name;
            // For conversion operators, we need the return type to get the target TypeId
            let target_type = if method_name.starts_with("opConv")
                || method_name.starts_with("opImplConv")
                || method_name.starts_with("opCast")
                || method_name.starts_with("opImplCast")
            {
                let return_dt = self.resolve_ffi_return_type_with_template(
                    &operator.return_type,
                    namespace,
                    template_context,
                )?;
                Some(return_dt.type_id)
            } else {
                None
            };

            if let Some(behavior) = OperatorBehavior::from_method_name(method_name, target_type) {
                operator_methods.entry(behavior).or_default().push(func_id);
            }
        }

        // Import properties
        for prop in &type_def.properties {
            let prop_accessors =
                self.import_property_with_template(prop, type_id, namespace, template_context)?;
            properties.insert(prop.name.name.to_string(), prop_accessors);
        }

        // Update the TypeDef::Class with the collected data
        let typedef = self.get_type_mut(type_id);
        if let TypeDef::Class {
            methods: class_methods,
            operator_methods: class_operators,
            properties: class_properties,
            ..
        } = typedef
        {
            *class_methods = method_ids;
            *class_operators = operator_methods;
            *class_properties = properties;
        }

        // Update TypeBehaviors with constructors and factories
        // TypeBehaviors was created in import_behaviors, now add constructors/factories
        if let Some(behaviors) = self.behaviors.get_mut(&type_id) {
            behaviors.constructors = constructor_ids;
            behaviors.factories = factory_ids;
        } else if !constructor_ids.is_empty() || !factory_ids.is_empty() {
            // If behaviors didn't exist (shouldn't happen), create one
            let mut behaviors = super::behaviors::TypeBehaviors::new();
            behaviors.constructors = constructor_ids;
            behaviors.factories = factory_ids;
            self.behaviors.insert(type_id, behaviors);
        }

        Ok(())
    }

    /// Import a method and return its FunctionId.
    /// `template_context` is the qualified name of the owning template type (e.g., "array")
    /// so that template parameters like `T` can be resolved correctly.
    /// FunctionId is taken from the NativeFn (assigned at FFI registration time).
    fn import_method(
        &mut self,
        method: &NativeMethodDef<'_>,
        object_type: TypeId,
        namespace: &[String],
        is_constructor: bool,
        is_operator: bool,
        template_context: Option<&str>,
    ) -> Result<FunctionId, ImportError> {
        let func_id = method.native_fn.id;
        let name = method.name.name.to_string();

        // Resolve parameter types
        let mut params = Vec::with_capacity(method.params.len());
        for param in method.params {
            let data_type =
                self.resolve_ffi_param_type_with_template(&param.ty, namespace, template_context)?;
            params.push(data_type);
        }

        // Resolve return type
        let return_type = self.resolve_ffi_return_type_with_template(
            &method.return_type,
            namespace,
            template_context,
        )?;

        // Build function traits
        let traits = FunctionTraits {
            is_constructor,
            is_destructor: name == "~" || name.starts_with('~'),
            is_final: false,
            is_virtual: !is_constructor && !is_operator,
            is_abstract: false,
            is_const: method.is_const,
            is_explicit: false,
            auto_generated: None,
        };

        // Create the FunctionDef
        let func_def = FunctionDef {
            id: func_id,
            name,
            namespace: namespace.to_vec(),
            params,
            return_type,
            object_type: Some(object_type),
            traits,
            is_native: true,
            default_args: Vec::new(), // TODO: Handle default args if needed
            visibility: Visibility::Public,
            signature_filled: true,
        };

        self.functions.insert(func_id, func_def);
        Ok(func_id)
    }

    /// Import a property and return its PropertyAccessors.
    fn import_property(
        &mut self,
        prop: &NativePropertyDef<'_>,
        object_type: TypeId,
        namespace: &[String],
    ) -> Result<PropertyAccessors, ImportError> {
        self.import_property_with_template(prop, object_type, namespace, None)
    }

    /// Import a property with optional template context and return its PropertyAccessors.
    /// FunctionIds are taken from the NativeFn (assigned at FFI registration time).
    fn import_property_with_template(
        &mut self,
        prop: &NativePropertyDef<'_>,
        object_type: TypeId,
        namespace: &[String],
        template_context: Option<&str>,
    ) -> Result<PropertyAccessors, ImportError> {
        let prop_name = prop.name.name.to_string();
        let prop_type =
            self.resolve_ffi_type_expr_with_template(prop.ty, namespace, template_context)?;

        // Create getter function (FunctionId from NativeFn)
        let getter_id = prop.getter.id;
        let getter_name = format!("get_{}", prop_name);
        let getter_def = FunctionDef {
            id: getter_id,
            name: getter_name,
            namespace: namespace.to_vec(),
            params: Vec::new(),
            return_type: prop_type.clone(),
            object_type: Some(object_type),
            traits: FunctionTraits {
                is_const: true,
                ..FunctionTraits::new()
            },
            is_native: true,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };
        self.functions.insert(getter_id, getter_def);

        // Create setter function if property is not const (FunctionId from NativeFn)
        let setter_id = if !prop.is_const {
            if let Some(ref setter) = prop.setter {
                let setter_id = setter.id;
                let setter_name = format!("set_{}", prop_name);
                let setter_def = FunctionDef {
                    id: setter_id,
                    name: setter_name,
                    namespace: namespace.to_vec(),
                    params: vec![prop_type],
                    return_type: DataType::simple(self.void_type),
                    object_type: Some(object_type),
                    traits: FunctionTraits::new(),
                    is_native: true,
                    default_args: Vec::new(),
                    visibility: Visibility::Public,
                    signature_filled: true,
                };
                self.functions.insert(setter_id, setter_def);
                Some(setter_id)
            } else {
                None
            }
        } else {
            None
        };

        Ok(PropertyAccessors {
            getter: Some(getter_id),
            setter: setter_id,
            visibility: Visibility::Public,
        })
    }

    /// Import a global function.
    /// FunctionId is taken from the NativeFn (assigned at FFI registration time).
    fn import_function(
        &mut self,
        func_def: &NativeFunctionDef<'_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let func_id = func_def.native_fn.id;
        let name = func_def.name.name.to_string();
        let qualified_name = self.build_qualified_name(&name, namespace);

        // Resolve parameter types
        let mut params = Vec::with_capacity(func_def.params.len());
        for param in func_def.params {
            let data_type = self.resolve_ffi_param_type(&param.ty, namespace)?;
            params.push(data_type);
        }

        // Resolve return type
        let return_type = self.resolve_ffi_return_type(&func_def.return_type, namespace)?;

        // Create the FunctionDef
        let function_def = FunctionDef {
            id: func_id,
            name,
            namespace: namespace.to_vec(),
            params,
            return_type,
            object_type: None,
            traits: func_def.traits,
            is_native: true,
            default_args: Vec::new(), // TODO: Handle default args
            visibility: func_def.visibility,
            signature_filled: true,
        };

        // Register the function
        self.functions.insert(func_id, function_def);
        self.func_by_name
            .entry(qualified_name)
            .or_default()
            .push(func_id);

        Ok(())
    }

    /// Import a global property.
    fn import_global_property(
        &mut self,
        prop_def: &crate::ffi::GlobalPropertyDef<'_, '_>,
        namespace: &[String],
    ) -> Result<(), ImportError> {
        let name = prop_def.name.name.to_string();
        let data_type = self.resolve_ffi_type_expr(&prop_def.ty, namespace)?;

        self.register_global_var(name, namespace.to_vec(), data_type);
        Ok(())
    }

    /// Convert an interface method to a MethodSignature.
    fn convert_interface_method(
        &self,
        method: &NativeInterfaceMethod<'_>,
        namespace: &[String],
    ) -> Result<MethodSignature, ImportError> {
        let name = method.name.name.to_string();

        // Resolve parameter types
        let mut params = Vec::with_capacity(method.params.len());
        for param in method.params {
            let data_type = self.resolve_ffi_param_type(&param.ty, namespace)?;
            params.push(data_type);
        }

        // Resolve return type
        let return_type = self.resolve_ffi_return_type(&method.return_type, namespace)?;

        Ok(MethodSignature {
            name,
            params,
            return_type,
        })
    }

    /// Resolve an FFI ParamType to a DataType.
    fn resolve_ffi_param_type(
        &self,
        param_type: &ParamType<'_>,
        namespace: &[String],
    ) -> Result<DataType, ImportError> {
        self.resolve_ffi_param_type_with_template(param_type, namespace, None)
    }

    /// Resolve an FFI ParamType to a DataType with optional template context.
    fn resolve_ffi_param_type_with_template(
        &self,
        param_type: &ParamType<'_>,
        namespace: &[String],
        template_context: Option<&str>,
    ) -> Result<DataType, ImportError> {
        let mut data_type =
            self.resolve_ffi_type_expr_with_template(&param_type.ty, namespace, template_context)?;

        // Apply reference modifier from ParamType
        data_type.ref_modifier = match param_type.ref_kind {
            RefKind::None => RefModifier::None,
            RefKind::Ref => RefModifier::InOut,
            RefKind::RefIn => RefModifier::In,
            RefKind::RefOut => RefModifier::Out,
            RefKind::RefInOut => RefModifier::InOut,
        };

        Ok(data_type)
    }

    /// Resolve an FFI ReturnType to a DataType.
    fn resolve_ffi_return_type(
        &self,
        return_type: &crate::ast::ReturnType<'_>,
        namespace: &[String],
    ) -> Result<DataType, ImportError> {
        self.resolve_ffi_return_type_with_template(return_type, namespace, None)
    }

    /// Resolve an FFI ReturnType to a DataType with optional template context.
    fn resolve_ffi_return_type_with_template(
        &self,
        return_type: &crate::ast::ReturnType<'_>,
        namespace: &[String],
        template_context: Option<&str>,
    ) -> Result<DataType, ImportError> {
        // ReturnType has a TypeExpr directly, check if it's void
        let type_expr = &return_type.ty;
        if matches!(type_expr.base, TypeBase::Primitive(AstPrimitiveType::Void)) {
            Ok(DataType::simple(self.void_type))
        } else {
            // Build a ParamType from the TypeExpr for consistent resolution
            let param_type = ParamType {
                ty: *type_expr,
                ref_kind: if return_type.is_ref {
                    RefKind::Ref
                } else {
                    RefKind::None
                },
                span: return_type.span,
            };
            self.resolve_ffi_param_type_with_template(&param_type, namespace, template_context)
        }
    }

    /// Resolve an FFI TypeExpr to a DataType.
    fn resolve_ffi_type_expr(
        &self,
        type_expr: &TypeExpr<'_>,
        namespace: &[String],
    ) -> Result<DataType, ImportError> {
        self.resolve_ffi_type_expr_with_template(type_expr, namespace, None)
    }

    /// Resolve an FFI TypeExpr to a DataType with optional template context.
    /// If `template_context` is provided (e.g., "array"), template parameters
    /// like `T` will be looked up as `array::$T`.
    fn resolve_ffi_type_expr_with_template(
        &self,
        type_expr: &TypeExpr<'_>,
        namespace: &[String],
        template_context: Option<&str>,
    ) -> Result<DataType, ImportError> {
        // Step 1: Resolve the base type
        let base_type_id =
            self.resolve_ffi_base_type_with_template(&type_expr.base, namespace, template_context)?;

        // Step 2: Handle template arguments
        let type_id = if !type_expr.template_args.is_empty() {
            // Resolve all template argument types, passing through template_context
            let mut arg_types = Vec::with_capacity(type_expr.template_args.len());
            for arg in type_expr.template_args {
                let dt =
                    self.resolve_ffi_type_expr_with_template(arg, namespace, template_context)?;
                arg_types.push(dt);
            }

            // Check for self-referential template pattern (e.g., array<T> within array template)
            // If the base type matches the template context AND all args are template params
            // of that template, use SELF_TYPE placeholder
            if let Some(template_name) = template_context {
                // Check if base type is the template we're importing
                let base_is_template_context =
                    self.lookup_type(template_name) == Some(base_type_id);

                // Check if all args are template params belonging to this template
                let all_args_are_template_params = arg_types.iter().all(|dt| {
                    if let TypeDef::TemplateParam { owner, .. } = self.get_type(dt.type_id) {
                        *owner == base_type_id
                    } else {
                        false
                    }
                });

                if base_is_template_context && all_args_are_template_params {
                    // This is a self-referential template type like array<T>
                    // Use SELF_TYPE which will be replaced at instantiation time
                    SELF_TYPE
                } else {
                    // Not self-referential, try cache lookup
                    let cache_key_ids: Vec<TypeId> =
                        arg_types.iter().map(|dt| dt.type_id).collect();
                    if let Some(&cached_id) =
                        self.template_cache.get(&(base_type_id, cache_key_ids))
                    {
                        cached_id
                    } else {
                        return Err(ImportError::TemplateInstantiationFailed(
                            "template instantiation not supported during FFI import; use concrete types or pre-register instances".to_string()
                        ));
                    }
                }
            } else {
                // No template context, try cache lookup
                let cache_key_ids: Vec<TypeId> = arg_types.iter().map(|dt| dt.type_id).collect();
                if let Some(&cached_id) = self.template_cache.get(&(base_type_id, cache_key_ids)) {
                    cached_id
                } else {
                    return Err(ImportError::TemplateInstantiationFailed(
                        "template instantiation not supported during FFI import; use concrete types or pre-register instances".to_string()
                    ));
                }
            }
        } else {
            base_type_id
        };

        // Step 3: Build DataType with modifiers
        let mut data_type = DataType::simple(type_id);

        // Apply const modifier
        if type_expr.is_const {
            data_type.is_const = true;
        }

        // Apply suffixes (handle, array)
        for suffix in type_expr.suffixes {
            match suffix {
                TypeSuffix::Handle { is_const } => {
                    data_type.is_handle = true;
                    data_type.is_handle_to_const = *is_const;
                    // Leading const + @ means handle to const
                    if type_expr.is_const && data_type.is_handle {
                        data_type.is_handle_to_const = true;
                        data_type.is_const = false;
                    }
                    break; // Only first @ matters
                }
            }
        }

        Ok(data_type)
    }

    /// Resolve a base type from a TypeBase.
    fn resolve_ffi_base_type(
        &self,
        base: &TypeBase<'_>,
        namespace: &[String],
    ) -> Result<TypeId, ImportError> {
        self.resolve_ffi_base_type_with_template(base, namespace, None)
    }

    /// Resolve a base type from a TypeBase with optional template context.
    /// If `template_context` is provided (e.g., "array"), template parameter names
    /// like `T` will be looked up as `array::$T`.
    fn resolve_ffi_base_type_with_template(
        &self,
        base: &TypeBase<'_>,
        namespace: &[String],
        template_context: Option<&str>,
    ) -> Result<TypeId, ImportError> {
        match base {
            TypeBase::Primitive(prim) => Ok(self.primitive_to_type_id(*prim)),

            TypeBase::Named(ident) => {
                let type_name = ident.name;

                // If we have a template context, check if this is a template parameter
                // Template parameters are registered as "<template_name>::$<param_name>"
                if let Some(template_name) = template_context {
                    let param_lookup = format!("{}::${}", template_name, type_name);
                    if let Some(type_id) = self.lookup_type(&param_lookup) {
                        return Ok(type_id);
                    }
                }

                // Try qualified name first
                let qualified = self.build_qualified_name(type_name, namespace);
                if let Some(type_id) = self.lookup_type(&qualified) {
                    return Ok(type_id);
                }

                // Try unqualified (global)
                if let Some(type_id) = self.lookup_type(type_name) {
                    return Ok(type_id);
                }

                Err(ImportError::TypeNotFound(type_name.to_string()))
            }

            TypeBase::TemplateParam(_) => {
                // Template parameters shouldn't appear in FFI type resolution
                Err(ImportError::TypeResolutionFailed {
                    type_name: "template parameter".to_string(),
                    reason: "template parameters not supported in FFI".to_string(),
                })
            }

            TypeBase::Auto | TypeBase::Unknown => Err(ImportError::TypeResolutionFailed {
                type_name: "auto/unknown".to_string(),
                reason: "auto and unknown types not supported in FFI".to_string(),
            }),
        }
    }

    /// Convert AST PrimitiveType to TypeId.
    fn primitive_to_type_id(&self, prim: AstPrimitiveType) -> TypeId {
        match prim {
            AstPrimitiveType::Void => VOID_TYPE,
            AstPrimitiveType::Bool => BOOL_TYPE,
            AstPrimitiveType::Int8 => INT8_TYPE,
            AstPrimitiveType::Int16 => INT16_TYPE,
            AstPrimitiveType::Int => INT32_TYPE,
            AstPrimitiveType::Int64 => INT64_TYPE,
            AstPrimitiveType::UInt8 => UINT8_TYPE,
            AstPrimitiveType::UInt16 => UINT16_TYPE,
            AstPrimitiveType::UInt => UINT32_TYPE,
            AstPrimitiveType::UInt64 => UINT64_TYPE,
            AstPrimitiveType::Float => FLOAT_TYPE,
            AstPrimitiveType::Double => DOUBLE_TYPE,
        }
    }

    /// Build a qualified name from a simple name and namespace.
    fn build_qualified_name(&self, name: &str, namespace: &[String]) -> String {
        if namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", namespace.join("::"), name)
        }
    }

    /// Register a type at a specific pre-assigned TypeId.
    ///
    /// This is used for FFI types where the TypeId was assigned during registration.
    fn register_type_at_id(&mut self, type_id: TypeId, typedef: TypeDef, name: &str) {
        let index = type_id.as_u32() as usize;

        // Ensure the types vec is large enough
        while self.types.len() <= index {
            // Add placeholder types
            self.types.push(TypeDef::Primitive {
                kind: PrimitiveType::Void,
            });
        }

        self.types[index] = typedef;
        self.type_by_name.insert(name.to_string(), type_id);
    }
}

impl<'ast> Default for Registry<'ast> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::types::type_def::Visibility;

    #[test]
    fn registry_new_has_primitives() {
        let registry = Registry::new();
        assert_eq!(registry.types.len(), 32);
    }

    #[test]
    fn registry_void_type() {
        let registry = Registry::new();
        let typedef = registry.get_type(VOID_TYPE);
        assert!(typedef.is_primitive());
        assert_eq!(typedef.name(), "void");
    }

    #[test]
    fn registry_bool_type() {
        let registry = Registry::new();
        let typedef = registry.get_type(BOOL_TYPE);
        assert!(typedef.is_primitive());
        assert_eq!(typedef.name(), "bool");
    }

    #[test]
    fn registry_int_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(INT8_TYPE).name(), "int8");
        assert_eq!(registry.get_type(INT16_TYPE).name(), "int16");
        assert_eq!(registry.get_type(INT32_TYPE).name(), "int");
        assert_eq!(registry.get_type(INT64_TYPE).name(), "int64");
    }

    #[test]
    fn registry_uint_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(UINT8_TYPE).name(), "uint8");
        assert_eq!(registry.get_type(UINT16_TYPE).name(), "uint16");
        assert_eq!(registry.get_type(UINT32_TYPE).name(), "uint");
        assert_eq!(registry.get_type(UINT64_TYPE).name(), "uint64");
    }

    #[test]
    fn registry_float_types() {
        let registry = Registry::new();

        assert_eq!(registry.get_type(FLOAT_TYPE).name(), "float");
        assert_eq!(registry.get_type(DOUBLE_TYPE).name(), "double");
    }

    #[test]
    fn lookup_primitive_by_name() {
        let registry = Registry::new();

        assert_eq!(registry.lookup_type("void"), Some(VOID_TYPE));
        assert_eq!(registry.lookup_type("bool"), Some(BOOL_TYPE));
        assert_eq!(registry.lookup_type("int"), Some(INT32_TYPE));
        assert_eq!(registry.lookup_type("uint"), Some(UINT32_TYPE));
        assert_eq!(registry.lookup_type("float"), Some(FLOAT_TYPE));
        assert_eq!(registry.lookup_type("double"), Some(DOUBLE_TYPE));
    }

    #[test]
    fn lookup_nonexistent_type() {
        let registry = Registry::new();
        assert_eq!(registry.lookup_type("NonExistent"), None);
    }

    #[test]
    fn register_simple_class() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };

        let type_id = registry.register_type(typedef, Some("Player"));
        assert_eq!(registry.lookup_type("Player"), Some(type_id));
        assert!(registry.get_type(type_id).is_class());
    }

    #[test]
    fn register_qualified_class() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
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
        };

        let type_id = registry.register_type(typedef, Some("Game::Player"));
        assert_eq!(registry.lookup_type("Game::Player"), Some(type_id));
    }

    #[test]
    fn register_interface() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Interface {
            name: "IDrawable".to_string(),
            qualified_name: "IDrawable".to_string(),
            methods: Vec::new(),
        };

        let type_id = registry.register_type(typedef, Some("IDrawable"));
        assert_eq!(registry.lookup_type("IDrawable"), Some(type_id));
        assert!(registry.get_type(type_id).is_interface());
    }

    #[test]
    fn register_enum() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
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
        let mut registry = Registry::new();

        let typedef = TypeDef::Funcdef {
            name: "Callback".to_string(),
            qualified_name: "Callback".to_string(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
        };

        let type_id = registry.register_type(typedef, Some("Callback"));
        assert_eq!(registry.lookup_type("Callback"), Some(type_id));
        assert!(registry.get_type(type_id).is_funcdef());
    }

    #[test]
    fn get_type_mut() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };

        let type_id = registry.register_type(typedef, Some("Player"));

        // Modify the type
        if let TypeDef::Class { fields, .. } = registry.get_type_mut(type_id) {
            fields.push(super::super::type_def::FieldDef::new(
                "health".to_string(),
                DataType::simple(INT32_TYPE),
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

    /// Helper to register a test template for template tests
    fn register_test_template(registry: &mut Registry, name: &str, param_count: usize) -> TypeId {
        // First register the template TypeId placeholder
        let template_id = registry.register_type(
            TypeDef::Primitive {
                kind: PrimitiveType::Void,
            }, // temporary placeholder
            Some(name),
        );

        // Register template param types and collect their TypeIds
        let template_params: Vec<TypeId> = (0..param_count)
            .map(|i| {
                let param_name = format!("T{}", i);
                let param_typedef = TypeDef::TemplateParam {
                    name: param_name.clone(),
                    index: i,
                    owner: template_id,
                };
                registry.register_type(param_typedef, None)
            })
            .collect();

        // Now update with the proper Class typedef
        let typedef = TypeDef::Class {
            name: name.to_string(),
            qualified_name: name.to_string(),
            fields: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params,
            template: None,
            type_args: Vec::new(),
        };
        *registry.get_type_mut(template_id) = typedef;
        template_id
    }

    #[test]
    fn instantiate_template() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestContainer", 1);

        let instance_id = registry
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let typedef = registry.get_type(instance_id);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_template_caching() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestContainer", 1);

        let instance1 = registry
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let instance2 = registry
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        assert_eq!(instance1, instance2);
    }

    #[test]
    fn instantiate_template_different_args() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestContainer", 1);

        let instance1 = registry
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        let instance2 = registry
            .instantiate_template(template_id, vec![DataType::simple(FLOAT_TYPE)])
            .unwrap();

        assert_ne!(instance1, instance2);
    }

    #[test]
    fn instantiate_dict_like_template() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestMap", 2);

        let instance_id = registry
            .instantiate_template(
                template_id,
                vec![DataType::simple(INT32_TYPE), DataType::simple(FLOAT_TYPE)],
            )
            .unwrap();

        let typedef = registry.get_type(instance_id);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_nested_template() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestContainer", 1);

        // TestContainer<int>
        let inner = registry
            .instantiate_template(template_id, vec![DataType::simple(INT32_TYPE)])
            .unwrap();

        // TestContainer<TestContainer<int>>
        let outer = registry
            .instantiate_template(template_id, vec![DataType::simple(inner)])
            .unwrap();

        let typedef = registry.get_type(outer);
        assert!(typedef.is_template_instance());
    }

    #[test]
    fn instantiate_non_template_fails() {
        let mut registry = Registry::new();

        let result = registry.instantiate_template(INT32_TYPE, vec![DataType::simple(INT32_TYPE)]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind, SemanticErrorKind::NotATemplate);
    }

    #[test]
    fn instantiate_wrong_arg_count_fails() {
        let mut registry = Registry::new();
        let template_id = register_test_template(&mut registry, "TestContainer", 1);

        // template expects 1 arg, give it 2
        let result = registry.instantiate_template(
            template_id,
            vec![DataType::simple(INT32_TYPE), DataType::simple(FLOAT_TYPE)],
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            SemanticErrorKind::WrongTemplateArgCount
        );
    }

    #[test]
    fn register_function() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let func_id = registry.register_function(func);
        assert_eq!(func_id, FunctionId::new(0));
    }

    #[test]
    fn lookup_function_by_name() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        registry.register_function(func);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0], FunctionId::new(0));
    }

    #[test]
    fn lookup_nonexistent_function() {
        let registry = Registry::new();
        let functions = registry.lookup_functions("nonexistent");
        assert_eq!(functions.len(), 0);
    }

    #[test]
    fn function_overloading() {
        let mut registry = Registry::new();

        // foo(int)
        let func1 = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        // foo(float)
        let func2 = FunctionDef {
            id: FunctionId::new(1),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(FLOAT_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        registry.register_function(func1);
        registry.register_function(func2);

        let functions = registry.lookup_functions("foo");
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn qualified_function_name() {
        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "update".to_string(),
            namespace: vec!["Game".to_string(), "Player".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        assert_eq!(func.qualified_name(), "Game::Player::update");
    }

    #[test]
    fn unqualified_function_name() {
        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        assert_eq!(func.qualified_name(), "foo");
    }

    #[test]
    fn get_function() {
        let mut registry = Registry::new();

        let func = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: Vec::new(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let func_id = registry.register_function(func.clone());
        let retrieved = registry.get_function(func_id);
        assert_eq!(retrieved.name, "foo");
    }

    #[test]
    fn get_methods_for_type() {
        let mut registry = Registry::new();

        let player_type = TypeId::new(100);

        // Method for Player
        let method1 = FunctionDef {
            id: FunctionId::new(0),
            name: "update".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        // Another method for Player
        let method2 = FunctionDef {
            id: FunctionId::new(1),
            name: "draw".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: Some(player_type),
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        // Global function (not a method)
        let global_func = FunctionDef {
            id: FunctionId::new(2),
            name: "main".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        registry.register_function(method1);
        registry.register_function(method2);
        registry.register_function(global_func);

        let methods = registry.get_methods(player_type);
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&FunctionId::new(0)));
        assert!(methods.contains(&FunctionId::new(1)));
        assert!(!methods.contains(&FunctionId::new(2)));
    }

    #[test]
    fn registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.types.len(), 32);
    }

    #[test]
    fn registry_clone() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register a constructor: Vector3(int, int, int)
        let int_type = DataType::simple(INT32_TYPE);
        let func_def = FunctionDef {
            id: FunctionId(0), // Will be reassigned by register_function
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![int_type.clone(), int_type.clone(), int_type.clone()],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
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
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register constructor: Vector3(int, int, int)
        let int_type = DataType::simple(INT32_TYPE);
        let func_def = FunctionDef {
            id: FunctionId(0),
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![int_type.clone(), int_type.clone(), int_type.clone()],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        registry.register_function(func_def);

        // Try to find constructor with different args (float, float, float)
        let float_type = DataType::simple(FLOAT_TYPE);
        let found = registry.find_constructor(
            type_id,
            &[float_type.clone(), float_type.clone(), float_type.clone()],
        );

        assert!(found.is_none());
    }

    #[test]
    fn is_constructor_explicit() {
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        // Register explicit constructor: Vector3(int) explicit
        let int_type = DataType::simple(INT32_TYPE);
        let func_def = FunctionDef {
            id: FunctionId(0),
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![int_type.clone()],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let func_id = registry.register_function(func_def);

        // Check if constructor is explicit
        assert!(registry.is_constructor_explicit(func_id));
    }

    #[test]
    fn find_all_constructors() {
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Vector3".to_string(),
            qualified_name: "Vector3".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Vector3"));

        let int_type = DataType::simple(INT32_TYPE);

        // Register default constructor
        let func_def1 = FunctionDef {
            id: FunctionId(0),
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: Vec::new(),
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        // Register single-param constructor
        let func_def2 = FunctionDef {
            id: FunctionId(0),
            name: "Vector3".to_string(),
            namespace: Vec::new(),
            params: vec![int_type.clone()],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
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
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create copy constructor with &in: Player(const Player&in)
        let copy_ctor_param = DataType::with_ref_in(type_id);
        let copy_ctor = FunctionDef {
            id: FunctionId(0),
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![copy_ctor_param],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
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
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create copy constructor with &inout: Player(const Player&inout)
        let copy_ctor_param = DataType::with_ref_inout(type_id);
        let copy_ctor = FunctionDef {
            id: FunctionId(0),
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![copy_ctor_param],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
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
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with two parameters (not a copy constructor)
        let param1 = DataType::with_ref_in(type_id);
        let param2 = DataType::simple(INT32_TYPE);
        let ctor = FunctionDef {
            id: FunctionId(0),
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param1, param2],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_not_found_wrong_ref_modifier() {
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with &out (wrong for copy constructor)
        let param = DataType::with_ref_out(type_id);
        let ctor = FunctionDef {
            id: FunctionId(0),
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_not_found_wrong_type() {
        let mut registry = Registry::new();

        // Register a class
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Create constructor with different type parameter (not same class)
        let param = DataType::with_ref_in(INT32_TYPE);
        let ctor = FunctionDef {
            id: FunctionId(0),
            name: "Player".to_string(),
            namespace: Vec::new(),
            params: vec![param],
            return_type: DataType::simple(registry.void_type),
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
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };

        let ctor_id = registry.register_function(ctor);
        registry.add_method_to_class(type_id, ctor_id);

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn find_copy_constructor_no_constructors() {
        let mut registry = Registry::new();

        // Register a class with no constructors
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("Player"));

        // Should NOT find a copy constructor
        let found = registry.find_copy_constructor(type_id);
        assert_eq!(found, None);
    }

    #[test]
    fn get_all_methods_with_inheritance() {
        let mut registry = Registry::new();

        // Create base class with a method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
            base_class: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![FunctionId::new(100)], // base method
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class with a method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
            base_class: Some(base_id),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: vec![FunctionId::new(200)], // derived method
            operator_methods: rustc_hash::FxHashMap::default(),
            properties: rustc_hash::FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
        };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Get all methods for derived class
        let all_methods = registry.get_all_methods(derived_id);

        // Should have both derived and base methods
        assert_eq!(all_methods.len(), 2);
        assert!(all_methods.contains(&FunctionId::new(200))); // derived
        assert!(all_methods.contains(&FunctionId::new(100))); // base
    }

    #[test]
    fn get_all_properties_with_inheritance() {
        let mut registry = Registry::new();

        // Create base class with a property
        let mut base_props = rustc_hash::FxHashMap::default();
        base_props.insert(
            "health".to_string(),
            PropertyAccessors::read_only(FunctionId::new(100)),
        );

        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
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
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class with a property
        let mut derived_props = rustc_hash::FxHashMap::default();
        derived_props.insert(
            "score".to_string(),
            PropertyAccessors::read_write(FunctionId::new(200), FunctionId::new(201)),
        );

        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
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
        let mut registry = Registry::new();

        // Register the base method first (gets FunctionId(0))
        let base_method = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None, // Set later
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with the method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
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
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class WITHOUT overriding the method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
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
        };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Should find base class method
        let found = registry.find_method(derived_id, "foo");
        assert_eq!(found, Some(base_method_id));
    }

    #[test]
    fn find_method_returns_most_derived() {
        let mut registry = Registry::new();

        // Register the base method
        let base_method = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with the method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
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
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Register the derived method (same name, overrides base)
        let derived_method = FunctionDef {
            id: FunctionId::new(1),
            name: "foo".to_string(),
            namespace: vec!["Derived".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };
        let derived_method_id = registry.register_function(derived_method);

        // Create derived class that OVERRIDES the method
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
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
        let mut registry = Registry::new();

        // Register the base method
        let base_method = FunctionDef {
            id: FunctionId::new(0),
            name: "foo".to_string(),
            namespace: vec!["Base".to_string()],
            params: Vec::new(),
            return_type: DataType::simple(VOID_TYPE),
            object_type: None,
            traits: FunctionTraits::new(),
            is_native: false,
            default_args: Vec::new(),
            visibility: Visibility::Public,
            signature_filled: true,
        };
        let base_method_id = registry.register_function(base_method);

        // Create base class with method
        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
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
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create middle class (no override)
        let middle_typedef = TypeDef::Class {
            name: "Middle".to_string(),
            qualified_name: "Middle".to_string(),
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
        };
        let middle_id = registry.register_type(middle_typedef, Some("Middle"));

        // Create most derived class (no override)
        let most_derived_typedef = TypeDef::Class {
            name: "MostDerived".to_string(),
            qualified_name: "MostDerived".to_string(),
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
        };
        let most_derived_id = registry.register_type(most_derived_typedef, Some("MostDerived"));

        // Should walk through Middle to Base and find method
        let found = registry.find_method(most_derived_id, "foo");
        assert_eq!(found, Some(base_method_id));
    }

    #[test]
    fn find_method_not_found_returns_none() {
        let mut registry = Registry::new();

        let typedef = TypeDef::Class {
            name: "MyClass".to_string(),
            qualified_name: "MyClass".to_string(),
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
        };
        let type_id = registry.register_type(typedef, Some("MyClass"));

        // Should return None for non-existent method
        let found = registry.find_method(type_id, "nonexistent");
        assert_eq!(found, None);
    }

    #[test]
    fn find_property_in_base_class() {
        let mut registry = Registry::new();

        // Create base class with a property
        let mut base_props = rustc_hash::FxHashMap::default();
        base_props.insert(
            "health".to_string(),
            PropertyAccessors::read_only(FunctionId::new(100)),
        );

        let base_typedef = TypeDef::Class {
            name: "Base".to_string(),
            qualified_name: "Base".to_string(),
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
        };
        let base_id = registry.register_type(base_typedef, Some("Base"));

        // Create derived class without that property
        let derived_typedef = TypeDef::Class {
            name: "Derived".to_string(),
            qualified_name: "Derived".to_string(),
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
        };
        let derived_id = registry.register_type(derived_typedef, Some("Derived"));

        // Should find property from base class
        let found = registry.find_property(derived_id, "health");
        assert!(found.is_some());
        assert!(found.unwrap().is_read_only());
    }

    // =========================================================================
    // FFI Import Tests
    // =========================================================================

    #[test]
    fn import_empty_modules() {
        let mut registry = Registry::new();
        let modules: Vec<Module<'_>> = Vec::new();

        let result = registry.import_modules(&modules);
        assert!(result.is_ok());
    }

    #[test]
    fn import_module_with_enum() {
        let mut registry = Registry::new();
        let mut module = Module::root();

        module
            .register_enum("Color")
            .value("Red", 0)
            .unwrap()
            .value("Green", 1)
            .unwrap()
            .value("Blue", 2)
            .unwrap()
            .build()
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        // Verify the enum was registered
        let type_id = registry.lookup_type("Color");
        assert!(type_id.is_some());

        let typedef = registry.get_type(type_id.unwrap());
        assert!(matches!(typedef, TypeDef::Enum { .. }));

        if let TypeDef::Enum { name, values, .. } = typedef {
            assert_eq!(name, "Color");
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], ("Red".to_string(), 0));
            assert_eq!(values[1], ("Green".to_string(), 1));
            assert_eq!(values[2], ("Blue".to_string(), 2));
        }
    }

    #[test]
    fn import_module_with_namespace() {
        let mut registry = Registry::new();
        let mut module = Module::new(&["game"]);

        module
            .register_enum("Direction")
            .auto_value("North")
            .unwrap()
            .auto_value("South")
            .unwrap()
            .build()
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        // Should be registered with qualified name
        let type_id = registry.lookup_type("game::Direction");
        assert!(type_id.is_some());

        // Unqualified lookup should not find it
        let unqualified = registry.lookup_type("Direction");
        assert!(unqualified.is_none());
    }

    #[test]
    fn import_module_with_interface() {
        let mut registry = Registry::new();
        let mut module = Module::root();

        module
            .register_interface("IDrawable")
            .method("void draw()")
            .unwrap()
            .method("int getWidth() const")
            .unwrap()
            .build()
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        let type_id = registry.lookup_type("IDrawable");
        assert!(type_id.is_some());

        let typedef = registry.get_type(type_id.unwrap());
        assert!(matches!(typedef, TypeDef::Interface { .. }));

        if let TypeDef::Interface { methods, .. } = typedef {
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "draw");
            assert_eq!(methods[1].name, "getWidth");
        }
    }

    #[test]
    fn import_module_with_funcdef() {
        let mut registry = Registry::new();
        let mut module = Module::root();

        module.register_funcdef("funcdef void Callback()").unwrap();
        module
            .register_funcdef("funcdef int Predicate(int value)")
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        let callback_id = registry.lookup_type("Callback");
        assert!(callback_id.is_some());

        let typedef = registry.get_type(callback_id.unwrap());
        if let TypeDef::Funcdef {
            name,
            params,
            return_type,
            ..
        } = typedef
        {
            assert_eq!(name, "Callback");
            assert!(params.is_empty());
            assert_eq!(return_type.type_id, VOID_TYPE);
        } else {
            panic!("Expected Funcdef");
        }

        let predicate_id = registry.lookup_type("Predicate");
        assert!(predicate_id.is_some());

        let typedef = registry.get_type(predicate_id.unwrap());
        if let TypeDef::Funcdef {
            name,
            params,
            return_type,
            ..
        } = typedef
        {
            assert_eq!(name, "Predicate");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].type_id, INT32_TYPE);
            assert_eq!(return_type.type_id, INT32_TYPE);
        } else {
            panic!("Expected Funcdef");
        }
    }

    #[test]
    fn import_module_with_global_function() {
        let mut registry = Registry::new();
        let mut module = Module::root();

        module
            .register_fn("int add(int a, int b)", |a: i32, b: i32| a + b)
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        let func_ids = registry.lookup_functions("add");
        assert_eq!(func_ids.len(), 1);

        let func = registry.get_function(func_ids[0]);
        assert_eq!(func.name, "add");
        assert!(func.is_native);
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.return_type.type_id, INT32_TYPE);
    }

    #[test]
    fn import_module_with_global_property() {
        let mut registry = Registry::new();
        let mut module = Module::root();

        let mut score: i32 = 0;
        module
            .register_global_property("int g_score", &mut score)
            .unwrap();

        let result = registry.import_modules(&[module]);
        assert!(result.is_ok());

        let var = registry.lookup_global_var("g_score");
        assert!(var.is_some());

        let var_def = var.unwrap();
        assert_eq!(var_def.name, "g_score");
        assert_eq!(var_def.data_type.type_id, INT32_TYPE);
    }

    #[test]
    fn import_multiple_modules() {
        let mut registry = Registry::new();

        let mut math = Module::new(&["math"]);
        math.register_enum("MathConst")
            .value("PI", 314)
            .unwrap()
            .value("E", 271)
            .unwrap()
            .build()
            .unwrap();

        let mut io = Module::new(&["io"]);
        io.register_enum("FileMode")
            .value("Read", 1)
            .unwrap()
            .value("Write", 2)
            .unwrap()
            .build()
            .unwrap();

        let result = registry.import_modules(&[math, io]);
        assert!(result.is_ok());

        assert!(registry.lookup_type("math::MathConst").is_some());
        assert!(registry.lookup_type("io::FileMode").is_some());
    }

    #[test]
    fn import_duplicate_type_error() {
        let mut registry = Registry::new();

        let mut module1 = Module::root();
        module1
            .register_enum("Color")
            .value("Red", 0)
            .unwrap()
            .build()
            .unwrap();

        // First import should succeed
        let result1 = registry.import_modules(&[module1]);
        assert!(result1.is_ok());

        // Second import of same type should fail
        let mut module2 = Module::root();
        module2
            .register_enum("Color")
            .value("Blue", 1)
            .unwrap()
            .build()
            .unwrap();

        let result2 = registry.import_modules(&[module2]);
        assert!(result2.is_err());
        assert!(matches!(result2, Err(ImportError::DuplicateType(_))));
    }

    #[test]
    fn import_error_display() {
        let err = ImportError::TypeNotFound("MyType".to_string());
        assert!(err.to_string().contains("type not found"));
        assert!(err.to_string().contains("MyType"));

        let err = ImportError::DuplicateType("Color".to_string());
        assert!(err.to_string().contains("duplicate type"));
        assert!(err.to_string().contains("Color"));

        let err = ImportError::TypeResolutionFailed {
            type_name: "Foo".to_string(),
            reason: "not found".to_string(),
        };
        assert!(err.to_string().contains("type resolution failed"));
        assert!(err.to_string().contains("Foo"));
    }
}
