//! Type definitions and identifiers for the AngelScript type system.
//!
//! This module provides the core type system structures including TypeId constants,
//! TypeDef variants, and all supporting types for representing AngelScript types.

use super::data_type::DataType;
use std::fmt;

/// A unique identifier for a type in the type system.
///
/// TypeIds are assigned sequentially with primitives at fixed indices (0-11),
/// built-in types at fixed indices (16-18), and user-defined types starting at 32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Create a new TypeId with the given value.
    #[inline]
    pub const fn new(id: u32) -> Self {
        TypeId(id)
    }

    /// Get the underlying u32 value.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

// Fixed TypeIds for primitive types (0-11)
pub const VOID_TYPE: TypeId = TypeId(0);
pub const BOOL_TYPE: TypeId = TypeId(1);
pub const INT8_TYPE: TypeId = TypeId(2);
pub const INT16_TYPE: TypeId = TypeId(3);
pub const INT32_TYPE: TypeId = TypeId(4);   // "int" alias
pub const INT64_TYPE: TypeId = TypeId(5);
pub const UINT8_TYPE: TypeId = TypeId(6);
pub const UINT16_TYPE: TypeId = TypeId(7);
pub const UINT32_TYPE: TypeId = TypeId(8);  // "uint" alias
pub const UINT64_TYPE: TypeId = TypeId(9);
pub const FLOAT_TYPE: TypeId = TypeId(10);
pub const DOUBLE_TYPE: TypeId = TypeId(11);

// Gap: TypeIds 12-15 reserved for future primitive types

// Built-in types (16-18)
pub const STRING_TYPE: TypeId = TypeId(16);
pub const ARRAY_TEMPLATE: TypeId = TypeId(17);
pub const DICT_TEMPLATE: TypeId = TypeId(18);

// Gap: TypeIds 19-31 reserved for future built-in types

/// First TypeId available for user-defined types
pub const FIRST_USER_TYPE_ID: u32 = 32;

/// Primitive type kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
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

impl PrimitiveType {
    /// Get the TypeId for this primitive type.
    pub const fn type_id(self) -> TypeId {
        match self {
            PrimitiveType::Void => VOID_TYPE,
            PrimitiveType::Bool => BOOL_TYPE,
            PrimitiveType::Int8 => INT8_TYPE,
            PrimitiveType::Int16 => INT16_TYPE,
            PrimitiveType::Int32 => INT32_TYPE,
            PrimitiveType::Int64 => INT64_TYPE,
            PrimitiveType::Uint8 => UINT8_TYPE,
            PrimitiveType::Uint16 => UINT16_TYPE,
            PrimitiveType::Uint32 => UINT32_TYPE,
            PrimitiveType::Uint64 => UINT64_TYPE,
            PrimitiveType::Float => FLOAT_TYPE,
            PrimitiveType::Double => DOUBLE_TYPE,
        }
    }

    /// Get the name of this primitive type.
    pub const fn name(self) -> &'static str {
        match self {
            PrimitiveType::Void => "void",
            PrimitiveType::Bool => "bool",
            PrimitiveType::Int8 => "int8",
            PrimitiveType::Int16 => "int16",
            PrimitiveType::Int32 => "int",
            PrimitiveType::Int64 => "int64",
            PrimitiveType::Uint8 => "uint8",
            PrimitiveType::Uint16 => "uint16",
            PrimitiveType::Uint32 => "uint",
            PrimitiveType::Uint64 => "uint64",
            PrimitiveType::Float => "float",
            PrimitiveType::Double => "double",
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Visibility modifier for class members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Public
    }
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

/// Function traits (special function behaviors)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FunctionTraits {
    /// This is a constructor
    pub is_constructor: bool,
    /// This is a destructor
    pub is_destructor: bool,
    /// This function is final (cannot be overridden)
    pub is_final: bool,
    /// This function is virtual (can be overridden)
    pub is_virtual: bool,
    /// This function is abstract (must be overridden)
    pub is_abstract: bool,
    /// This function is const (doesn't modify object state)
    pub is_const: bool,
}

impl FunctionTraits {
    /// Create default function traits (no special behaviors)
    pub const fn new() -> Self {
        Self {
            is_constructor: false,
            is_destructor: false,
            is_final: false,
            is_virtual: false,
            is_abstract: false,
            is_const: false,
        }
    }
}

/// A unique identifier for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u32);

impl FunctionId {
    /// Create a new FunctionId
    #[inline]
    pub const fn new(id: u32) -> Self {
        FunctionId(id)
    }

    /// Get the underlying u32 value
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FunctionId({})", self.0)
    }
}

/// A field definition in a class
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type
    pub data_type: DataType,
    /// Field visibility
    pub visibility: Visibility,
}

