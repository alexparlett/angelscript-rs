//! Type substitution for template instantiation.
//!
//! Provides functions to substitute template parameters with concrete types.

use angelscript_core::{CompilationError, DataType, Param, Span, TypeHash};
use rustc_hash::FxHashMap;

/// Map from template parameter hash to concrete type.
pub type SubstitutionMap = FxHashMap<TypeHash, DataType>;

/// Build a substitution map from template parameters and type arguments.
///
/// # Arguments
/// * `template_params` - Hashes of template parameter entries (e.g., hash of "array::T")
/// * `type_args` - Concrete types to substitute (e.g., DataType for int)
///
/// # Errors
/// Returns error if the number of arguments doesn't match parameters.
pub fn build_substitution_map(
    template_params: &[TypeHash],
    type_args: &[DataType],
    span: Span,
) -> Result<SubstitutionMap, CompilationError> {
    if template_params.len() != type_args.len() {
        return Err(CompilationError::TemplateArgCountMismatch {
            expected: template_params.len(),
            got: type_args.len(),
            span,
        });
    }

    let mut map = FxHashMap::default();
    for (param_hash, arg) in template_params.iter().zip(type_args.iter()) {
        map.insert(*param_hash, *arg);
    }
    Ok(map)
}

/// Substitute template parameters in a type.
///
/// If the type is a template parameter, replaces it with the concrete type.
/// Preserves modifiers (const, handle, ref) from the original type.
pub fn substitute_type(data_type: DataType, subst_map: &SubstitutionMap) -> DataType {
    // Check if this is a template parameter that needs substitution
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        // Combine modifiers: original modifiers take precedence, but we OR them
        // For example: const T with T=int@ becomes const int@
        DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const: data_type.is_handle_to_const || replacement.is_handle_to_const,
            ref_modifier: data_type.ref_modifier, // Keep original ref modifier
            is_mixin: replacement.is_mixin,       // Inherit mixin status from replacement
            is_interface: replacement.is_interface, // Inherit interface status from replacement
            is_enum: replacement.is_enum,         // Inherit enum status from replacement
        }
    } else {
        // Not a template param, return unchanged
        data_type
    }
}

/// Substitute template parameters in function parameters.
///
/// Applies `if_handle_then_const` flag during substitution: if the parameter
/// has this flag set and the substituted type is a handle, the result is
/// handle-to-const.
pub fn substitute_params(params: &[Param], subst_map: &SubstitutionMap) -> Vec<Param> {
    params
        .iter()
        .map(|p| {
            let substituted =
                substitute_type_with_handle_const(p.data_type, subst_map, p.if_handle_then_const);
            Param {
                name: p.name.clone(),
                data_type: substituted,
                has_default: p.has_default,
                // Flag is consumed during substitution, so clear it on the result
                if_handle_then_const: false,
            }
        })
        .collect()
}

