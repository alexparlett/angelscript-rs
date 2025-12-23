//! Class type entry.
//!
//! This module provides `ClassEntry` for class types, including template
//! definitions and template instances.

use rustc_hash::FxHashMap;

use crate::{DataType, QualifiedName, TypeBehaviors, TypeHash, TypeKind};

use super::{PropertyEntry, TypeSource};

/// Maps interface type hash to its itable (list of method hashes in slot order).
pub type ITableMap = FxHashMap<TypeHash, Vec<TypeHash>>;

/// Virtual method table for polymorphic dispatch.
///
/// The vtable contains all callable methods (inherited + own) in slot order.
/// At compile time, we look up methods by signature hash to find their slot.
/// At runtime, we dispatch using the slot index directly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VTable {
    /// Method hashes in slot order (inherited first, then own).
    /// The hash at index N is the method to call for slot N.
    pub slots: Vec<TypeHash>,
    /// Maps signature hash to slot index.
    /// Signature hash = name + params (excludes owner for override matching).
    /// Used at compile-time to find the slot for a method call.
    pub index: FxHashMap<u64, u16>,
    /// Maps method name to vtable slots for overload resolution.
    /// A single name may have multiple slots (one per overload).
    /// Used to find candidate slots when resolving `obj.foo(...)`.
    pub slots_by_name: FxHashMap<String, Vec<u16>>,
}

impl VTable {
    /// Create an empty vtable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the slot index for a method by its signature hash.
    pub fn slot_by_signature(&self, sig_hash: u64) -> Option<u16> {
        self.index.get(&sig_hash).copied()
    }

    /// Get all slots for methods with a given name (for overload resolution).
    pub fn slots_for_name(&self, name: &str) -> &[u16] {
        self.slots_by_name
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the method hash at a given slot.
    pub fn method_at(&self, slot: u16) -> Option<TypeHash> {
        self.slots.get(slot as usize).copied()
    }

    /// Check if the vtable is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Get the number of slots.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Add a method to the vtable, or override if signature already exists.
    /// Returns the slot index assigned to the method.
    pub fn add_method(&mut self, name: &str, sig_hash: u64, method_hash: TypeHash) -> u16 {
        if let Some(&slot) = self.index.get(&sig_hash) {
            // Override existing slot (same signature = override)
            self.slots[slot as usize] = method_hash;
            slot
        } else {
            // New method: add new slot
            let slot = self.slots.len() as u16;
            self.slots.push(method_hash);
            self.index.insert(sig_hash, slot);
            self.slots_by_name
                .entry(name.to_string())
                .or_default()
                .push(slot);
            slot
        }
    }

    /// Override an existing slot with a new method.
    pub fn override_slot(&mut self, slot: u16, method_hash: TypeHash) {
        if let Some(existing) = self.slots.get_mut(slot as usize) {
            *existing = method_hash;
        }
    }
}

/// Registry entry for a class type.
///
/// This covers regular classes, template definitions (like `array<T>`),
/// and template instances (like `array<int>`).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassEntry {
    /// Structured qualified name for name-based lookup.
    pub qname: QualifiedName,
    /// Unqualified name.
    pub name: String,
    /// Namespace path (e.g., `["Game", "Entities"]`).
    pub namespace: Vec<String>,
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
    /// Included mixin type hashes (can include multiple mixins).
    pub mixins: Vec<TypeHash>,
    /// Implemented interface type hashes.
    pub interfaces: Vec<TypeHash>,

    // === Members ===
    /// Lifecycle behaviors (constructors, factories, destructor, etc.).
    pub behaviors: TypeBehaviors,
    /// Method function hashes indexed by name for O(1) lookup.
    /// Maps method name to list of overload hashes (actual FunctionEntry stored in registry).
    pub methods: FxHashMap<String, Vec<TypeHash>>,
    /// Virtual properties (backed by getter/setter methods).
    pub properties: Vec<PropertyEntry>,

    // === Template Info ===
    /// Template parameter type hashes (refs to TemplateParamEntry in registry).
    /// Non-empty = template definition.
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
    /// Class is a mixin (not a real type, code gets copied into including classes).
    pub is_mixin: bool,

    // === VTable/ITable (for polymorphic dispatch) ===
    /// Virtual method table for polymorphic dispatch.
    pub vtable: VTable,
    /// Interface tables: maps interface type hash to list of method hashes in interface slot order.
    /// Used for interface method dispatch.
    pub itables: ITableMap,
}

