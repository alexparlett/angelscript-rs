//! Class type entry.
//!
//! This module provides `ClassEntry` for class types, including template
//! definitions and template instances.

use crate::{DataType, TypeBehaviors, TypeHash, TypeKind};

use super::{FieldEntry, FunctionEntry, PropertyEntry, TypeSource};

/// Registry entry for a class type.
///
/// This covers regular classes, template definitions (like `array<T>`),
/// and template instances (like `array<int>`).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassEntry {
    /// Unqualified name.
    pub name: String,
    /// Fully qualified name (with namespace).
    pub qualified_name: String,
    /// Type hash for identity.
    pub type_hash: TypeHash,
    /// Type kind (value, reference, script object).
    pub type_kind: TypeKind,
    /// Source (FFI or script).
    pub source: TypeSource,

    // === Inheritance ===
    /// Base class type hash (single inheritance).
    pub base_class: Option<TypeHash>,
    /// Implemented interface type hashes.
    pub interfaces: Vec<TypeHash>,

    // === Members ===
    /// Lifecycle behaviors (constructors, factories, destructor, etc.).
    pub behaviors: TypeBehaviors,
    /// Methods (function entries, not just hashes).
    pub methods: Vec<FunctionEntry>,
    /// Virtual properties.
    pub properties: Vec<PropertyEntry>,
    /// Direct field members.
    pub fields: Vec<FieldEntry>,

    // === Template Info ===
    /// Template parameter type hashes (non-empty = template definition).
    pub template_params: Vec<TypeHash>,
    /// Template this was instantiated from (for template instances).
    pub template: Option<TypeHash>,
    /// Type arguments for template instances.
    pub type_args: Vec<DataType>,

    // === Modifiers ===
    /// Class is marked `final`.
    pub is_final: bool,
    /// Class is marked `abstract`.
    pub is_abstract: bool,
}

