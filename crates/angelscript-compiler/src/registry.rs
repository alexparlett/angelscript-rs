//! Symbol registry — central store for all types, functions, and globals.
//!
//! The registry is indexed by `QualifiedName` as the primary key.
//! `TypeId` hashes are computed lazily and cached for bytecode generation.

use std::collections::HashMap;

use angelscript_core::{
    AccessModifier, DataType, FunctionId, FunctionKind, FunctionSignature, FunctionTraits,
    PropertyInfo, QualifiedName, TypeId,
};

// ── Type flags ────────────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Flags that describe a registered type.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TypeFlags: u32 {
        const SHARED   = 1 << 0;
        const ABSTRACT = 1 << 1;
        const FINAL    = 1 << 2;
        const EXTERNAL = 1 << 3;
    }
}

// ── Type entries ──────────────────────────────────────────────────────────

/// A registered type in the symbol table.
#[derive(Debug, Clone)]
pub enum TypeEntry {
    Class(ClassEntry),
    Interface(InterfaceEntry),
    Enum(EnumEntry),
    Funcdef(FuncdefEntry),
    Typedef(TypedefEntry),
}

impl TypeEntry {
    /// Get the qualified name of this type entry.
    pub fn name(&self) -> &QualifiedName {
        match self {
            TypeEntry::Class(e) => &e.name,
            TypeEntry::Interface(e) => &e.name,
            TypeEntry::Enum(e) => &e.name,
            TypeEntry::Funcdef(e) => &e.name,
            TypeEntry::Typedef(e) => &e.name,
        }
    }

    /// Get the cached TypeId.
    pub fn type_id(&self) -> TypeId {
        match self {
            TypeEntry::Class(e) => e.type_id,
            TypeEntry::Interface(e) => e.type_id,
            TypeEntry::Enum(e) => e.type_id,
            TypeEntry::Funcdef(e) => e.type_id,
            TypeEntry::Typedef(e) => e.type_id,
        }
    }

    /// Get the flags for this type.
    pub fn flags(&self) -> TypeFlags {
        match self {
            TypeEntry::Class(e) => e.flags,
            TypeEntry::Interface(e) => e.flags,
            TypeEntry::Enum(e) => e.flags,
            TypeEntry::Funcdef(e) => e.flags,
            TypeEntry::Typedef(_) => TypeFlags::empty(),
        }
    }
}

/// A registered class type.
#[derive(Debug, Clone)]
pub struct ClassEntry {
    pub name: QualifiedName,
    pub type_id: TypeId,
    pub flags: TypeFlags,
    pub properties: Vec<PropertyInfo>,
    pub methods: Vec<FunctionId>,
    pub base_class: Option<QualifiedName>,
    pub interfaces: Vec<QualifiedName>,
    pub vtable: Vec<FunctionId>,
    pub size: usize,
}

impl ClassEntry {
    pub fn new(name: QualifiedName) -> Self {
        let type_id = name.type_id();
        ClassEntry {
            name,
            type_id,
            flags: TypeFlags::empty(),
            properties: Vec::new(),
            methods: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            vtable: Vec::new(),
            size: 0,
        }
    }
}

/// A registered interface type.
#[derive(Debug, Clone)]
pub struct InterfaceEntry {
    pub name: QualifiedName,
    pub type_id: TypeId,
    pub flags: TypeFlags,
    pub methods: Vec<FunctionId>,
    pub bases: Vec<QualifiedName>,
}

impl InterfaceEntry {
    pub fn new(name: QualifiedName) -> Self {
        let type_id = name.type_id();
        InterfaceEntry {
            name,
            type_id,
            flags: TypeFlags::empty(),
            methods: Vec::new(),
            bases: Vec::new(),
        }
    }
}

/// A registered enum type.
#[derive(Debug, Clone)]
pub struct EnumEntry {
    pub name: QualifiedName,
    pub type_id: TypeId,
    pub flags: TypeFlags,
    pub values: Vec<EnumValueEntry>,
}

impl EnumEntry {
    pub fn new(name: QualifiedName) -> Self {
        let type_id = name.type_id();
        EnumEntry {
            name,
            type_id,
            flags: TypeFlags::empty(),
            values: Vec::new(),
        }
    }
}