impl ClassEntry {
    /// Create a new class entry.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        type_kind: TypeKind,
        source: TypeSource,
    ) -> Self {
        let name = name.into();
        let qualified_name = qualified_name.into();
        let qname = QualifiedName::new(name.clone(), namespace.clone());
        Self {
            qname,
            name,
            namespace,
            qualified_name,
            type_hash,
            type_kind,
            source,
            base_class: None,
            mixins: Vec::new(),
            interfaces: Vec::new(),
            behaviors: TypeBehaviors::default(),
            methods: FxHashMap::default(),
            properties: Vec::new(),
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            is_final: false,
            is_abstract: false,
            is_mixin: false,
            vtable: VTable::default(),
            itables: ITableMap::default(),
        }
    }

    /// Create a class entry from a QualifiedName.
    pub fn with_qname(qname: QualifiedName, type_kind: TypeKind, source: TypeSource) -> Self {
        let type_hash = qname.to_type_hash();
        Self {
            name: qname.simple_name().to_string(),
            namespace: qname.namespace_path().to_vec(),
            qualified_name: qname.to_string(),
            qname,
            type_hash,
            type_kind,
            source,
            base_class: None,
            mixins: Vec::new(),
            interfaces: Vec::new(),
            behaviors: TypeBehaviors::default(),
            methods: FxHashMap::default(),
            properties: Vec::new(),
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            is_final: false,
            is_abstract: false,
            is_mixin: false,
            vtable: VTable::default(),
            itables: ITableMap::default(),
        }
    }

    /// Create an FFI class entry in the global namespace.
    pub fn ffi(name: impl Into<String>, type_kind: TypeKind) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        let qname = QualifiedName::global(name.clone());
        Self {
            qname,
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name,
            type_hash,
            type_kind,
            source: TypeSource::ffi_untyped(),
            base_class: None,
            mixins: Vec::new(),
            interfaces: Vec::new(),
            behaviors: TypeBehaviors::default(),
            methods: FxHashMap::default(),
            properties: Vec::new(),
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            is_final: false,
            is_abstract: false,
            is_mixin: false,
            vtable: VTable::default(),
            itables: ITableMap::default(),
        }
    }

    /// Create a script class entry.
    pub fn script(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        source: TypeSource,
    ) -> Self {
        let name = name.into();
        let qualified_name = qualified_name.into();
        let type_hash = TypeHash::from_name(&qualified_name);
        let qname = QualifiedName::new(name.clone(), namespace.clone());
        Self {
            qname,
            name,
            namespace,
            qualified_name,
            type_hash,
            type_kind: TypeKind::ScriptObject,
            source,
            base_class: None,
            mixins: Vec::new(),
            interfaces: Vec::new(),
            behaviors: TypeBehaviors::default(),
            methods: FxHashMap::default(),
            properties: Vec::new(),
            template_params: Vec::new(),
            template: None,
            type_args: Vec::new(),
            is_final: false,
            is_abstract: false,
            is_mixin: false,
            vtable: VTable::default(),
            itables: ITableMap::default(),
        }
    }

    /// Get the structured qualified name.
    pub fn qname(&self) -> &QualifiedName {
        &self.qname
    }

    // === Builder Methods ===

    /// Set the base class.
    pub fn with_base(mut self, base: TypeHash) -> Self {
        self.base_class = Some(base);
        self
    }

    /// Add an included mixin.
    pub fn with_mixin(mut self, mixin: TypeHash) -> Self {
        self.mixins.push(mixin);
        self
    }

    /// Add an implemented interface.
    pub fn with_interface(mut self, interface: TypeHash) -> Self {
        self.interfaces.push(interface);
        self
    }

    /// Add a method by name and function hash.
    ///
    /// Methods with the same name are stored together as overloads.
    pub fn with_method(mut self, name: impl Into<String>, method_hash: TypeHash) -> Self {
        self.methods
            .entry(name.into())
            .or_default()
            .push(method_hash);
        self
    }

    /// Add a method by name and function hash (mutable version).
    pub fn add_method(&mut self, name: impl Into<String>, method_hash: TypeHash) {
        self.methods
            .entry(name.into())
            .or_default()
            .push(method_hash);
    }

    /// Add a property.
    pub fn with_property(mut self, property: PropertyEntry) -> Self {
        self.properties.push(property);
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

    /// Mark as mixin.
    pub fn as_mixin(mut self) -> Self {
        self.is_mixin = true;
        self
    }

    /// Create a script mixin class entry.
    pub fn script_mixin(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        source: TypeSource,
    ) -> Self {
        Self::script(name, namespace, qualified_name, source).as_mixin()
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

    /// Check if this class has a method with the given hash.
    pub fn has_method(&self, method_hash: TypeHash) -> bool {
        self.methods
            .values()
            .any(|overloads| overloads.contains(&method_hash))
    }

    /// Check if this class has any method with the given name.
    pub fn has_method_by_name(&self, name: &str) -> bool {
        self.methods.contains_key(name)
    }

    /// Find all method overloads by name. Returns empty slice if not found.
    pub fn find_methods(&self, name: &str) -> &[TypeHash] {
        self.methods.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all method hashes (flattened from all names).
    pub fn all_methods(&self) -> impl Iterator<Item = TypeHash> + '_ {
        self.methods.values().flatten().copied()
    }

    /// Find a property by name.
    pub fn find_property(&self, name: &str) -> Option<&PropertyEntry> {
        self.properties.iter().find(|p| p.name == name)
    }

    /// Check if this class implements a specific interface.
    pub fn implements(&self, interface: TypeHash) -> bool {
        self.interfaces.contains(&interface)
    }

    // === VTable/ITable Methods ===

    /// Find all callable methods by name, including inherited.
    ///
    /// This uses the vtable for lookup, which contains all methods
    /// (own + inherited). Use this for overload resolution.
    ///
    /// For own methods only (docs/LSP), use `find_methods` instead.
    pub fn find_callable_methods(&self, name: &str) -> Vec<TypeHash> {
        self.vtable
            .slots_for_name(name)
            .iter()
            .filter_map(|&slot| self.vtable.method_at(slot))
            .collect()
    }

    /// Get vtable slot index for a method by its signature hash.
    /// Returns None if method not in vtable.
    pub fn vtable_slot(&self, sig_hash: u64) -> Option<u16> {
        self.vtable.slot_by_signature(sig_hash)
    }

    /// Get all vtable slots for methods with a given name (for overload resolution).
    pub fn vtable_slots_by_name(&self, name: &str) -> &[u16] {
        self.vtable.slots_for_name(name)
    }

    /// Get method hash at vtable slot.
    pub fn vtable_method(&self, slot: u16) -> Option<TypeHash> {
        self.vtable.method_at(slot)
    }

    /// Get the itable for a specific interface.
    pub fn itable(&self, interface: TypeHash) -> Option<&Vec<TypeHash>> {
        self.itables.get(&interface)
    }

    /// Get method hash at itable slot for a specific interface.
    pub fn itable_method(&self, interface: TypeHash, slot: u16) -> Option<TypeHash> {
        self.itables
            .get(&interface)
            .and_then(|itable| itable.get(slot as usize).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives;

    #[test]
    fn class_entry_ffi() {
        let entry = ClassEntry::ffi("Player", TypeKind::reference());

        assert_eq!(entry.name, "Player");
        assert_eq!(entry.qualified_name, "Player");
        assert!(
            entry.namespace.is_empty(),
            "ffi() should create empty namespace"
        );
        assert!(entry.source.is_ffi());
        assert!(entry.is_reference_type());
        assert!(!entry.is_template());
        assert!(!entry.is_template_instance());
    }

    #[test]
    fn class_entry_with_namespace() {
        let entry = ClassEntry::new(
            "Enemy",
            vec!["Game".to_string(), "Entities".to_string()],
            "Game::Entities::Enemy",
            TypeHash::from_name("Game::Entities::Enemy"),
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );

        assert_eq!(entry.name, "Enemy");
        assert_eq!(
            entry.namespace,
            vec!["Game".to_string(), "Entities".to_string()]
        );
        assert_eq!(entry.qualified_name, "Game::Entities::Enemy");
        assert_eq!(
            entry.type_hash,
            TypeHash::from_name("Game::Entities::Enemy")
        );
    }

    #[test]
    fn class_entry_script() {
        let source = TypeSource::script(crate::UnitId::new(0), crate::Span::new(1, 0, 10));
        let entry = ClassEntry::script("Entity", vec!["Game".to_string()], "Game::Entity", source);

        assert_eq!(entry.name, "Entity");
        assert_eq!(entry.namespace, vec!["Game".to_string()]);
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
        let method_hash = TypeHash::from_name("Entity::update");
        let entry =
            ClassEntry::ffi("Entity", TypeKind::reference()).with_method("update", method_hash);

        assert_eq!(entry.methods.len(), 1);
        assert_eq!(entry.find_methods("update"), &[method_hash]);
        assert!(entry.has_method(method_hash));
        assert!(!entry.has_method(TypeHash::from_name("nonexistent")));
    }

    #[test]
    fn class_entry_with_property() {
        let getter = TypeHash::from_name("get_health");
        let prop = PropertyEntry::read_only("health", DataType::simple(primitives::INT32), getter);
        let entry = ClassEntry::ffi("Player", TypeKind::reference()).with_property(prop);

        assert!(entry.find_property("health").is_some());
    }

    #[test]
    fn class_entry_template() {
        let t_hash = TypeHash::from_name("array::T");
        let entry =
            ClassEntry::ffi("array", TypeKind::reference()).with_template_params(vec![t_hash]);

        assert!(entry.is_template());
        assert!(!entry.is_template_instance());
        assert_eq!(entry.template_params[0], t_hash);
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

    // VTable tests
    #[test]
    fn vtable_new_is_empty() {
        let vtable = VTable::new();
        assert_eq!(vtable.len(), 0);
        assert!(vtable.is_empty());
    }

    #[test]
    fn vtable_add_method_creates_slot() {
        let mut vtable = VTable::new();
        let method_hash = TypeHash::from_name("Entity::update");
        let sig_hash = 12345u64;

        let slot = vtable.add_method("update", sig_hash, method_hash);

        assert_eq!(slot, 0);
        assert_eq!(vtable.len(), 1);
        assert_eq!(vtable.method_at(0), Some(method_hash));
    }

    #[test]
    fn vtable_add_multiple_methods() {
        let mut vtable = VTable::new();
        let update_hash = TypeHash::from_name("Entity::update");
        let render_hash = TypeHash::from_name("Entity::render");

        let slot1 = vtable.add_method("update", 111, update_hash);
        let slot2 = vtable.add_method("render", 222, render_hash);

        assert_eq!(slot1, 0);
        assert_eq!(slot2, 1);
        assert_eq!(vtable.len(), 2);
    }

    #[test]
    fn vtable_override_uses_same_slot() {
        let mut vtable = VTable::new();
        let base_foo = TypeHash::from_name("Base::foo");
        let derived_foo = TypeHash::from_name("Derived::foo");
        let sig_hash = 12345u64;

        // Add base method
        let slot1 = vtable.add_method("foo", sig_hash, base_foo);
        assert_eq!(vtable.method_at(0), Some(base_foo));

        // Override with derived method (same signature hash)
        let slot2 = vtable.add_method("foo", sig_hash, derived_foo);

        // Same slot, but now points to derived method
        assert_eq!(slot1, slot2);
        assert_eq!(vtable.len(), 1);
        assert_eq!(vtable.method_at(0), Some(derived_foo));
    }

    #[test]
    fn vtable_overload_creates_new_slot() {
        let mut vtable = VTable::new();
        let foo_int = TypeHash::from_name("Entity::foo_int");
        let foo_float = TypeHash::from_name("Entity::foo_float");

        // Different signature hashes = different overloads
        let slot1 = vtable.add_method("foo", 111, foo_int);
        let slot2 = vtable.add_method("foo", 222, foo_float);

        assert_ne!(slot1, slot2);
        assert_eq!(vtable.len(), 2);
    }

    #[test]
    fn vtable_slots_by_name() {
        let mut vtable = VTable::new();
        let foo1 = TypeHash::from_name("Entity::foo1");
        let foo2 = TypeHash::from_name("Entity::foo2");
        let bar = TypeHash::from_name("Entity::bar");

        vtable.add_method("foo", 111, foo1);
        vtable.add_method("foo", 222, foo2);
        vtable.add_method("bar", 333, bar);

        let foo_slots = vtable.slots_for_name("foo");
        assert_eq!(foo_slots.len(), 2);

        let bar_slots = vtable.slots_for_name("bar");
        assert_eq!(bar_slots.len(), 1);

        let nonexistent = vtable.slots_for_name("nonexistent");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn vtable_slot_by_signature() {
        let mut vtable = VTable::new();
        let method = TypeHash::from_name("Entity::update");
        let sig_hash = 12345u64;

        vtable.add_method("update", sig_hash, method);

        assert_eq!(vtable.slot_by_signature(sig_hash), Some(0));
        assert_eq!(vtable.slot_by_signature(99999), None);
    }

    #[test]
    fn vtable_override_slot() {
        let mut vtable = VTable::new();
        let original = TypeHash::from_name("Base::foo");
        let replacement = TypeHash::from_name("Derived::foo");

        vtable.add_method("foo", 111, original);
        assert_eq!(vtable.method_at(0), Some(original));

        vtable.override_slot(0, replacement);
        assert_eq!(vtable.method_at(0), Some(replacement));
    }

    #[test]
    fn vtable_slots_for_name_returns_method_hashes() {
        let mut vtable = VTable::new();
        let foo1 = TypeHash::from_name("Entity::foo1");
        let foo2 = TypeHash::from_name("Entity::foo2");

        vtable.add_method("foo", 111, foo1);
        vtable.add_method("foo", 222, foo2);

        let slots = vtable.slots_for_name("foo");
        assert_eq!(slots.len(), 2);
        // Verify we can get the methods from slots
        let methods: Vec<TypeHash> = slots
            .iter()
            .filter_map(|&slot| vtable.method_at(slot))
            .collect();
        assert!(methods.contains(&foo1));
        assert!(methods.contains(&foo2));
    }

    #[test]
    fn find_callable_methods_uses_vtable() {
        let mut class = ClassEntry::ffi("Entity", TypeKind::reference());
        let update_hash = TypeHash::from_name("Entity::update");

        // Add to vtable
        class.vtable.add_method("update", 111, update_hash);

        let methods = class.find_callable_methods("update");
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], update_hash);
    }
}
