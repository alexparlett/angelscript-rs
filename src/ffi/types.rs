//! Core types for FFI registration.
//!
//! These types are used during registration to specify type information.
//! They are converted to semantic analysis types during `apply_to_registry()`.

use crate::semantic::types::data_type::RefModifier;
use crate::semantic::types::type_def::{FunctionTraits, Visibility};

use super::native_fn::NativeFn;

/// AngelScript type specification - stored explicitly, NOT inferred from Rust types.
///
/// This allows declaring signatures like `int@` (handle to primitive) that have
/// no Rust equivalent, or `const Foo@` vs `Foo @const` distinctions.
///
/// # Examples
///
/// ```ignore
/// // Simple type: int
/// TypeSpec::simple("int")
///
/// // Const type: const int
/// TypeSpec::new("int").with_const()
///
/// // Handle: Foo@
/// TypeSpec::new("Foo").with_handle()
///
/// // Handle to const: const Foo@
/// TypeSpec::new("Foo").with_handle().with_handle_to_const()
///
/// // Reference parameter: const string &in
/// TypeSpec::new("string").with_const().with_ref(RefModifier::In)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSpec {
    /// The type name (resolved to TypeId during apply)
    pub type_name: String,
    /// `const T` - the value is const
    pub is_const: bool,
    /// `T@` - this is a handle
    pub is_handle: bool,
    /// `const T@` - handle points to const
    pub is_handle_to_const: bool,
    /// `T@+` - auto handle (automatic AddRef/Release)
    pub is_auto_handle: bool,
    /// Reference modifier (&in, &out, &inout)
    pub ref_modifier: RefModifier,
}

impl TypeSpec {
    /// Create a simple type specification with no modifiers.
    pub fn simple(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            is_auto_handle: false,
            ref_modifier: RefModifier::None,
        }
    }

    /// Create a new type specification (same as `simple`).
    pub fn new(type_name: impl Into<String>) -> Self {
        Self::simple(type_name)
    }

    /// Create a void type specification.
    pub fn void() -> Self {
        Self::simple("void")
    }

    /// Mark as const.
    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    /// Mark as a handle (`T@`).
    pub fn with_handle(mut self) -> Self {
        self.is_handle = true;
        self
    }

    /// Mark as handle to const (`const T@`).
    pub fn with_handle_to_const(mut self) -> Self {
        self.is_handle_to_const = true;
        self
    }

    /// Mark as auto handle (`T@+`).
    pub fn with_auto_handle(mut self) -> Self {
        self.is_auto_handle = true;
        self
    }

    /// Set reference modifier.
    pub fn with_ref(mut self, modifier: RefModifier) -> Self {
        self.ref_modifier = modifier;
        self
    }

    /// Set as `&in` reference.
    pub fn ref_in(mut self) -> Self {
        self.ref_modifier = RefModifier::In;
        self
    }

    /// Set as `&out` reference.
    pub fn ref_out(mut self) -> Self {
        self.ref_modifier = RefModifier::Out;
        self
    }

    /// Set as `&inout` reference.
    pub fn ref_inout(mut self) -> Self {
        self.ref_modifier = RefModifier::InOut;
        self
    }

    /// Check if this is a void type.
    pub fn is_void(&self) -> bool {
        self.type_name == "void"
            && !self.is_const
            && !self.is_handle
            && self.ref_modifier == RefModifier::None
    }

    /// Check if this is a variable parameter type (`?`).
    pub fn is_any_type(&self) -> bool {
        self.type_name == "?"
    }
}

impl Default for TypeSpec {
    fn default() -> Self {
        Self::void()
    }
}

/// A parameter definition for FFI functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDef {
    /// Parameter name
    pub name: String,
    /// Parameter type specification
    pub type_spec: TypeSpec,
}

impl ParamDef {
    /// Create a new parameter definition.
    pub fn new(name: impl Into<String>, type_spec: TypeSpec) -> Self {
        Self {
            name: name.into(),
            type_spec,
        }
    }

    /// Create a variable parameter (`?&in`).
    pub fn any_in(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_spec: TypeSpec::new("?").ref_in(),
        }
    }

    /// Create a variable out parameter (`?&out`).
    pub fn any_out(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_spec: TypeSpec::new("?").ref_out(),
        }
    }
}

