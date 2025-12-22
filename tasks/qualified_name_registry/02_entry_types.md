# Phase 2: Entry Type Updates

## Overview

Update entry types in `angelscript-core` to support deferred type resolution using `QualifiedName` as the primary identifier and storing unresolved references for later completion.

**Files:**
- `crates/angelscript-core/src/entries/class.rs` (update)
- `crates/angelscript-core/src/entries/interface.rs` (update)
- `crates/angelscript-core/src/entries/funcdef.rs` (update)
- `crates/angelscript-core/src/function_def.rs` (update)

---

## Key Changes

### 1. Lazy TypeHash Computation

All entries will store `QualifiedName` as the primary identifier. `TypeHash` is computed lazily via `OnceCell` when needed for bytecode generation.

```rust
use std::cell::OnceCell;

/// Pattern for all entry types:
pub struct TypeEntry {
    /// Primary identifier - used for registry lookup
    pub qualified_name: QualifiedName,

    /// Cached TypeHash - computed lazily on first access
    type_hash_cache: OnceCell<TypeHash>,
}

impl TypeEntry {
    /// Get or compute the TypeHash
    pub fn type_hash(&self) -> TypeHash {
        *self.type_hash_cache.get_or_init(|| {
            self.qualified_name.to_type_hash()
        })
    }
}
```

### 2. Unresolved Inheritance

Inheritance references stored as `UnresolvedType` during registration, resolved to `QualifiedName` in completion pass.

---

## ClassEntry Updates

```rust
// crates/angelscript-core/src/entries/class.rs

use crate::{QualifiedName, UnresolvedType};
use std::cell::OnceCell;

#[derive(Debug, Clone)]
pub struct ClassEntry {
    // === Identity (NEW: QualifiedName as primary) ===
    /// Primary identifier
    pub qualified_name: QualifiedName,
    /// Cached type hash (lazy)
    type_hash_cache: OnceCell<TypeHash>,

    // === Source ===
    pub type_kind: TypeKind,
    pub source: TypeSource,

    // === Inheritance (CHANGED: unresolved during registration) ===
    /// Base class - unresolved during registration, resolved in completion
    pub base_class: InheritanceRef,
    /// Included mixins
    pub mixins: Vec<InheritanceRef>,
    /// Implemented interfaces
    pub interfaces: Vec<InheritanceRef>,

    // === Members (UNCHANGED) ===
    pub behaviors: TypeBehaviors,
    pub methods: FxHashMap<String, Vec<TypeHash>>,
    pub properties: Vec<PropertyEntry>,

    // === Template Info (UNCHANGED) ===
    pub template_params: Vec<TypeHash>,
    pub template: Option<TypeHash>,
    pub type_args: Vec<DataType>,

    // === Modifiers (UNCHANGED) ===
    pub is_final: bool,
    pub is_abstract: bool,
    pub is_mixin: bool,

    // === VTable/ITable (UNCHANGED - built in completion) ===
    pub vtable: VTable,
    pub itables: ITableMap,
}

/// Inheritance reference - either unresolved (from registration) or resolved (after completion)
#[derive(Debug, Clone)]
pub enum InheritanceRef {
    /// Unresolved - stores raw name and context for later resolution
    Unresolved(UnresolvedType),
    /// Resolved - stores the qualified name of the base type
    Resolved(QualifiedName),
}

impl InheritanceRef {
    /// Check if resolved
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved(_))
    }

    /// Get as resolved name (panics if unresolved)
    pub fn as_resolved(&self) -> &QualifiedName {
        match self {
            Self::Resolved(name) => name,
            Self::Unresolved(_) => panic!("Inheritance not resolved"),
        }
    }

    /// Get as unresolved (returns None if resolved)
    pub fn as_unresolved(&self) -> Option<&UnresolvedType> {
        match self {
            Self::Unresolved(ty) => Some(ty),
            Self::Resolved(_) => None,
        }
    }
}

impl ClassEntry {
    /// Create a new class entry (registration phase)
    pub fn new(
        qualified_name: QualifiedName,
        type_kind: TypeKind,
        source: TypeSource,
    ) -> Self {
        Self {
            qualified_name,
            type_hash_cache: OnceCell::new(),
            type_kind,
            source,
            base_class: InheritanceRef::Unresolved(UnresolvedType::default()), // void = no base
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

    /// Get the type hash (lazy computation)
    pub fn type_hash(&self) -> TypeHash {
        *self.type_hash_cache.get_or_init(|| {
            self.qualified_name.to_type_hash()
        })
    }

    /// Get simple name
    pub fn name(&self) -> &str {
        self.qualified_name.simple_name()
    }

    /// Get namespace
    pub fn namespace(&self) -> &[String] {
        self.qualified_name.namespace()
    }

    // === Builder methods for unresolved inheritance ===

    /// Set unresolved base class
    pub fn with_unresolved_base(mut self, base: UnresolvedType) -> Self {
        self.base_class = InheritanceRef::Unresolved(base);
        self
    }

    /// Add unresolved mixin
    pub fn with_unresolved_mixin(mut self, mixin: UnresolvedType) -> Self {
        self.mixins.push(InheritanceRef::Unresolved(mixin));
        self
    }

    /// Add unresolved interface
    pub fn with_unresolved_interface(mut self, interface: UnresolvedType) -> Self {
        self.interfaces.push(InheritanceRef::Unresolved(interface));
        self
    }

    // === Resolution methods (called in completion pass) ===

    /// Resolve base class
    pub fn resolve_base(&mut self, resolved: QualifiedName) {
        self.base_class = InheritanceRef::Resolved(resolved);
    }

    /// Resolve mixin at index
    pub fn resolve_mixin(&mut self, index: usize, resolved: QualifiedName) {
        self.mixins[index] = InheritanceRef::Resolved(resolved);
    }

    /// Resolve interface at index
    pub fn resolve_interface(&mut self, index: usize, resolved: QualifiedName) {
        self.interfaces[index] = InheritanceRef::Resolved(resolved);
    }

    /// Check if all inheritance is resolved
    pub fn is_fully_resolved(&self) -> bool {
        self.base_class.is_resolved()
            && self.mixins.iter().all(|m| m.is_resolved())
            && self.interfaces.iter().all(|i| i.is_resolved())
    }
}
```

