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

use crate::ast::{FunctionSignatureDecl, Parser, PropertyDecl};
use crate::ffi::{
    GlobalPropertyDef, IntoNativeFn, NativeCallable, NativeFn, NativeFuncdefDef, NativeFunctionDef,
    NativeInterfaceDef, NativeTypeDef,
};
use crate::semantic::types::type_def::{FunctionId, FunctionTraits, TypeId, Visibility};

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
    /// Arena for storing parsed AST nodes
    arena: Bump,

    /// Namespace path for all items.
    /// Empty = root namespace, ["math"] = single level,
    /// ["std", "collections"] = nested namespace
    namespace: Vec<String>,

    /// Registered native functions
    /// The 'static lifetime is transmuted - actual lifetime is tied to arena
    functions: Vec<NativeFunctionDef<'static>>,

    /// Registered native types
    /// The 'static lifetime is transmuted - actual lifetime is tied to arena
    types: Vec<NativeTypeDef<'static>>,

    /// Registered enums (name -> values)
    enums: Vec<NativeEnumDef>,

    /// Registered interfaces
    /// The 'static lifetime is transmuted - actual lifetime is tied to arena
    interfaces: Vec<NativeInterfaceDef<'static>>,

    /// Registered funcdefs (function pointer types)
    /// The 'static lifetime is transmuted - actual lifetime is tied to arena
    funcdefs: Vec<NativeFuncdefDef<'static>>,

    /// Registered templates
    templates: Vec<NativeTemplateDef>,

    /// Global properties (app-owned references)
    /// The lifetime is tied to the module's arena via a transmute in register_global_property
    global_properties: Vec<GlobalPropertyDef<'static, 'app>>,
}

/// A native enum definition.
#[derive(Debug, Clone)]
pub struct NativeEnumDef {
    /// Unique type ID (assigned at registration via TypeId::next())
    pub id: TypeId,
    /// Enum name
    pub name: String,
    /// Enum values (name -> value)
    pub values: Vec<(String, i64)>,
}

/// A native template definition (placeholder for Task 06).
#[derive(Debug)]
pub struct NativeTemplateDef {
    /// Template name
    pub name: String,
    /// Number of type parameters
    pub param_count: usize,
    // Validator and instance builder will be added in Task 06
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
            templates: Vec::new(),
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

    /// Internal helper to build a NativeFunctionDef from parsed signature.
    fn build_function_def(
        &self,
        sig: FunctionSignatureDecl<'_>,
        native_fn: NativeFn,
    ) -> NativeFunctionDef<'static> {
        // Build function traits from the parsed signature
        let mut traits = FunctionTraits::default();
        if sig.is_const {
            traits.is_const = true;
        }
        // Note: property attribute is stored in FuncAttr but not in FunctionTraits
        // This will be used during semantic analysis if needed

        // SAFETY: The arena is owned by self and lives as long as self.
        // We transmute the lifetime to 'static for storage, but the actual
        // lifetime is tied to the module. This is safe because:
        // 1. The arena is never moved or replaced
        // 2. The functions vec is dropped before the arena
        // 3. We never expose references with incorrect lifetimes
        let name = unsafe { std::mem::transmute(sig.name) };
        let params = unsafe { std::mem::transmute(sig.params) };
        let return_type = unsafe { std::mem::transmute(sig.return_type) };

        NativeFunctionDef {
            id: FunctionId::next(),
            name,
            params,
            return_type,
            object_type: None, // Global functions have no object type
            traits,
            default_exprs: Vec::new(), // TODO: Parse default expressions in Task 04
            visibility: Visibility::Public,
            native_fn,
        }
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
    pub fn functions(&self) -> &[NativeFunctionDef<'static>] {
        &self.functions
    }

    // =========================================================================
    // Type registration (placeholder - ClassBuilder in Task 04)
    // =========================================================================

