//! TypeDef - type definitions for the AngelScript type system.
//!
//! This module provides `TypeDef`, an enum representing all type kinds in AngelScript:
//! primitives, classes, interfaces, enums, funcdefs, and template parameters.
//!
//! # Example
//!
//! ```
//! use angelscript_core::{TypeDef, TypeHash, PrimitiveKind};
//!
//! // Create a primitive type
//! let int_type = TypeDef::Primitive {
//!     kind: PrimitiveKind::Int32,
//!     type_hash: TypeHash::from_name("int"),
//! };
//! assert!(int_type.is_primitive());
//! assert_eq!(int_type.name(), "int");
//! ```

use std::fmt;

use rustc_hash::FxHashMap;

use crate::{DataType, TypeHash};

/// Primitive type kinds.
///
/// These are the built-in numeric and boolean types in AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float,
    Double,
}

impl PrimitiveKind {
    /// Get the TypeHash for this primitive type.
    pub const fn type_hash(self) -> TypeHash {
        use crate::primitives;
        match self {
            PrimitiveKind::Void => primitives::VOID,
            PrimitiveKind::Bool => primitives::BOOL,
            PrimitiveKind::Int8 => primitives::INT8,
            PrimitiveKind::Int16 => primitives::INT16,
            PrimitiveKind::Int32 => primitives::INT32,
            PrimitiveKind::Int64 => primitives::INT64,
            PrimitiveKind::Uint8 => primitives::UINT8,
            PrimitiveKind::Uint16 => primitives::UINT16,
            PrimitiveKind::Uint32 => primitives::UINT32,
            PrimitiveKind::Uint64 => primitives::UINT64,
            PrimitiveKind::Float => primitives::FLOAT,
            PrimitiveKind::Double => primitives::DOUBLE,
        }
    }

    /// Get the name of this primitive type.
    pub const fn name(self) -> &'static str {
        match self {
            PrimitiveKind::Void => "void",
            PrimitiveKind::Bool => "bool",
            PrimitiveKind::Int8 => "int8",
            PrimitiveKind::Int16 => "int16",
            PrimitiveKind::Int32 => "int",
            PrimitiveKind::Int64 => "int64",
            PrimitiveKind::Uint8 => "uint8",
            PrimitiveKind::Uint16 => "uint16",
            PrimitiveKind::Uint32 => "uint",
            PrimitiveKind::Uint64 => "uint64",
            PrimitiveKind::Float => "float",
            PrimitiveKind::Double => "double",
        }
    }
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Visibility modifier for class members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Public,
    Protected,
    Private,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "public"),
            Visibility::Protected => write!(f, "protected"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

/// Type kind determines memory semantics for types.
///
/// This enum is used both during FFI registration (to specify how native types
/// should be managed) and in the semantic layer (to determine constructor vs
/// factory lookup during type instantiation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    /// Value type - stack allocated, copied on assignment.
    /// Requires: constructor, destructor, copy/assignment behaviors.
    /// Uses constructors for instantiation.
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
    /// Uses factories for instantiation (FFI types like array, dictionary).
    Reference {
        /// The kind of reference type
        kind: ReferenceKind,
    },

    /// Script object - reference semantics but VM-managed allocation.
    /// Uses constructors for instantiation (VM handles allocation).
    /// This is the type kind for all script-defined classes.
    ScriptObject,
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

    /// Create a value type kind with explicit size and alignment.
    pub const fn value_sized(size: usize, align: usize, is_pod: bool) -> Self {
        TypeKind::Value {
            size,
            align,
            is_pod,
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

    /// Create a script object type kind.
    pub fn script_object() -> Self {
        TypeKind::ScriptObject
    }

    /// Check if this is a value type.
    pub fn is_value(&self) -> bool {
        matches!(self, TypeKind::Value { .. })
    }

    /// Check if this is a reference type (FFI reference, uses factories).
    pub fn is_reference(&self) -> bool {
        matches!(self, TypeKind::Reference { .. })
    }

    /// Check if this is a script object (reference semantics, uses constructors).
    pub fn is_script_object(&self) -> bool {
        matches!(self, TypeKind::ScriptObject)
    }

    /// Check if this type uses factories for instantiation.
    /// Only FFI Reference types use factories.
    pub fn uses_factories(&self) -> bool {
        matches!(self, TypeKind::Reference { .. })
    }

    /// Check if this type uses constructors for instantiation.
    /// Value types and ScriptObjects use constructors.
    pub fn uses_constructors(&self) -> bool {
        matches!(self, TypeKind::Value { .. } | TypeKind::ScriptObject)
    }

    /// Check if this is a POD type.
    pub fn is_pod(&self) -> bool {
        matches!(self, TypeKind::Value { is_pod: true, .. })
    }
}

impl Default for TypeKind {
    /// Default to script object (most common for script-defined classes).
    fn default() -> Self {
        TypeKind::ScriptObject
    }
}

/// Reference type variants for different ownership/lifetime semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferenceKind {
    /// Standard reference type - full handle support with AddRef/Release ref counting.
    #[default]
    Standard,

    /// Scoped reference type - RAII-style, destroyed at scope exit, no handles.
    Scoped,

    /// Single-ref type - app-controlled lifetime, no handles in script.
    SingleRef,

    /// Generic handle - type-erased container that can hold any type.
    GenericHandle,
}

/// A field definition in a class.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    /// Field name.
    pub name: String,
    /// Field type.
    pub data_type: DataType,
    /// Field visibility.
    pub visibility: Visibility,
}

