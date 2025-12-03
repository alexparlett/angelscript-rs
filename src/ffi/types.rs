//! Core types for FFI registration.
//!
//! These types are used during registration to specify type information.
//! Type specifications use AST primitives parsed from declaration strings.
//!
//! IDs are assigned at registration time using the global atomic counters
//! (`TypeId::next()` and `FunctionId::next()`).

use crate::ast::{FunctionParam, Ident, ReturnType, TypeExpr};
use crate::semantic::types::type_def::{FunctionId, FunctionTraits, TypeId, Visibility};
use crate::semantic::types::DataType;

use super::list_buffer::ListPattern;
use super::native_fn::NativeFn;

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
        /// Plain Old Data - no constructor/destructor needed, can memcpy
        is_pod: bool,
    },

    /// Reference type - heap allocated via factory, handle semantics.
    /// The `kind` field specifies the reference semantics.
    Reference {
        /// The kind of reference type
        kind: ReferenceKind,
    },
}

impl TypeKind {
    /// Create a value type kind with size and alignment from a type.
    pub fn value<T>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_pod: false,
        }
    }

    /// Create a POD value type kind.
    pub fn pod<T>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_pod: true,
        }
    }

    /// Create a standard reference type kind.
    pub fn reference() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::Standard,
        }
    }

    /// Create a scoped reference type kind.
    pub fn scoped() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::Scoped,
        }
    }

    /// Create a single-ref type kind.
    pub fn single_ref() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::SingleRef,
        }
    }

    /// Create a generic handle type kind.
    pub fn generic_handle() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::GenericHandle,
        }
    }

    /// Check if this is a value type.
    pub fn is_value(&self) -> bool {
        matches!(self, TypeKind::Value { .. })
    }

    /// Check if this is a reference type.
    pub fn is_reference(&self) -> bool {
        matches!(self, TypeKind::Reference { .. })
    }

    /// Check if this is a POD type.
    pub fn is_pod(&self) -> bool {
        matches!(self, TypeKind::Value { is_pod: true, .. })
    }
}

/// Reference type variants for different ownership/lifetime semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// Standard reference type - full handle support with AddRef/Release ref counting.
    Standard,

    /// Scoped reference type - RAII-style, destroyed at scope exit, no handles.
    Scoped,

    /// Single-ref type - app-controlled lifetime, no handles in script.
    SingleRef,

    /// Generic handle - type-erased container that can hold any type.
    GenericHandle,
}

/// List construction behavior with its pattern.
///
/// Used by `list_construct` and `list_factory` to define how initialization
/// lists are processed.
#[derive(Debug)]
pub struct ListBehavior {
    /// Native function to call with the list data
    pub native_fn: NativeFn,
    /// Expected list pattern (repeat, fixed, or repeat-tuple)
    pub pattern: ListPattern,
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
    /// List constructor - construct from initialization list (value types)
    /// Used for syntax like: `MyStruct s = {1, 2, 3};`
    pub list_construct: Option<ListBehavior>,
    /// List factory - create from initialization list (reference types)
    /// Used for syntax like: `array<int> a = {1, 2, 3};`
    pub list_factory: Option<ListBehavior>,
}

impl Behaviors {
    /// Create empty behaviors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if has addref behavior.
    pub fn has_addref(&self) -> bool {
        self.addref.is_some()
    }

    /// Check if has release behavior.
    pub fn has_release(&self) -> bool {
        self.release.is_some()
    }

    /// Check if has destruct behavior.
    pub fn has_destruct(&self) -> bool {
        self.destruct.is_some()
    }

    /// Check if has list_construct behavior.
    pub fn has_list_construct(&self) -> bool {
        self.list_construct.is_some()
    }

    /// Check if has list_factory behavior.
    pub fn has_list_factory(&self) -> bool {
        self.list_factory.is_some()
    }
}

/// Information about a template instantiation for validation callback.
#[derive(Debug, Clone)]
pub struct TemplateInstanceInfo {
    /// The template name (e.g., "array")
    pub template_name: String,
    /// The type arguments (e.g., [int] for array<int>)
    pub sub_types: Vec<DataType>,
}

impl TemplateInstanceInfo {
    /// Create a new template instance info.
    pub fn new(template_name: impl Into<String>, sub_types: Vec<DataType>) -> Self {
        Self {
            template_name: template_name.into(),
            sub_types,
        }
    }
}

