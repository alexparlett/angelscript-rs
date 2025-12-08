//! TypeEntry enum for unified type storage.
//!
//! This module provides `TypeEntry`, a single enum that wraps all type entry
//! kinds for unified storage and iteration in the registry.

use crate::TypeHash;

use super::{
    ClassEntry, EnumEntry, FuncdefEntry, InterfaceEntry, PrimitiveEntry,
    TemplateParamEntry, TypeSource,
};

/// Unified type entry for registry storage.
///
/// Wraps all type kinds in a single enum for unified storage, iteration,
/// and lookup in the registry.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeEntry {
    /// Primitive type (int, float, bool, etc.).
    Primitive(PrimitiveEntry),
    /// Class type (including templates and template instances).
    Class(ClassEntry),
    /// Enum type.
    Enum(EnumEntry),
    /// Interface type.
    Interface(InterfaceEntry),
    /// Funcdef (function pointer) type.
    Funcdef(FuncdefEntry),
    /// Template parameter placeholder.
    TemplateParam(TemplateParamEntry),
}

impl TypeEntry {
    /// Get the type hash for this entry.
    pub fn type_hash(&self) -> TypeHash {
        match self {
            TypeEntry::Primitive(e) => e.type_hash,
            TypeEntry::Class(e) => e.type_hash,
            TypeEntry::Enum(e) => e.type_hash,
            TypeEntry::Interface(e) => e.type_hash,
            TypeEntry::Funcdef(e) => e.type_hash,
            TypeEntry::TemplateParam(e) => e.type_hash,
        }
    }

    /// Get the unqualified name.
    pub fn name(&self) -> &str {
        match self {
            TypeEntry::Primitive(e) => e.name(),
            TypeEntry::Class(e) => &e.name,
            TypeEntry::Enum(e) => &e.name,
            TypeEntry::Interface(e) => &e.name,
            TypeEntry::Funcdef(e) => &e.name,
            TypeEntry::TemplateParam(e) => &e.name,
        }
    }

    /// Get the qualified name (with namespace).
    pub fn qualified_name(&self) -> &str {
        match self {
            TypeEntry::Primitive(e) => e.name(),
            TypeEntry::Class(e) => &e.qualified_name,
            TypeEntry::Enum(e) => &e.qualified_name,
            TypeEntry::Interface(e) => &e.qualified_name,
            TypeEntry::Funcdef(e) => &e.qualified_name,
            TypeEntry::TemplateParam(e) => &e.name,
        }
    }

    /// Get the source (FFI or script). Returns None for primitives.
    pub fn source(&self) -> Option<&TypeSource> {
        match self {
            TypeEntry::Primitive(_) => None,
            TypeEntry::Class(e) => Some(&e.source),
            TypeEntry::Enum(e) => Some(&e.source),
            TypeEntry::Interface(e) => Some(&e.source),
            TypeEntry::Funcdef(e) => Some(&e.source),
            TypeEntry::TemplateParam(_) => None,
        }
    }

    // === Type Checks ===