    /// Register a native type (placeholder).
    ///
    /// The full implementation with ClassBuilder will be added in Task 04.
    /// For now, this stores a raw NativeTypeDef.
    pub fn add_type(&mut self, type_def: NativeTypeDef<'static>) {
        self.types.push(type_def);
    }

    /// Get the registered types.
    pub fn types(&self) -> &[NativeTypeDef<'static>] {
        &self.types
    }

    // =========================================================================
    // Enum registration (placeholder - EnumBuilder in Task 05)
    // =========================================================================

    /// Register a native enum (placeholder).
    ///
    /// The full implementation with EnumBuilder will be added in Task 05.
    pub fn add_enum(&mut self, enum_def: NativeEnumDef) {
        self.enums.push(enum_def);
    }

    /// Get the registered enums.
    pub fn enums(&self) -> &[NativeEnumDef] {
        &self.enums
    }

    // =========================================================================
    // Interface registration (placeholder - InterfaceBuilder in Task 05)
    // =========================================================================

    /// Register a native interface (placeholder).
    ///
    /// The full implementation with InterfaceBuilder will be added in Task 05.
    /// For now, this stores a raw NativeInterfaceDef.
    pub fn add_interface(&mut self, interface_def: NativeInterfaceDef<'static>) {
        self.interfaces.push(interface_def);
    }

    /// Get the registered interfaces.
    pub fn interfaces(&self) -> &[NativeInterfaceDef<'static>] {
        &self.interfaces
    }

    // =========================================================================
    // Funcdef registration (placeholder - will be implemented in Task 05)
    // =========================================================================

    /// Register a native funcdef (placeholder).
    ///
    /// The full implementation will be added in Task 05.
    /// For now, this stores a raw NativeFuncdefDef.
    pub fn add_funcdef(&mut self, funcdef_def: NativeFuncdefDef<'static>) {
        self.funcdefs.push(funcdef_def);
    }

    /// Get the registered funcdefs.
    pub fn funcdefs(&self) -> &[NativeFuncdefDef<'static>] {
        &self.funcdefs
    }

    // =========================================================================
    // Template registration (placeholder - TemplateBuilder in Task 06)
    // =========================================================================

    /// Register a native template (placeholder).
    ///
    /// The full implementation with TemplateBuilder will be added in Task 06.
    pub fn add_template(&mut self, template_def: NativeTemplateDef) {
        self.templates.push(template_def);
    }

    /// Get the registered templates.
    pub fn templates(&self) -> &[NativeTemplateDef] {
        &self.templates
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
            + self.templates.len()
            + self.global_properties.len()
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
            .field("templates", &self.templates)
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

        module.add_enum(NativeEnumDef {
            id: TypeId::next(),
            name: "Color".to_string(),
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        });

        assert_eq!(module.enums().len(), 1);
        assert_eq!(module.enums()[0].name, "Color");
        assert_eq!(module.enums()[0].values.len(), 3);
    }

    #[test]
    fn module_add_template() {
        let mut module = Module::<'static>::root();

        module.add_template(NativeTemplateDef {
            name: "array".to_string(),
            param_count: 1,
        });

        assert_eq!(module.templates().len(), 1);
        assert_eq!(module.templates()[0].name, "array");
        assert_eq!(module.templates()[0].param_count, 1);
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
    fn native_enum_def_clone() {
        let enum_def = NativeEnumDef {
            id: TypeId::next(),
            name: "Color".to_string(),
            values: vec![("Red".to_string(), 0)],
        };
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
        assert_eq!(module.functions()[0].name.name, "add");
        assert_eq!(module.functions()[0].params.len(), 2);
    }

    #[test]
    fn register_fn_no_params() {
        let mut module = Module::<'static>::root();

        module
            .register_fn("float pi()", || std::f64::consts::PI)
            .unwrap();

        assert_eq!(module.functions().len(), 1);
        assert_eq!(module.functions()[0].name.name, "pi");
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
        assert_eq!(module.functions()[0].name.name, "greet");
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
        assert_eq!(module.functions()[0].name.name, "add");
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
}
