//! Type kind and reference kind for memory semantics.

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
    ///
    /// POD (Plain Old Data) types are safe to memcpy and have no
    /// constructor or destructor requirements.
    ///
    /// Requires `T: Copy` to ensure the type is truly POD:
    /// - No `Drop` implementation that could be skipped
    /// - Bitwise copy is safe (no internal pointers that would be duplicated)
    pub fn pod<T: Copy>() -> Self {
        TypeKind::Value {
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            is_pod: true,
        }
    }

    /// Create a POD value type kind without the `Copy` bound.
    ///
    /// # Safety
    ///
    /// Caller must ensure the type is safe to memcpy:
    /// - No `Drop` implementation that could be skipped
    /// - No internal pointers that would be duplicated
    /// - No invariants that could be violated by bitwise copy
    ///
    /// Prefer `pod<T>()` when possible.
    pub unsafe fn pod_unchecked<T>() -> Self {
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

    /// Create a no-count reference type kind.
    ///
    /// No-count types have app-managed memory but handles still work.
    /// Scripts can pass handles around, but no AddRef/Release calls are made.
    pub fn no_count() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::NoCount,
        }
    }

    /// Create a no-handle (single-reference) type kind.
    ///
    /// No-handle types cannot have handles in script (`T@` is invalid).
    /// They also cannot be used as function parameters.
    /// Only accessible via global properties or return values.
    pub fn no_handle() -> Self {
        TypeKind::Reference {
            kind: ReferenceKind::NoHandle,
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

    /// Get the reference kind if this is a reference type.
    ///
    /// Returns `None` for value types and script objects.
    pub fn reference_kind(&self) -> Option<ReferenceKind> {
        match self {
            TypeKind::Reference { kind } => Some(*kind),
            _ => None,
        }
    }

    /// Check if this type supports handles (`T@`).
    ///
    /// Returns `true` for reference types that support handles and script objects.
    /// Returns `false` for value types and NoHandle/Scoped reference types.
    pub fn supports_handles(&self) -> bool {
        match self {
            TypeKind::Reference { kind } => kind.supports_handles(),
            TypeKind::ScriptObject => true, // Script objects always support handles
            TypeKind::Value { .. } => false,
        }
    }
}

impl Default for TypeKind {
    /// Default to script object (most common for script-defined classes).
    fn default() -> Self {
        TypeKind::ScriptObject
    }
}

/// Reference type variants for different ownership/lifetime semantics.
///
/// These correspond to AngelScript's reference type flags:
/// - `Standard` = `asOBJ_REF` (full ref counting with AddRef/Release)
/// - `NoCount` = `asOBJ_REF | asOBJ_NOCOUNT` (app manages memory, handles work)
/// - `NoHandle` = `asOBJ_REF | asOBJ_NOHANDLE` (single-reference, no handles allowed)
/// - `Scoped` = `asOBJ_REF | asOBJ_SCOPED` (RAII-style, destroyed at scope exit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferenceKind {
    /// Standard reference type - full handle support with AddRef/Release ref counting.
    /// Requires: AddRef and Release behaviors.
    #[default]
    Standard,

    /// Scoped reference type - RAII-style, destroyed at scope exit.
    /// Requires: Release behavior (called at scope exit).
    /// Forbids: AddRef behavior.
    /// Handles: Not allowed in script (except return values from app functions).
    Scoped,

    /// No-count reference type - application manages memory, no ref counting.
    /// Handles ARE allowed (`T@` is valid) - scripts can pass handles around.
    /// Forbids: AddRef and Release behaviors.
    /// Use case: Pooled objects, arena-allocated types, externally managed objects.
    NoCount,

    /// No-handle (single-reference) type - scripts cannot store handles.
    /// Handles NOT allowed (`T@` is invalid type).
    /// Forbids: AddRef, Release, and Factory behaviors.
    /// Cannot be used as function parameter (would create stack reference).
    /// Only accessible via global properties or return values.
    /// Use case: Singleton managers, game state objects.
    NoHandle,

    /// Generic handle - type-erased container that can hold any type.
    GenericHandle,
}

impl ReferenceKind {
    /// Whether this reference kind supports handle types (`T@`).
    ///
    /// Returns `true` for Standard, NoCount, and GenericHandle.
    /// Returns `false` for Scoped and NoHandle.
    pub fn supports_handles(&self) -> bool {
        match self {
            ReferenceKind::Standard | ReferenceKind::NoCount | ReferenceKind::GenericHandle => true,
            ReferenceKind::Scoped | ReferenceKind::NoHandle => false,
        }
    }

    /// Whether AddRef behavior is allowed for this reference kind.
    ///
    /// Only Standard reference types can have AddRef.
    pub fn allows_addref(&self) -> bool {
        matches!(self, ReferenceKind::Standard)
    }

    /// Whether Release behavior is allowed for this reference kind.
    ///
    /// Standard and Scoped types can have Release.
    pub fn allows_release(&self) -> bool {
        matches!(self, ReferenceKind::Standard | ReferenceKind::Scoped)
    }

    /// Whether AddRef/Release are required for this reference kind.
    ///
    /// Only Standard reference types require ref counting behaviors.
    pub fn requires_ref_counting(&self) -> bool {
        matches!(self, ReferenceKind::Standard)
    }

    /// Whether factories are allowed for this reference kind.
    ///
    /// NoHandle types cannot have factories (factories return handles).
    pub fn allows_factories(&self) -> bool {
        !matches!(self, ReferenceKind::NoHandle)
    }

    /// Whether this type can be used as a function parameter.
    ///
    /// NoHandle types cannot be parameters (would create stack reference).
    pub fn allows_as_parameter(&self) -> bool {
        !matches!(self, ReferenceKind::NoHandle)
    }

    /// Get a human-readable name for this reference kind.
    pub fn name(&self) -> &'static str {
        match self {
            ReferenceKind::Standard => "Standard",
            ReferenceKind::Scoped => "Scoped",
            ReferenceKind::NoCount => "NoCount",
            ReferenceKind::NoHandle => "NoHandle",
            ReferenceKind::GenericHandle => "GenericHandle",
        }
    }
}
