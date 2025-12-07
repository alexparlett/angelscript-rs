//! ClassBuilder for registering native types with the FFI system.
//!
//! ClassBuilder provides a fluent API for registering native Rust types
//! (value types, reference types, and templates) with constructors, methods,
//! properties, operators, and behaviors.
//!
//! # Example
//!
//! ```ignore
//! // Value type
//! module.register_type::<Vec3>("Vec3")
//!     .value_type()
//!     .constructor("void f()", || Vec3::default())?
//!     .constructor("void f(float x, float y, float z)", Vec3::new)?
//!     .method("float length() const", |v: &Vec3| v.length())?
//!     .property("float x", |v| v.x, |v, x| v.x = x)?
//!     .operator("Vec3 opAdd(const Vec3 &in)", |a, b| *a + *b)?
//!     .build()?;
//!
//! // Reference type
//! module.register_type::<Entity>("Entity")
//!     .reference_type()
//!     .factory("Entity@ f()", || Entity::new())?
//!     .addref(Entity::add_ref)
//!     .release(Entity::release)
//!     .method("string getName() const", |e| e.name.clone())?
//!     .build()?;
//!
//! // Template type
//! module.register_type::<ScriptArray>("array<class T>")
//!     .reference_type()
//!     .template_callback(|info| TemplateValidation::valid())?
//!     .factory("array<T>@ f()", || ScriptArray::new())?
//!     .method("void insertLast(const T &in)", array_insert_last)?
//!     .build()?;
//! ```

use std::marker::PhantomData;
use std::sync::Arc;

use angelscript_parser::ast::Parser;
use angelscript_core::{Param, ReferenceKind};
use angelscript_ffi::{
    type_expr_to_data_type, function_param_to_ffi, return_type_to_data_type,
    FfiPropertyDef, FfiTypeDef, TypeKind,
    ListPattern, ListBehavior, TemplateInstanceInfo, TemplateValidation,
    CallContext, NativeCallable, NativeFn,
    FromScript, IntoNativeFn, NativeType, ToScript,
};

use crate::{RegistrationError, FunctionBuilder, Module};

/// Builder for registering native types with the FFI system.
///
/// Created by calling `Module::register_type::<T>(name)`.
///
/// # Type Parameters
///
/// - `'m`: Lifetime of the mutable borrow of the Module
/// - `'app`: Application lifetime for global property references
/// - `T`: The Rust type being registered (must implement `NativeType`)
pub struct ClassBuilder<'m, 'app, T: NativeType> {
    /// Reference to the module where the type will be registered
    module: &'m mut Module<'app>,
    /// Type name (base name without template params)
    name: String,
    /// Template parameter names (for template types like array<T>)
    template_params: Vec<String>,
    /// Type kind (value or reference)
    type_kind: TypeKind,

    // === Behaviors ===

    /// Constructors (for value types)
    constructors: Vec<FunctionBuilder>,
    /// Factory functions (for reference types)
    factories: Vec<FunctionBuilder>,
    /// AddRef behavior
    addref: Option<NativeFn>,
    /// Release behavior
    release: Option<NativeFn>,
    /// Destructor behavior
    destruct: Option<NativeFn>,
    /// List constructor behavior
    list_construct: Option<ListBehavior>,
    /// List factory behavior
    list_factory: Option<ListBehavior>,
    /// Get weak ref flag behavior
    get_weakref_flag: Option<NativeFn>,
    /// Template callback
    template_callback:
        Option<Arc<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,

    // === Type members ===

    /// Methods
    methods: Vec<FunctionBuilder>,
    /// Properties
    properties: Vec<FfiPropertyDef>,
    /// Operators
    operators: Vec<FunctionBuilder>,
    /// Marker for the type parameter
    _marker: PhantomData<T>,
}

