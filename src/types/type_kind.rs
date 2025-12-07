//! Type kind definitions for memory semantics.
//!
//! This module contains `TypeKind` and `ReferenceKind` which determine
//! how types are managed in memory (stack vs heap, value vs reference).

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_kind_value() {
        let kind = TypeKind::value::<i32>();
        match kind {
            TypeKind::Value {
                size,
                align,
                is_pod,
            } => {
                assert_eq!(size, 4);
                assert_eq!(align, 4);
                assert!(!is_pod);
            }
            _ => panic!("Expected Value variant"),
        }
        assert!(kind.is_value());
        assert!(!kind.is_reference());
        assert!(!kind.is_pod());
    }

    #[test]
    fn type_kind_pod() {
        let kind = TypeKind::pod::<i32>();
        match kind {
            TypeKind::Value {
                size,
                align,
                is_pod,
            } => {
                assert_eq!(size, 4);
                assert_eq!(align, 4);
                assert!(is_pod);
            }
            _ => panic!("Expected Value variant"),
        }
        assert!(kind.is_pod());
    }

    #[test]
    fn type_kind_reference() {
        let kind = TypeKind::reference();
        assert!(kind.is_reference());
        assert!(!kind.is_value());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::Standard);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_scoped() {
        let kind = TypeKind::scoped();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::Scoped);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_single_ref() {
        let kind = TypeKind::single_ref();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::SingleRef);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_generic_handle() {
        let kind = TypeKind::generic_handle();
        assert!(kind.is_reference());
        match kind {
            TypeKind::Reference { kind } => {
                assert_eq!(kind, ReferenceKind::GenericHandle);
            }
            _ => panic!("Expected Reference variant"),
        }
    }

    #[test]
    fn type_kind_default() {
        let kind = TypeKind::default();
        assert!(kind.is_script_object());
        assert!(kind.uses_constructors());
        assert!(!kind.uses_factories());
    }

    #[test]
    fn type_kind_script_object() {
        let kind = TypeKind::script_object();
        assert!(kind.is_script_object());
        assert!(!kind.is_value());
        assert!(!kind.is_reference());
        assert!(kind.uses_constructors());
        assert!(!kind.uses_factories());
    }

    #[test]
    fn reference_kind_default() {
        let kind = ReferenceKind::default();
        assert_eq!(kind, ReferenceKind::Standard);
    }
}
