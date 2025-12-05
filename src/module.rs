//! Native module for FFI registration.
//!
//! A `Module` is a namespaced collection of native functions, types, and global
//! properties that can be registered with the scripting engine.
//!
//! # Example
//!
//! ```ignore
//! use angelscript::Module;
//!
//! // Create a module with a namespace
//! let mut math = Module::new(&["math"]);
//! math.register_fn("sqrt", |x: f64| x.sqrt());
//!
//! // Create a root namespace module (no prefix needed in scripts)
//! let mut globals = Module::root();
//! globals.register_fn("print", |s: &str| println!("{}", s));
//! ```

use bumpalo::Bump;
use thiserror::Error;

use crate::ast::{Parser, PropertyDecl, TypeBase};
use crate::ffi::{
    ClassBuilder, EnumBuilder, FfiRegistryBuilder, GlobalPropertyDef, InterfaceBuilder,
    IntoNativeFn, NativeCallable, NativeFn, NativeType,
};
use crate::semantic::types::type_def::TypeId;
use crate::types::{
    function_param_to_ffi, return_type_to_ffi, FfiDataType, FfiEnumDef, FfiFuncdefDef,
    FfiFunctionDef, FfiInterfaceDef, FfiParam, FfiTypeDef,
};

/// A namespaced collection of native functions, types, and global properties.
///
/// # Lifetimes
///
/// - `'app`: The application lifetime for global property value references
///
/// The module owns an arena for storing parsed AST nodes (types, identifiers).
///
/// # Namespaces
///
/// Modules can have namespaces that determine how script code accesses their contents:
///
/// - `Module::root()` - Root namespace, items accessible without prefix
/// - `Module::new(&["math"])` - Single namespace, items accessible as `math::item`
/// - `Module::new(&["std", "collections"])` - Nested namespace, items accessible as `std::collections::item`
///
/// # Example
///
/// ```ignore
/// use angelscript::Module;
///
/// // Math module - functions accessible as math::sqrt(), math::sin()
/// let mut math = Module::new(&["math"]);
/// math.register_fn("sqrt", |x: f64| x.sqrt());
/// math.register_fn("sin", |x: f64| x.sin());
///
/// // Root module - functions accessible directly as print()
/// let mut globals = Module::root();
/// globals.register_fn("print", |s: &str| println!("{}", s));
/// ```
pub struct Module<'app> {
    /// Arena for storing parsed AST nodes (still needed for parsing declarations)
    arena: Bump,

    /// Namespace path for all items.
    /// Empty = root namespace, ["math"] = single level,
    /// ["std", "collections"] = nested namespace
    namespace: Vec<String>,

    /// Registered native functions (owned, no arena lifetime)
    functions: Vec<FfiFunctionDef>,

    /// Registered native types (owned, no arena lifetime)
    types: Vec<FfiTypeDef>,

    /// Registered enums (owned, no arena lifetime)
    enums: Vec<FfiEnumDef>,

    /// Registered interfaces (owned, no arena lifetime)
    interfaces: Vec<FfiInterfaceDef>,

    /// Registered funcdefs (function pointer types, owned, no arena lifetime)
    funcdefs: Vec<FfiFuncdefDef>,

    /// Global properties (app-owned references)
    /// The lifetime is tied to the module's arena via a transmute in register_global_property
    global_properties: Vec<GlobalPropertyDef<'static, 'app>>,
}

impl<'app> Module<'app> {
    /// Create a new module with the given namespace path.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Single-level namespace
    /// let math = Module::new(&["math"]);
    /// // Items accessible as math::sqrt(), math::Vec3, etc.
    ///
    /// // Nested namespace
    /// let collections = Module::new(&["std", "collections"]);
    /// // Items accessible as std::collections::HashMap, etc.
    /// ```
    pub fn new(namespace: &[&str]) -> Self {
        Self {
            arena: Bump::new(),
            namespace: namespace.iter().map(|s| (*s).to_string()).collect(),
            functions: Vec::new(),
            types: Vec::new(),
            enums: Vec::new(),
            interfaces: Vec::new(),
            funcdefs: Vec::new(),
            global_properties: Vec::new(),
        }
    }

    /// Create a module in the root namespace.
    ///
    /// Items in this module are accessible without a namespace prefix.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut globals = Module::root();
    /// globals.register_fn("print", |s: &str| println!("{}", s));
    /// // In script: print("hello") - no namespace prefix needed
    /// ```
    pub fn root() -> Self {
        Self::new(&[])
    }

    /// Get the namespace path for this module.
    pub fn namespace(&self) -> &[String] {
        &self.namespace
    }

    /// Check if this is the root namespace.
    pub fn is_root(&self) -> bool {
        self.namespace.is_empty()
    }