/// Result of template validation callback.
#[derive(Debug, Clone)]
pub struct TemplateValidation {
    /// Is this instantiation valid?
    pub is_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Should this instance use garbage collection?
    pub needs_gc: bool,
}

impl TemplateValidation {
    /// Create a valid template validation result.
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error: None,
            needs_gc: false,
        }
    }

    /// Create an invalid template validation result with an error message.
    pub fn invalid(msg: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error: Some(msg.into()),
            needs_gc: false,
        }
    }

    /// Create a valid result that needs garbage collection.
    pub fn with_gc() -> Self {
        Self {
            is_valid: true,
            error: None,
            needs_gc: true,
        }
    }
}

impl Default for TemplateValidation {
    fn default() -> Self {
        Self::valid()
    }
}

/// Native function registration (global functions).
/// Uses AST primitives: Ident, FunctionParam, ReturnType.
#[derive(Debug)]
pub struct NativeFunctionDef<'ast> {
    /// Unique function ID (assigned at registration via FunctionId::next())
    pub id: FunctionId,
    /// Function name
    pub name: Ident<'ast>,
    /// Parameter definitions (parsed from declaration string)
    pub params: &'ast [FunctionParam<'ast>],
    /// Return type (parsed from declaration string)
    pub return_type: ReturnType<'ast>,
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

/// Native type registration (value types, reference types).
/// Uses AST primitives: Ident for template params.
pub struct NativeTypeDef<'ast> {
    /// Unique type ID (assigned at registration via TypeId::next())
    pub id: TypeId,
    /// Type name (unqualified)
    pub name: String,
    /// Template parameters (e.g., ["T"] or ["K", "V"])
    pub template_params: Option<&'ast [Ident<'ast>]>,
    /// Type kind (value or reference)
    pub type_kind: TypeKind,
    /// Object behaviors
    pub behaviors: Behaviors,
    /// Constructors
    pub constructors: Vec<NativeMethodDef<'ast>>,
    /// Factory functions (for reference types)
    pub factories: Vec<NativeMethodDef<'ast>>,
    /// Methods
    pub methods: Vec<NativeMethodDef<'ast>>,
    /// Properties
    pub properties: Vec<NativePropertyDef<'ast>>,
    /// Operators
    pub operators: Vec<NativeMethodDef<'ast>>,
    /// Template callback for validation (if this is a template type)
    pub template_callback:
        Option<Box<dyn Fn(&TemplateInstanceInfo) -> TemplateValidation + Send + Sync>>,
    /// Rust TypeId for runtime type checking
    pub rust_type_id: std::any::TypeId,
}

impl<'ast> std::fmt::Debug for NativeTypeDef<'ast> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeTypeDef")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("template_params", &self.template_params)
            .field("type_kind", &self.type_kind)
            .field("behaviors", &self.behaviors)
            .field("constructors", &self.constructors)
            .field("factories", &self.factories)
            .field("methods", &self.methods)
            .field("properties", &self.properties)
            .field("operators", &self.operators)
            .field("template_callback", &self.template_callback.as_ref().map(|_| "..."))
            .field("rust_type_id", &self.rust_type_id)
            .finish()
    }
}

/// Native method - same structure as NativeFunctionDef but for class methods.
#[derive(Debug)]
pub struct NativeMethodDef<'ast> {
    /// Method name
    pub name: Ident<'ast>,
    /// Parameter definitions
    pub params: &'ast [FunctionParam<'ast>],
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Whether this is a const method
    pub is_const: bool,
    /// The native function implementation
    pub native_fn: NativeFn,
}

/// A property definition with getter and optional setter.
/// Uses AST primitives for type.
#[derive(Debug)]
pub struct NativePropertyDef<'ast> {
    /// Property name
    pub name: Ident<'ast>,
    /// Property type
    pub ty: &'ast TypeExpr<'ast>,
    /// Whether this is read-only
    pub is_const: bool,
    /// Getter function
    pub getter: NativeFn,
    /// Setter function (if writable)
    pub setter: Option<NativeFn>,
}

/// Native interface definition.
/// Interfaces define abstract method signatures that classes can implement.
/// Uses AST primitives for method signatures.
#[derive(Debug)]
pub struct NativeInterfaceDef<'ast> {
    /// Unique type ID (assigned at registration via TypeId::next())
    pub id: TypeId,
    /// Interface name
    pub name: String,
    /// Abstract method signatures
    pub methods: Vec<NativeInterfaceMethod<'ast>>,
}