impl FieldDef {
    /// Create a new field definition.
    pub fn new(name: impl Into<String>, data_type: DataType, visibility: Visibility) -> Self {
        Self {
            name: name.into(),
            data_type,
            visibility,
        }
    }

    /// Create a public field.
    pub fn public(name: impl Into<String>, data_type: DataType) -> Self {
        Self::new(name, data_type, Visibility::Public)
    }

    /// Create a private field.
    pub fn private(name: impl Into<String>, data_type: DataType) -> Self {
        Self::new(name, data_type, Visibility::Private)
    }
}

/// A method signature for interfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    /// Method name.
    pub name: String,
    /// Parameter types.
    pub params: Vec<DataType>,
    /// Return type.
    pub return_type: DataType,
    /// Whether the method is const.
    pub is_const: bool,
}

impl MethodSignature {
    /// Create a new method signature.
    pub fn new(name: impl Into<String>, params: Vec<DataType>, return_type: DataType) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const: false,
        }
    }

    /// Create a new const method signature.
    pub fn new_const(
        name: impl Into<String>,
        params: Vec<DataType>,
        return_type: DataType,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            return_type,
            is_const: true,
        }
    }
}

/// Property accessor function hashes.
///
/// Properties can have:
/// - Read-only: getter only
/// - Write-only: setter only
/// - Read-write: both getter and setter
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PropertyAccessors {
    /// Getter function hash (must be const method).
    pub getter: Option<TypeHash>,
    /// Setter function hash (receives value parameter).
    pub setter: Option<TypeHash>,
    /// Visibility of the property.
    pub visibility: Visibility,
}

impl PropertyAccessors {
    /// Create a read-only property.
    pub fn read_only(getter: TypeHash) -> Self {
        Self {
            getter: Some(getter),
            setter: None,
            visibility: Visibility::Public,
        }
    }

    /// Create a read-only property with specified visibility.
    pub fn read_only_with_visibility(getter: TypeHash, visibility: Visibility) -> Self {
        Self {
            getter: Some(getter),
            setter: None,
            visibility,
        }
    }

    /// Create a write-only property.
    pub fn write_only(setter: TypeHash) -> Self {
        Self {
            getter: None,
            setter: Some(setter),
            visibility: Visibility::Public,
        }
    }

    /// Create a write-only property with specified visibility.
    pub fn write_only_with_visibility(setter: TypeHash, visibility: Visibility) -> Self {
        Self {
            getter: None,
            setter: Some(setter),
            visibility,
        }
    }

    /// Create a read-write property.
    pub fn read_write(getter: TypeHash, setter: TypeHash) -> Self {
        Self {
            getter: Some(getter),
            setter: Some(setter),
            visibility: Visibility::Public,
        }
    }

    /// Create a read-write property with specified visibility.
    pub fn read_write_with_visibility(
        getter: TypeHash,
        setter: TypeHash,
        visibility: Visibility,
    ) -> Self {
        Self {
            getter: Some(getter),
            setter: Some(setter),
            visibility,
        }
    }

    /// Check if this property is read-only.
    pub fn is_read_only(&self) -> bool {
        self.getter.is_some() && self.setter.is_none()
    }

