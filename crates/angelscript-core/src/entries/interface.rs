//! Interface type entry.
//!
//! This module provides `InterfaceEntry` for interface types.

use rustc_hash::FxHashMap;

use crate::{MethodSignature, TypeHash};

use super::TypeSource;

/// Interface method table for dispatch.
///
/// Similar to VTable but for interface methods. Maps signature hashes
/// to slot indices and provides name-based lookup for overload resolution.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ITable {
    /// Maps signature hash to slot index.
    /// Signature hash = name + params (excludes owner for override matching).
    pub index: FxHashMap<u64, u16>,
    /// Maps method name to itable slots for overload resolution.
    /// A single name may have multiple slots (one per overload).
    pub slots_by_name: FxHashMap<String, Vec<u16>>,
    /// Method function hashes in slot order (for overload resolution).
    /// These are the abstract interface method hashes registered during compilation.
    pub methods: Vec<TypeHash>,
}

impl ITable {
    /// Create an empty itable.
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

    /// Get the method function hash at a given slot.
    pub fn method_at(&self, slot: u16) -> Option<TypeHash> {
        self.methods.get(slot as usize).copied()
    }

    /// Check if the itable is empty.
    pub fn is_empty(&self) -> bool {
        self.methods.is_empty()
    }

    /// Get the total number of slots.
    pub fn len(&self) -> u16 {
        self.methods.len() as u16
    }

    /// Add a method to the itable.
    /// Returns the slot index assigned to the method.
    pub fn add_method(&mut self, name: &str, sig_hash: u64, func_hash: TypeHash) -> u16 {
        let slot = self.methods.len() as u16;
        self.methods.push(func_hash);
        self.index.insert(sig_hash, slot);
        self.slots_by_name
            .entry(name.to_string())
            .or_default()
            .push(slot);
        slot
    }

    /// Find all method function hashes by name (for overload resolution).
    pub fn find_methods(&self, name: &str) -> Vec<TypeHash> {
        self.slots_for_name(name)
            .iter()
            .filter_map(|&slot| self.method_at(slot))
            .collect()
    }
}

/// Registry entry for an interface type.
///
/// Interfaces define a contract of methods that implementing classes must provide.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceEntry {
    /// Unqualified name.
    pub name: String,
    /// Namespace path (e.g., `["Game", "Interfaces"]`).
    pub namespace: Vec<String>,
    /// Fully qualified name (with namespace).
    pub qualified_name: String,
    /// Type hash for identity.
    pub type_hash: TypeHash,
    /// Source (FFI or script).
    pub source: TypeSource,
    /// Required method signatures.
    pub methods: Vec<MethodSignature>,
    /// Base interface type hashes.
    pub base_interfaces: Vec<TypeHash>,
    /// Interface method table for dispatch.
    /// Maps signature hashes to slot indices.
    pub itable: ITable,
}

impl InterfaceEntry {
    /// Create a new interface entry.
    pub fn new(
        name: impl Into<String>,
        namespace: Vec<String>,
        qualified_name: impl Into<String>,
        type_hash: TypeHash,
        source: TypeSource,
    ) -> Self {
        Self {
            name: name.into(),
            namespace,
            qualified_name: qualified_name.into(),
            type_hash,
            source,
            methods: Vec::new(),
            base_interfaces: Vec::new(),
            itable: ITable::default(),
        }
    }

    /// Create an FFI interface entry in the global namespace.
    pub fn ffi(name: impl Into<String>) -> Self {
        let name = name.into();
        let type_hash = TypeHash::from_name(&name);
        Self {
            name: name.clone(),
            namespace: Vec::new(),
            qualified_name: name,
            type_hash,
            source: TypeSource::ffi_untyped(),
            methods: Vec::new(),
            base_interfaces: Vec::new(),
            itable: ITable::default(),
        }
    }

    /// Add a method signature.
    pub fn with_method(mut self, method: MethodSignature) -> Self {
        self.methods.push(method);
        self
    }

    /// Add a base interface.
    pub fn with_base(mut self, base: TypeHash) -> Self {
        self.base_interfaces.push(base);
        self
    }

    /// Find a method by name.
    pub fn find_method(&self, name: &str) -> Option<&MethodSignature> {
        self.methods.iter().find(|m| m.name == name)
    }

    /// Check if this interface has a specific base interface.
    pub fn has_base(&self, base: TypeHash) -> bool {
        self.base_interfaces.contains(&base)
    }

    /// Get the itable slot index for a method by its signature hash.
    pub fn method_slot(&self, sig_hash: u64) -> Option<u16> {
        self.itable.slot_by_signature(sig_hash)
    }

    /// Get all itable slots for methods with a given name (for overload resolution).
    pub fn method_slots_by_name(&self, name: &str) -> &[u16] {
        self.itable.slots_for_name(name)
    }

    /// Get total number of itable slots (includes inherited methods).
    pub fn total_slots(&self) -> u16 {
        self.itable.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataType, primitives};