    /// Check if this is a primitive type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeEntry::Primitive(_))
    }

    /// Check if this is a class type.
    pub fn is_class(&self) -> bool {
        matches!(self, TypeEntry::Class(_))
    }

    /// Check if this is an enum type.
    pub fn is_enum(&self) -> bool {
        matches!(self, TypeEntry::Enum(_))
    }

    /// Check if this is an interface type.
    pub fn is_interface(&self) -> bool {
        matches!(self, TypeEntry::Interface(_))
    }

    /// Check if this is a funcdef type.
    pub fn is_funcdef(&self) -> bool {
        matches!(self, TypeEntry::Funcdef(_))
    }

    /// Check if this is a template parameter.
    pub fn is_template_param(&self) -> bool {
        matches!(self, TypeEntry::TemplateParam(_))
    }

    /// Check if this is a template definition.
    pub fn is_template(&self) -> bool {
        matches!(self, TypeEntry::Class(e) if e.is_template())
    }

    /// Check if this is a template instance.
    pub fn is_template_instance(&self) -> bool {
        matches!(self, TypeEntry::Class(e) if e.is_template_instance())
    }

    // === Downcasting ===

    /// Get as a primitive entry.
    pub fn as_primitive(&self) -> Option<&PrimitiveEntry> {
        match self {
            TypeEntry::Primitive(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a class entry.
    pub fn as_class(&self) -> Option<&ClassEntry> {
        match self {
            TypeEntry::Class(e) => Some(e),
            _ => None,
        }
    }

    /// Get as an enum entry.
    pub fn as_enum(&self) -> Option<&EnumEntry> {
        match self {
            TypeEntry::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Get as an interface entry.
    pub fn as_interface(&self) -> Option<&InterfaceEntry> {
        match self {
            TypeEntry::Interface(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a funcdef entry.
    pub fn as_funcdef(&self) -> Option<&FuncdefEntry> {
        match self {
            TypeEntry::Funcdef(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a template parameter entry.
    pub fn as_template_param(&self) -> Option<&TemplateParamEntry> {
        match self {
            TypeEntry::TemplateParam(e) => Some(e),
            _ => None,
        }
    }

    // === Mutable Downcasting ===

    /// Get as a mutable class entry.
    pub fn as_class_mut(&mut self) -> Option<&mut ClassEntry> {
        match self {
            TypeEntry::Class(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a mutable enum entry.
    pub fn as_enum_mut(&mut self) -> Option<&mut EnumEntry> {
        match self {
            TypeEntry::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a mutable interface entry.
    pub fn as_interface_mut(&mut self) -> Option<&mut InterfaceEntry> {
        match self {
            TypeEntry::Interface(e) => Some(e),
            _ => None,
        }
    }

    /// Get as a mutable funcdef entry.
    pub fn as_funcdef_mut(&mut self) -> Option<&mut FuncdefEntry> {
        match self {
            TypeEntry::Funcdef(e) => Some(e),
            _ => None,
        }
    }
}

// === From Implementations ===

impl From<PrimitiveEntry> for TypeEntry {
    fn from(entry: PrimitiveEntry) -> Self {
        TypeEntry::Primitive(entry)
    }
}

impl From<ClassEntry> for TypeEntry {
    fn from(entry: ClassEntry) -> Self {
        TypeEntry::Class(entry)
    }
}

impl From<EnumEntry> for TypeEntry {
    fn from(entry: EnumEntry) -> Self {
        TypeEntry::Enum(entry)
    }
}

impl From<InterfaceEntry> for TypeEntry {
    fn from(entry: InterfaceEntry) -> Self {
        TypeEntry::Interface(entry)
    }
}

impl From<FuncdefEntry> for TypeEntry {
    fn from(entry: FuncdefEntry) -> Self {
        TypeEntry::Funcdef(entry)
    }
}

impl From<TemplateParamEntry> for TypeEntry {
    fn from(entry: TemplateParamEntry) -> Self {
        TypeEntry::TemplateParam(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives, DataType, PrimitiveKind, TypeKind};

    #[test]
    fn type_entry_primitive() {
        let entry: TypeEntry = PrimitiveEntry::new(PrimitiveKind::Int32).into();

        assert!(entry.is_primitive());
        assert!(!entry.is_class());
        assert_eq!(entry.type_hash(), primitives::INT32);
        assert_eq!(entry.name(), "int");
        assert_eq!(entry.qualified_name(), "int");
        assert!(entry.source().is_none());
        assert!(entry.as_primitive().is_some());
    }

    #[test]
    fn type_entry_class() {
        let class = ClassEntry::ffi("Player", TypeKind::reference());
        let entry: TypeEntry = class.into();

        assert!(entry.is_class());
        assert!(!entry.is_primitive());
        assert_eq!(entry.name(), "Player");
        assert!(entry.source().is_some());
        assert!(entry.as_class().is_some());
    }

    #[test]
    fn type_entry_enum() {
        let enum_entry = EnumEntry::ffi("Color")
            .with_value("Red", 0)
            .with_value("Green", 1);
        let entry: TypeEntry = enum_entry.into();

        assert!(entry.is_enum());
        assert_eq!(entry.name(), "Color");
        assert!(entry.as_enum().is_some());
    }

    #[test]
    fn type_entry_interface() {
        let interface = InterfaceEntry::ffi("IDrawable");
        let entry: TypeEntry = interface.into();

        assert!(entry.is_interface());
        assert_eq!(entry.name(), "IDrawable");
        assert!(entry.as_interface().is_some());
    }

    #[test]
    fn type_entry_funcdef() {
        let funcdef = FuncdefEntry::ffi("Callback", vec![], DataType::void());
        let entry: TypeEntry = funcdef.into();

        assert!(entry.is_funcdef());
        assert_eq!(entry.name(), "Callback");
        assert!(entry.as_funcdef().is_some());
    }

    #[test]
    fn type_entry_template_param() {
        let owner = TypeHash::from_name("array");
        let param = TemplateParamEntry::for_template("T", 0, owner, "array");
        let entry: TypeEntry = param.into();

        assert!(entry.is_template_param());
        assert_eq!(entry.name(), "T");
        assert!(entry.as_template_param().is_some());
    }

    #[test]
    fn type_entry_template() {
        let t_hash = TypeHash::from_name("array::T");
        let class = ClassEntry::ffi("array", TypeKind::reference())
            .with_template_params(vec![t_hash]);
        let entry: TypeEntry = class.into();

        assert!(entry.is_template());
        assert!(!entry.is_template_instance());
    }

    #[test]
    fn type_entry_template_instance() {
        let template = TypeHash::from_name("array");
        let class = ClassEntry::ffi("array<int>", TypeKind::reference())
            .with_template_instance(template, vec![DataType::simple(primitives::INT32)]);
        let entry: TypeEntry = class.into();

        assert!(entry.is_template_instance());
        assert!(!entry.is_template());
    }

    #[test]
    fn type_entry_mutable_access() {
        let class = ClassEntry::ffi("Mutable", TypeKind::reference());
        let mut entry: TypeEntry = class.into();

        if let Some(class) = entry.as_class_mut() {
            class.is_final = true;
        }

        assert!(entry.as_class().unwrap().is_final);
    }
}