    /// Check if this property is write-only.
    pub fn is_write_only(&self) -> bool {
        self.getter.is_none() && self.setter.is_some()
    }

    /// Check if this property is read-write.
    pub fn is_read_write(&self) -> bool {
        self.getter.is_some() && self.setter.is_some()
    }
}

/// Operator behavior for overloadable operators.
///
/// AngelScript allows classes to define special methods for operator overloading
/// and type conversions. This enum identifies which operator behavior a method provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorBehavior {
    // === Type Conversion Operators ===
    /// Explicit value conversion: `T opConv()`
    OpConv(TypeHash),
    /// Implicit value conversion: `T opImplConv()`
    OpImplConv(TypeHash),
    /// Explicit handle cast: `T@ opCast()`
    OpCast(TypeHash),
    /// Implicit handle cast: `T@ opImplCast()`
    OpImplCast(TypeHash),

    // === Unary Operators (Prefix) ===
    /// Unary minus: `-obj`
    OpNeg,
    /// Bitwise complement: `~obj`
    OpCom,
    /// Pre-increment: `++obj`
    OpPreInc,
    /// Pre-decrement: `--obj`
    OpPreDec,

    // === Unary Operators (Postfix) ===
    /// Post-increment: `obj++`
    OpPostInc,
    /// Post-decrement: `obj--`
    OpPostDec,

    // === Binary Operators ===
    /// Addition: `a + b`
    OpAdd,
    /// Addition (reverse): called on right operand
    OpAddR,
    /// Subtraction: `a - b`
    OpSub,
    /// Subtraction (reverse)
    OpSubR,
    /// Multiplication: `a * b`
    OpMul,
    /// Multiplication (reverse)
    OpMulR,
    /// Division: `a / b`
    OpDiv,
    /// Division (reverse)
    OpDivR,
    /// Modulo: `a % b`
    OpMod,
    /// Modulo (reverse)
    OpModR,
    /// Power: `a ** b`
    OpPow,
    /// Power (reverse)
    OpPowR,

    // === Bitwise Binary Operators ===
    /// Bitwise AND: `a & b`
    OpAnd,
    /// Bitwise AND (reverse)
    OpAndR,
    /// Bitwise OR: `a | b`
    OpOr,
    /// Bitwise OR (reverse)
    OpOrR,
    /// Bitwise XOR: `a ^ b`
    OpXor,
    /// Bitwise XOR (reverse)
    OpXorR,
    /// Left shift: `a << b`
    OpShl,
    /// Left shift (reverse)
    OpShlR,
    /// Arithmetic right shift: `a >> b`
    OpShr,
    /// Arithmetic right shift (reverse)
    OpShrR,
    /// Logical right shift: `a >>> b`
    OpUShr,
    /// Logical right shift (reverse)
    OpUShrR,

    // === Comparison Operators ===
    /// Equality: `a == b` returns bool
    OpEquals,
    /// Comparison: returns int (negative/0/positive)
    OpCmp,

    // === Assignment Operators ===
    /// Assignment: `a = b`
    OpAssign,
    /// Add-assign: `a += b`
    OpAddAssign,
    /// Subtract-assign: `a -= b`
    OpSubAssign,
    /// Multiply-assign: `a *= b`
    OpMulAssign,
    /// Divide-assign: `a /= b`
    OpDivAssign,
    /// Modulo-assign: `a %= b`
    OpModAssign,
    /// Power-assign: `a **= b`
    OpPowAssign,
    /// Bitwise AND-assign: `a &= b`
    OpAndAssign,
    /// Bitwise OR-assign: `a |= b`
    OpOrAssign,
    /// Bitwise XOR-assign: `a ^= b`
    OpXorAssign,
    /// Left shift-assign: `a <<= b`
    OpShlAssign,
    /// Arithmetic right shift-assign: `a >>= b`
    OpShrAssign,
    /// Logical right shift-assign: `a >>>= b`
    OpUShrAssign,

    // === Index Operator ===
    /// Index access: `obj[idx]`
    OpIndex,
    /// Index getter: `x = obj[idx]`
    OpIndexGet,
    /// Index setter: `obj[idx] = x`
    OpIndexSet,

    // === Function Call Operator ===
    /// Function call: `obj(args)`
    OpCall,

    // === Foreach Loop Operators ===
    /// Begin foreach iteration
    OpForBegin,
    /// End foreach check
    OpForEnd,
    /// Next foreach iteration
    OpForNext,
    /// Foreach value (single, equivalent to OpForValueN(0))
    OpForValue,
    /// Foreach value at index N (for multi-value iteration)
    /// The index is dynamic, allowing any number of iteration variables
    OpForValueN(u8),
}