/// Type kind determines memory semantics for registered types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    /// Value type - stack allocated, copied on assignment.
    /// Requires: constructor, destructor, copy/assignment behaviors.
    Value {
        /// Size in bytes for stack allocation
        size: usize,
        /// Alignment requirement
        align: usize,
    },

    /// Reference type - heap allocated via factory, handle semantics.
    /// Requires: factory, addref, release behaviors.
    Reference,

    /// Scoped reference type - RAII-style, destroyed at scope exit, no handles.
    Scoped,

    /// Single-ref type - app-controlled lifetime, no handles in script.
    SingleRef,
}

impl TypeKind {
    /// Create a value type kind with size and alignment from a type.
    pub fn value<T>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }

    /// Create a reference type kind.
    pub fn reference() -> Self {
        TypeKind::Reference
    }

    /// Check if this is a value type.
    pub fn is_value(&self) -> bool {
        matches!(self, TypeKind::Value { .. })
    }

    /// Check if this is a reference type.
    pub fn is_reference(&self) -> bool {
        matches!(self, TypeKind::Reference)
    }
}

/// Object behaviors for lifecycle management.
///
/// These are registered but executed by the VM. The FFI layer stores
/// the function pointers; the VM calls them at appropriate times.
#[derive(Debug, Default)]
pub struct Behaviors {
    /// Factory function - creates new instance (reference types)
    pub factory: Option<NativeFn>,
    /// AddRef - increment reference count (reference types)
    pub addref: Option<NativeFn>,
    /// Release - decrement reference count, delete if zero (reference types)
    pub release: Option<NativeFn>,
    /// Constructor - initialize value in pre-allocated memory (value types)
    pub construct: Option<NativeFn>,
    /// Destructor - cleanup before deallocation (value types)
    pub destruct: Option<NativeFn>,
    /// Copy constructor - initialize from another instance (value types)
    pub copy_construct: Option<NativeFn>,
    /// Assignment - copy contents from another instance
    pub assign: Option<NativeFn>,
}

impl Behaviors {
    /// Create empty behaviors.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Internal storage for a native function definition.
///
/// This is the lifetime-free internal representation that gets converted
/// to `FunctionDef` during `apply_to_registry()`.
#[derive(Debug)]
pub struct NativeFunctionDef {
    /// Function name
    pub name: String,
    /// Parameter definitions
    pub params: Vec<ParamDef>,
    /// Return type specification
    pub return_type: TypeSpec,
    /// Owning type name for methods (None for global functions)
    pub object_type: Option<String>,
    /// Function traits (const, constructor, etc.)
    pub traits: FunctionTraits,
    /// Default argument expressions (parsed during apply)
    pub default_exprs: Vec<Option<String>>,
    /// Function visibility
    pub visibility: Visibility,
    /// The native function implementation
    pub native_fn: NativeFn,
}

impl NativeFunctionDef {
    /// Create a new native function definition.
    pub fn new(name: impl Into<String>, native_fn: NativeFn) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
            return_type: TypeSpec::void(),
            object_type: None,
            traits: FunctionTraits::new(),
            default_exprs: Vec::new(),
            visibility: Visibility::Public,
            native_fn,
        }
    }
}

/// Internal storage for a native type definition.
///
/// This is used by ClassBuilder to collect type information before
/// it's applied to the registry.
#[derive(Debug)]
pub struct NativeTypeDef {
    /// Type name (unqualified)
    pub name: String,
    /// Type kind (value or reference)
    pub type_kind: TypeKind,
    /// Object behaviors
    pub behaviors: Behaviors,
    /// Constructors
    pub constructors: Vec<NativeFunctionDef>,
    /// Methods
    pub methods: Vec<NativeFunctionDef>,
    /// Properties (name -> (getter, setter))
    pub properties: Vec<NativePropertyDef>,
    /// Rust TypeId for runtime type checking
    pub rust_type_id: std::any::TypeId,
}

/// A property definition with optional getter and setter.
#[derive(Debug)]
pub struct NativePropertyDef {
    /// Property name
    pub name: String,
    /// Property type
    pub type_spec: TypeSpec,
    /// Getter function (if readable)
    pub getter: Option<NativeFn>,
    /// Setter function (if writable)
    pub setter: Option<NativeFn>,
    /// Visibility
    pub visibility: Visibility,
}