impl<'m, 'app, T: NativeType> ClassBuilder<'m, 'app, T> {
    /// Create a new ClassBuilder for the given type.
    ///
    /// This is called internally by `Module::register_type()`.
    pub(crate) fn new(
        module: &'m mut Module<'app>,
        name: String,
        template_params: Vec<String>,
    ) -> Self {
        Self {
            module,
            name,
            template_params,
            type_kind: TypeKind::value::<T>(), // Default to value type
            constructors: Vec::new(),
            factories: Vec::new(),
            addref: None,
            release: None,
            destruct: None,
            list_construct: None,
            list_factory: None,
            get_weakref_flag: None,
            template_callback: None,
            methods: Vec::new(),
            properties: Vec::new(),
            operators: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Mark this type as a value type (the default).
    ///
    /// Value types are stack-allocated and copied on assignment.
    /// They require constructors and optionally destructors.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .constructor("void f()", || Vec3::default())?
    ///     .build()?;
    /// ```
    pub fn value_type(mut self) -> Self {
        self.type_kind = TypeKind::value::<T>();
        self
    }

    /// Mark this type as a POD (Plain Old Data) value type.
    ///
    /// POD types can be memcpy'd and don't need constructors/destructors.
    pub fn pod_type(mut self) -> Self {
        self.type_kind = TypeKind::pod::<T>();
        self
    }

    /// Mark this type as a reference type with standard ref counting.
    ///
    /// Reference types are heap-allocated via factories and use handle semantics.
    /// They require factory, addref, and release behaviors.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Entity>("Entity")
    ///     .reference_type()
    ///     .factory("Entity@ f()", || Entity::new())?
    ///     .addref(Entity::add_ref)
    ///     .release(Entity::release)
    ///     .build()?;
    /// ```
    pub fn reference_type(mut self) -> Self {
        self.type_kind = TypeKind::Reference {
            kind: ReferenceKind::Standard,
        };
        self
    }

    /// Mark this type as a scoped reference type.
    ///
    /// Scoped types are RAII-style and destroyed at scope exit.
    /// They don't support handles in script code.
    pub fn scoped_type(mut self) -> Self {
        self.type_kind = TypeKind::Reference {
            kind: ReferenceKind::Scoped,
        };
        self
    }

    /// Mark this type as a single-ref type.
    ///
    /// Single-ref types have app-controlled lifetime and no handles in script.
    pub fn single_ref_type(mut self) -> Self {
        self.type_kind = TypeKind::Reference {
            kind: ReferenceKind::SingleRef,
        };
        self
    }

    /// Register a template validation callback.
    ///
    /// This callback is called when the template is instantiated with specific
    /// type arguments (e.g., `array<int>`). Return `TemplateValidation::valid()`
    /// to accept the instantiation, or `TemplateValidation::invalid(msg)` to reject.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<ScriptDict>("dictionary<class K, class V>")
    ///     .reference_type()
    ///     .template_callback(|info| {
    ///         if is_hashable(&info.sub_types[0]) {
    ///             TemplateValidation::valid()
    ///         } else {
    ///             TemplateValidation::invalid("Key must be hashable")
    ///         }
    ///     })?
    ///     .build()?;
    /// ```
    pub fn template_callback<F>(mut self, f: F) -> Self
    where
        F: Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync + 'static,
    {
        self.template_callback = Some(Arc::new(f));
        self
    }

    /// Register a constructor for value types.
    ///
    /// The declaration should be in the form `"void f(params)"`.
    ///
    /// # Parameters
    ///
    /// - `decl`: Constructor declaration (e.g., `"void f()"`, `"void f(float x, float y)"`)
    /// - `f`: The constructor function
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .constructor("void f()", || Vec3::default())?
    ///     .constructor("void f(float x, float y, float z)", Vec3::new)?
    ///     .build()?;
    /// ```
    pub fn constructor<F, Args>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: IntoNativeFn<Args, T>,
    {
        let method_def = self.parse_method_decl(decl, f.into_native_fn())?;
        self.constructors.push(method_def);
        Ok(self)
    }

    /// Register a factory function for reference types.
    ///
    /// The declaration should return a handle: `"T@ f(params)"`.
    ///
    /// # Parameters
    ///
    /// - `decl`: Factory declaration (e.g., `"T@ f()"`, `"T@ f(const string &in name)"`)
    /// - `f`: The factory function
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Entity>("Entity")
    ///     .reference_type()
    ///     .factory("Entity@ f()", || Entity::new())?
    ///     .factory("Entity@ f(const string &in name)", Entity::with_name)?
    ///     .build()?;
    /// ```
    pub fn factory<F, Args>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: IntoNativeFn<Args, T>,
    {
        let method_def = self.parse_method_decl(decl, f.into_native_fn())?;
        self.factories.push(method_def);
        Ok(self)
    }

    /// Register a factory with raw CallContext access.
    ///
    /// Use this for factory functions that need full control over argument handling,
    /// such as template types that need access to type information.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<ScriptArray>("array<class T>")
    ///     .reference_type()
    ///     .factory_raw("array<T>@ f()", |ctx| {
    ///         // VM pushes element TypeHash as first argument
    ///         let element_type: TypeHash = ctx.arg(0)?;
    ///         let arr = ScriptArray::new(element_type);
    ///         ctx.set_return(arr)?;
    ///         Ok(())
    ///     })?
    ///     .build()?;
    /// ```
    pub fn factory_raw<F>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        let method_def = self.parse_method_decl(decl, NativeFn::new(f))?;
        self.factories.push(method_def);
        Ok(self)
    }

    /// Register the AddRef behavior for reference types.
    ///
    /// Called when a new handle reference is created.
    pub fn addref<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.addref = Some(NativeFn::new(move |ctx: &mut CallContext| {
            let this: &T = ctx.this()?;
            f(this);
            Ok(())
        }));
        self
    }

