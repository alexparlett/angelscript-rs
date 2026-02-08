//! Type system definitions for AngelScript.
//!
//! This module provides core type system types:
//! - [`PrimitiveKind`]: Built-in numeric and boolean types
//! - [`Visibility`]: Access modifiers for class members
//! - [`TypeKind`]: Memory semantics (value, reference, script object)
//! - [`ReferenceKind`]: Reference type variants
//! - [`MethodSignature`]: Interface method signatures

mod method_signature;
mod primitive_kind;
mod type_kind;
mod visibility;

pub use method_signature::MethodSignature;
pub use primitive_kind::PrimitiveKind;
pub use type_kind::{ReferenceKind, TypeKind};
pub use visibility::Visibility;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DataType, primitives};

    #[test]
    fn primitive_kind_names() {
        assert_eq!(PrimitiveKind::Void.name(), "void");
        assert_eq!(PrimitiveKind::Bool.name(), "bool");
        assert_eq!(PrimitiveKind::Int8.name(), "int8");
        assert_eq!(PrimitiveKind::Int16.name(), "int16");
        assert_eq!(PrimitiveKind::Int32.name(), "int");
        assert_eq!(PrimitiveKind::Int64.name(), "int64");
        assert_eq!(PrimitiveKind::Uint8.name(), "uint8");
        assert_eq!(PrimitiveKind::Uint16.name(), "uint16");
        assert_eq!(PrimitiveKind::Uint32.name(), "uint");
        assert_eq!(PrimitiveKind::Uint64.name(), "uint64");
        assert_eq!(PrimitiveKind::Float.name(), "float");
        assert_eq!(PrimitiveKind::Double.name(), "double");
    }

    #[test]
    fn primitive_kind_hashes() {
        assert_eq!(PrimitiveKind::Void.type_hash(), primitives::VOID);
        assert_eq!(PrimitiveKind::Bool.type_hash(), primitives::BOOL);
        assert_eq!(PrimitiveKind::Int32.type_hash(), primitives::INT32);
        assert_eq!(PrimitiveKind::Float.type_hash(), primitives::FLOAT);
    }

    #[test]
    fn primitive_kind_display() {
        assert_eq!(format!("{}", PrimitiveKind::Int32), "int");
        assert_eq!(format!("{}", PrimitiveKind::Float), "float");
    }

    #[test]
    fn visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn visibility_display() {
        assert_eq!(format!("{}", Visibility::Public), "public");
        assert_eq!(format!("{}", Visibility::Protected), "protected");
        assert_eq!(format!("{}", Visibility::Private), "private");
    }

    #[test]
    fn type_kind_constructors() {
        let value = TypeKind::value_sized(8, 8, true);
        assert!(value.is_value());
        assert!(!value.is_reference());

        let reference = TypeKind::reference();
        assert!(reference.is_reference());
        assert!(!reference.is_value());

        let script = TypeKind::script_object();
        assert!(script.is_script_object());
    }

    #[test]
    fn type_kind_default() {
        assert_eq!(TypeKind::default(), TypeKind::ScriptObject);
    }

    #[test]
    fn method_signature_creation() {
        let sig = MethodSignature::new(
            "update",
            vec![DataType::simple(primitives::FLOAT)],
            DataType::void(),
        );
        assert_eq!(sig.name, "update");
        assert!(!sig.is_const);

        let const_sig =
            MethodSignature::new_const("get_value", vec![], DataType::simple(primitives::INT32));
        assert!(const_sig.is_const);
    }

    // =========================================================================
    // ReferenceKind Tests
    // =========================================================================

    #[test]
    fn reference_kind_supports_handles() {
        assert!(ReferenceKind::Standard.supports_handles());
        assert!(ReferenceKind::NoCount.supports_handles());
        assert!(ReferenceKind::GenericHandle.supports_handles());
        assert!(!ReferenceKind::Scoped.supports_handles());
        assert!(!ReferenceKind::NoHandle.supports_handles());
    }

    #[test]
    fn reference_kind_allows_addref() {
        assert!(ReferenceKind::Standard.allows_addref());
        assert!(!ReferenceKind::Scoped.allows_addref());
        assert!(!ReferenceKind::NoCount.allows_addref());
        assert!(!ReferenceKind::NoHandle.allows_addref());
        assert!(!ReferenceKind::GenericHandle.allows_addref());
    }

    #[test]
    fn reference_kind_allows_release() {
        assert!(ReferenceKind::Standard.allows_release());
        assert!(ReferenceKind::Scoped.allows_release());
        assert!(!ReferenceKind::NoCount.allows_release());
        assert!(!ReferenceKind::NoHandle.allows_release());
        assert!(!ReferenceKind::GenericHandle.allows_release());
    }

    #[test]
    fn reference_kind_requires_ref_counting() {
        assert!(ReferenceKind::Standard.requires_ref_counting());
        assert!(!ReferenceKind::Scoped.requires_ref_counting());
        assert!(!ReferenceKind::NoCount.requires_ref_counting());
        assert!(!ReferenceKind::NoHandle.requires_ref_counting());
        assert!(!ReferenceKind::GenericHandle.requires_ref_counting());
    }

    #[test]
    fn reference_kind_allows_factories() {
        assert!(ReferenceKind::Standard.allows_factories());
        assert!(ReferenceKind::Scoped.allows_factories());
        assert!(ReferenceKind::NoCount.allows_factories());
        assert!(ReferenceKind::GenericHandle.allows_factories());
        assert!(!ReferenceKind::NoHandle.allows_factories());
    }

    #[test]
    fn reference_kind_allows_as_parameter() {
        assert!(ReferenceKind::Standard.allows_as_parameter());
        assert!(ReferenceKind::Scoped.allows_as_parameter());
        assert!(ReferenceKind::NoCount.allows_as_parameter());
        assert!(ReferenceKind::GenericHandle.allows_as_parameter());
        assert!(!ReferenceKind::NoHandle.allows_as_parameter());
    }

    #[test]
    fn reference_kind_names() {
        assert_eq!(ReferenceKind::Standard.name(), "Standard");
        assert_eq!(ReferenceKind::Scoped.name(), "Scoped");
        assert_eq!(ReferenceKind::NoCount.name(), "NoCount");
        assert_eq!(ReferenceKind::NoHandle.name(), "NoHandle");
        assert_eq!(ReferenceKind::GenericHandle.name(), "GenericHandle");
    }

    #[test]
    fn type_kind_no_count() {
        let kind = TypeKind::no_count();
        assert!(kind.is_reference());
        assert_eq!(kind.reference_kind(), Some(ReferenceKind::NoCount));
        assert!(kind.supports_handles());
    }

    #[test]
    fn type_kind_no_handle() {
        let kind = TypeKind::no_handle();
        assert!(kind.is_reference());
        assert_eq!(kind.reference_kind(), Some(ReferenceKind::NoHandle));
        assert!(!kind.supports_handles());
    }

    #[test]
    fn type_kind_supports_handles() {
        assert!(TypeKind::reference().supports_handles());
        assert!(TypeKind::no_count().supports_handles());
        assert!(!TypeKind::no_handle().supports_handles());
        assert!(!TypeKind::scoped().supports_handles());
        assert!(TypeKind::generic_handle().supports_handles());
        assert!(TypeKind::script_object().supports_handles());
        assert!(!TypeKind::value::<i32>().supports_handles());
        assert!(!TypeKind::pod::<i32>().supports_handles());
    }

    #[test]
    fn type_kind_reference_kind() {
        assert_eq!(
            TypeKind::reference().reference_kind(),
            Some(ReferenceKind::Standard)
        );
        assert_eq!(
            TypeKind::scoped().reference_kind(),
            Some(ReferenceKind::Scoped)
        );
        assert_eq!(
            TypeKind::no_count().reference_kind(),
            Some(ReferenceKind::NoCount)
        );
        assert_eq!(
            TypeKind::no_handle().reference_kind(),
            Some(ReferenceKind::NoHandle)
        );
        assert_eq!(
            TypeKind::generic_handle().reference_kind(),
            Some(ReferenceKind::GenericHandle)
        );
        assert_eq!(TypeKind::script_object().reference_kind(), None);
        assert_eq!(TypeKind::value::<i32>().reference_kind(), None);
        assert_eq!(TypeKind::pod::<i32>().reference_kind(), None);
    }

    // MethodSignature::signature_hash tests
    #[test]
    fn method_signature_hash_same_signature_same_hash() {
        let sig1 = MethodSignature::new(
            "foo",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        let sig2 = MethodSignature::new(
            "foo",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        assert_eq!(sig1.signature_hash(), sig2.signature_hash());
    }

    #[test]
    fn method_signature_hash_different_name_different_hash() {
        let foo = MethodSignature::new("foo", vec![], DataType::void());
        let bar = MethodSignature::new("bar", vec![], DataType::void());
        assert_ne!(foo.signature_hash(), bar.signature_hash());
    }

    #[test]
    fn method_signature_hash_different_params_different_hash() {
        let foo_int = MethodSignature::new(
            "foo",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        let foo_float = MethodSignature::new(
            "foo",
            vec![DataType::simple(primitives::FLOAT)],
            DataType::void(),
        );
        assert_ne!(foo_int.signature_hash(), foo_float.signature_hash());
    }

    #[test]
    fn method_signature_hash_const_differs() {
        let non_const = MethodSignature::new("foo", vec![], DataType::void());
        let const_method = MethodSignature::new_const("foo", vec![], DataType::void());
        assert_ne!(non_const.signature_hash(), const_method.signature_hash());
    }

    #[test]
    fn method_signature_hash_ignores_return_type() {
        let returns_void = MethodSignature::new("foo", vec![], DataType::void());
        let returns_int = MethodSignature::new("foo", vec![], DataType::simple(primitives::INT32));
        assert_eq!(returns_void.signature_hash(), returns_int.signature_hash());
    }

    #[test]
    fn method_signature_hash_param_modifiers_differ() {
        let val = MethodSignature::new(
            "foo",
            vec![DataType::simple(primitives::INT32)],
            DataType::void(),
        );
        let ref_in = MethodSignature::new(
            "foo",
            vec![DataType::with_ref_in(primitives::INT32)],
            DataType::void(),
        );
        assert_ne!(val.signature_hash(), ref_in.signature_hash());
    }

    #[test]
    fn method_signature_hash_param_order_matters() {
        let int_float = MethodSignature::new(
            "foo",
            vec![
                DataType::simple(primitives::INT32),
                DataType::simple(primitives::FLOAT),
            ],
            DataType::void(),
        );
        let float_int = MethodSignature::new(
            "foo",
            vec![
                DataType::simple(primitives::FLOAT),
                DataType::simple(primitives::INT32),
            ],
            DataType::void(),
        );
        assert_ne!(int_float.signature_hash(), float_int.signature_hash());
    }
}