impl OperatorBehavior {
    /// Parse operator method name to determine behavior.
    ///
    /// For conversion operators (opConv, opImplConv, opCast, opImplCast),
    /// requires target_type. For other operators, target_type is ignored.
    pub fn from_method_name(name: &str, target_type: Option<TypeHash>) -> Option<Self> {
        match name {
            // Conversion operators (require target type)
            "opConv" => target_type.map(OperatorBehavior::OpConv),
            "opImplConv" => target_type.map(OperatorBehavior::OpImplConv),
            "opCast" => target_type.map(OperatorBehavior::OpCast),
            "opImplCast" => target_type.map(OperatorBehavior::OpImplCast),

            // Unary operators (prefix)
            "opNeg" => Some(OperatorBehavior::OpNeg),
            "opCom" => Some(OperatorBehavior::OpCom),
            "opPreInc" => Some(OperatorBehavior::OpPreInc),
            "opPreDec" => Some(OperatorBehavior::OpPreDec),

            // Unary operators (postfix)
            "opPostInc" => Some(OperatorBehavior::OpPostInc),
            "opPostDec" => Some(OperatorBehavior::OpPostDec),

            // Binary operators
            "opAdd" => Some(OperatorBehavior::OpAdd),
            "opAdd_r" => Some(OperatorBehavior::OpAddR),
            "opSub" => Some(OperatorBehavior::OpSub),
            "opSub_r" => Some(OperatorBehavior::OpSubR),
            "opMul" => Some(OperatorBehavior::OpMul),
            "opMul_r" => Some(OperatorBehavior::OpMulR),
            "opDiv" => Some(OperatorBehavior::OpDiv),
            "opDiv_r" => Some(OperatorBehavior::OpDivR),
            "opMod" => Some(OperatorBehavior::OpMod),
            "opMod_r" => Some(OperatorBehavior::OpModR),
            "opPow" => Some(OperatorBehavior::OpPow),
            "opPow_r" => Some(OperatorBehavior::OpPowR),

            // Bitwise operators
            "opAnd" => Some(OperatorBehavior::OpAnd),
            "opAnd_r" => Some(OperatorBehavior::OpAndR),
            "opOr" => Some(OperatorBehavior::OpOr),
            "opOr_r" => Some(OperatorBehavior::OpOrR),
            "opXor" => Some(OperatorBehavior::OpXor),
            "opXor_r" => Some(OperatorBehavior::OpXorR),
            "opShl" => Some(OperatorBehavior::OpShl),
            "opShl_r" => Some(OperatorBehavior::OpShlR),
            "opShr" => Some(OperatorBehavior::OpShr),
            "opShr_r" => Some(OperatorBehavior::OpShrR),
            "opUShr" => Some(OperatorBehavior::OpUShr),
            "opUShr_r" => Some(OperatorBehavior::OpUShrR),

            // Comparison operators
            "opEquals" => Some(OperatorBehavior::OpEquals),
            "opCmp" => Some(OperatorBehavior::OpCmp),

            // Assignment operators
            "opAssign" => Some(OperatorBehavior::OpAssign),
            "opAddAssign" => Some(OperatorBehavior::OpAddAssign),
            "opSubAssign" => Some(OperatorBehavior::OpSubAssign),
            "opMulAssign" => Some(OperatorBehavior::OpMulAssign),
            "opDivAssign" => Some(OperatorBehavior::OpDivAssign),
            "opModAssign" => Some(OperatorBehavior::OpModAssign),
            "opPowAssign" => Some(OperatorBehavior::OpPowAssign),
            "opAndAssign" => Some(OperatorBehavior::OpAndAssign),
            "opOrAssign" => Some(OperatorBehavior::OpOrAssign),
            "opXorAssign" => Some(OperatorBehavior::OpXorAssign),
            "opShlAssign" => Some(OperatorBehavior::OpShlAssign),
            "opShrAssign" => Some(OperatorBehavior::OpShrAssign),
            "opUShrAssign" => Some(OperatorBehavior::OpUShrAssign),

            // Index and call operators
            "opIndex" => Some(OperatorBehavior::OpIndex),
            "get_opIndex" => Some(OperatorBehavior::OpIndexGet),
            "set_opIndex" => Some(OperatorBehavior::OpIndexSet),
            "opCall" => Some(OperatorBehavior::OpCall),

            // Foreach operators
            "opForBegin" => Some(OperatorBehavior::OpForBegin),
            "opForEnd" => Some(OperatorBehavior::OpForEnd),
            "opForNext" => Some(OperatorBehavior::OpForNext),
            "opForValue" => Some(OperatorBehavior::OpForValue),

            // Dynamic opForValue{N} - parse the index
            _ if name.starts_with("opForValue") => {
                let suffix = &name[10..]; // "opForValue".len() == 10
                suffix.parse::<u8>().ok().map(OperatorBehavior::OpForValueN)
            }

            _ => None,
        }
    }