impl FieldDef {
    /// Create a new field definition
    pub fn new(name: String, data_type: DataType, visibility: Visibility) -> Self {
        Self {
            name,
            data_type,
            visibility,
        }
    }
}

/// A method signature for interfaces
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    /// Method name
    pub name: String,
    /// Parameter types
    pub params: Vec<DataType>,
    /// Return type
    pub return_type: DataType,
}

impl MethodSignature {
    /// Create a new method signature
    pub fn new(name: String, params: Vec<DataType>, return_type: DataType) -> Self {
        Self {
            name,
            params,
            return_type,
        }
    }
}

/// Type definition - represents a complete type in the system
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef {
    /// Primitive type (int, float, bool, etc.)
    Primitive {
        kind: PrimitiveType,
    },

    /// User-defined class
    Class {
        name: String,
        qualified_name: String,
        fields: Vec<FieldDef>,
        methods: Vec<FunctionId>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
    },

    /// Interface definition
    Interface {
        name: String,
        qualified_name: String,
        methods: Vec<MethodSignature>,
    },

    /// Enumeration type
    Enum {
        name: String,
        qualified_name: String,
        values: Vec<(String, i64)>,
    },

    /// Function pointer type (funcdef)
    Funcdef {
        name: String,
        qualified_name: String,
        params: Vec<DataType>,
        return_type: DataType,
    },

    /// Template definition (array, dictionary, etc.)
    Template {
        name: String,
        param_count: usize,
    },

    /// Template instantiation (array<int>, etc.)
    TemplateInstance {
        template: TypeId,
        sub_types: Vec<DataType>,
    },
}

impl TypeDef {
    /// Get the name of this type (unqualified)
    pub fn name(&self) -> &str {
        match self {
            TypeDef::Primitive { kind } => kind.name(),
            TypeDef::Class { name, .. } => name,
            TypeDef::Interface { name, .. } => name,
            TypeDef::Enum { name, .. } => name,
            TypeDef::Funcdef { name, .. } => name,
            TypeDef::Template { name, .. } => name,
            TypeDef::TemplateInstance { .. } => "<template instance>",
        }
    }

    /// Get the qualified name of this type (with namespace)
    pub fn qualified_name(&self) -> &str {
        match self {
            TypeDef::Primitive { kind } => kind.name(),
            TypeDef::Class { qualified_name, .. } => qualified_name,
            TypeDef::Interface { qualified_name, .. } => qualified_name,
            TypeDef::Enum { qualified_name, .. } => qualified_name,
            TypeDef::Funcdef { qualified_name, .. } => qualified_name,
            TypeDef::Template { name, .. } => name,
            TypeDef::TemplateInstance { .. } => "<template instance>",
        }
    }