/// Substitute with if_handle_then_const flag support.
///
/// AngelScript has a special rule: when a parameter is declared as `const T`
/// and T is substituted with a handle type, it becomes handle-to-const.
pub fn substitute_type_with_handle_const(
    data_type: DataType,
    subst_map: &SubstitutionMap,
    if_handle_then_const: bool,
) -> DataType {
    if let Some(replacement) = subst_map.get(&data_type.type_hash) {
        let is_handle_to_const = data_type.is_handle_to_const
            || replacement.is_handle_to_const
            || (if_handle_then_const && replacement.is_handle && data_type.is_const);

        DataType {
            type_hash: replacement.type_hash,
            is_const: data_type.is_const || replacement.is_const,
            is_handle: data_type.is_handle || replacement.is_handle,
            is_handle_to_const,
            ref_modifier: data_type.ref_modifier,
            is_mixin: replacement.is_mixin,
            is_interface: replacement.is_interface,
            is_enum: replacement.is_enum,
        }
    } else {
        data_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{RefModifier, primitives};

    fn make_template_param(name: &str) -> TypeHash {
        TypeHash::from_name(name)
    }

    #[test]
    fn build_map_single_param() {
        let t_hash = make_template_param("array::T");
        let int_type = DataType::simple(primitives::INT32);

        let map = build_substitution_map(&[t_hash], &[int_type], Span::default()).unwrap();

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&t_hash), Some(&int_type));
    }

    #[test]
    fn build_map_multiple_params() {
        let k_hash = make_template_param("dict::K");
        let v_hash = make_template_param("dict::V");
        let string_type = DataType::simple(primitives::STRING);
        let int_type = DataType::simple(primitives::INT32);

        let map =
            build_substitution_map(&[k_hash, v_hash], &[string_type, int_type], Span::default())
                .unwrap();

        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&k_hash), Some(&string_type));
        assert_eq!(map.get(&v_hash), Some(&int_type));
    }

    #[test]
    fn build_map_count_mismatch_too_few() {
        let t_hash = make_template_param("array::T");

        let result = build_substitution_map(&[t_hash], &[], Span::default());

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TemplateArgCountMismatch { expected, got, .. } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 0);
            }
            e => panic!("Expected TemplateArgCountMismatch, got {:?}", e),
        }
    }

    #[test]
    fn build_map_count_mismatch_too_many() {
        let t_hash = make_template_param("array::T");
        let int_type = DataType::simple(primitives::INT32);
        let float_type = DataType::simple(primitives::DOUBLE);

        let result = build_substitution_map(&[t_hash], &[int_type, float_type], Span::default());

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::TemplateArgCountMismatch { expected, got, .. } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 2);
            }
            e => panic!("Expected TemplateArgCountMismatch, got {:?}", e),
        }
    }

    #[test]
    fn substitute_simple_type() {
        let t_hash = make_template_param("T");
        let int_type = DataType::simple(primitives::INT32);

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, int_type);

        let input = DataType::simple(t_hash);
        let result = substitute_type(input, &map);

        assert_eq!(result.type_hash, primitives::INT32);
        assert!(!result.is_const);
        assert!(!result.is_handle);
    }

    #[test]
    fn substitute_preserves_const() {
        let t_hash = make_template_param("T");
        let int_type = DataType::simple(primitives::INT32);

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, int_type);

        // const T -> const int
        let input = DataType {
            type_hash: t_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };
        let result = substitute_type(input, &map);

        assert_eq!(result.type_hash, primitives::INT32);
        assert!(result.is_const);
    }

    #[test]
    fn substitute_preserves_ref_modifier() {
        let t_hash = make_template_param("T");
        let int_type = DataType::simple(primitives::INT32);

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, int_type);

        // T& in -> int& in
        let input = DataType {
            type_hash: t_hash,
            is_const: false,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::In,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };
        let result = substitute_type(input, &map);

        assert_eq!(result.type_hash, primitives::INT32);
        assert_eq!(result.ref_modifier, RefModifier::In);
    }

    #[test]
    fn substitute_combines_handle() {
        let t_hash = make_template_param("T");
        // T is already a handle type
        let handle_type = DataType {
            type_hash: primitives::INT32,
            is_const: false,
            is_handle: true,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, handle_type);

        // T with T=int@ -> int@
        let input = DataType::simple(t_hash);
        let result = substitute_type(input, &map);

        assert!(result.is_handle);
    }

    #[test]
    fn substitute_non_param_unchanged() {
        let t_hash = make_template_param("T");
        let int_type = DataType::simple(primitives::INT32);

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, int_type);

        // float is not a template param, should be unchanged
        let input = DataType::simple(primitives::DOUBLE);
        let result = substitute_type(input, &map);

        assert_eq!(result.type_hash, primitives::DOUBLE);
    }

    #[test]
    fn substitute_params_all() {
        let t_hash = make_template_param("T");
        let int_type = DataType::simple(primitives::INT32);

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, int_type);

        let params = vec![
            Param::new("value", DataType::simple(t_hash)),
            Param::new("count", DataType::simple(primitives::INT32)),
        ];

        let result = substitute_params(&params, &map);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].data_type.type_hash, primitives::INT32);
        assert_eq!(result[1].data_type.type_hash, primitives::INT32);
    }

    #[test]
    fn substitute_with_handle_const_flag() {
        let t_hash = make_template_param("T");
        // T is substituted with a handle type
        let handle_type = DataType {
            type_hash: primitives::INT32,
            is_const: false,
            is_handle: true,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, handle_type);

        // const T with T=int@ and if_handle_then_const=true -> const int@ (handle-to-const)
        let input = DataType {
            type_hash: t_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };
        let result = substitute_type_with_handle_const(input, &map, true);

        assert!(result.is_handle);
        assert!(result.is_handle_to_const);
    }

    #[test]
    fn substitute_with_handle_const_flag_disabled() {
        let t_hash = make_template_param("T");
        let handle_type = DataType {
            type_hash: primitives::INT32,
            is_const: false,
            is_handle: true,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };

        let mut map = SubstitutionMap::default();
        map.insert(t_hash, handle_type);

        // const T with T=int@ but if_handle_then_const=false -> const int@ (NOT handle-to-const)
        let input = DataType {
            type_hash: t_hash,
            is_const: true,
            is_handle: false,
            is_handle_to_const: false,
            ref_modifier: RefModifier::None,
            is_mixin: false,
            is_interface: false,
            is_enum: false,
        };
        let result = substitute_type_with_handle_const(input, &map, false);

        assert!(result.is_handle);
        assert!(!result.is_handle_to_const);
    }
}