impl ClassEntry {
    /// Create a new class entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        type_kind: TypeKind,
        source: TypeSource,
    ) -> Self {
        Self {
            name: name.into(),
            qualified_name: qualified_name.into(),
            type_hash,
            type_kind,
            source,
            base_class: None,
            interfaces: Vec::new(),
            behaviors: TypeBehaviors::default(),
            methods: Vec::new(),
            properties: Vec::new(),
            fields: Vec::new(),
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            is_final: false,
            is_abstract: false,
        }
    }

    /// Create an FFI class entry.
    pub fn ffi(name: impl Into<String>, type_kind: TypeKind) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        Self::new(name.clone(), name, type_hash, type_kind, TypeSource::ffi_untyped())
    }

    /// Create a script class entry.
    pub fn script(
        name: impl Into<String>,
        qualified_name: impl Into<String>,
        source: TypeSource,
    ) -> Self {
        let name = name.into();
        let qualified_name = qualified_name.into();
        let type_hash = TypeHash::from_name(&qualified_name);
        Self::new(name, qualified_name, type_hash, TypeKind::ScriptObject, source)
    }

    // === Builder Methods ===

    /// Set the base class.
    pub fn with_base(mut self, base: TypeHash) -> Self {
        self.base_class = Some(base);
        self
    }

    /// Add an implemented interface.
    pub fn with_interface(mut self, interface: TypeHash) -> Self {
        self.interfaces.push(interface);
        self
    }

    /// Add a method.
    pub fn with_method(mut self, method: FunctionEntry) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a property.
    pub fn with_property(mut self, property: PropertyEntry) -> Self {
        self.properties.push(property);
        self
    }

    /// Add a field.
    pub fn with_field(mut self, field: FieldEntry) -> Self {
        self.fields.push(field);
        self
    }

    /// Set template parameters (makes this a template definition).
    pub fn with_template_params(mut self, params: Vec<TypeHash>) -> Self {
        self.template_params = params;
        self
    }

    /// Set template origin (makes this a template instance).
    pub fn with_template_instance(mut self, template: TypeHash, type_args: Vec<DataType>) -> Self {
        self.template = Some(template);
        self.type_args = type_args;
        self
    }

    /// Mark as final.
    pub fn as_final(mut self) -> Self {
        self.is_final = true;
        self
    }

    /// Mark as abstract.
    pub fn as_abstract(mut self) -> Self {
        self.is_abstract = true;
        self
    }

    // === Query Methods ===

    /// Check if this is a template definition.
    pub fn is_template(&self) -> bool {
        !self.template_params.is_empty()
    }

    /// Check if this is a template instance.
    pub fn is_template_instance(&self) -> bool {
        self.template.is_some()
    }

    /// Check if this is a value type.
    pub fn is_value_type(&self) -> bool {
        self.type_kind.is_value()
    }

    /// Check if this is a reference type.
    pub fn is_reference_type(&self) -> bool {
        self.type_kind.is_reference()
    }

    /// Check if this is a script object.
    pub fn is_script_object(&self) -> bool {
        self.type_kind.is_script_object()
    }

    /// Find a method by name.
    pub fn find_method(&self, name: &str) -> Option<&FunctionEntry> {
        self.methods.iter().find(|m| m.def.name == name)
    }

    /// Find a property by name.
    pub fn find_property(&self, name: &str) -> Option<&PropertyEntry> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// Find a field by name.
    pub fn find_field(&self, name: &str) -> Option<&FieldEntry> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Check if this class implements a specific interface.
    pub fn implements(&self, interface: TypeHash) -> bool {
        self.interfaces.contains(&interface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives, FunctionDef, FunctionTraits, Visibility};

    fn make_test_method(name: &str) -> FunctionEntry {
        let func_hash = TypeHash::from_name(name);
        FunctionEntry::ffi(FunctionDef::new(
            func_hash,
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            true,
            Visibility::Public,
        ))
    }

    #[test]
    fn class_entry_ffi() {
        let entry = ClassEntry::ffi("Player", TypeKind::reference());

        assert_eq!(entry.name, "Player");
        assert_eq!(entry.qualified_name, "Player");
        assert!(entry.source.is_ffi());
        assert!(entry.is_reference_type());
        assert!(!entry.is_template());
        assert!(!entry.is_template_instance());
    }

    #[test]
    fn class_entry_script() {
        let source = TypeSource::script(crate::UnitId::new(0), crate::Span::new(1, 0, 10));
        let entry = ClassEntry::script("Entity", "Game::Entity", source);

        assert_eq!(entry.name, "Entity");
        assert_eq!(entry.qualified_name, "Game::Entity");
        assert!(entry.source.is_script());
        assert!(entry.is_script_object());
    }

    #[test]
    fn class_entry_with_base() {
        let base = TypeHash::from_name("Entity");
        let entry = ClassEntry::ffi("Player", TypeKind::reference()).with_base(base);

        assert_eq!(entry.base_class, Some(base));
    }

    #[test]
    fn class_entry_with_interface() {
        let drawable = TypeHash::from_name("IDrawable");
        let updatable = TypeHash::from_name("IUpdatable");
        let entry = ClassEntry::ffi("Sprite", TypeKind::reference())
            .with_interface(drawable)
            .with_interface(updatable);

        assert!(entry.implements(drawable));
        assert!(entry.implements(updatable));
        assert!(!entry.implements(TypeHash::from_name("IOther")));
    }

    #[test]
    fn class_entry_with_method() {
        let method = make_test_method("update");
        let entry = ClassEntry::ffi("Entity", TypeKind::reference()).with_method(method);

        assert_eq!(entry.methods.len(), 1);
        assert!(entry.find_method("update").is_some());
        assert!(entry.find_method("nonexistent").is_none());
    }

    #[test]
    fn class_entry_with_property() {
        let getter = TypeHash::from_name("get_health");
        let prop = PropertyEntry::read_only("health", DataType::simple(primitives::INT32), getter);
        let entry = ClassEntry::ffi("Player", TypeKind::reference()).with_property(prop);

        assert!(entry.find_property("health").is_some());
    }

    #[test]
    fn class_entry_with_field() {
        let field = FieldEntry::public("x", DataType::simple(primitives::FLOAT));
        let entry = ClassEntry::ffi("Vector2", TypeKind::value::<[f32; 2]>()).with_field(field);

        assert!(entry.find_field("x").is_some());
    }

    #[test]
    fn class_entry_template() {
        let t_param = TypeHash::from_name("array::T");
        let entry = ClassEntry::ffi("array", TypeKind::reference())
            .with_template_params(vec![t_param]);

        assert!(entry.is_template());
        assert!(!entry.is_template_instance());
    }

    #[test]
    fn class_entry_template_instance() {
        let array_template = TypeHash::from_name("array");
        let entry = ClassEntry::ffi("array<int>", TypeKind::reference())
            .with_template_instance(array_template, vec![DataType::simple(primitives::INT32)]);

        assert!(entry.is_template_instance());
        assert!(!entry.is_template());
        assert_eq!(entry.template, Some(array_template));
        assert_eq!(entry.type_args.len(), 1);
    }

    #[test]
    fn class_entry_modifiers() {
        let final_entry = ClassEntry::ffi("FinalClass", TypeKind::reference()).as_final();
        let abstract_entry = ClassEntry::ffi("AbstractClass", TypeKind::reference()).as_abstract();

        assert!(final_entry.is_final);
        assert!(!final_entry.is_abstract);
        assert!(abstract_entry.is_abstract);
        assert!(!abstract_entry.is_final);
    }
}