---

## InterfaceEntry Updates

```rust
// crates/angelscript-core/src/entries/interface.rs

use crate::{QualifiedName, UnresolvedType, UnresolvedSignature};
use std::cell::OnceCell;

#[derive(Debug, Clone)]
pub struct InterfaceEntry {
    // === Identity (NEW) ===
    pub qualified_name: QualifiedName,
    type_hash_cache: OnceCell<TypeHash>,

    // === Source ===
    pub source: TypeSource,

    // === Methods (CHANGED: unresolved during registration) ===
    /// Method signatures - unresolved during registration
    pub unresolved_methods: Vec<UnresolvedSignature>,
    /// Resolved method signatures (populated in completion)
    pub methods: Vec<MethodSignature>,

    // === Base Interfaces (CHANGED) ===
    pub base_interfaces: Vec<InheritanceRef>,

    // === ITable (UNCHANGED - built in completion) ===
    pub itable: ITable,
}

impl InterfaceEntry {
    /// Create a new interface entry (registration phase)
    pub fn new(qualified_name: QualifiedName, source: TypeSource) -> Self {
        Self {
            qualified_name,
            type_hash_cache: OnceCell::new(),
            source,
            unresolved_methods: Vec::new(),
            methods: Vec::new(),
            base_interfaces: Vec::new(),
            itable: ITable::default(),
        }
    }

    /// Get the type hash (lazy)
    pub fn type_hash(&self) -> TypeHash {
        *self.type_hash_cache.get_or_init(|| {
            self.qualified_name.to_type_hash()
        })
    }

    /// Get simple name
    pub fn name(&self) -> &str {
        self.qualified_name.simple_name()
    }

    /// Get namespace
    pub fn namespace(&self) -> &[String] {
        self.qualified_name.namespace()
    }

    // === Builder methods ===

    /// Add unresolved method signature
    pub fn with_unresolved_method(mut self, method: UnresolvedSignature) -> Self {
        self.unresolved_methods.push(method);
        self
    }

    /// Add unresolved base interface
    pub fn with_unresolved_base(mut self, base: UnresolvedType) -> Self {
        self.base_interfaces.push(InheritanceRef::Unresolved(base));
        self
    }

    // === Resolution methods ===

    /// Add resolved method (called in completion)
    pub fn add_resolved_method(&mut self, method: MethodSignature) {
        self.methods.push(method);
    }

    /// Resolve base interface at index
    pub fn resolve_base(&mut self, index: usize, resolved: QualifiedName) {
        self.base_interfaces[index] = InheritanceRef::Resolved(resolved);
    }
}
```