impl NativePropertyDef {
    /// Create a new property definition.
    pub fn new(name: impl Into<String>, type_spec: TypeSpec) -> Self {
        Self {
            name: name.into(),
            type_spec,
            getter: None,
            setter: None,
            visibility: Visibility::Public,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_spec_simple() {
        let spec = TypeSpec::simple("int");
        assert_eq!(spec.type_name, "int");
        assert!(!spec.is_const);
        assert!(!spec.is_handle);
        assert!(!spec.is_handle_to_const);
        assert!(!spec.is_auto_handle);
        assert_eq!(spec.ref_modifier, RefModifier::None);
    }

    #[test]
    fn type_spec_void() {
        let spec = TypeSpec::void();
        assert_eq!(spec.type_name, "void");
        assert!(spec.is_void());
    }

    #[test]
    fn type_spec_with_modifiers() {
        let spec = TypeSpec::new("Foo")
            .with_const()
            .with_handle()
            .with_handle_to_const();
        assert_eq!(spec.type_name, "Foo");
        assert!(spec.is_const);
        assert!(spec.is_handle);
        assert!(spec.is_handle_to_const);
    }

    #[test]
    fn type_spec_with_ref() {
        let spec = TypeSpec::new("string").with_const().ref_in();
        assert!(spec.is_const);
        assert_eq!(spec.ref_modifier, RefModifier::In);
    }

    #[test]
    fn type_spec_ref_out() {
        let spec = TypeSpec::new("int").ref_out();
        assert_eq!(spec.ref_modifier, RefModifier::Out);
    }

    #[test]
    fn type_spec_ref_inout() {
        let spec = TypeSpec::new("MyClass").ref_inout();
        assert_eq!(spec.ref_modifier, RefModifier::InOut);
    }

    #[test]
    fn type_spec_auto_handle() {
        let spec = TypeSpec::new("Foo").with_handle().with_auto_handle();
        assert!(spec.is_handle);
        assert!(spec.is_auto_handle);
    }

    #[test]
    fn type_spec_is_any_type() {
        let spec = TypeSpec::new("?").ref_in();
        assert!(spec.is_any_type());

        let spec = TypeSpec::new("int");
        assert!(!spec.is_any_type());
    }

    #[test]
    fn type_spec_default_is_void() {
        let spec = TypeSpec::default();
        assert!(spec.is_void());
    }

    #[test]
    fn param_def_new() {
        let param = ParamDef::new("count", TypeSpec::simple("int"));
        assert_eq!(param.name, "count");
        assert_eq!(param.type_spec.type_name, "int");
    }

    #[test]
    fn param_def_any_in() {
        let param = ParamDef::any_in("value");
        assert_eq!(param.name, "value");
        assert!(param.type_spec.is_any_type());
        assert_eq!(param.type_spec.ref_modifier, RefModifier::In);
    }

    #[test]
    fn param_def_any_out() {
        let param = ParamDef::any_out("result");
        assert_eq!(param.name, "result");
        assert!(param.type_spec.is_any_type());
        assert_eq!(param.type_spec.ref_modifier, RefModifier::Out);
    }

    #[test]
    fn type_kind_value() {
        let kind = TypeKind::value::<i32>();
        match kind {
            TypeKind::Value { size, align } => {
                assert_eq!(size, 4);
                assert_eq!(align, 4);
            }
            _ => panic!("Expected Value variant"),
        }
        assert!(kind.is_value());
        assert!(!kind.is_reference());
    }

    #[test]
    fn type_kind_reference() {
        let kind = TypeKind::reference();
        assert!(kind.is_reference());
        assert!(!kind.is_value());
    }

    #[test]
    fn behaviors_default() {
        let behaviors = Behaviors::new();
        assert!(behaviors.factory.is_none());
        assert!(behaviors.addref.is_none());
        assert!(behaviors.release.is_none());
        assert!(behaviors.construct.is_none());
        assert!(behaviors.destruct.is_none());
        assert!(behaviors.copy_construct.is_none());
        assert!(behaviors.assign.is_none());
    }

    #[test]
    fn native_property_def_new() {
        let prop = NativePropertyDef::new("health", TypeSpec::simple("int"));
        assert_eq!(prop.name, "health");
        assert_eq!(prop.type_spec.type_name, "int");
        assert!(prop.getter.is_none());
        assert!(prop.setter.is_none());
        assert_eq!(prop.visibility, Visibility::Public);
    }
}