    /// Get the fully qualified name for an item in this module.
    ///
    /// For root namespace, returns just the name.
    /// For other namespaces, returns "namespace::name".
    pub fn qualified_name(&self, name: &str) -> String {
        if self.namespace.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace.join("::"), name)
        }
    }

    // =========================================================================
    // Function registration
    // =========================================================================

    /// Register a native function using a declaration string and raw callable.
    ///
    /// This is the low-level registration method that works directly with
    /// `NativeCallable` implementations. For most use cases, prefer `register_fn`
    /// which provides type-safe closure wrapping.
    ///
    /// # Parameters
    ///
    /// - `decl`: Declaration string like `"int add(int a, int b)"` or `"void print(const string& in msg)"`
    /// - `f`: The native function implementation
    ///
    /// # Declaration Syntax
    ///
    /// ```text
    /// return_type name(param_type [ref_modifier] [param_name] [= default], ...) [const]
    /// ```
    ///
    /// Examples:
    /// - `"int add(int a, int b)"` - Two int parameters, returns int
    /// - `"void print(const string& in msg)"` - String reference parameter
    /// - `"float getValue() const"` - Const method (for class registration)
    /// - `"void callback(?& in)"` - Auto-handle parameter
    ///
    /// # Example
    ///
    /// ```ignore
    /// use angelscript::{Module, ffi::{NativeFn, CallContext}};
    ///
    /// let mut module = Module::root();
    /// module.register_fn_raw("int add(int a, int b)", |ctx: &mut CallContext| {
    ///     let a: i32 = ctx.arg(0)?;
    ///     let b: i32 = ctx.arg(1)?;
    ///     ctx.set_return(a + b)?;
    ///     Ok(())
    /// })?;
    /// ```
    pub fn register_fn_raw<F>(
        &mut self,
        decl: &str,
        f: F,
    ) -> Result<&mut Self, FfiModuleError>
    where
        F: NativeCallable + Send + Sync + 'static,
    {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let sig = Parser::function_decl(decl, &self.arena).map_err(|errors| {
            FfiModuleError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Build the function definition
        let func_def = self.build_function_def(sig, NativeFn::new(f));

        self.functions.push(func_def);
        Ok(self)
    }

    /// Register a type-safe native function using a declaration string.
    ///
    /// This method wraps typed Rust closures, automatically converting arguments
    /// from script values and return values to script values.
    ///
    /// # Parameters
    ///
    /// - `decl`: Declaration string (see `register_fn_raw` for syntax)
    /// - `f`: A Rust closure or function
    ///
    /// # Supported Signatures
    ///
    /// The closure can have 0-8 parameters of types that implement `FromScript`,
    /// and can optionally return a value that implements `ToScript`.
    ///
    /// Supported parameter types include:
    /// - Primitives: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`, `bool`
    /// - Strings: `String`, `&str` (via cloning)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use angelscript::Module;
    ///
    /// let mut module = Module::root();
    ///
    /// // Simple function
    /// module.register_fn("int add(int a, int b)", |a: i32, b: i32| a + b)?;
    ///
    /// // Void return
    /// module.register_fn("void greet(string name)", |name: String| {
    ///     println!("Hello, {}!", name);
    /// })?;
    ///
    /// // No parameters
    /// module.register_fn("float pi()", || std::f64::consts::PI)?;
    /// ```
    pub fn register_fn<F, Args, Ret>(
        &mut self,
        decl: &str,
        f: F,
    ) -> Result<&mut Self, FfiModuleError>
    where
        F: IntoNativeFn<Args, Ret>,
    {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let sig = Parser::function_decl(decl, &self.arena).map_err(|errors| {
            FfiModuleError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Convert the closure to NativeFn
        let native_fn = f.into_native_fn();

        // Build the function definition
        let func_def = self.build_function_def(sig, native_fn);

        self.functions.push(func_def);
        Ok(self)
    }

    /// Internal helper to build a FfiFunctionDef from parsed signature.
    fn build_function_def(
        &self,
        sig: crate::ast::FunctionSignatureDecl<'_>,
        native_fn: NativeFn,
    ) -> FfiFunctionDef {
        use crate::types::signature_to_ffi_function;

        // Use the conversion helper to build the FfiFunctionDef
        signature_to_ffi_function(&sig, native_fn)
    }

    /// Internal helper to build a GlobalPropertyDef from parsed property.
    fn build_property_def<T: 'static>(
        &self,
        prop: PropertyDecl<'_>,
        value: &'app mut T,
    ) -> GlobalPropertyDef<'static, 'app> {
        // SAFETY: The arena is owned by self and lives as long as self.
        // We transmute the lifetime to 'static for storage, but the actual
        // lifetime is tied to the module. This is safe because:
        // 1. The arena is never moved or replaced
        // 2. The global_properties vec is dropped before the arena
        // 3. We never expose references with incorrect lifetimes
        let ty = unsafe { std::mem::transmute(prop.ty) };
        let name = unsafe { std::mem::transmute(prop.name) };

        GlobalPropertyDef::new(name, ty, value)
    }

    /// Get the registered functions.
    pub fn functions(&self) -> &[FfiFunctionDef] {
        &self.functions
    }

    // =========================================================================
    // Type registration
    // =========================================================================

    /// Register a native type using the ClassBuilder.
    ///
    /// The type declaration can be a simple name like `"Vec3"` or a template
    /// declaration like `"array<class T>"` or `"dictionary<class K, class V>"`.
    ///
    /// # Parameters
    ///
    /// - `decl`: Type declaration (e.g., `"Vec3"`, `"array<class T>"`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simple value type
    /// module.register_type::<Vec3>("Vec3")
    ///     .value_type()
    ///     .constructor("void f()", || Vec3::default())?
    ///     .method("float length() const", |v: &Vec3| v.length())?
    ///     .build()?;
    ///
    /// // Template type
    /// module.register_type::<ScriptArray>("array<class T>")
    ///     .reference_type()
    ///     .template_callback(|_| TemplateValidation::valid())?
    ///     .build()?;
    /// ```
    pub fn register_type<T: NativeType>(&mut self, decl: &str) -> ClassBuilder<'_, 'app, T> {
        // Parse the type declaration as a TypeExpr (e.g., "array<class T>")
        let type_expr = Parser::type_expr(decl, &self.arena)
            .expect("Invalid type declaration"); // TODO: Return Result in future

        // Extract the type name from the base
        let name = match type_expr.base {
            TypeBase::Named(ident) => ident.name.to_string(),
            TypeBase::Primitive(p) => format!("{:?}", p).to_lowercase(),
            _ => panic!("Invalid type declaration: expected named type"),
        };

        // Extract template param names from template_args as owned strings.
        // Only TemplateParam types (e.g., "class T") are template parameters.
        // Named types (e.g., "string") are concrete type constraints, not parameters.
        let template_params: Vec<String> = type_expr
            .template_args
            .iter()
            .filter_map(|ty| {
                if let TypeBase::TemplateParam(ident) = ty.base {
                    Some(ident.name.to_string())
                } else {
                    None
                }
            })
            .collect();

        ClassBuilder::new(self, name, template_params)
    }

    /// Internal method to add a type definition.
    ///
    /// Called by ClassBuilder::build().
    pub(crate) fn add_type(&mut self, type_def: FfiTypeDef) {
        self.types.push(type_def);
    }

    /// Get the registered types.
    pub fn types(&self) -> &[FfiTypeDef] {
        &self.types
    }

    /// Get access to the module's arena for parsing.
    ///
    /// This is used internally by ClassBuilder.
    pub(crate) fn arena(&self) -> &Bump {
        &self.arena
    }

    // =========================================================================
    // Enum registration
    // =========================================================================

    /// Register a native enum type using the EnumBuilder.
    ///
    /// Returns an `EnumBuilder` that allows adding enum values with explicit
    /// or auto-incremented integer values.
    ///
    /// # Parameters
    ///
    /// - `name`: The enum type name (e.g., `"Color"`, `"Direction"`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Basic enum with explicit values
    /// module.register_enum("Color")
    ///     .value("Red", 0)?
    ///     .value("Green", 1)?
    ///     .value("Blue", 2)?
    ///     .build()?;
    ///
    /// // Auto-numbered enum (values: 0, 1, 2, 3)
    /// module.register_enum("Direction")
    ///     .auto_value("North")?
    ///     .auto_value("East")?
    ///     .auto_value("South")?
    ///     .auto_value("West")?
    ///     .build()?;
    ///
    /// // Flags with power-of-2 values
    /// module.register_enum("FileFlags")
    ///     .value("None", 0)?
    ///     .value("Read", 1)?
    ///     .value("Write", 2)?
    ///     .value("Execute", 4)?
    ///     .value("All", 7)?
    ///     .build()?;
    /// ```
    pub fn register_enum(&mut self, name: &str) -> EnumBuilder<'_, 'app> {
        EnumBuilder::new(self, name.to_string())
    }

    /// Internal method to add an enum definition.
    ///
    /// Called by EnumBuilder::build().
    pub(crate) fn add_enum(&mut self, enum_def: FfiEnumDef) {
        self.enums.push(enum_def);
    }

    /// Get the registered enums.
    pub fn enums(&self) -> &[FfiEnumDef] {
        &self.enums
    }

    // =========================================================================
    // Interface registration
    // =========================================================================

    /// Register a native interface type using the InterfaceBuilder.
    ///
    /// Returns an `InterfaceBuilder` that allows adding abstract method signatures
    /// that script classes can implement.
    ///
    /// # Parameters
    ///
    /// - `name`: The interface type name (e.g., `"IDrawable"`, `"ISerializable"`)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simple interface
    /// module.register_interface("IDrawable")
    ///     .method("void draw() const")?
    ///     .method("void setVisible(bool visible)")?
    ///     .build()?;
    ///
    /// // Serialization interface
    /// module.register_interface("ISerializable")
    ///     .method("string serialize() const")?
    ///     .method("void deserialize(const string &in data)")?
    ///     .build()?;
    ///
    /// // Complex interface with multiple methods
    /// module.register_interface("IGameEntity")
    ///     .method("string getName() const")?
    ///     .method("void setName(const string &in name)")?
    ///     .method("Vec3 getPosition() const")?
    ///     .method("void setPosition(const Vec3 &in pos)")?
    ///     .method("void update(float deltaTime)")?
    ///     .method("void render() const")?
    ///     .build()?;
    /// ```
    pub fn register_interface(&mut self, name: &str) -> InterfaceBuilder<'_, 'app> {
        InterfaceBuilder::new(self, name.to_string())
    }

    /// Internal method to add an interface definition.
    ///
    /// Called by InterfaceBuilder::build().
    pub(crate) fn add_interface(&mut self, interface_def: FfiInterfaceDef) {
        self.interfaces.push(interface_def);
    }

    /// Get the registered interfaces.
    pub fn interfaces(&self) -> &[FfiInterfaceDef] {
        &self.interfaces
    }

    // =========================================================================
    // Funcdef registration
    // =========================================================================

    /// Register a funcdef (function pointer type) with a declaration string.
    ///
    /// Funcdefs define function pointer types that can be used to pass
    /// callbacks or store function references in scripts.
    ///
    /// # Declaration Format
    ///
    /// ```text
    /// funcdef ReturnType Name(params)
    /// ```
    ///
    /// # Parameters
    ///
    /// - `decl`: Full funcdef declaration string including the `funcdef` keyword
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Simple callback
    /// module.register_funcdef("funcdef void Callback()")?;
    ///
    /// // Predicate function
    /// module.register_funcdef("funcdef bool Predicate(int value)")?;
    ///
    /// // Event handler with multiple parameters
    /// module.register_funcdef("funcdef void EventHandler(const string &in event, int data)")?;
    ///
    /// // Factory function returning a handle
    /// module.register_funcdef("funcdef Entity@ EntityFactory(const string &in name)")?;
    ///
    /// // Comparator function
    /// module.register_funcdef("funcdef int Comparator(const string &in a, const string &in b)")?;
    /// ```
    pub fn register_funcdef(&mut self, decl: &str) -> Result<&mut Self, FfiModuleError> {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the funcdef declaration using the module's arena
        let fd = Parser::funcdef_decl(decl, &self.arena).map_err(|errors| {
            FfiModuleError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Build the funcdef definition
        let funcdef_def = self.build_funcdef_def(fd);

        self.funcdefs.push(funcdef_def);
        Ok(self)
    }

    /// Internal helper to build a FfiFuncdefDef from parsed funcdef declaration.
    fn build_funcdef_def(&self, fd: crate::ast::FuncdefDecl<'_>) -> FfiFuncdefDef {
        // Convert params to owned FfiParam
        let params: Vec<FfiParam> = fd.params.iter().map(function_param_to_ffi).collect();

        // Convert return type to FfiDataType
        let return_type = return_type_to_ffi(&fd.return_type);

        FfiFuncdefDef::new(TypeId::next(), fd.name.name.to_string(), params, return_type)
    }

    /// Internal method to add a funcdef definition.
    ///
    /// Called internally by register_funcdef().
    pub(crate) fn add_funcdef(&mut self, funcdef_def: FfiFuncdefDef) {
        self.funcdefs.push(funcdef_def);
    }

    /// Get the registered funcdefs.
    pub fn funcdefs(&self) -> &[FfiFuncdefDef] {
        &self.funcdefs
    }

    // =========================================================================
    // Global property registration
    // =========================================================================

    /// Register a global property using a declaration string.
    ///
    /// The app owns the data; scripts read/write via reference.
    ///
    /// # Parameters
    ///
    /// - `decl`: Declaration string like `"int score"` or `"const float PI"`
    /// - `value`: Mutable reference to the value (must outlive the module)
    ///
    /// # Declaration Syntax
    ///
    /// ```text
    /// [const] type name
    /// ```
    ///
    /// Examples:
    /// - `"int score"` - mutable int
    /// - `"const double PI"` - read-only double
    /// - `"string name"` - mutable string
    /// - `"const MyClass@ obj"` - read-only handle to MyClass
    ///
    /// # Example
    ///
    /// ```ignore
    /// use angelscript::Module;
    ///
    /// let mut score: i32 = 0;
    /// let mut pi = std::f64::consts::PI;
    ///
    /// let mut module = Module::root();
    /// module.register_global_property("int g_score", &mut score)?;
    /// module.register_global_property("const double PI", &mut pi)?;
    /// ```
    pub fn register_global_property<T: 'static>(
        &mut self,
        decl: &str,
        value: &'app mut T,
    ) -> Result<(), FfiModuleError> {
        let decl = decl.trim();
        if decl.is_empty() {
            return Err(FfiModuleError::InvalidDeclaration(
                "empty declaration".to_string(),
            ));
        }

        // Parse the declaration using the module's arena
        let prop = Parser::property_decl(decl, &self.arena).map_err(|errors| {
            FfiModuleError::InvalidDeclaration(format!("parse error: {}", errors))
        })?;

        // Build the property definition
        let prop_def = self.build_property_def(prop, value);

        self.global_properties.push(prop_def);
        Ok(())
    }

    /// Get the registered global properties.
    pub fn global_properties(&self) -> &[GlobalPropertyDef<'static, 'app>] {
        &self.global_properties
    }

    /// Get mutable access to the registered global properties.
    pub fn global_properties_mut(&mut self) -> &mut [GlobalPropertyDef<'static, 'app>] {
        &mut self.global_properties
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get the total number of items registered in this module.
    pub fn item_count(&self) -> usize {
        self.functions.len()
            + self.types.len()
            + self.enums.len()
            + self.interfaces.len()
            + self.funcdefs.len()
            + self.global_properties.len()
    }

    // =========================================================================
    // Installation into FfiRegistryBuilder
    // =========================================================================

    /// Install all registered FFI data from this module into an FfiRegistryBuilder.
    ///
    /// This transfers all types, functions, enums, interfaces, and funcdefs to the
    /// builder. The builder will resolve types and build an immutable FfiRegistry
    /// when `build()` is called.
    ///
    /// # Parameters
    ///
    /// - `builder`: The FfiRegistryBuilder to install into
    ///
    /// # Errors
    ///
    /// Returns an error if any registration fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut module = Module::root();
    /// module.register_fn("int add(int a, int b)", |a: i32, b: i32| a + b)?;
    ///
    /// let mut builder = FfiRegistryBuilder::new();
    /// module.install_into(&mut builder)?;
    ///
    /// let registry = builder.build()?;
    /// ```
    pub fn install_into(&self, builder: &mut FfiRegistryBuilder) -> Result<(), FfiModuleError> {
        // Register namespace if not root
        if !self.namespace.is_empty() {
            builder.register_namespace(&self.namespace.join("::"));
        }

        // Install types first (so functions can reference them)
        for type_def in &self.types {
            self.install_type(builder, type_def)?;
        }

        // Install enums
        for enum_def in &self.enums {
            self.install_enum(builder, enum_def);
        }

        // Install interfaces
        for interface_def in &self.interfaces {
            self.install_interface(builder, interface_def);
        }

        // Install funcdefs
        for funcdef_def in &self.funcdefs {
            self.install_funcdef(builder, funcdef_def);
        }

        // Install functions last (they may reference types)
        for func_def in &self.functions {
            self.install_function(builder, func_def);
        }

        Ok(())
    }

    /// Install a type definition into the builder.
    fn install_type(
        &self,
        builder: &mut FfiRegistryBuilder,
        type_def: &FfiTypeDef,
    ) -> Result<(), FfiModuleError> {
        use crate::semantic::types::type_def::TypeDef;
        use crate::types::TypeKind;
        use rustc_hash::FxHashMap;

        let qualified_name = self.qualified_name(&type_def.name);

        // Create and register template parameters
        // Each param gets a unique TypeId and is registered with a name like "array::$T"
        let template_param_ids: Vec<TypeId> = type_def
            .template_params
            .iter()
            .enumerate()
            .map(|(index, param_name)| {
                let param_id = TypeId::next();

                // Register the template param as a TypeDef
                let param_typedef = TypeDef::TemplateParam {
                    name: param_name.clone(),
                    index,
                    owner: type_def.id,
                };
                let param_qualified_name = format!("{}::${}", qualified_name, param_name);
                builder.register_type_with_id(param_id, param_typedef, Some(&param_qualified_name));

                // Also register just the param name for simple lookups within template context
                builder.register_type_alias(param_name, param_id);

                param_id
            })
            .collect();

        // Build the TypeDef
        let typedef = TypeDef::Class {
            name: type_def.name.clone(),
            qualified_name: qualified_name.clone(),
            fields: Vec::new(), // FFI types don't have script-visible fields
            methods: Vec::new(), // Will be populated when methods are installed
            base_class: None, // TODO: Support base class in FfiTypeDef
            interfaces: Vec::new(), // TODO: Support interfaces in FfiTypeDef
            operator_methods: FxHashMap::default(), // Will be populated when operators are installed
            properties: FxHashMap::default(), // Will be populated when properties are installed
            is_final: false, // FFI types are not final by default
            is_abstract: false, // FFI types are not abstract by default
            template_params: template_param_ids,
            template: None,
            type_args: Vec::new(),
            type_kind: type_def.type_kind.clone(),
        };

        // Register the type with its pre-assigned ID
        builder.register_type_with_id(type_def.id, typedef, Some(&qualified_name));

        // Register template callback if present
        if let Some(callback) = &type_def.template_callback {
            builder.register_template_callback_arc(type_def.id, callback.clone());
        }

        // Install behaviors
        if matches!(type_def.type_kind, TypeKind::Value { .. }) {
            // For value types, register constructors
            self.install_type_constructors(builder, type_def);
        } else {
            // For reference types, register factories
            self.install_type_factories(builder, type_def);
        }

        // Install methods
        self.install_type_methods(builder, type_def);

        // Install properties
        self.install_type_properties(builder, type_def);

        // Install operators
        self.install_type_operators(builder, type_def);

        Ok(())
    }

    /// Install constructors for a value type.
    fn install_type_constructors(&self, builder: &mut FfiRegistryBuilder, type_def: &FfiTypeDef) {
        for ctor in &type_def.constructors {
            // Add to behaviors
            let behaviors = builder.behaviors_mut(type_def.id);
            behaviors.constructors.push(ctor.id);

            // Build a new FfiFunctionDef for registration
            let ctor_func = self.clone_function_def(ctor);
            let native_fn = ctor.native_fn.as_ref().map(|nf| nf.clone_arc());
            builder.register_function(ctor_func, native_fn);
        }
    }

    /// Install factories for a reference type.
    fn install_type_factories(&self, builder: &mut FfiRegistryBuilder, type_def: &FfiTypeDef) {
        for factory in &type_def.factories {
            // Add to behaviors
            let behaviors = builder.behaviors_mut(type_def.id);
            behaviors.factories.push(factory.id);

            // Build a new FfiFunctionDef for registration
            let factory_func = self.clone_function_def(factory);
            let native_fn = factory.native_fn.as_ref().map(|nf| nf.clone_arc());
            builder.register_function(factory_func, native_fn);
        }
    }

    /// Install methods for a type.
    fn install_type_methods(&self, builder: &mut FfiRegistryBuilder, type_def: &FfiTypeDef) {
        use crate::semantic::types::type_def::TypeDef;

        // Collect method IDs
        let method_ids: Vec<_> = type_def.methods.iter().map(|m| m.id).collect();

        // Update the TypeDef with method IDs
        if let Some(TypeDef::Class { methods, .. }) = builder.get_type_mut(type_def.id) {
            *methods = method_ids;
        }

        // Register each method function
        for method in &type_def.methods {
            let mut method_func = self.clone_function_def(method);
            method_func.owner_type = Some(type_def.id);
            let native_fn = method.native_fn.as_ref().map(|nf| nf.clone_arc());
            builder.register_function(method_func, native_fn);
        }
    }

    /// Install properties for a type.
    fn install_type_properties(&self, builder: &mut FfiRegistryBuilder, type_def: &FfiTypeDef) {
        use crate::semantic::types::type_def::{FunctionId, PropertyAccessors, TypeDef, Visibility};

        if type_def.properties.is_empty() {
            return;
        }

        // Build properties map
        let mut properties_map = rustc_hash::FxHashMap::default();

        for prop in &type_def.properties {
            // Create getter function
            let getter_id = FunctionId::next();
            let getter_func = FfiFunctionDef::new(getter_id, format!("get_{}", prop.name))
                .with_return_type(prop.data_type.clone())
                .with_const(true)
                .with_native_fn(prop.getter.clone_arc());

            builder.register_function(getter_func, Some(prop.getter.clone_arc()));

            // Create setter function if writable
            let setter_id = if let Some(setter) = &prop.setter {
                let setter_id = FunctionId::next();
                let setter_func = FfiFunctionDef::new(setter_id, format!("set_{}", prop.name))
                    .with_params(vec![FfiParam::new("value", prop.data_type.clone())])
                    .with_return_type(FfiDataType::void())
                    .with_native_fn(setter.clone_arc());

                builder.register_function(setter_func, Some(setter.clone_arc()));
                Some(setter_id)
            } else {
                None
            };

            properties_map.insert(
                prop.name.clone(),
                PropertyAccessors {
                    getter: Some(getter_id),
                    setter: setter_id,
                    visibility: Visibility::Public,
                },
            );
        }

        // Update the TypeDef with properties
        if let Some(TypeDef::Class { properties, .. }) = builder.get_type_mut(type_def.id) {
            *properties = properties_map;
        }
    }

    /// Install operators for a type.
    fn install_type_operators(&self, builder: &mut FfiRegistryBuilder, type_def: &FfiTypeDef) {
        use crate::semantic::types::type_def::TypeDef;

        if type_def.operators.is_empty() {
            return;
        }

        // Build operator methods map
        let mut operator_map: rustc_hash::FxHashMap<_, Vec<_>> = rustc_hash::FxHashMap::default();

        for func in &type_def.operators {
            // Operators should have their operator field set
            if let Some(operator) = func.operator {
                let mut op_func = self.clone_function_def(func);
                op_func.owner_type = Some(type_def.id);
                op_func.operator = Some(operator);

                operator_map.entry(operator).or_default().push(func.id);

                let native_fn = func.native_fn.as_ref().map(|nf| nf.clone_arc());
                builder.register_function(op_func, native_fn);
            }
        }

        // Update the TypeDef with operators
        if let Some(TypeDef::Class { operator_methods, .. }) = builder.get_type_mut(type_def.id) {
            *operator_methods = operator_map;
        }
    }

    /// Install an enum definition into the builder.
    fn install_enum(&self, builder: &mut FfiRegistryBuilder, enum_def: &FfiEnumDef) {
        use crate::semantic::types::type_def::TypeDef;

        let qualified_name = self.qualified_name(&enum_def.name);

        let typedef = TypeDef::Enum {
            name: enum_def.name.clone(),
            qualified_name: qualified_name.clone(),
            values: enum_def.values.clone(),
        };

        builder.register_type_with_id(enum_def.id, typedef, Some(&qualified_name));
    }

    /// Install an interface definition into the builder.
    fn install_interface(&self, builder: &mut FfiRegistryBuilder, interface_def: &FfiInterfaceDef) {
        let qualified_name = self.qualified_name(interface_def.name());
        builder.register_interface(interface_def.clone(), &qualified_name);
    }

    /// Install a funcdef definition into the builder.
    fn install_funcdef(&self, builder: &mut FfiRegistryBuilder, funcdef_def: &FfiFuncdefDef) {
        let qualified_name = self.qualified_name(&funcdef_def.name);
        builder.register_funcdef(funcdef_def.clone(), &qualified_name);
    }

    /// Install a function definition into the builder.
    fn install_function(&self, builder: &mut FfiRegistryBuilder, func_def: &FfiFunctionDef) {
        let mut func = self.clone_function_def(func_def);

        // Set namespace based on module namespace
        if !self.namespace.is_empty() {
            func.namespace = self.namespace.clone();
        }

        let native_fn = func_def.native_fn.as_ref().map(|nf| nf.clone_arc());
        builder.register_function(func, native_fn);
    }

    /// Clone a FfiFunctionDef (helper to work around non-Clone NativeFn).
    fn clone_function_def(&self, func: &FfiFunctionDef) -> FfiFunctionDef {
        FfiFunctionDef {
            id: func.id,
            name: func.name.clone(),
            namespace: func.namespace.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            traits: func.traits.clone(),
            owner_type: func.owner_type,
            operator: func.operator,
            visibility: func.visibility,
            native_fn: None, // Native fn handled separately
        }
    }
}

impl Default for Module<'_> {
    fn default() -> Self {
        Self::root()
    }
}

// Manual Debug implementation since Bump doesn't implement Debug
impl std::fmt::Debug for Module<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("namespace", &self.namespace)
            .field("functions", &self.functions)
            .field("types", &self.types)
            .field("enums", &self.enums)
            .field("interfaces", &self.interfaces)
            .field("funcdefs", &self.funcdefs)
            .field("global_properties", &self.global_properties)
            .finish()
    }
}

/// Errors that can occur during FFI module operations.
#[derive(Debug, Clone, Error)]
pub enum FfiModuleError {
    /// Invalid declaration string
    #[error("invalid declaration: {0}")]
    InvalidDeclaration(String),

    /// Duplicate registration
    #[error("duplicate registration: {name} already registered as {kind}")]
    DuplicateRegistration { name: String, kind: String },

    /// Duplicate enum value name
    #[error("duplicate enum value: '{value_name}' already exists in enum '{enum_name}'")]
    DuplicateEnumValue { enum_name: String, value_name: String },

    /// Type not found
    #[error("type not found: {0}")]
    TypeNotFound(String),

    /// Invalid type for operation
    #[error("invalid type: {0}")]
    InvalidType(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_new_with_namespace() {
        let module = Module::new(&["math"]);
        assert_eq!(module.namespace(), &["math"]);
        assert!(!module.is_root());
    }

    #[test]
    fn module_new_nested_namespace() {
        let module = Module::new(&["std", "collections"]);
        assert_eq!(module.namespace(), &["std", "collections"]);
    }

    #[test]
    fn module_root() {
        let module = Module::<'static>::root();
        assert!(module.namespace().is_empty());
        assert!(module.is_root());
    }

    #[test]
    fn module_default_is_root() {
        let module = Module::<'static>::default();
        assert!(module.is_root());
    }

    #[test]
    fn module_qualified_name_root() {
        let module = Module::<'static>::root();
        assert_eq!(module.qualified_name("print"), "print");
    }

    #[test]
    fn module_qualified_name_single_namespace() {
        let module = Module::<'static>::new(&["math"]);
        assert_eq!(module.qualified_name("sqrt"), "math::sqrt");
    }

    #[test]
    fn module_qualified_name_nested_namespace() {
        let module = Module::<'static>::new(&["std", "collections"]);
        assert_eq!(
            module.qualified_name("HashMap"),
            "std::collections::HashMap"
        );
    }

    #[test]
    fn module_register_global_property_i32() {
        use crate::ast::PrimitiveType;

        let mut value: i32 = 42;
        let mut module = Module::root();

        module.register_global_property("int score", &mut value).unwrap();

        assert_eq!(module.global_properties().len(), 1);
        assert_eq!(module.global_properties()[0].name.name, "score");
        assert!(!module.global_properties()[0].is_const());
        assert!(matches!(
            module.global_properties()[0].ty.base,
            crate::ast::TypeBase::Primitive(PrimitiveType::Int)
        ));
    }

    #[test]
    fn module_register_global_property_const() {
        let mut value: f64 = std::f64::consts::PI;
        let mut module = Module::root();

        module.register_global_property("const double PI", &mut value).unwrap();

        assert!(module.global_properties()[0].is_const());
    }

    #[test]
    fn module_register_global_property_various_types() {
        use crate::ast::{PrimitiveType, TypeBase};

        let mut i32_val: i32 = 42;
        let mut f64_val: f64 = 3.14;
        let mut bool_val = true;
        let mut string_val = String::from("hello");

        let mut module = Module::root();

        module.register_global_property("int score", &mut i32_val).unwrap();
        module.register_global_property("const double pi", &mut f64_val).unwrap();
        module.register_global_property("bool enabled", &mut bool_val).unwrap();
        module.register_global_property("string greeting", &mut string_val).unwrap();

        assert_eq!(module.global_properties().len(), 4);
        assert!(matches!(module.global_properties()[0].ty.base, TypeBase::Primitive(PrimitiveType::Int)));
        assert!(matches!(module.global_properties()[1].ty.base, TypeBase::Primitive(PrimitiveType::Double)));
        assert!(matches!(module.global_properties()[2].ty.base, TypeBase::Primitive(PrimitiveType::Bool)));
        assert!(matches!(module.global_properties()[3].ty.base, TypeBase::Named(ident) if ident.name == "string"));
    }

    #[test]
    fn module_register_global_property_handle() {
        struct MyClass {
            value: i32,
        }

        let mut obj = MyClass { value: 42 };
        let mut module = Module::root();

        module.register_global_property("MyClass@ obj", &mut obj).unwrap();

        let prop = &module.global_properties()[0];
        assert_eq!(prop.name.name, "obj");
        assert!(prop.ty.has_handle());
    }

    #[test]
    fn module_register_global_property_const_handle() {
        struct MyClass {
            value: i32,
        }

        let mut obj = MyClass { value: 42 };
        let mut module = Module::root();

        module.register_global_property("const MyClass@ obj", &mut obj).unwrap();

        let prop = &module.global_properties()[0];
        assert!(prop.ty.has_handle());
        assert!(prop.ty.is_const); // const T@ means handle to const
    }

    #[test]
    fn module_register_global_property_complex_struct() {
        // A complex struct with multiple fields and nested types
        #[derive(Debug)]
        struct Vector3 {
            x: f32,
            y: f32,
            z: f32,
        }

        #[derive(Debug)]
        struct Transform {
            position: Vector3,
            rotation: Vector3,
            scale: Vector3,
            name: String,
            enabled: bool,
        }

        let mut transform = Transform {
            position: Vector3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            rotation: Vector3 {
                x: 0.0,
                y: 90.0,
                z: 0.0,
            },
            scale: Vector3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            name: String::from("Player"),
            enabled: true,
        };

        let mut module = Module::root();
        module
            .register_global_property("Transform g_transform", &mut transform)
            .unwrap();

        assert_eq!(module.global_properties().len(), 1);
        let prop = &module.global_properties()[0];
        assert_eq!(prop.name.name, "g_transform");
        assert!(!prop.is_const());

        // Verify we can downcast and access the complex struct
        let downcast = prop.downcast_ref::<Transform>().unwrap();
        assert_eq!(downcast.position.x, 1.0);
        assert_eq!(downcast.rotation.y, 90.0);
        assert_eq!(downcast.name, "Player");
        assert!(downcast.enabled);
    }

    #[test]
    fn module_register_global_property_complex_struct_mutation() {
        #[derive(Debug)]
        struct GameState {
            score: i32,
            level: u32,
            player_name: String,
            is_paused: bool,
        }

        let mut state = GameState {
            score: 0,
            level: 1,
            player_name: String::from("Hero"),
            is_paused: false,
        };

        let mut module = Module::root();
        module
            .register_global_property("GameState g_state", &mut state)
            .unwrap();

        // Get mutable reference and modify
        let prop = &mut module.global_properties_mut()[0];
        if let Some(game_state) = prop.downcast_mut::<GameState>() {
            game_state.score = 100;
            game_state.level = 5;
            game_state.is_paused = true;
        }

        // Verify mutations persisted
        let prop = &module.global_properties()[0];
        let downcast = prop.downcast_ref::<GameState>().unwrap();
        assert_eq!(downcast.score, 100);
        assert_eq!(downcast.level, 5);
        assert!(downcast.is_paused);
    }

    #[test]
    fn module_register_global_property_with_vec() {
        let mut items: Vec<i32> = vec![1, 2, 3, 4, 5];
        let mut module = Module::root();

        module
            .register_global_property("array<int> g_items", &mut items)
            .unwrap();

        let prop = &module.global_properties()[0];
        let downcast = prop.downcast_ref::<Vec<i32>>().unwrap();
        assert_eq!(downcast.len(), 5);
        assert_eq!(downcast[0], 1);
    }

    #[test]
    fn module_register_global_property_invalid_decl_empty() {
        let mut value: i32 = 0;
        let mut module = Module::root();
        assert!(module.register_global_property("", &mut value).is_err());
    }

    #[test]
    fn module_register_global_property_invalid_decl_no_name() {
        let mut value: i32 = 0;
        let mut module = Module::root();
        // Just a type without a name should fail
        assert!(module.register_global_property("int", &mut value).is_err());
    }

    #[test]
    fn module_item_count() {
        let mut value: i32 = 0;
        let mut module = Module::root();

        assert_eq!(module.item_count(), 0);

        module.register_global_property("int score", &mut value).unwrap();

        assert_eq!(module.item_count(), 1);
    }

    #[test]
    fn module_add_enum() {
        let mut module = Module::<'static>::root();

        module.add_enum(FfiEnumDef::new(
            TypeId::next(),
            "Color".to_string(),
            vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        ));

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].name, "Color");
        assert_eq!(module.enums()[0].values.len(), 3);
    }

    #[test]
    fn module_debug() {
        let module = Module::<'static>::new(&["math"]);
        let debug = format!("{:?}", module);
        assert!(debug.contains("Module"));
        assert!(debug.contains("math"));
    }

    #[test]
    fn ffi_module_error_display() {
        let err = FfiModuleError::InvalidDeclaration("bad decl".to_string());
        assert!(err.to_string().contains("invalid declaration"));
        assert!(err.to_string().contains("bad decl"));

        let err = FfiModuleError::DuplicateRegistration {
            name: "foo".to_string(),
            kind: "function".to_string(),
        };
        assert!(err.to_string().contains("duplicate registration"));
        assert!(err.to_string().contains("foo"));

        let err = FfiModuleError::TypeNotFound("Bar".to_string());
        assert!(err.to_string().contains("type not found"));

        let err = FfiModuleError::InvalidType("bad type".to_string());
        assert!(err.to_string().contains("invalid type"));
    }

    #[test]
    fn ffi_enum_def_clone() {
        let enum_def = FfiEnumDef::new(
            TypeId::next(),
            "Color".to_string(),
            vec![("Red".to_string(), 0)],
        );
        let cloned = enum_def.clone();
        assert_eq!(cloned.name, "Color");
    }

    // =========================================================================
    // Function registration tests
    // =========================================================================

    #[test]
    fn register_fn_simple() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("int add(int a, int b)", |a: i32, b: i32| a + b)
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].name, "add");
        assert_eq!(module.functions()[0].params.len(), 2);
    }

    #[test]
    fn register_fn_no_params() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("float pi()", || std::f64::consts::PI)
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].name, "pi");
        assert_eq!(module.functions()[0].params.len(), 0);
    }

    #[test]
    fn register_fn_void_return() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("void greet(string name)", |_name: String| {
                // In a real use case, would print or do something
            })
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].name, "greet");
    }

    #[test]
    fn register_fn_multiple() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("int add(int a, int b)", |a: i32, b: i32| a + b)
            .unwrap()
            .register_fn("int sub(int a, int b)", |a: i32, b: i32| a - b)
            .unwrap()
            .register_fn("int mul(int a, int b)", |a: i32, b: i32| a * b)
            .unwrap();

        assert_eq!(module.functions().len(), 3);
    }

    #[test]
    fn register_fn_const_method() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("int getValue() const", || 42i32)
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert!(module.functions()[0].traits.is_const);
    }

    #[test]
    fn register_fn_raw_simple() {
        use crate::ffi::CallContext;

        let mut module = Module::<'static>::root();

        module
            .register_fn_raw("int add(int a, int b)", |ctx: &mut CallContext| {
                let a: i32 = ctx.arg(0)?;
                let b: i32 = ctx.arg(1)?;
                ctx.set_return(a + b)?;
                Ok(())
            })
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].name, "add");
    }

    #[test]
    fn register_fn_invalid_decl_empty() {
        let mut module = Module::<'static>::root();

        let result = module.register_fn("", |a: i32, b: i32| a + b);
        assert!(result.is_err());
    }

    #[test]
    fn register_fn_invalid_decl_syntax() {
        let mut module = Module::<'static>::root();

        // Missing return type
        let result = module.register_fn("add(int a, int b)", |a: i32, b: i32| a + b);
        assert!(result.is_err());
    }

    #[test]
    fn register_fn_raw_invalid_decl_empty() {
        use crate::ffi::CallContext;

        let mut module = Module::<'static>::root();

        let result = module.register_fn_raw("", |_ctx: &mut CallContext| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn register_fn_with_namespaced_module() {
        let mut module = Module::<'static>::new(&["math"]);

        module
            .register_fn("float sqrt(float x)", |x: f64| x.sqrt())
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.qualified_name("sqrt"), "math::sqrt");
    }

    #[test]
    fn register_fn_many_args() {
        let mut module = Module::<'static>::root();

        // Test with 4 arguments
        module
            .register_fn(
                "int sum4(int a, int b, int c, int d)",
                |a: i32, b: i32, c: i32, d: i32| a + b + c + d,
            )
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].params.len(), 4);
    }

    #[test]
    fn register_fn_string_param() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("int length(string s)", |s: String| s.len() as i32)
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].params.len(), 1);
    }

    // =========================================================================
    // Funcdef registration tests
    // =========================================================================

    #[test]
    fn register_funcdef_simple() {
        let mut module = Module::<'static>::root();

        module
            .register_funcdef("funcdef void Callback()")
            .unwrap();

        assert_eq!(module.funcdefs().len(), 1);
        assert_eq!(module.funcdefs()[0].name, "Callback");
        assert_eq!(module.funcdefs()[0].params.len(), 0);
    }

    #[test]
    fn register_funcdef_with_params() {
        let mut module = Module::<'static>::root();

        module
            .register_funcdef("funcdef bool Predicate(int value)")
            .unwrap();

        assert_eq!(module.funcdefs().len(), 1);
        assert_eq!(module.funcdefs()[0].name, "Predicate");
        assert_eq!(module.funcdefs()[0].params.len(), 1);
    }

    #[test]
    fn register_funcdef_multiple_params() {
        let mut module = Module::<'static>::root();

        module
            .register_funcdef("funcdef void EventHandler(const string &in event, int data)")
            .unwrap();

        assert_eq!(module.funcdefs().len(), 1);
        assert_eq!(module.funcdefs()[0].name, "EventHandler");
        assert_eq!(module.funcdefs()[0].params.len(), 2);
    }

    #[test]
    fn register_funcdef_with_handle_return() {
        let mut module = Module::<'static>::root();

        module
            .register_funcdef("funcdef Entity@ EntityFactory(const string &in name)")
            .unwrap();

        assert_eq!(module.funcdefs().len(), 1);
        assert_eq!(module.funcdefs()[0].name, "EntityFactory");
    }

    #[test]
    fn register_funcdef_chained() {
        let mut module = Module::<'static>::root();

        module
            .register_funcdef("funcdef void Callback()")
            .unwrap()
            .register_funcdef("funcdef bool Predicate(int value)")
            .unwrap()
            .register_funcdef("funcdef int Comparator(int a, int b)")
            .unwrap();

        assert_eq!(module.funcdefs().len(), 3);
    }

    #[test]
    fn register_funcdef_empty_decl() {
        let mut module = Module::<'static>::root();

        let result = module.register_funcdef("");
        assert!(result.is_err());
    }

    #[test]
    fn register_funcdef_invalid_decl() {
        let mut module = Module::<'static>::root();

        let result = module.register_funcdef("not a valid funcdef");
        assert!(result.is_err());
    }

    #[test]
    fn register_funcdef_missing_funcdef_keyword() {
        let mut module = Module::<'static>::root();

        // Missing "funcdef" keyword
        let result = module.register_funcdef("void Callback()");
        assert!(result.is_err());
    }
}