    /// Register the Release behavior for reference types.
    ///
    /// Called when a handle reference is released.
    pub fn release<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) + Send + Sync + 'static,
    {
        self.release = Some(NativeFn::new(move |ctx: &mut CallContext| {
            let this: &T = ctx.this()?;
            f(this);
            Ok(())
        }));
        self
    }

    /// Register the destructor for value types.
    ///
    /// Called when a value type instance goes out of scope.
    pub fn destructor<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut T) + Send + Sync + 'static,
    {
        self.destruct = Some(NativeFn::new(move |ctx: &mut CallContext| {
            let this: &mut T = ctx.this_mut()?;
            f(this);
            Ok(())
        }));
        self
    }

    /// Register a list constructor for value types.
    ///
    /// Enables initialization list syntax: `MyStruct s = {1, 2, 3};`
    ///
    /// # Parameters
    ///
    /// - `pattern`: The expected list pattern (e.g., `ListPattern::repeat(INT_TYPE)`)
    /// - `f`: Native function that receives the list data via `CallContext`
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<MyStruct>("MyStruct")
    ///     .value_type()
    ///     .list_construct(ListPattern::fixed(vec![INT_TYPE, STRING_TYPE]), |ctx| {
    ///         let int_val: i32 = ctx.arg(0)?;
    ///         let str_val: String = ctx.arg(1)?;
    ///         // Construct the value...
    ///         Ok(())
    ///     })?
    ///     .build()?;
    /// ```
    pub fn list_construct<F>(mut self, pattern: ListPattern, f: F) -> Self
    where
        F: Fn(&mut CallContext) -> Result<(), angelscript_ffi::NativeError> + Send + Sync + 'static,
    {
        self.list_construct = Some(ListBehavior {
            native_fn: NativeFn::new(f),
            pattern,
        });
        self
    }

    /// Register a list factory for reference types.
    ///
    /// Enables initialization list syntax: `array<int> a = {1, 2, 3};`
    ///
    /// # Parameters
    ///
    /// - `pattern`: The expected list pattern (e.g., `ListPattern::repeat(TYPE_ID)`)
    /// - `f`: Native function that receives the list data via `CallContext`
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<ScriptArray>("array<class T>")
    ///     .reference_type()
    ///     .list_factory(ListPattern::repeat(TypeHash(0)), |ctx| {
    ///         // 0 is a placeholder - actual type comes from template instantiation
    ///         // Build array from list buffer...
    ///         Ok(())
    ///     })?
    ///     .build()?;
    /// ```
    pub fn list_factory<F>(mut self, pattern: ListPattern, f: F) -> Self
    where
        F: Fn(&mut CallContext) -> Result<(), angelscript_ffi::NativeError> + Send + Sync + 'static,
    {
        self.list_factory = Some(ListBehavior {
            native_fn: NativeFn::new(f),
            pattern,
        });
        self
    }

    /// Register a method that takes `&self` (shared borrow).
    ///
    /// The `is_const` flag is determined from the declaration string
    /// (presence of `const` keyword, e.g., `"float length() const"`).
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .method("float length() const", |v: &Vec3| v.length())?
    ///     .method("float dot(const Vec3 &in) const", |v: &Vec3, other: ???| v.dot(other))?
    ///     .build()?;
    /// ```
    pub fn method<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: IntoNativeFn<Args, Ret>,
    {
        let method_def = self.parse_method_decl(decl, f.into_native_fn())?;
        self.methods.push(method_def);
        Ok(self)
    }

    /// Register a method that takes `&mut self` (exclusive borrow).
    ///
    /// The `is_const` flag is determined from the declaration string.
    /// Use this for methods that need to mutate the object.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .method_mut("void normalize()", |v: &mut Vec3| v.normalize())?
    ///     .build()?;
    /// ```
    pub fn method_mut<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: IntoNativeFn<Args, Ret>,
    {
        let method_def = self.parse_method_decl(decl, f.into_native_fn())?;
        self.methods.push(method_def);
        Ok(self)
    }

    /// Register a method with raw CallContext access.
    ///
    /// Use this for methods that need full control over argument handling,
    /// such as methods with `?&` (any type) parameters.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Formatter>("Formatter")
    ///     .value_type()
    ///     .method_raw("void format(?&in value)", |ctx| {
    ///         let this: &Formatter = ctx.this()?;
    ///         let value = ctx.arg_any(0)?;
    ///         // ...
    ///         Ok(())
    ///     })?
    ///     .build()?;
    /// ```
    pub fn method_raw<F>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        let method_def = self.parse_method_decl(decl, NativeFn::new(f))?;
        self.methods.push(method_def);
        Ok(self)
    }

    /// Register a read-only property.
    ///
    /// The declaration format is `"Type name"` (e.g., `"float lengthSq"`).
    ///
    /// # Parameters
    ///
    /// - `decl`: Property declaration
    /// - `getter`: Function to get the property value
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .property_get("float lengthSq", |v| v.length_squared())?
    ///     .build()?;
    /// ```
    pub fn property_get<V, F>(mut self, decl: &str, getter: F) -> Result<Self, RegistrationError>
    where
        V: ToScript + 'static,
        F: Fn(&T) -> V + Send + Sync + 'static,
    {
        let prop_def = self.parse_readonly_property_decl(decl, getter)?;
        self.properties.push(prop_def);
        Ok(self)
    }

    /// Register a read-write property.
    ///
    /// The declaration format is `"Type name"` (e.g., `"float x"`).
    ///
    /// # Parameters
    ///
    /// - `decl`: Property declaration
    /// - `getter`: Function to get the property value
    /// - `setter`: Function to set the property value
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .property("float x", |v| v.x, |v, x| v.x = x)?
    ///     .property("float y", |v| v.y, |v, y| v.y = y)?
    ///     .build()?;
    /// ```
    pub fn property<V, G, S>(
        mut self,
        decl: &str,
        getter: G,
        setter: S,
    ) -> Result<Self, RegistrationError>
    where
        V: ToScript + FromScript + 'static,
        G: Fn(&T) -> V + Send + Sync + 'static,
        S: Fn(&mut T, V) + Send + Sync + 'static,
    {
        let prop_def = self.parse_readwrite_property_decl(decl, getter, setter)?;
        self.properties.push(prop_def);
        Ok(self)
    }

    /// Register an operator.
    ///
    /// The declaration format is `"ReturnType opName(params)"`.
    ///
    /// Common operators:
    /// - `opAdd`, `opSub`, `opMul`, `opDiv` - Arithmetic
    /// - `opEquals`, `opCmp` - Comparison
    /// - `opIndex` - Array indexing
    /// - `opAssign` - Assignment
    /// - `opNeg`, `opPreInc`, `opPreDec` - Unary
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .operator("Vec3 opAdd(const Vec3 &in)", |a, b| *a + *b)?
    ///     .operator("Vec3 opSub(const Vec3 &in)", |a, b| *a - *b)?
    ///     .operator("bool opEquals(const Vec3 &in)", |a, b| a == b)?
    ///     .build()?;
    /// ```
    pub fn operator<F, Args, Ret>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: IntoNativeFn<Args, Ret>,
    {
        use angelscript_core::OperatorBehavior;

        let mut operator_def = self.parse_method_decl(decl, f.into_native_fn())?;

        // Auto-detect operator behavior from method name
        if let Some(operator) = OperatorBehavior::from_method_name(&operator_def.name, None) {
            operator_def = operator_def.with_operator(operator);
        }

        self.operators.push(operator_def);
        Ok(self)
    }

    /// Register an operator with raw call context access.
    ///
    /// Similar to `operator()`, but uses a raw callback with full `CallContext` access.
    /// This is useful for template types or complex operators that need direct context access.
    ///
    /// # Example
    ///
    /// ```ignore
    /// module.register_type::<ScriptArray>("array<class T>")
    ///     .reference_type()
    ///     .operator_raw("T &opIndex(uint index)", |ctx: &mut CallContext| {
    ///         let index: u32 = ctx.arg(0)?;
    ///         let arr: &ScriptArray = ctx.this()?;
    ///         // ... access element
    ///         Ok(())
    ///     })?
    ///     .build()?;
    /// ```
    pub fn operator_raw<F>(mut self, decl: &str, f: F) -> Result<Self, RegistrationError>
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        use angelscript_core::OperatorBehavior;

        let mut operator_def = self.parse_method_decl(decl, NativeFn::new(f))?;

        // Auto-detect operator behavior from method name
        if let Some(operator) = OperatorBehavior::from_method_name(&operator_def.name, None) {
            operator_def = operator_def.with_operator(operator);
        }

        self.operators.push(operator_def);
        Ok(self)
    }

    /// Finish building and register the type with the module.
    ///
    /// This consumes the builder and adds the type definition to the module.
    ///
    /// # Errors
    ///
    /// Returns an error if the type configuration is invalid (e.g., reference type
    /// without factory, value type without constructor).
    pub fn build(self) -> Result<(), RegistrationError> {
        // Compute the qualified name and type hash
        let qualified_name = self.module.qualified_name(&self.name);
        let type_hash = angelscript_core::TypeHash::from_name(&qualified_name);

        // Build the FfiTypeDef
        let mut type_def = FfiTypeDef::new::<T>(type_hash, self.name, self.type_kind);

        // Set template params
        type_def.template_params = self.template_params;

        // Set owner_type on all constructors and recompute their hashes
        for mut ctor in self.constructors {
            ctor.owner_type = Some(type_hash);
            ctor.recompute_hash_pub();
            let func_hash = ctor.func_hash;
            if let Some(native_fn) = ctor.native_fn.take() {
                type_def.native_fns.insert(func_hash, native_fn);
            }
            type_def.constructors.push(ctor.build());
        }

        // Set owner_type on all factories and recompute their hashes
        for mut factory in self.factories {
            factory.owner_type = Some(type_hash);
            factory.recompute_hash_pub();
            let func_hash = factory.func_hash;
            if let Some(native_fn) = factory.native_fn.take() {
                type_def.native_fns.insert(func_hash, native_fn);
            }
            type_def.factories.push(factory.build());
        }

        // Set behaviors
        type_def.addref = self.addref;
        type_def.release = self.release;
        type_def.destruct = self.destruct;
        type_def.list_construct = self.list_construct;
        type_def.list_factory = self.list_factory;
        type_def.get_weakref_flag = self.get_weakref_flag;
        type_def.template_callback = self.template_callback;

        // Set owner_type on all methods and recompute their hashes
        for mut method in self.methods {
            method.owner_type = Some(type_hash);
            method.recompute_hash_pub();
            let func_hash = method.func_hash;
            if let Some(native_fn) = method.native_fn.take() {
                type_def.native_fns.insert(func_hash, native_fn);
            }
            type_def.methods.push(method.build());
        }

        type_def.properties = self.properties;

        // Set owner_type on all operators and recompute their hashes
        for mut op in self.operators {
            op.owner_type = Some(type_hash);
            op.recompute_hash_pub();
            let func_hash = op.func_hash;
            if let Some(native_fn) = op.native_fn.take() {
                type_def.native_fns.insert(func_hash, native_fn);
            }
            if let Some(behavior) = op.operator {
                type_def.operator_behaviors.insert(func_hash, behavior);
            }
            type_def.operators.push(op.build());
        }

        // Add to module
        self.module.add_type(type_def);
        Ok(())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Parse a method declaration and convert to FunctionBuilder.
    fn parse_method_decl(&self, decl: &str, native_fn: NativeFn) -> Result<FunctionBuilder, RegistrationError> {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(RegistrationError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let sig = Parser::function_decl(decl, self.module.arena()).map_err(|errors| {
            RegistrationError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Convert params
        let params: Vec<Param> = sig.params.iter().map(function_param_to_ffi).collect();

        // Convert return type
        let return_type = return_type_to_data_type(&sig.return_type);

        // Build the function definition
        Ok(FunctionBuilder::new(sig.name.name.to_string())
            .with_params(params)
            .with_return_type(return_type)
            .with_const(sig.is_const)
            .with_native_fn(native_fn))
    }

    /// Parse a property declaration and convert to FfiPropertyDef.
    fn parse_readonly_property_decl<V, G>(
        &self,
        decl: &str,
        getter: G,
    ) -> Result<FfiPropertyDef, RegistrationError>
    where
        V: ToScript + 'static,
        G: Fn(&T) -> V + Send + Sync + 'static,
    {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(RegistrationError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let prop = Parser::property_decl(decl, self.module.arena()).map_err(|errors| {
            RegistrationError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Convert the type expression to DataType
        let data_type = type_expr_to_data_type(&prop.ty);

        // Build the getter function
        let getter_fn = NativeFn::new(move |ctx: &mut CallContext| {
            let this: &T = ctx.this()?;
            let value = getter(this);
            ctx.set_return(value)?;
            Ok(())
        });

        Ok(FfiPropertyDef::read_only(
            prop.name.name.to_string(),
            data_type,
            getter_fn,
        ))
    }

    /// Parse a property declaration and convert to FfiPropertyDef (read-write).
    fn parse_readwrite_property_decl<V, G, S>(
        &self,
        decl: &str,
        getter: G,
        setter: S,
    ) -> Result<FfiPropertyDef, RegistrationError>
    where
        V: ToScript + FromScript + 'static,
        G: Fn(&T) -> V + Send + Sync + 'static,
        S: Fn(&mut T, V) + Send + Sync + 'static,
    {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(RegistrationError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let prop = Parser::property_decl(decl, self.module.arena()).map_err(|errors| {
            RegistrationError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Convert the type expression to DataType
        let data_type = type_expr_to_data_type(&prop.ty);

        // Build the getter function
        let getter_fn = NativeFn::new(move |ctx: &mut CallContext| {
            let this: &T = ctx.this()?;
            let value = getter(this);
            ctx.set_return(value)?;
            Ok(())
        });

        // Build the setter function
        let setter_fn = NativeFn::new(move |ctx: &mut CallContext| {
            let value: V = ctx.arg(0)?;
            let this: &mut T = ctx.this_mut()?;
            setter(this, value);
            Ok(())
        });

        Ok(FfiPropertyDef::read_write(
            prop.name.name.to_string(),
            data_type,
            getter_fn,
            setter_fn,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_ffi::{ConversionError, ToScript, VmSlot};
    use angelscript_core::primitives;

    // Test type for unit tests
    struct TestVec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    impl NativeType for TestVec3 {
        const NAME: &'static str = "TestVec3";
    }

    // Implement ToScript so constructors can return TestVec3
    impl ToScript for TestVec3 {
        fn to_vm(self, slot: &mut VmSlot) -> Result<(), ConversionError> {
            // In a real implementation, this would allocate the object on a heap.
            // For testing, we just use Void since we're only testing the registration API.
            let _ = self; // consume self
            *slot = VmSlot::Void;
            Ok(())
        }
    }

    impl TestVec3 {
        fn new(x: f32, y: f32, z: f32) -> Self {
            Self { x, y, z }
        }

        fn length(&self) -> f32 {
            (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
        }
    }

    #[test]
    fn class_builder_value_type() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .build()
            .unwrap();

        assert_eq!(module.types().len(), 1);
        assert_eq!(module.types()[0].name, "TestVec3");
        assert!(module.types()[0].type_kind.is_value());
    }

    #[test]
    fn class_builder_reference_type() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .build()
            .unwrap();

        assert_eq!(module.types().len(), 1);
        assert!(module.types()[0].type_kind.is_reference());
    }

    #[test]
    fn class_builder_with_constructor() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .constructor("void f()", || TestVec3::new(0.0, 0.0, 0.0))
            .unwrap()
            .constructor("void f(float x, float y, float z)", TestVec3::new)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].constructors.len(), 2);
    }

    #[test]
    fn class_builder_with_method_raw() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_raw("float length() const", |ctx: &mut CallContext| {
                let this: &TestVec3 = ctx.this()?;
                let len = (this.x * this.x + this.y * this.y + this.z * this.z).sqrt();
                ctx.set_return(len)?;
                Ok(())
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert!(module.types()[0].methods[0].is_const());
    }

    #[test]
    fn class_builder_with_method() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method("float length() const", |v: &TestVec3| v.length())
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert!(module.types()[0].methods[0].is_const());
    }

    #[test]
    fn class_builder_with_method_mut() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_mut("void reset()", |v: &mut TestVec3| {
                v.x = 0.0;
                v.y = 0.0;
                v.z = 0.0;
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert!(!module.types()[0].methods[0].is_const());
    }

    #[test]
    fn class_builder_with_property() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property("float x", |v: &TestVec3| v.x, |v: &mut TestVec3, x| v.x = x)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].properties.len(), 1);
        assert!(!module.types()[0].properties[0].is_const);
        assert!(module.types()[0].properties[0].setter.is_some());
    }

    #[test]
    fn class_builder_with_property_get() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property_get("float lengthSq", |v: &TestVec3| {
                v.x * v.x + v.y * v.y + v.z * v.z
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].properties.len(), 1);
        assert!(module.types()[0].properties[0].setter.is_none());
    }

    #[test]
    fn class_builder_with_operator() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_raw("bool opEquals(const TestVec3 &in)", |ctx: &mut CallContext| {
                let other_x: f32 = ctx.arg(0)?;
                let other_y: f32 = ctx.arg(1)?;
                let other_z: f32 = ctx.arg(2)?;
                let this: &TestVec3 = ctx.this()?;
                let eq = this.x == other_x && this.y == other_y && this.z == other_z;
                ctx.set_return(eq)?;
                Ok(())
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
    }

    #[test]
    fn class_builder_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_raw("invalid declaration", |_ctx: &mut CallContext| Ok(()));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_raw("", |_ctx: &mut CallContext| Ok(()));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_template_callback() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .template_callback(|_| TemplateValidation::valid())
            .build()
            .unwrap();

        assert!(module.types()[0].template_callback.is_some());
    }

    #[test]
    fn class_builder_pod_type() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .pod_type()
            .build()
            .unwrap();

        assert!(module.types()[0].type_kind.is_pod());
    }

    #[test]
    fn class_builder_scoped_type() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .scoped_type()
            .build()
            .unwrap();

        assert!(module.types()[0].type_kind.is_reference());
        match &module.types()[0].type_kind {
            TypeKind::Reference { kind } => assert_eq!(*kind, ReferenceKind::Scoped),
            _ => panic!("Expected reference type"),
        }
    }

    #[test]
    fn class_builder_single_ref_type() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .single_ref_type()
            .build()
            .unwrap();

        assert!(module.types()[0].type_kind.is_reference());
        match &module.types()[0].type_kind {
            TypeKind::Reference { kind } => assert_eq!(*kind, ReferenceKind::SingleRef),
            _ => panic!("Expected reference type"),
        }
    }

    #[test]
    fn class_builder_with_factory() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .factory("TestVec3@ f()", || TestVec3::new(0.0, 0.0, 0.0))
            .unwrap()
            .factory("TestVec3@ f(float x, float y, float z)", TestVec3::new)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].factories.len(), 2);
    }

    #[test]
    fn class_builder_with_addref() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static ADDREF_COUNT: AtomicUsize = AtomicUsize::new(0);

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .addref(|_: &TestVec3| {
                ADDREF_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .build()
            .unwrap();

        assert!(module.types()[0].addref.is_some());
    }

    #[test]
    fn class_builder_with_release() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static RELEASE_COUNT: AtomicUsize = AtomicUsize::new(0);

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .release(|_: &TestVec3| {
                RELEASE_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .build()
            .unwrap();

        assert!(module.types()[0].release.is_some());
    }

    #[test]
    fn class_builder_with_destructor() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .destructor(|_: &mut TestVec3| {
                // cleanup logic would go here
            })
            .build()
            .unwrap();

        assert!(module.types()[0].destruct.is_some());
    }

    #[test]
    fn class_builder_with_operator_method() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .operator("bool opEquals(const TestVec3 &in)", |_a: &TestVec3, _b: f32| true)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].operators.len(), 1);
    }

    #[test]
    fn class_builder_method_with_args() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method("float dot(float x, float y, float z) const", |v: &TestVec3, x: f32, y: f32, z: f32| {
                v.x * x + v.y * y + v.z * z
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert_eq!(module.types()[0].methods[0].params.len(), 3);
    }

    #[test]
    fn class_builder_method_mut_with_args() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_mut("void set(float x, float y, float z)", |v: &mut TestVec3, x: f32, y: f32, z: f32| {
                v.x = x;
                v.y = y;
                v.z = z;
            })
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert_eq!(module.types()[0].methods[0].params.len(), 3);
    }

    #[test]
    fn class_builder_method_non_const_from_decl() {
        // Method without 'const' in declaration should have is_const = false
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method("float compute()", |v: &TestVec3| v.length())
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].methods.len(), 1);
        assert!(!module.types()[0].methods[0].is_const());
    }

    #[test]
    fn class_builder_multiple_properties() {
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property("float x", |v: &TestVec3| v.x, |v: &mut TestVec3, x| v.x = x)
            .unwrap()
            .property("float y", |v: &TestVec3| v.y, |v: &mut TestVec3, y| v.y = y)
            .unwrap()
            .property("float z", |v: &TestVec3| v.z, |v: &mut TestVec3, z| v.z = z)
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(module.types()[0].properties.len(), 3);
    }

    #[test]
    fn class_builder_full_reference_type() {
        // Test a complete reference type with factory, addref, release
        use std::sync::atomic::{AtomicUsize, Ordering};
        static REF_COUNT: AtomicUsize = AtomicUsize::new(0);

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .factory("TestVec3@ f()", || TestVec3::new(0.0, 0.0, 0.0))
            .unwrap()
            .addref(|_: &TestVec3| {
                REF_COUNT.fetch_add(1, Ordering::SeqCst);
            })
            .release(|_: &TestVec3| {
                REF_COUNT.fetch_sub(1, Ordering::SeqCst);
            })
            .method("float length() const", |v: &TestVec3| v.length())
            .unwrap()
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert!(ty.type_kind.is_reference());
        assert_eq!(ty.factories.len(), 1);
        assert!(ty.addref.is_some());
        assert!(ty.release.is_some());
        assert_eq!(ty.methods.len(), 1);
    }

    #[test]
    fn class_builder_full_value_type() {
        // Test a complete value type with constructor, destructor, methods, properties
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .constructor("void f()", || TestVec3::new(0.0, 0.0, 0.0))
            .unwrap()
            .constructor("void f(float x, float y, float z)", TestVec3::new)
            .unwrap()
            .destructor(|_: &mut TestVec3| {})
            .method("float length() const", |v: &TestVec3| v.length())
            .unwrap()
            .method_mut("void reset()", |v: &mut TestVec3| {
                v.x = 0.0;
                v.y = 0.0;
                v.z = 0.0;
            })
            .unwrap()
            .property("float x", |v: &TestVec3| v.x, |v: &mut TestVec3, x| v.x = x)
            .unwrap()
            .property_get("float lengthSq", |v: &TestVec3| v.x * v.x + v.y * v.y + v.z * v.z)
            .unwrap()
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert!(ty.type_kind.is_value());
        assert_eq!(ty.constructors.len(), 2);
        assert!(ty.destruct.is_some());
        assert_eq!(ty.methods.len(), 2);
        assert_eq!(ty.properties.len(), 2);
    }

    #[test]
    fn class_builder_constructor_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .constructor("not valid", || TestVec3::new(0.0, 0.0, 0.0));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_factory_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .factory("invalid", || TestVec3::new(0.0, 0.0, 0.0));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_property_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property("not a valid property", |v: &TestVec3| v.x, |v: &mut TestVec3, x| v.x = x);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_operator_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .operator("invalid", |_a: &TestVec3, _b: f32| true);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_template_single_param() {
        // Test template like array<class T>
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("array<class T>")
            .unwrap()
            .reference_type()
            .template_callback(|_| TemplateValidation::valid())
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert_eq!(ty.name, "array");
        assert!(!ty.template_params.is_empty());
        assert_eq!(ty.template_params.len(), 1);
        assert_eq!(ty.template_params[0], "T");
    }

    #[test]
    fn class_builder_template_multiple_params() {
        // Test template like dictionary<class K, class V>
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("dictionary<class K, class V>")
            .unwrap()
            .reference_type()
            .template_callback(|_| TemplateValidation::valid())
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert_eq!(ty.name, "dictionary");
        assert!(!ty.template_params.is_empty());
        assert_eq!(ty.template_params.len(), 2);
        assert_eq!(ty.template_params[0], "K");
        assert_eq!(ty.template_params[1], "V");
    }

    #[test]
    fn class_builder_template_with_concrete_type() {
        // Test template like stringmap<class T> where key is always string
        // This is like dict<string, class T> - only T is a template param
        let mut module = Module::root();
        module
            .register_type::<TestVec3>("stringmap<string, class T>")
            .unwrap()
            .reference_type()
            .template_callback(|_| TemplateValidation::valid())
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert_eq!(ty.name, "stringmap");
        assert!(!ty.template_params.is_empty());
        // Only "T" should be captured as a template param, not "string"
        assert_eq!(ty.template_params.len(), 1);
        assert_eq!(ty.template_params[0], "T");
    }

    #[test]
    fn class_builder_property_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property("", |v: &TestVec3| v.x, |v: &mut TestVec3, x| v.x = x);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_property_get_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property_get("", |v: &TestVec3| v.x);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_property_get_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .property_get("not a valid property declaration", |v: &TestVec3| v.x);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_method_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method("", |v: &TestVec3| v.length());

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_method_mut_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_mut("", |v: &mut TestVec3| {
                v.x = 0.0;
            });

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_method_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method("not valid syntax", |v: &TestVec3| v.length());

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_method_mut_invalid_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_mut("not valid syntax", |v: &mut TestVec3| {
                v.x = 0.0;
            });

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_constructor_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .constructor("", || TestVec3::new(0.0, 0.0, 0.0));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_factory_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .factory("", || TestVec3::new(0.0, 0.0, 0.0));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_operator_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .operator("", |_a: &TestVec3, _b: f32| true);

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_method_raw_empty_decl() {
        let mut module = Module::root();
        let result = module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .method_raw("", |_ctx: &mut CallContext| Ok(()));

        assert!(result.is_err());
    }

    #[test]
    fn class_builder_with_list_construct() {
        ;

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .value_type()
            .list_construct(
                ListPattern::fixed(vec![primitives::FLOAT, primitives::FLOAT, primitives::FLOAT]),
                |_ctx: &mut CallContext| Ok(()),
            )
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert!(ty.list_construct.is_some());
        assert!(ty.list_construct.is_some());

        let list_behavior = ty.list_construct.as_ref().unwrap();
        assert!(matches!(list_behavior.pattern, ListPattern::Fixed(_)));
    }

    #[test]
    fn class_builder_with_list_factory() {
        ;

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .list_factory(
                ListPattern::repeat(primitives::INT32),
                |_ctx: &mut CallContext| Ok(()),
            )
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert!(ty.list_factory.is_some());
        assert!(ty.list_factory.is_some());

        let list_behavior = ty.list_factory.as_ref().unwrap();
        assert!(matches!(list_behavior.pattern, ListPattern::Repeat(_)));
    }

    #[test]
    fn class_builder_with_list_factory_repeat_tuple() {
        ;

        let mut module = Module::root();
        module
            .register_type::<TestVec3>("TestVec3")
            .unwrap()
            .reference_type()
            .list_factory(
                ListPattern::repeat_tuple(vec![primitives::STRING, primitives::INT32]),
                |_ctx: &mut CallContext| Ok(()),
            )
            .build()
            .unwrap();

        let ty = &module.types()[0];
        assert!(ty.list_factory.is_some());

        let list_behavior = ty.list_factory.as_ref().unwrap();
        assert!(matches!(list_behavior.pattern, ListPattern::RepeatTuple(_)));
    }
}