/// A single enum value.
#[derive(Debug, Clone)]
pub struct EnumValueEntry {
    pub name: String,
    pub value: i64,
}

/// A registered funcdef (function pointer type).
#[derive(Debug, Clone)]
pub struct FuncdefEntry {
    pub name: QualifiedName,
    pub type_id: TypeId,
    pub flags: TypeFlags,
    pub signature: Option<FunctionSignature>,
}

impl FuncdefEntry {
    pub fn new(name: QualifiedName) -> Self {
        let type_id = name.type_id();
        FuncdefEntry {
            name,
            type_id,
            flags: TypeFlags::empty(),
            signature: None,
        }
    }
}

/// A registered typedef (alias for a primitive).
#[derive(Debug, Clone)]
pub struct TypedefEntry {
    pub name: QualifiedName,
    pub type_id: TypeId,
    pub aliased_type: DataType,
}

impl TypedefEntry {
    pub fn new(name: QualifiedName, aliased_type: DataType) -> Self {
        let type_id = name.type_id();
        TypedefEntry {
            name,
            type_id,
            aliased_type,
        }
    }
}

// ── Function entries ──────────────────────────────────────────────────────

/// A registered function.
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    pub id: FunctionId,
    pub name: QualifiedName,
    pub signature: FunctionSignature,
    pub kind: FunctionKind,
    pub traits: FunctionTraits,
    /// The owning type for methods (class or interface name).
    pub owner: Option<QualifiedName>,
    pub access: AccessModifier,
}

// ── Global entries ────────────────────────────────────────────────────────

/// A registered global variable.
#[derive(Debug, Clone)]
pub struct GlobalEntry {
    pub name: QualifiedName,
    pub data_type: DataType,
    pub access: AccessModifier,
}

// ── Registry ──────────────────────────────────────────────────────────────