    /// Get the target type for conversion operators.
    ///
    /// Returns `Some(TypeHash)` for conversion operators (OpConv, OpImplConv, OpCast, OpImplCast),
    /// or `None` for non-conversion operators.
    pub fn target_type(&self) -> Option<TypeHash> {
        match self {
            OperatorBehavior::OpConv(t)
            | OperatorBehavior::OpImplConv(t)
            | OperatorBehavior::OpCast(t)
            | OperatorBehavior::OpImplCast(t) => Some(*t),
            _ => None,
        }
    }

    /// Check if this is a conversion operator.
    pub fn is_conversion(&self) -> bool {
        matches!(
            self,
            OperatorBehavior::OpConv(_)
                | OperatorBehavior::OpImplConv(_)
                | OperatorBehavior::OpCast(_)
                | OperatorBehavior::OpImplCast(_)
        )
    }

    /// Check if this is an implicit operator.
    pub fn is_implicit(&self) -> bool {
        matches!(
            self,
            OperatorBehavior::OpImplConv(_) | OperatorBehavior::OpImplCast(_)
        )
    }
}

/// Type definition - represents a complete type in the system.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum TypeDef {
    /// Primitive type (int, float, bool, etc.)
    Primitive {
        kind: PrimitiveKind,
        type_hash: TypeHash,
    },

    /// User-defined class, template definition, or template instance.
    Class {
        /// Unqualified name.
        name: String,
        /// Fully qualified name (with namespace).
        qualified_name: String,
        /// Deterministic hash for this type.
        type_hash: TypeHash,
        /// Fields in the class.
        fields: Vec<FieldDef>,
        /// Method function hashes.
        methods: Vec<TypeHash>,
        /// Base class TypeHash (if any).
        base_class: Option<TypeHash>,
        /// Implemented interface TypeHashes.
        interfaces: Vec<TypeHash>,
        /// Operator methods by behavior.
        operator_methods: FxHashMap<OperatorBehavior, Vec<TypeHash>>,
        /// Property accessors by name.
        properties: FxHashMap<String, PropertyAccessors>,
        /// Class is marked 'final'.
        is_final: bool,
        /// Class is marked 'abstract'.
        is_abstract: bool,
        /// Template parameter TypeHashes (non-empty = template definition).
        template_params: Vec<TypeHash>,
        /// Template this was instantiated from (for template instances).
        template: Option<TypeHash>,
        /// Type arguments for template instances.
        type_args: Vec<DataType>,
        /// Type kind (value, reference, or script object).
        type_kind: TypeKind,
    },

    /// Interface definition.
    Interface {
        name: String,
        qualified_name: String,
        type_hash: TypeHash,
        methods: Vec<MethodSignature>,
    },

    /// Enumeration type.
    Enum {
        name: String,
        qualified_name: String,
        type_hash: TypeHash,
        values: Vec<(String, i64)>,
    },

    /// Function pointer type (funcdef).
    Funcdef {
        name: String,
        qualified_name: String,
        type_hash: TypeHash,
        params: Vec<DataType>,
        return_type: DataType,
    },

    /// Template parameter placeholder (e.g., T in array<T>).
    TemplateParam {
        /// Parameter name (e.g., "T", "K", "V").
        name: String,
        /// Parameter index within the template.
        index: usize,
        /// The template TypeHash this parameter belongs to.
        owner: TypeHash,
        /// Deterministic hash for this template parameter.
        type_hash: TypeHash,
    },
}

