//! Function entry for the registry.
//!
//! This module provides `FunctionEntry` which combines `FunctionDef` with
//! implementation details and source tracking.

use crate::{FunctionDef, NativeFn, UnitId};

use super::FunctionSource;

/// Registry entry for a function.
///
/// Combines the function definition (signature) with its implementation
/// and source tracking.
#[derive(Debug, Clone)]
pub struct FunctionEntry {
    /// Function definition (signature, traits, etc.).
    pub def: FunctionDef,
    /// Function implementation.
    pub implementation: FunctionImpl,
    /// Source (FFI or script).
    pub source: FunctionSource,
}

impl FunctionEntry {
    /// Create a new function entry.
    pub fn new(def: FunctionDef, implementation: FunctionImpl, source: FunctionSource) -> Self {
        Self {
            def,
            implementation,
            source,
        }
    }

    /// Create an FFI function entry with a native implementation.
    pub fn ffi(def: FunctionDef) -> Self {
        Self {
            def,
            implementation: FunctionImpl::Native(None),
            source: FunctionSource::Ffi,
        }
    }

    /// Create an FFI function entry with a native function pointer.
    pub fn ffi_with_native(def: FunctionDef, native_fn: NativeFn) -> Self {
        Self {
            def,
            implementation: FunctionImpl::Native(Some(native_fn)),
            source: FunctionSource::Ffi,
        }
    }

    /// Create a script function entry.
    pub fn script(def: FunctionDef, unit_id: UnitId, source: FunctionSource) -> Self {
        Self {
            def,
            implementation: FunctionImpl::Script { unit_id },
            source,
        }
    }

    /// Create an abstract method entry.
    pub fn abstract_method(def: FunctionDef, source: FunctionSource) -> Self {
        Self {
            def,
            implementation: FunctionImpl::Abstract,
            source,
        }
    }

    /// Check if this is a native (FFI) function.
    pub fn is_native(&self) -> bool {
        matches!(self.implementation, FunctionImpl::Native(_))
    }

    /// Check if this is a script function.
    pub fn is_script(&self) -> bool {
        matches!(self.implementation, FunctionImpl::Script { .. })
    }

    /// Check if this is an abstract method.
    pub fn is_abstract(&self) -> bool {
        matches!(self.implementation, FunctionImpl::Abstract)
    }

    /// Check if this is an external function.
    pub fn is_external(&self) -> bool {
        matches!(self.implementation, FunctionImpl::External { .. })
    }

    /// Get the native function if available.
    pub fn native_fn(&self) -> Option<&NativeFn> {
        match &self.implementation {
            FunctionImpl::Native(Some(f)) => Some(f),
            _ => None,
        }
    }
}

impl PartialEq for FunctionEntry {
    fn eq(&self, other: &Self) -> bool {
        // Compare by definition and source, not implementation
        // (NativeFn doesn't implement PartialEq)
        self.def == other.def && self.source == other.source
    }
}

/// Function implementation kind.
///
/// Describes how a function is implemented - as native code, script code,
/// an abstract declaration, or an external reference.
#[derive(Debug, Clone)]
pub enum FunctionImpl {
    /// Native (Rust) function.
    ///
    /// The `NativeFn` is optional because it may be set later during
    /// registration or may not be needed for metadata-only entries.
    Native(Option<NativeFn>),

    /// Script-defined function.
    Script {
        /// The compilation unit containing this function.
        unit_id: UnitId,
    },

    /// Abstract method (no implementation, must be overridden).
    Abstract,

    /// External function from another module.
    External {
        /// The module name containing this function.
        module: String,
    },
}

impl FunctionImpl {
    /// Create a script implementation.
    pub fn script(unit_id: UnitId) -> Self {
        FunctionImpl::Script { unit_id }
    }

    /// Create an external implementation.
    pub fn external(module: impl Into<String>) -> Self {
        FunctionImpl::External {
            module: module.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataType, FunctionTraits, Span, TypeHash, Visibility};

    fn make_test_def(name: &str) -> FunctionDef {
        FunctionDef::new(
            TypeHash::from_name(name),
            name.to_string(),
            vec![],
            vec![],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        )
    }

    #[test]
    fn function_entry_ffi() {
        let def = make_test_def("print");
        let entry = FunctionEntry::ffi(def);

        assert!(entry.is_native());
        assert!(!entry.is_script());
        assert!(!entry.is_abstract());
        assert!(entry.source.is_ffi());
        assert!(entry.native_fn().is_none()); // No implementation set
    }

    #[test]
    fn function_entry_script() {
        let def = make_test_def("update");
        let unit_id = UnitId::new(1);
        let source = FunctionSource::script(Span::new(1, 10, 40));
        let entry = FunctionEntry::script(def, unit_id, source);

        assert!(entry.is_script());
        assert!(!entry.is_native());
        assert!(entry.source.is_script());
    }

    #[test]
    fn function_entry_abstract() {
        let mut def = make_test_def("render");
        def.traits.is_abstract = true;
        let source = FunctionSource::script(Span::new(1, 0, 20));
        let entry = FunctionEntry::abstract_method(def, source);

        assert!(entry.is_abstract());
        assert!(!entry.is_native());
        assert!(!entry.is_script());
    }

    #[test]
    fn function_impl_external() {
        let impl_kind = FunctionImpl::external("graphics_module");
        match impl_kind {
            FunctionImpl::External { module } => assert_eq!(module, "graphics_module"),
            _ => panic!("Expected External variant"),
        }
    }

    #[test]
    fn function_entry_equality() {
        let def1 = make_test_def("test");
        let def2 = make_test_def("test");
        let entry1 = FunctionEntry::ffi(def1);
        let entry2 = FunctionEntry::ffi(def2);

        assert_eq!(entry1, entry2);
    }
}