/// Central symbol registry. Primary key for types is `QualifiedName`.
#[derive(Debug, Clone)]
pub struct SymbolRegistry {
    /// All registered types, indexed by qualified name.
    types: HashMap<QualifiedName, TypeEntry>,
    /// TypeId → QualifiedName reverse index (built during completion).
    type_id_index: HashMap<TypeId, QualifiedName>,
    /// All registered functions, indexed by FunctionId.
    functions: Vec<FunctionEntry>,
    /// Function name → list of FunctionIds (for overload lookup).
    function_name_index: HashMap<QualifiedName, Vec<FunctionId>>,
    /// Global variables, indexed by qualified name.
    globals: HashMap<QualifiedName, GlobalEntry>,
    /// Known namespaces.
    namespaces: Vec<Vec<String>>,
    /// Next function ID to allocate.
    next_function_id: u32,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        SymbolRegistry {
            types: HashMap::new(),
            type_id_index: HashMap::new(),
            functions: Vec::new(),
            function_name_index: HashMap::new(),
            globals: HashMap::new(),
            namespaces: Vec::new(),
            next_function_id: 0,
        }
    }

    // ── Type registration ─────────────────────────────────────────────

    /// Register a type. Returns an error if the name is already taken.
    pub fn register_type(&mut self, entry: TypeEntry) -> Result<(), RegistryError> {
        let name = entry.name().clone();
        if self.types.contains_key(&name) {
            return Err(RegistryError::DuplicateType(name));
        }
        self.types.insert(name, entry);
        Ok(())
    }

    /// Look up a type by qualified name.
    pub fn get_type(&self, name: &QualifiedName) -> Option<&TypeEntry> {
        self.types.get(name)
    }

    /// Look up a type mutably.
    pub fn get_type_mut(&mut self, name: &QualifiedName) -> Option<&mut TypeEntry> {
        self.types.get_mut(name)
    }

    /// Look up a type by TypeId (requires the type_id_index to be built).
    pub fn get_type_by_id(&self, id: TypeId) -> Option<&TypeEntry> {
        self.type_id_index
            .get(&id)
            .and_then(|name| self.types.get(name))
    }

    /// Check if a type is registered.
    pub fn has_type(&self, name: &QualifiedName) -> bool {
        self.types.contains_key(name)
    }

    /// Iterate over all registered types.
    pub fn types(&self) -> impl Iterator<Item = (&QualifiedName, &TypeEntry)> {
        self.types.iter()
    }

    /// Build the TypeId → QualifiedName reverse index.
    pub fn build_type_id_index(&mut self) {
        self.type_id_index.clear();
        for (name, entry) in &self.types {
            self.type_id_index.insert(entry.type_id(), name.clone());
        }
    }

    // ── Function registration ─────────────────────────────────────────

    /// Register a function and return its assigned FunctionId.
    pub fn register_function(&mut self, entry: FunctionEntry) -> FunctionId {
        let id = entry.id;
        let name = entry.name.clone();

        self.functions.push(entry);
        self.function_name_index.entry(name).or_default().push(id);
        id
    }

    /// Allocate the next FunctionId.
    pub fn next_function_id(&mut self) -> FunctionId {
        let id = FunctionId(self.next_function_id);
        self.next_function_id += 1;
        id
    }

    /// Look up a function by ID.
    pub fn get_function(&self, id: FunctionId) -> Option<&FunctionEntry> {
        self.functions.get(id.0 as usize)
    }

    /// Look up a function mutably by ID.
    pub fn get_function_mut(&mut self, id: FunctionId) -> Option<&mut FunctionEntry> {
        self.functions.get_mut(id.0 as usize)
    }

    /// Get all function IDs for a given qualified name (overloads).
    pub fn get_functions_by_name(&self, name: &QualifiedName) -> &[FunctionId] {
        self.function_name_index
            .get(name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Iterate over all registered functions.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionEntry> {
        self.functions.iter()
    }

    /// Total number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    // ── Global registration ───────────────────────────────────────────

    /// Register a global variable.
    pub fn register_global(&mut self, entry: GlobalEntry) -> Result<(), RegistryError> {
        let name = entry.name.clone();
        if self.globals.contains_key(&name) {
            return Err(RegistryError::DuplicateGlobal(name));
        }
        self.globals.insert(name, entry);
        Ok(())
    }

    /// Look up a global variable.
    pub fn get_global(&self, name: &QualifiedName) -> Option<&GlobalEntry> {
        self.globals.get(name)
    }

    /// Iterate over all global variables.
    pub fn globals(&self) -> impl Iterator<Item = (&QualifiedName, &GlobalEntry)> {
        self.globals.iter()
    }

    // ── Namespace registration ────────────────────────────────────────

    /// Register a namespace path (e.g., `["Game", "Entities"]`).
    pub fn register_namespace(&mut self, path: Vec<String>) {
        if !self.namespaces.contains(&path) {
            self.namespaces.push(path);
        }
    }

    /// Check if a namespace is registered.
    pub fn has_namespace(&self, path: &[String]) -> bool {
        self.namespaces.iter().any(|ns| ns == path)
    }

    // ── Utility ───────────────────────────────────────────────────────

    /// Resolve a name in the given namespace context.
    /// Tries the full qualified name first, then searches parent namespaces.
    pub fn resolve_name(&self, name: &str, namespace: &[String]) -> Option<QualifiedName> {
        // Try in the current namespace first, then walk up to global.
        let mut ns = namespace.to_vec();
        loop {
            let qn = QualifiedName::new(ns.clone(), name);
            if self.types.contains_key(&qn) || self.globals.contains_key(&qn) {
                return Some(qn);
            }
            if ns.is_empty() {
                break;
            }
            ns.pop();
        }
        None
    }
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Errors ────────────────────────────────────────────────────────────────

/// Errors that can occur during registry operations.
#[derive(Debug, Clone)]
pub enum RegistryError {
    DuplicateType(QualifiedName),
    DuplicateGlobal(QualifiedName),
    TypeNotFound(QualifiedName),
    FunctionNotFound(FunctionId),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::DuplicateType(n) => write!(f, "duplicate type: {}", n),
            RegistryError::DuplicateGlobal(n) => write!(f, "duplicate global: {}", n),
            RegistryError::TypeNotFound(n) => write!(f, "type not found: {}", n),
            RegistryError::FunctionNotFound(id) => write!(f, "function not found: {}", id),
        }
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{DataType, PrimitiveType};

    #[test]
    fn register_and_lookup_class() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("Player");
        let entry = ClassEntry::new(name.clone());
        reg.register_type(TypeEntry::Class(entry)).unwrap();

        assert!(reg.has_type(&name));
        let te = reg.get_type(&name).unwrap();
        assert_eq!(te.name(), &name);
    }

    #[test]
    fn duplicate_type_error() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("Player");
        reg.register_type(TypeEntry::Class(ClassEntry::new(name.clone())))
            .unwrap();
        let result = reg.register_type(TypeEntry::Class(ClassEntry::new(name)));
        assert!(result.is_err());
    }

    #[test]
    fn register_and_lookup_enum() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("Color");
        let mut entry = EnumEntry::new(name.clone());
        entry.values.push(EnumValueEntry {
            name: "Red".into(),
            value: 0,
        });
        entry.values.push(EnumValueEntry {
            name: "Green".into(),
            value: 1,
        });
        reg.register_type(TypeEntry::Enum(entry)).unwrap();

        let te = reg.get_type(&name).unwrap();
        match te {
            TypeEntry::Enum(e) => assert_eq!(e.values.len(), 2),
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn register_function_and_overloads() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("print");

        // Register two overloads.
        let id1 = reg.next_function_id();
        reg.register_function(FunctionEntry {
            id: id1,
            name: name.clone(),
            signature: FunctionSignature::new(name.clone(), DataType::void(), vec![]),
            kind: FunctionKind::Script,
            traits: FunctionTraits::new(),
            owner: None,
            access: AccessModifier::Public,
        });

        let id2 = reg.next_function_id();
        reg.register_function(FunctionEntry {
            id: id2,
            name: name.clone(),
            signature: FunctionSignature::new(
                name.clone(),
                DataType::void(),
                vec![angelscript_core::ParamInfo::new(
                    "msg",
                    DataType::primitive(PrimitiveType::Int32),
                )],
            ),
            kind: FunctionKind::Script,
            traits: FunctionTraits::new(),
            owner: None,
            access: AccessModifier::Public,
        });

        let overloads = reg.get_functions_by_name(&name);
        assert_eq!(overloads.len(), 2);
        assert_eq!(overloads[0], id1);
        assert_eq!(overloads[1], id2);
    }

    #[test]
    fn register_global() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("g_count");
        reg.register_global(GlobalEntry {
            name: name.clone(),
            data_type: DataType::primitive(PrimitiveType::Int32),
            access: AccessModifier::Public,
        })
        .unwrap();

        let g = reg.get_global(&name).unwrap();
        assert!(g.data_type.is_primitive());
    }

    #[test]
    fn type_id_reverse_index() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::global("Enemy");
        let entry = ClassEntry::new(name.clone());
        let tid = entry.type_id;
        reg.register_type(TypeEntry::Class(entry)).unwrap();
        reg.build_type_id_index();

        let te = reg.get_type_by_id(tid).unwrap();
        assert_eq!(te.name(), &name);
    }

    #[test]
    fn resolve_name_in_namespace() {
        let mut reg = SymbolRegistry::new();
        let name = QualifiedName::new(vec!["Game".into()], "Player");
        reg.register_type(TypeEntry::Class(ClassEntry::new(name.clone())))
            .unwrap();

        // Resolve from within the Game namespace.
        let resolved = reg.resolve_name("Player", &["Game".into()]);
        assert_eq!(resolved, Some(name.clone()));

        // Not resolvable from global without qualification.
        let resolved_global = reg.resolve_name("Player", &[]);
        assert_eq!(resolved_global, None);
    }

    #[test]
    fn namespace_registration() {
        let mut reg = SymbolRegistry::new();
        reg.register_namespace(vec!["Game".into(), "Entities".into()]);
        assert!(reg.has_namespace(&["Game".into(), "Entities".into()]));
        assert!(!reg.has_namespace(&["Game".into()]));
    }
}