impl TypeDef {
    /// Get the name of this type (unqualified).
    pub fn name(&self) -> &str {
        match self {
            TypeDef::Primitive { kind, .. } => kind.name(),
            TypeDef::Class { name, .. } => name,
            TypeDef::Interface { name, .. } => name,
            TypeDef::Enum { name, .. } => name,
            TypeDef::Funcdef { name, .. } => name,
            TypeDef::TemplateParam { name, .. } => name,
        }
    }

    /// Get the qualified name of this type (with namespace).
    pub fn qualified_name(&self) -> &str {
        match self {
            TypeDef::Primitive { kind, .. } => kind.name(),
            TypeDef::Class { qualified_name, .. } => qualified_name,
            TypeDef::Interface { qualified_name, .. } => qualified_name,
            TypeDef::Enum { qualified_name, .. } => qualified_name,
            TypeDef::Funcdef { qualified_name, .. } => qualified_name,
            TypeDef::TemplateParam { name, .. } => name,
        }
    }

    /// Get the type hash for this type.
    pub fn type_hash(&self) -> TypeHash {
        match self {
            TypeDef::Primitive { type_hash, .. } => *type_hash,
            TypeDef::Class { type_hash, .. } => *type_hash,
            TypeDef::Interface { type_hash, .. } => *type_hash,
            TypeDef::Enum { type_hash, .. } => *type_hash,
            TypeDef::Funcdef { type_hash, .. } => *type_hash,
            TypeDef::TemplateParam { type_hash, .. } => *type_hash,
        }
    }

    /// Check if this is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeDef::Primitive { .. })
    }

    /// Check if this is a class type.
    pub fn is_class(&self) -> bool {
        matches!(self, TypeDef::Class { .. })
    }

    /// Check if this is an interface type.
    pub fn is_interface(&self) -> bool {
        matches!(self, TypeDef::Interface { .. })
    }

    /// Check if this is an enum type.
    pub fn is_enum(&self) -> bool {
        matches!(self, TypeDef::Enum { .. })
    }

    /// Check if this is a funcdef type.
    pub fn is_funcdef(&self) -> bool {
        matches!(self, TypeDef::Funcdef { .. })
    }

    /// Check if this is a template parameter.
    pub fn is_template_param(&self) -> bool {
        matches!(self, TypeDef::TemplateParam { .. })
    }

    /// Check if this is a template definition.
    pub fn is_template(&self) -> bool {
        matches!(self, TypeDef::Class { template_params, .. } if !template_params.is_empty())
    }

    /// Check if this is a template instance.
    pub fn is_template_instance(&self) -> bool {
        matches!(
            self,
            TypeDef::Class {
                template: Some(_),
                ..
            }
        )
    }

    /// Get the template parameter TypeHashes if this is a template definition.
    pub fn get_template_params(&self) -> Option<&[TypeHash]> {
        match self {
            TypeDef::Class {
                template_params, ..
            } if !template_params.is_empty() => Some(template_params),
            _ => None,
        }
    }

    /// Get the template TypeHash this class was instantiated from.
    pub fn template_origin(&self) -> Option<TypeHash> {
        match self {
            TypeDef::Class { template, .. } => *template,
            _ => None,
        }
    }

    /// Get the type arguments for template instances.
    pub fn type_arguments(&self) -> &[DataType] {
        match self {
            TypeDef::Class { type_args, .. } => type_args,
            _ => &[],
        }
    }

    /// Get the type kind for this type.
    pub fn type_kind(&self) -> TypeKind {
        match self {
            TypeDef::Primitive { .. } => TypeKind::value_sized(0, 0, true),
            TypeDef::Class { type_kind, .. } => type_kind.clone(),
            TypeDef::Interface { .. } => TypeKind::reference(),
            TypeDef::Enum { .. } => TypeKind::value_sized(4, 4, true),
            TypeDef::Funcdef { .. } => TypeKind::reference(),
            TypeDef::TemplateParam { .. } => TypeKind::reference(),
        }
    }

    /// Check if this type is a value type.
    pub fn is_value_type(&self) -> bool {
        self.type_kind().is_value()
    }

    /// Check if this type is a reference type.
    pub fn is_reference_type(&self) -> bool {
        self.type_kind().is_reference()
    }
}

