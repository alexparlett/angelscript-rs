use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Primitive types supported by AngelScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float,
    Double,
}

impl PrimitiveType {
    /// Size in bytes on the stack/in memory.
    pub fn size(self) -> usize {
        match self {
            PrimitiveType::Void => 0,
            PrimitiveType::Bool | PrimitiveType::Int8 | PrimitiveType::UInt8 => 1,
            PrimitiveType::Int16 | PrimitiveType::UInt16 => 2,
            PrimitiveType::Int32 | PrimitiveType::UInt32 | PrimitiveType::Float => 4,
            PrimitiveType::Int64 | PrimitiveType::UInt64 | PrimitiveType::Double => 8,
        }
    }

    /// Size in dwords (4-byte units) on the VM stack.
    pub fn stack_dwords(self) -> u32 {
        match self.size() {
            0 => 0,
            1..=4 => 1,
            5..=8 => 2,
            _ => unreachable!(),
        }
    }

    /// Whether this is a signed integer type.
    pub fn is_signed_integer(self) -> bool {
        matches!(
            self,
            PrimitiveType::Int8
                | PrimitiveType::Int16
                | PrimitiveType::Int32
                | PrimitiveType::Int64
        )
    }

    /// Whether this is an unsigned integer type.
    pub fn is_unsigned_integer(self) -> bool {
        matches!(
            self,
            PrimitiveType::UInt8
                | PrimitiveType::UInt16
                | PrimitiveType::UInt32
                | PrimitiveType::UInt64
        )
    }

    /// Whether this is any integer type (signed or unsigned).
    pub fn is_integer(self) -> bool {
        self.is_signed_integer() || self.is_unsigned_integer()
    }

    /// Whether this is a floating-point type.
    pub fn is_float(self) -> bool {
        matches!(self, PrimitiveType::Float | PrimitiveType::Double)
    }

    /// Whether this is a numeric type (integer or float).
    pub fn is_numeric(self) -> bool {
        self.is_integer() || self.is_float()
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Void => write!(f, "void"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::Int8 => write!(f, "int8"),
            PrimitiveType::Int16 => write!(f, "int16"),
            PrimitiveType::Int32 => write!(f, "int"),
            PrimitiveType::Int64 => write!(f, "int64"),
            PrimitiveType::UInt8 => write!(f, "uint8"),
            PrimitiveType::UInt16 => write!(f, "uint16"),
            PrimitiveType::UInt32 => write!(f, "uint"),
            PrimitiveType::UInt64 => write!(f, "uint64"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::Double => write!(f, "double"),
        }
    }
}

/// Deterministic 64-bit hash uniquely identifying a type or function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeId(pub u64);

impl TypeId {
    /// Compute a TypeId from a qualified name.
    pub fn from_name(name: &QualifiedName) -> Self {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        TypeId(hasher.finish())
    }

    /// Compute a TypeId from a raw string.
    pub fn from_str(s: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        TypeId(hasher.finish())
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TypeId({:#018x})", self.0)
    }
}

/// A fully qualified type/symbol name with namespace path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QualifiedName {
    /// Namespace segments, e.g. `["Game", "Entities"]` for `Game::Entities::Player`.
    pub namespace: Vec<String>,
    /// The bare name, e.g. `"Player"`.
    pub name: String,
}

impl QualifiedName {
    /// Create a name in the global namespace.
    pub fn global(name: impl Into<String>) -> Self {
        QualifiedName {
            namespace: Vec::new(),
            name: name.into(),
        }
    }

    /// Create a name in a specific namespace.
    pub fn new(namespace: Vec<String>, name: impl Into<String>) -> Self {
        QualifiedName {
            namespace,
            name: name.into(),
        }
    }

    /// Parse from a `::` separated string like `"Game::Entities::Player"`.
    pub fn from_qualified_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split("::").collect();
        if parts.len() == 1 {
            QualifiedName::global(parts[0])
        } else {
            let namespace = parts[..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let name = parts[parts.len() - 1].to_string();
            QualifiedName { namespace, name }
        }
    }