    #[test]
    fn interface_entry_creation() {
        let entry = InterfaceEntry::ffi("IDrawable");

        assert_eq!(entry.name, "IDrawable");
        assert_eq!(entry.qualified_name, "IDrawable");
        assert!(
            entry.namespace.is_empty(),
            "ffi() should create empty namespace"
        );
        assert!(entry.source.is_ffi());
        assert!(entry.methods.is_empty());
        assert!(entry.base_interfaces.is_empty());
    }

    #[test]
    fn interface_entry_with_namespace() {
        let entry = InterfaceEntry::new(
            "IUpdatable",
            vec!["Game".to_string(), "Interfaces".to_string()],
            "Game::Interfaces::IUpdatable",
            TypeHash::from_name("Game::Interfaces::IUpdatable"),
            TypeSource::ffi_untyped(),
        );

        assert_eq!(entry.name, "IUpdatable");
        assert_eq!(
            entry.namespace,
            vec!["Game".to_string(), "Interfaces".to_string()]
        );
        assert_eq!(entry.qualified_name, "Game::Interfaces::IUpdatable");
        assert_eq!(
            entry.type_hash,
            TypeHash::from_name("Game::Interfaces::IUpdatable")
        );
    }

    #[test]
    fn interface_entry_with_method() {
        let draw_method = MethodSignature::new(
            "draw",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::INT32),
            ],
            DataType::void(),
        );

        let entry = InterfaceEntry::ffi("IDrawable").with_method(draw_method);

        assert_eq!(entry.methods.len(), 1);
        assert_eq!(entry.methods[0].name, "draw");
    }

    #[test]
    fn interface_entry_find_method() {
        let update = MethodSignature::new(
            "update",
            vec![DataType::simple(primitives::FLOAT)],
            DataType::void(),
        );
        let render = MethodSignature::new_const("render", vec![], DataType::void());

        let entry = InterfaceEntry::ffi("IEntity")
            .with_method(update)
            .with_method(render);

        assert!(entry.find_method("update").is_some());
        assert!(entry.find_method("render").is_some());
        assert!(entry.find_method("nonexistent").is_none());
    }

    #[test]
    fn interface_entry_with_base() {
        let base = TypeHash::from_name("IBase");
        let entry = InterfaceEntry::ffi("IDerived").with_base(base);

        assert!(entry.has_base(base));
        assert!(!entry.has_base(TypeHash::from_name("IOther")));
    }

    // ITable tests
    #[test]
    fn itable_new_is_empty() {
        let itable = ITable::new();
        assert_eq!(itable.len(), 0);
    }

    #[test]
    fn itable_add_method_creates_slot() {
        let mut itable = ITable::new();
        let sig_hash = 12345u64;
        let func_hash = TypeHash::from_name("draw");

        let slot = itable.add_method("draw", sig_hash, func_hash);

        assert_eq!(slot, 0);
        assert_eq!(itable.len(), 1);
        assert_eq!(itable.method_at(0), Some(func_hash));
    }

    #[test]
    fn itable_add_multiple_methods() {
        let mut itable = ITable::new();
        let draw_hash = TypeHash::from_name("draw");
        let update_hash = TypeHash::from_name("update");

        let slot1 = itable.add_method("draw", 111, draw_hash);
        let slot2 = itable.add_method("update", 222, update_hash);

        assert_eq!(slot1, 0);
        assert_eq!(slot2, 1);
        assert_eq!(itable.len(), 2);
    }

    #[test]
    fn itable_slots_by_name() {
        let mut itable = ITable::new();
        let draw1_hash = TypeHash::from_name("draw1");
        let draw2_hash = TypeHash::from_name("draw2");
        let update_hash = TypeHash::from_name("update");

        // Two overloads of "draw"
        itable.add_method("draw", 111, draw1_hash);
        itable.add_method("draw", 222, draw2_hash);
        itable.add_method("update", 333, update_hash);

        let draw_slots = itable.slots_for_name("draw");
        assert_eq!(draw_slots.len(), 2);

        let update_slots = itable.slots_for_name("update");
        assert_eq!(update_slots.len(), 1);

        let nonexistent = itable.slots_for_name("nonexistent");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn itable_slot_by_signature() {
        let mut itable = ITable::new();
        let sig_hash = 12345u64;
        let func_hash = TypeHash::from_name("draw");

        itable.add_method("draw", sig_hash, func_hash);

        assert_eq!(itable.slot_by_signature(sig_hash), Some(0));
        assert_eq!(itable.slot_by_signature(99999), None);
    }

    #[test]
    fn itable_overload_different_signatures() {
        let mut itable = ITable::new();
        let foo1_hash = TypeHash::from_name("foo1");
        let foo2_hash = TypeHash::from_name("foo2");

        // Same name, different signatures (different overloads)
        let slot1 = itable.add_method("foo", 111, foo1_hash);
        let slot2 = itable.add_method("foo", 222, foo2_hash);

        assert_ne!(slot1, slot2);
        assert_eq!(itable.len(), 2);
        assert_eq!(itable.slot_by_signature(111), Some(0));
        assert_eq!(itable.slot_by_signature(222), Some(1));
    }
}