impl fmt::Display for TypeDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.qualified_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn primitive_kind_names() {
        assert_eq!(PrimitiveKind::Void.name(), "void");
        assert_eq!(PrimitiveKind::Bool.name(), "bool");
        assert_eq!(PrimitiveKind::Int8.name(), "int8");
        assert_eq!(PrimitiveKind::Int16.name(), "int16");
        assert_eq!(PrimitiveKind::Int32.name(), "int");
        assert_eq!(PrimitiveKind::Int64.name(), "int64");
        assert_eq!(PrimitiveKind::Uint8.name(), "uint8");
        assert_eq!(PrimitiveKind::Uint16.name(), "uint16");
        assert_eq!(PrimitiveKind::Uint32.name(), "uint");
        assert_eq!(PrimitiveKind::Uint64.name(), "uint64");
        assert_eq!(PrimitiveKind::Float.name(), "float");
        assert_eq!(PrimitiveKind::Double.name(), "double");
    }

    #[test]
    fn primitive_kind_hashes() {
        assert_eq!(PrimitiveKind::Void.type_hash(), primitives::VOID);
        assert_eq!(PrimitiveKind::Bool.type_hash(), primitives::BOOL);
        assert_eq!(PrimitiveKind::Int32.type_hash(), primitives::INT32);
        assert_eq!(PrimitiveKind::Float.type_hash(), primitives::FLOAT);
    }

    #[test]
    fn primitive_kind_display() {
        assert_eq!(format!("{}", PrimitiveKind::Int32), "int");
        assert_eq!(format!("{}", PrimitiveKind::Float), "float");
    }

    #[test]
    fn visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn visibility_display() {
        assert_eq!(format!("{}", Visibility::Public), "public");
        assert_eq!(format!("{}", Visibility::Protected), "protected");
        assert_eq!(format!("{}", Visibility::Private), "private");
    }

    #[test]
    fn type_kind_constructors() {
        let value = TypeKind::value_sized(8, 8, true);
        assert!(value.is_value());
        assert!(!value.is_reference());

        let reference = TypeKind::reference();
        assert!(reference.is_reference());
        assert!(!reference.is_value());

        let script = TypeKind::script_object();
        assert!(script.is_script_object());
    }

    #[test]
    fn type_kind_default() {
        assert_eq!(TypeKind::default(), TypeKind::ScriptObject);
    }

    #[test]
    fn field_def_creation() {
        let field = FieldDef::new(
            "health",
            DataType::simple(primitives::INT32),
            Visibility::Public,
        );
        assert_eq!(field.name, "health");
        assert_eq!(field.visibility, Visibility::Public);

        let private = FieldDef::private("secret", DataType::simple(primitives::INT32));
        assert_eq!(private.visibility, Visibility::Private);
    }

    #[test]
    fn method_signature_creation() {
        let sig = MethodSignature::new(
            "update",
            vec![DataType::simple(primitives::FLOAT)],
            DataType::void(),
        );
        assert_eq!(sig.name, "update");
        assert!(!sig.is_const);

        let const_sig =
            MethodSignature::new_const("get_value", vec![], DataType::simple(primitives::INT32));
        assert!(const_sig.is_const);
    }

    #[test]
    fn property_accessors() {
        let getter = TypeHash::from_name("getter");
        let setter = TypeHash::from_name("setter");

        let read_only = PropertyAccessors::read_only(getter);
        assert!(read_only.is_read_only());
        assert!(!read_only.is_write_only());
        assert!(!read_only.is_read_write());

        let write_only = PropertyAccessors::write_only(setter);
        assert!(write_only.is_write_only());

        let read_write = PropertyAccessors::read_write(getter, setter);
        assert!(read_write.is_read_write());
    }

    #[test]
    fn operator_behavior_from_method_name() {
        assert_eq!(
            OperatorBehavior::from_method_name("opAdd", None),
            Some(OperatorBehavior::OpAdd)
        );
        assert_eq!(
            OperatorBehavior::from_method_name("opAssign", None),
            Some(OperatorBehavior::OpAssign)
        );
        assert_eq!(OperatorBehavior::from_method_name("unknown", None), None);

        // Conversion operators need target type
        let target = TypeHash::from_name("Target");
        assert_eq!(
            OperatorBehavior::from_method_name("opConv", Some(target)),
            Some(OperatorBehavior::OpConv(target))
        );
        assert_eq!(OperatorBehavior::from_method_name("opConv", None), None);
    }

    #[test]
    fn operator_behavior_is_conversion() {
        let target = TypeHash::from_name("Target");
        assert!(OperatorBehavior::OpConv(target).is_conversion());
        assert!(OperatorBehavior::OpImplConv(target).is_conversion());
        assert!(!OperatorBehavior::OpAdd.is_conversion());
    }

    #[test]
    fn operator_behavior_is_implicit() {
        let target = TypeHash::from_name("Target");
        assert!(OperatorBehavior::OpImplConv(target).is_implicit());
        assert!(OperatorBehavior::OpImplCast(target).is_implicit());
        assert!(!OperatorBehavior::OpConv(target).is_implicit());
    }

    #[test]
    fn typedef_primitive() {
        let typedef = TypeDef::Primitive {
            kind: PrimitiveKind::Int32,
            type_hash: primitives::INT32,
        };
        assert_eq!(typedef.name(), "int");
        assert_eq!(typedef.qualified_name(), "int");
        assert!(typedef.is_primitive());
        assert!(!typedef.is_class());
        assert!(typedef.is_value_type());
    }

    #[test]
    fn typedef_class() {
        let type_hash = TypeHash::from_name("Player");
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            type_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::script_object(),
        };
        assert_eq!(typedef.name(), "Player");
        assert_eq!(typedef.qualified_name(), "Game::Player");
        assert!(typedef.is_class());
        assert!(!typedef.is_primitive());
        assert!(!typedef.is_template());
        assert!(!typedef.is_template_instance());
    }

    #[test]
    fn typedef_template() {
        let type_hash = TypeHash::from_name("array");
        let t_param = TypeHash::from_name("array::T");
        let typedef = TypeDef::Class {
            name: "array".to_string(),
            qualified_name: "array".to_string(),
            type_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![t_param],
            template: None,
            type_args: vec![],
            type_kind: TypeKind::reference(),
        };
        assert!(typedef.is_template());
        assert!(!typedef.is_template_instance());
        assert_eq!(typedef.get_template_params(), Some(&[t_param][..]));
    }

    #[test]
    fn typedef_template_instance() {
        let template_hash = TypeHash::from_name("array");
        let instance_hash = TypeHash::from_name("array<int>");
        let typedef = TypeDef::Class {
            name: "array<int>".to_string(),
            qualified_name: "array<int>".to_string(),
            type_hash: instance_hash,
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
            operator_methods: FxHashMap::default(),
            properties: FxHashMap::default(),
            is_final: false,
            is_abstract: false,
            template_params: vec![],
            template: Some(template_hash),
            type_args: vec![DataType::simple(primitives::INT32)],
            type_kind: TypeKind::reference(),
        };
        assert!(typedef.is_template_instance());
        assert!(!typedef.is_template());
        assert_eq!(typedef.template_origin(), Some(template_hash));
        assert_eq!(typedef.type_arguments().len(), 1);
    }

    #[test]
    fn typedef_interface() {
        let type_hash = TypeHash::from_name("IDrawable");
        let typedef = TypeDef::Interface {
            name: "IDrawable".to_string(),
            qualified_name: "Graphics::IDrawable".to_string(),
            type_hash,
            methods: vec![],
        };
        assert!(typedef.is_interface());
        assert!(!typedef.is_class());
        assert!(typedef.is_reference_type());
    }

    #[test]
    fn typedef_enum() {
        let type_hash = TypeHash::from_name("Color");
        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
            type_hash,
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        };
        assert!(typedef.is_enum());
        assert!(typedef.is_value_type());
    }

    #[test]
    fn typedef_funcdef() {
        let type_hash = TypeHash::from_name("Callback");
        let typedef = TypeDef::Funcdef {
            name: "Callback".to_string(),
            qualified_name: "Callback".to_string(),
            type_hash,
            params: vec![DataType::simple(primitives::INT32)],
            return_type: DataType::void(),
        };
        assert!(typedef.is_funcdef());
    }

    #[test]
    fn typedef_template_param() {
        let owner = TypeHash::from_name("array");
        let type_hash = TypeHash::from_name("array::T");
        let typedef = TypeDef::TemplateParam {
            name: "T".to_string(),
            index: 0,
            owner,
            type_hash,
        };
        assert!(typedef.is_template_param());
        assert_eq!(typedef.name(), "T");
    }

    #[test]
    fn typedef_display() {
        let typedef = TypeDef::Primitive {
            kind: PrimitiveKind::Int32,
            type_hash: primitives::INT32,
        };
        assert_eq!(format!("{}", typedef), "int");
    }
}