    /// Create a child name within this name's namespace.
    /// e.g. `Game::Entities` + `Player` → `Game::Entities::Player`
    pub fn child(&self, name: impl Into<String>) -> Self {
        let mut ns = self.namespace.clone();
        ns.push(self.name.clone());
        QualifiedName {
            namespace: ns,
            name: name.into(),
        }
    }

    /// Get the parent namespace as a `QualifiedName`, if any.
    pub fn parent(&self) -> Option<Self> {
        if self.namespace.is_empty() {
            None
        } else {
            let mut ns = self.namespace.clone();
            let parent_name = ns.pop().unwrap();
            Some(QualifiedName {
                namespace: ns,
                name: parent_name,
            })
        }
    }

    /// Whether this name is in the global namespace.
    pub fn is_global(&self) -> bool {
        self.namespace.is_empty()
    }

    /// Full string representation with `::` separators.
    pub fn full_name(&self) -> String {
        if self.namespace.is_empty() {
            self.name.clone()
        } else {
            format!("{}::{}", self.namespace.join("::"), self.name)
        }
    }

    /// Compute the TypeId for this name.
    pub fn type_id(&self) -> TypeId {
        TypeId::from_name(self)
    }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

impl From<&str> for QualifiedName {
    fn from(s: &str) -> Self {
        QualifiedName::from_qualified_string(s)
    }
}

impl From<String> for QualifiedName {
    fn from(s: String) -> Self {
        QualifiedName::from_qualified_string(&s)
    }
}

/// How a type is classified.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeKind {
    /// A primitive scalar type.
    Primitive(PrimitiveType),
    /// A class or struct type.
    Object(TypeId),
    /// An enumeration.
    Enum(TypeId),
    /// A function definition (function pointer type).
    Funcdef(TypeId),
    /// Auto type (to be inferred).
    Auto,
}

/// Reference passing mode for function parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefMode {
    /// Not a reference.
    None,
    /// Input reference (`&in`).
    In,
    /// Output reference (`&out`).
    Out,
    /// Input/output reference (`&inout` or just `&`).
    InOut,
}

/// Access modifier for properties and methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
}

impl Default for AccessModifier {
    fn default() -> Self {
        AccessModifier::Public
    }
}

/// Complete representation of a type with all modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataType {
    /// The underlying type kind.
    pub kind: TypeKind,
    /// Whether this is a `const` type.
    pub is_const: bool,
    /// Whether this is a handle type (`@`).
    pub is_handle: bool,
    /// Whether this is a handle to a const object (`const @`).
    pub is_handle_to_const: bool,
    /// Reference passing mode.
    pub ref_mode: RefMode,
    /// Whether the type is read-only.
    pub is_read_only: bool,
}