/// An abstract method signature in an interface.
/// Interface methods have no implementation (no FunctionId).
#[derive(Debug)]
pub struct NativeInterfaceMethod<'ast> {
    /// Method name
    pub name: Ident<'ast>,
    /// Parameter definitions
    pub params: &'ast [FunctionParam<'ast>],
    /// Return type
    pub return_type: ReturnType<'ast>,
    /// Whether this is a const method
    pub is_const: bool,
}

/// Native funcdef (function pointer type) definition.
/// Uses AST primitives for the function signature.
#[derive(Debug)]
pub struct NativeFuncdefDef<'ast> {
    /// Unique type ID (assigned at registration via TypeId::next())
    pub id: TypeId,
    /// Funcdef name
    pub name: Ident<'ast>,
    /// Parameter definitions
    pub params: &'ast [FunctionParam<'ast>],
    /// Return type
    pub return_type: ReturnType<'ast>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_kind_value() {
        let kind = TypeKind::value::<i32>();
        match kind {
            TypeKind::Value {
                size,
                align,
                is_pod,
            } => {
                assert_eq!(size, 4);
                assert_eq!(align, 4);
                assert!(!is_pod);
            }
            _ => panic!("Expected Value variant"),
        }
        assert!(kind.is_value());
        assert!(!kind.is_reference());
        assert!(!kind.is_pod());
    }

    #[test]
    fn type_kind_pod() {
        let kind = TypeKind::pod::<i32>();
        match kind {
            TypeKind::Value {
                size,
                align,
                is_pod,
            } => {
                assert_eq!(size, 4);
                assert_eq!(align, 4);
                assert!(is_pod);
            }
            _ => panic!("Expected Value variant"),
        }
        assert!(kind.is_pod());
    }

    #[test]
    fn type_kind_reference() {
        let kind = TypeKind::reference();
        assert!(kind.is_reference());
        assert!(!kind.is_value());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::Standard);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_scoped() {
        let kind = TypeKind::scoped();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::Scoped);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_single_ref() {
        let kind = TypeKind::single_ref();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::SingleRef);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_generic_handle() {
        let kind = TypeKind::generic_handle();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::GenericHandle);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn behaviors_default() {
        let behaviors = Behaviors::new();
        assert!(!behaviors.has_addref());
        assert!(!behaviors.has_release());
        assert!(!behaviors.has_destruct());
    }

    #[test]
    fn behaviors_with_addref() {
        use super::super::native_fn::CallContext;
        let behaviors = Behaviors {
            addref: Some(NativeFn::new(|_: &mut CallContext| Ok(()))),
            ..Default::default()
        };
        assert!(behaviors.has_addref());
        assert!(!behaviors.has_release());
    }

    #[test]
    fn behaviors_debug() {
        let behaviors = Behaviors::new();
        let debug = format!("{:?}", behaviors);
        assert!(debug.contains("Behaviors"));
        assert!(debug.contains("addref"));
    }

    #[test]
    fn template_instance_info_new() {
        let info = TemplateInstanceInfo::new("array", vec![]);
        assert_eq!(info.template_name, "array");
        assert!(info.sub_types.is_empty());
    }

    #[test]
    fn template_validation_valid() {
        let v = TemplateValidation::valid();
        assert!(v.is_valid);
        assert!(v.error.is_none());
        assert!(!v.needs_gc);
    }

    #[test]
    fn template_validation_invalid() {
        let v = TemplateValidation::invalid("Key must be hashable");
        assert!(!v.is_valid);
        assert_eq!(v.error, Some("Key must be hashable".to_string()));
        assert!(!v.needs_gc);
    }

    #[test]
    fn template_validation_with_gc() {
        let v = TemplateValidation::with_gc();
        assert!(v.is_valid);
        assert!(v.error.is_none());
        assert!(v.needs_gc);
    }

    #[test]
    fn template_validation_default() {
        let v = TemplateValidation::default();
        assert!(v.is_valid);
    }

    #[test]
    fn native_interface_def_debug() {
        let interface = NativeInterfaceDef {
            id: TypeId::next(),
            name: "ISerializable".to_string(),
            methods: Vec::new(),
        };
        let debug = format!("{:?}", interface);
        assert!(debug.contains("NativeInterfaceDef"));
        assert!(debug.contains("ISerializable"));
    }

}