---

## FuncdefEntry Updates

```rust
// crates/angelscript-core/src/entries/funcdef.rs

use crate::{QualifiedName, UnresolvedType, UnresolvedParam};
use std::cell::OnceCell;

#[derive(Debug, Clone)]
pub struct FuncdefEntry {
    // === Identity (NEW) ===
    pub qualified_name: QualifiedName,
    type_hash_cache: OnceCell<TypeHash>,

    // === Source ===
    pub source: TypeSource,

    // === Signature (CHANGED: unresolved during registration) ===
    /// Unresolved parameter types
    pub unresolved_params: Vec<UnresolvedParam>,
    /// Unresolved return type
    pub unresolved_return_type: UnresolvedType,

    /// Resolved parameter types (populated in completion)
    pub params: Vec<DataType>,
    /// Resolved return type (populated in completion)
    pub return_type: DataType,

    // === Parent (UNCHANGED for template child funcdefs) ===
    pub parent_type: Option<QualifiedName>,
}

impl FuncdefEntry {
    /// Create a new funcdef entry (registration phase)
    pub fn new(
        qualified_name: QualifiedName,
        source: TypeSource,
        unresolved_params: Vec<UnresolvedParam>,
        unresolved_return_type: UnresolvedType,
    ) -> Self {
        Self {
            qualified_name,
            type_hash_cache: OnceCell::new(),
            source,
            unresolved_params,
            unresolved_return_type,
            params: Vec::new(),
            return_type: DataType::void(),
            parent_type: None,
        }
    }

    /// Get the type hash (lazy)
    pub fn type_hash(&self) -> TypeHash {
        *self.type_hash_cache.get_or_init(|| {
            self.qualified_name.to_type_hash()
        })
    }

    /// Get simple name
    pub fn name(&self) -> &str {
        self.qualified_name.simple_name()
    }

    /// Get namespace
    pub fn namespace(&self) -> &[String] {
        self.qualified_name.namespace()
    }

    /// Set as child of parent type
    pub fn with_parent(mut self, parent: QualifiedName) -> Self {
        self.parent_type = Some(parent);
        self
    }

    /// Resolve the signature (called in completion)
    pub fn resolve_signature(&mut self, params: Vec<DataType>, return_type: DataType) {
        self.params = params;
        self.return_type = return_type;
    }

    /// Check if signature is resolved
    pub fn is_resolved(&self) -> bool {
        !self.params.is_empty() || self.unresolved_params.is_empty()
    }
}
```

---

## FunctionDef Updates