impl DataType {
    /// Create a void type.
    pub fn void() -> Self {
        DataType {
            kind: TypeKind::Primitive(PrimitiveType::Void),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Create a primitive type with no modifiers.
    pub fn primitive(prim: PrimitiveType) -> Self {
        DataType {
            kind: TypeKind::Primitive(prim),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Create an object type by TypeId.
    pub fn object(type_id: TypeId) -> Self {
        DataType {
            kind: TypeKind::Object(type_id),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Create an enum type by TypeId.
    pub fn enum_type(type_id: TypeId) -> Self {
        DataType {
            kind: TypeKind::Enum(type_id),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Create a funcdef type by TypeId.
    pub fn funcdef(type_id: TypeId) -> Self {
        DataType {
            kind: TypeKind::Funcdef(type_id),
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Create an auto type (to be inferred).
    pub fn auto() -> Self {
        DataType {
            kind: TypeKind::Auto,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_mode: RefMode::None,
            is_read_only: false,
        }
    }

    /// Return a new DataType with the const modifier set.
    pub fn with_const(mut self) -> Self {
        self.is_const = true;
        self
    }

    /// Return a new DataType as a handle type.
    pub fn with_handle(mut self) -> Self {
        self.is_handle = true;
        self
    }

    /// Return a new DataType with a handle-to-const modifier.
    pub fn with_handle_to_const(mut self) -> Self {
        self.is_handle = true;
        self.is_handle_to_const = true;
        self
    }

    /// Return a new DataType with a reference mode.
    pub fn with_ref(mut self, mode: RefMode) -> Self {
        self.ref_mode = mode;
        self
    }

    /// Return a new DataType marked read-only.
    pub fn with_read_only(mut self) -> Self {
        self.is_read_only = true;
        self
    }

    /// Whether this is a void type.
    pub fn is_void(&self) -> bool {
        matches!(self.kind, TypeKind::Primitive(PrimitiveType::Void))
    }

    /// Whether this is a primitive type (non-void).
    pub fn is_primitive(&self) -> bool {
        matches!(self.kind, TypeKind::Primitive(p) if p != PrimitiveType::Void)
    }

    /// Whether this is an object type.
    pub fn is_object(&self) -> bool {
        matches!(self.kind, TypeKind::Object(_))
    }

    /// Whether this is an enum type.
    pub fn is_enum(&self) -> bool {
        matches!(self.kind, TypeKind::Enum(_))
    }

    /// Whether this is a funcdef type.
    pub fn is_funcdef(&self) -> bool {
        matches!(self.kind, TypeKind::Funcdef(_))
    }

    /// Whether this is a reference (any mode).
    pub fn is_reference(&self) -> bool {
        self.ref_mode != RefMode::None
    }

    /// Whether this is an auto type.
    pub fn is_auto(&self) -> bool {
        matches!(self.kind, TypeKind::Auto)
    }

    /// Get the primitive type, if this is one.
    pub fn as_primitive(&self) -> Option<PrimitiveType> {
        match self.kind {
            TypeKind::Primitive(p) => Some(p),
            _ => None,
        }
    }

    /// Get the TypeId for object/enum/funcdef types.
    pub fn type_id(&self) -> Option<TypeId> {
        match &self.kind {
            TypeKind::Object(id) | TypeKind::Enum(id) | TypeKind::Funcdef(id) => Some(*id),
            _ => None,
        }
    }

    /// Size on the VM stack in dwords.
    pub fn stack_size_dwords(&self) -> u32 {
        if self.is_handle || self.is_reference() {
            // Handles and references are pointer-sized
            (std::mem::size_of::<usize>() / 4) as u32
        } else {
            match &self.kind {
                TypeKind::Primitive(p) => p.stack_dwords(),
                TypeKind::Enum(_) => 1, // Enums are int-sized
                TypeKind::Object(_) | TypeKind::Funcdef(_) => {
                    // Objects on stack are pointers
                    (std::mem::size_of::<usize>() / 4) as u32
                }
                TypeKind::Auto => 0, // Should be resolved before this
            }
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_const {
            write!(f, "const ")?;
        }
        match &self.kind {
            TypeKind::Primitive(p) => write!(f, "{}", p)?,
            TypeKind::Object(id) => write!(f, "object({})", id)?,
            TypeKind::Enum(id) => write!(f, "enum({})", id)?,
            TypeKind::Funcdef(id) => write!(f, "funcdef({})", id)?,
            TypeKind::Auto => write!(f, "auto")?,
        }
        if self.is_handle {
            write!(f, "@")?;
        }
        match self.ref_mode {
            RefMode::None => {}
            RefMode::In => write!(f, " &in")?,
            RefMode::Out => write!(f, " &out")?,
            RefMode::InOut => write!(f, " &")?,
        }
        Ok(())
    }
}

impl Default for DataType {
    fn default() -> Self {
        DataType::void()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_sizes() {
        assert_eq!(PrimitiveType::Void.size(), 0);
        assert_eq!(PrimitiveType::Bool.size(), 1);
        assert_eq!(PrimitiveType::Int8.size(), 1);
        assert_eq!(PrimitiveType::Int16.size(), 2);
        assert_eq!(PrimitiveType::Int32.size(), 4);
        assert_eq!(PrimitiveType::Int64.size(), 8);
        assert_eq!(PrimitiveType::Float.size(), 4);
        assert_eq!(PrimitiveType::Double.size(), 8);
    }

    #[test]
    fn primitive_categories() {
        assert!(PrimitiveType::Int32.is_signed_integer());
        assert!(PrimitiveType::UInt32.is_unsigned_integer());
        assert!(PrimitiveType::Int32.is_integer());
        assert!(PrimitiveType::UInt64.is_integer());
        assert!(PrimitiveType::Float.is_float());
        assert!(PrimitiveType::Double.is_float());
        assert!(PrimitiveType::Int32.is_numeric());
        assert!(PrimitiveType::Float.is_numeric());
        assert!(!PrimitiveType::Bool.is_numeric());
        assert!(!PrimitiveType::Void.is_numeric());
    }

    #[test]
    fn qualified_name_global() {
        let name = QualifiedName::global("Player");
        assert!(name.is_global());
        assert_eq!(name.full_name(), "Player");
        assert_eq!(name.parent(), None);
    }

    #[test]
    fn qualified_name_namespaced() {
        let name = QualifiedName::new(vec!["Game".into(), "Entities".into()], "Player");
        assert!(!name.is_global());
        assert_eq!(name.full_name(), "Game::Entities::Player");
        let parent = name.parent().unwrap();
        assert_eq!(parent.full_name(), "Game::Entities");
    }

    #[test]
    fn qualified_name_from_string() {
        let name = QualifiedName::from_qualified_string("Game::Entities::Player");
        assert_eq!(name.namespace, vec!["Game", "Entities"]);
        assert_eq!(name.name, "Player");

        let simple = QualifiedName::from_qualified_string("Foo");
        assert!(simple.is_global());
        assert_eq!(simple.name, "Foo");
    }

    #[test]
    fn qualified_name_child() {
        let ns = QualifiedName::new(vec!["Game".into()], "Entities");
        let child = ns.child("Player");
        assert_eq!(child.full_name(), "Game::Entities::Player");
    }

    #[test]
    fn type_id_deterministic() {
        let name = QualifiedName::global("Player");
        let id1 = TypeId::from_name(&name);
        let id2 = TypeId::from_name(&name);
        assert_eq!(id1, id2);
    }

    #[test]
    fn type_id_different_names_differ() {
        let id1 = TypeId::from_name(&QualifiedName::global("Player"));
        let id2 = TypeId::from_name(&QualifiedName::global("Enemy"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn data_type_void() {
        let dt = DataType::void();
        assert!(dt.is_void());
        assert!(!dt.is_primitive());
        assert!(!dt.is_object());
        assert_eq!(dt.stack_size_dwords(), 0);
    }

    #[test]
    fn data_type_primitive() {
        let dt = DataType::primitive(PrimitiveType::Int32);
        assert!(dt.is_primitive());
        assert!(!dt.is_void());
        assert_eq!(dt.stack_size_dwords(), 1);
    }

    #[test]
    fn data_type_modifiers() {
        let dt = DataType::object(TypeId(42))
            .with_const()
            .with_handle()
            .with_ref(RefMode::In);
        assert!(dt.is_const);
        assert!(dt.is_handle);
        assert!(dt.is_reference());
        assert_eq!(dt.ref_mode, RefMode::In);
    }

    #[test]
    fn data_type_display() {
        let dt = DataType::primitive(PrimitiveType::Int32);
        assert_eq!(format!("{}", dt), "int");

        let dt = DataType::primitive(PrimitiveType::Float).with_const();
        assert_eq!(format!("{}", dt), "const float");
    }
}