    /// Check if this is a primitive type
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeDef::Primitive { .. })
    }

    /// Check if this is a class type
    pub fn is_class(&self) -> bool {
        matches!(self, TypeDef::Class { .. })
    }

    /// Check if this is an interface type
    pub fn is_interface(&self) -> bool {
        matches!(self, TypeDef::Interface { .. })
    }

    /// Check if this is an enum type
    pub fn is_enum(&self) -> bool {
        matches!(self, TypeDef::Enum { .. })
    }

    /// Check if this is a funcdef type
    pub fn is_funcdef(&self) -> bool {
        matches!(self, TypeDef::Funcdef { .. })
    }

    /// Check if this is a template
    pub fn is_template(&self) -> bool {
        matches!(self, TypeDef::Template { .. })
    }

    /// Check if this is a template instance
    pub fn is_template_instance(&self) -> bool {
        matches!(self, TypeDef::TemplateInstance { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_id_creation() {
        let id = TypeId::new(42);
        assert_eq!(id.as_u32(), 42);
    }

    #[test]
    fn type_id_equality() {
        let id1 = TypeId::new(10);
        let id2 = TypeId::new(10);
        let id3 = TypeId::new(20);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn type_id_ordering() {
        let id1 = TypeId::new(10);
        let id2 = TypeId::new(20);
        assert!(id1 < id2);
        assert!(id2 > id1);
    }

    #[test]
    fn type_id_display() {
        let id = TypeId::new(42);
        assert_eq!(format!("{}", id), "TypeId(42)");
    }

    #[test]
    fn primitive_type_constants() {
        assert_eq!(VOID_TYPE, TypeId(0));
        assert_eq!(BOOL_TYPE, TypeId(1));
        assert_eq!(INT8_TYPE, TypeId(2));
        assert_eq!(INT16_TYPE, TypeId(3));
        assert_eq!(INT32_TYPE, TypeId(4));
        assert_eq!(INT64_TYPE, TypeId(5));
        assert_eq!(UINT8_TYPE, TypeId(6));
        assert_eq!(UINT16_TYPE, TypeId(7));
        assert_eq!(UINT32_TYPE, TypeId(8));
        assert_eq!(UINT64_TYPE, TypeId(9));
        assert_eq!(FLOAT_TYPE, TypeId(10));
        assert_eq!(DOUBLE_TYPE, TypeId(11));
    }

    #[test]
    fn builtin_type_constants() {
        assert_eq!(STRING_TYPE, TypeId(16));
        assert_eq!(ARRAY_TEMPLATE, TypeId(17));
        assert_eq!(DICT_TEMPLATE, TypeId(18));
    }

    #[test]
    fn first_user_type_id() {
        assert_eq!(FIRST_USER_TYPE_ID, 32);
    }

    #[test]
    fn primitive_type_ids() {
        assert_eq!(PrimitiveType::Void.type_id(), VOID_TYPE);
        assert_eq!(PrimitiveType::Bool.type_id(), BOOL_TYPE);
        assert_eq!(PrimitiveType::Int8.type_id(), INT8_TYPE);
        assert_eq!(PrimitiveType::Int16.type_id(), INT16_TYPE);
        assert_eq!(PrimitiveType::Int32.type_id(), INT32_TYPE);
        assert_eq!(PrimitiveType::Int64.type_id(), INT64_TYPE);
        assert_eq!(PrimitiveType::Uint8.type_id(), UINT8_TYPE);
        assert_eq!(PrimitiveType::Uint16.type_id(), UINT16_TYPE);
        assert_eq!(PrimitiveType::Uint32.type_id(), UINT32_TYPE);
        assert_eq!(PrimitiveType::Uint64.type_id(), UINT64_TYPE);
        assert_eq!(PrimitiveType::Float.type_id(), FLOAT_TYPE);
        assert_eq!(PrimitiveType::Double.type_id(), DOUBLE_TYPE);
    }

    #[test]
    fn primitive_type_names() {
        assert_eq!(PrimitiveType::Void.name(), "void");
        assert_eq!(PrimitiveType::Bool.name(), "bool");
        assert_eq!(PrimitiveType::Int8.name(), "int8");
        assert_eq!(PrimitiveType::Int16.name(), "int16");
        assert_eq!(PrimitiveType::Int32.name(), "int");
        assert_eq!(PrimitiveType::Int64.name(), "int64");
        assert_eq!(PrimitiveType::Uint8.name(), "uint8");
        assert_eq!(PrimitiveType::Uint16.name(), "uint16");
        assert_eq!(PrimitiveType::Uint32.name(), "uint");
        assert_eq!(PrimitiveType::Uint64.name(), "uint64");
        assert_eq!(PrimitiveType::Float.name(), "float");
        assert_eq!(PrimitiveType::Double.name(), "double");
    }

    #[test]
    fn primitive_type_display() {
        assert_eq!(format!("{}", PrimitiveType::Int32), "int");
        assert_eq!(format!("{}", PrimitiveType::Float), "float");
        assert_eq!(format!("{}", PrimitiveType::Bool), "bool");
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
    fn function_traits_default() {
        let traits = FunctionTraits::default();
        assert!(!traits.is_constructor);
        assert!(!traits.is_destructor);
        assert!(!traits.is_final);
        assert!(!traits.is_virtual);
        assert!(!traits.is_abstract);
        assert!(!traits.is_const);
    }

    #[test]
    fn function_traits_new() {
        let traits = FunctionTraits::new();
        assert!(!traits.is_constructor);
        assert!(!traits.is_destructor);
    }

    #[test]
    fn function_traits_modification() {
        let mut traits = FunctionTraits::new();
        traits.is_virtual = true;
        traits.is_const = true;
        assert!(traits.is_virtual);
        assert!(traits.is_const);
        assert!(!traits.is_final);
    }

    #[test]
    fn function_id_creation() {
        let id = FunctionId::new(100);
        assert_eq!(id.as_u32(), 100);
    }

    #[test]
    fn function_id_equality() {
        let id1 = FunctionId::new(10);
        let id2 = FunctionId::new(10);
        let id3 = FunctionId::new(20);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn function_id_display() {
        let id = FunctionId::new(42);
        assert_eq!(format!("{}", id), "FunctionId(42)");
    }

    #[test]
    fn field_def_creation() {
        let field = FieldDef::new(
            "health".to_string(),
            DataType::simple(INT32_TYPE),
            Visibility::Public,
        );
        assert_eq!(field.name, "health");
        assert_eq!(field.data_type, DataType::simple(INT32_TYPE));
        assert_eq!(field.visibility, Visibility::Public);
    }

    #[test]
    fn method_signature_creation() {
        let sig = MethodSignature::new(
            "update".to_string(),
            vec![DataType::simple(FLOAT_TYPE)],
            DataType::simple(VOID_TYPE),
        );
        assert_eq!(sig.name, "update");
        assert_eq!(sig.params.len(), 1);
        assert_eq!(sig.return_type, DataType::simple(VOID_TYPE));
    }

    #[test]
    fn typedef_primitive() {
        let typedef = TypeDef::Primitive { kind: PrimitiveType::Int32 };
        assert_eq!(typedef.name(), "int");
        assert_eq!(typedef.qualified_name(), "int");
        assert!(typedef.is_primitive());
        assert!(!typedef.is_class());
    }

    #[test]
    fn typedef_class() {
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Game::Player".to_string(),
            fields: vec![],
            methods: vec![],
            base_class: None,
            interfaces: vec![],
        };
        assert_eq!(typedef.name(), "Player");
        assert_eq!(typedef.qualified_name(), "Game::Player");
        assert!(typedef.is_class());
        assert!(!typedef.is_primitive());
    }

    #[test]
    fn typedef_class_with_fields() {
        let typedef = TypeDef::Class {
            name: "Player".to_string(),
            qualified_name: "Player".to_string(),
            fields: vec![
                FieldDef::new(
                    "health".to_string(),
                    DataType::simple(INT32_TYPE),
                    Visibility::Public,
                ),
            ],
            methods: vec![FunctionId::new(0)],
            base_class: Some(TypeId::new(50)),
            interfaces: vec![TypeId::new(60)],
        };

        if let TypeDef::Class { fields, methods, base_class, interfaces, .. } = typedef {
            assert_eq!(fields.len(), 1);
            assert_eq!(methods.len(), 1);
            assert_eq!(base_class, Some(TypeId::new(50)));
            assert_eq!(interfaces.len(), 1);
        } else {
            panic!("Expected Class variant");
        }
    }

    #[test]
    fn typedef_interface() {
        let typedef = TypeDef::Interface {
            name: "IDrawable".to_string(),
            qualified_name: "Graphics::IDrawable".to_string(),
            methods: vec![],
        };
        assert_eq!(typedef.name(), "IDrawable");
        assert_eq!(typedef.qualified_name(), "Graphics::IDrawable");
        assert!(typedef.is_interface());
        assert!(!typedef.is_class());
    }

    #[test]
    fn typedef_enum() {
        let typedef = TypeDef::Enum {
            name: "Color".to_string(),
            qualified_name: "Color".to_string(),
            values: vec![
                ("Red".to_string(), 0),
                ("Green".to_string(), 1),
                ("Blue".to_string(), 2),
            ],
        };
        assert_eq!(typedef.name(), "Color");
        assert!(typedef.is_enum());

        if let TypeDef::Enum { values, .. } = typedef {
            assert_eq!(values.len(), 3);
            assert_eq!(values[0].0, "Red");
            assert_eq!(values[0].1, 0);
        }
    }

    #[test]
    fn typedef_funcdef() {
        let typedef = TypeDef::Funcdef {
            name: "Callback".to_string(),
            qualified_name: "Callback".to_string(),
            params: vec![DataType::simple(INT32_TYPE)],
            return_type: DataType::simple(VOID_TYPE),
        };
        assert_eq!(typedef.name(), "Callback");
        assert!(typedef.is_funcdef());

        if let TypeDef::Funcdef { params, return_type, .. } = typedef {
            assert_eq!(params.len(), 1);
            assert_eq!(return_type, DataType::simple(VOID_TYPE));
        }
    }

    #[test]
    fn typedef_template() {
        let typedef = TypeDef::Template {
            name: "array".to_string(),
            param_count: 1,
        };
        assert_eq!(typedef.name(), "array");
        assert!(typedef.is_template());
        assert!(!typedef.is_template_instance());
    }

    #[test]
    fn typedef_template_instance() {
        let typedef = TypeDef::TemplateInstance {
            template: ARRAY_TEMPLATE,
            sub_types: vec![DataType::simple(INT32_TYPE)],
        };
        assert!(typedef.is_template_instance());
        assert!(!typedef.is_template());

        if let TypeDef::TemplateInstance { template, sub_types } = typedef {
            assert_eq!(template, ARRAY_TEMPLATE);
            assert_eq!(sub_types.len(), 1);
        }
    }

    #[test]
    fn typedef_all_type_checks() {
        let primitive = TypeDef::Primitive { kind: PrimitiveType::Int32 };
        assert!(primitive.is_primitive());
        assert!(!primitive.is_class());
        assert!(!primitive.is_interface());
        assert!(!primitive.is_enum());
        assert!(!primitive.is_funcdef());
        assert!(!primitive.is_template());
        assert!(!primitive.is_template_instance());
    }
}