```rust
// crates/angelscript-core/src/function_def.rs

use crate::{QualifiedName, UnresolvedType, UnresolvedParam};
use std::cell::OnceCell;

#[derive(Debug, Clone)]
pub struct FunctionDef {
    // === Identity (NEW) ===
    /// Qualified name for this function (namespace::name)
    pub qualified_name: QualifiedName,
    /// Object type if this is a method (as QualifiedName now)
    pub object_type: Option<QualifiedName>,
    /// Cached function hash
    func_hash_cache: OnceCell<TypeHash>,

    // === Unresolved Signature (NEW - during registration) ===
    /// Unresolved parameter types
    pub unresolved_params: Vec<UnresolvedParam>,
    /// Unresolved return type
    pub unresolved_return_type: UnresolvedType,

    // === Resolved Signature (populated in completion) ===
    pub params: Vec<Param>,
    pub return_type: DataType,

    // === Traits (UNCHANGED) ===
    pub traits: FunctionTraits,
    pub is_native: bool,
    pub visibility: Visibility,
    pub template_params: Vec<TypeHash>,
    pub is_variadic: bool,
}

impl FunctionDef {
    /// Create a new function definition (registration phase)
    pub fn new_unresolved(
        qualified_name: QualifiedName,
        object_type: Option<QualifiedName>,
        unresolved_params: Vec<UnresolvedParam>,
        unresolved_return_type: UnresolvedType,
        traits: FunctionTraits,
        is_native: bool,
        visibility: Visibility,
    ) -> Self {
        Self {
            qualified_name,
            object_type,
            func_hash_cache: OnceCell::new(),
            unresolved_params,
            unresolved_return_type,
            params: Vec::new(),
            return_type: DataType::void(),
            traits,
            is_native,
            visibility,
            template_params: Vec::new(),
            is_variadic: false,
        }
    }

    /// Get function hash (lazy - computed from qualified name + resolved params)
    pub fn func_hash(&self) -> TypeHash {
        *self.func_hash_cache.get_or_init(|| {
            // For function hash, we need resolved params for overload discrimination
            // If not resolved yet, use qualified name only
            if self.params.is_empty() && !self.unresolved_params.is_empty() {
                // Not yet resolved - use name only (temporary)
                TypeHash::from_name(&self.qualified_name.to_string())
            } else {
                // Resolved - include param types
                let param_hashes: Vec<TypeHash> = self.params
                    .iter()
                    .map(|p| p.data_type.type_hash)
                    .collect();
                TypeHash::from_function(&self.qualified_name.to_string(), &param_hashes)
            }
        })
    }

    /// Get simple name
    pub fn name(&self) -> &str {
        self.qualified_name.simple_name()
    }

    /// Get namespace
    pub fn namespace(&self) -> &[String] {
        self.qualified_name.namespace()
    }

    /// Resolve the signature (called in completion)
    pub fn resolve_signature(&mut self, params: Vec<Param>, return_type: DataType) {
        self.params = params;
        self.return_type = return_type;
        // Clear cache to recompute with resolved params
        self.func_hash_cache = OnceCell::new();
    }

    /// Check if signature is resolved
    pub fn is_resolved(&self) -> bool {
        !self.params.is_empty() || self.unresolved_params.is_empty()
    }
}
```

---

## Migration Notes

### Breaking Changes

1. `type_hash` field replaced with `type_hash()` method
2. `name` and `namespace` fields replaced with `qualified_name` field
3. `qualified_name` field (string) replaced with `QualifiedName` struct
4. Inheritance fields changed from `TypeHash`/`Vec<TypeHash>` to `InheritanceRef`/`Vec<InheritanceRef>`
5. Signature fields split into unresolved (registration) and resolved (completion) versions

### FFI Entries

FFI entries are created fully resolved (no unresolved types):

```rust
impl ClassEntry {
    /// Create an FFI class entry (fully resolved)
    pub fn ffi(name: impl Into<String>, type_kind: TypeKind) -> Self {
        let name = name.into();
        let qualified_name = QualifiedName::global(name);
        Self {
            qualified_name,
            type_hash_cache: OnceCell::new(),
            type_kind,
            source: TypeSource::ffi_untyped(),
            // No unresolved inheritance for FFI
            base_class: InheritanceRef::Resolved(QualifiedName::global("void")),
            mixins: Vec::new(),
            interfaces: Vec::new(),
            // ... rest unchanged
        }
    }
}
```

---

## Dependencies

These updates depend on Phase 1 (Core Types):
- `QualifiedName`
- `UnresolvedType`
- `UnresolvedParam`
- `UnresolvedSignature`

Phase 3 (Registry) will use these updated entry types.
