//! Cost-based ranking for overload resolution.
//!
//! This module handles selecting the best match from multiple viable candidates
//! based on their conversion costs, with tie-breaking rules.

use angelscript_core::{CompilationError, Span, TypeHash};

use super::OverloadMatch;
use crate::context::CompilationContext;

/// Find the best match from viable candidates.
///
/// Selects the candidate with the lowest total conversion cost. If multiple
/// candidates tie with the same cost, applies tie-breaking rules.
///
/// # Arguments
///
/// * `viable` - Candidates that passed argument type checking
/// * `all_candidates` - All original candidates (for error messages)
/// * `ctx` - Compilation context
/// * `span` - Source location for error reporting
///
/// # Returns
///
/// * `Ok(OverloadMatch)` - The best matching candidate
/// * `Err(CompilationError::AmbiguousOverload)` - Multiple candidates tie
pub fn find_best_match(
    viable: &[OverloadMatch],
    all_candidates: &[TypeHash],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OverloadMatch, CompilationError> {
    assert!(!viable.is_empty());

    if viable.len() == 1 {
        return Ok(viable[0].clone());
    }

    // Sort by total cost (ascending)
    let mut sorted: Vec<_> = viable.iter().collect();
    sorted.sort_by_key(|m| m.total_cost);

    let best = sorted[0];
    let second = sorted[1];

    // Check for ambiguity (same cost)
    if best.total_cost == second.total_cost {
        // Try tie-breakers
        if let Some(winner) = break_tie(best, second, ctx) {
            return Ok(winner.clone());
        }

        // Truly ambiguous - report error
        return Err(ambiguous_overload_error(
            best,
            second,
            all_candidates,
            ctx,
            span,
        ));
    }

    Ok(best.clone())
}

/// Try to break a tie between two candidates with equal cost.
///
/// Tie-breaking rules (in order):
/// 1. Prefer more exact matches over conversions
/// 2. Prefer non-template function over template instantiation
/// 3. Prefer parameter types that are more derived in inheritance hierarchy
fn break_tie<'a>(
    a: &'a OverloadMatch,
    b: &'a OverloadMatch,
    ctx: &CompilationContext<'_>,
) -> Option<&'a OverloadMatch> {
    // Rule 1: Count exact matches (identity conversions)
    let a_exact = count_exact_matches(a);
    let b_exact = count_exact_matches(b);

    if a_exact > b_exact {
        return Some(a);
    }
    if b_exact > a_exact {
        return Some(b);
    }

    // Rule 2: Prefer non-template over template instantiation
    let a_is_template = ctx
        .get_function(a.func_hash)
        .is_some_and(|f| f.def.is_template());
    let b_is_template = ctx
        .get_function(b.func_hash)
        .is_some_and(|f| f.def.is_template());

    if !a_is_template && b_is_template {
        return Some(a);
    }
    if a_is_template && !b_is_template {
        return Some(b);
    }

    // Rule 3: Prefer more derived parameter types
    if let Some(winner) = prefer_more_derived(a, b, ctx) {
        return Some(winner);
    }

    None // Truly ambiguous - user should use explicit cast or different overload
}

/// Prefer the candidate whose parameter types are more derived.
///
/// If candidate A has a parameter type that is derived from B's parameter type,
/// A is preferred (more specific match).
fn prefer_more_derived<'a>(
    a: &'a OverloadMatch,
    b: &'a OverloadMatch,
    ctx: &CompilationContext<'_>,
) -> Option<&'a OverloadMatch> {
    let func_a = ctx.get_function(a.func_hash)?;
    let func_b = ctx.get_function(b.func_hash)?;

    let params_a = &func_a.def.params;
    let params_b = &func_b.def.params;

    // Must have same number of parameters to compare
    if params_a.len() != params_b.len() {
        return None;
    }

    let mut a_more_derived = 0;
    let mut b_more_derived = 0;

    for (pa, pb) in params_a.iter().zip(params_b.iter()) {
        let type_a = pa.data_type.type_hash;
        let type_b = pb.data_type.type_hash;

        // If same type, no preference
        if type_a == type_b {
            continue;
        }

        // Check if A's type is derived from B's type
        if ctx.is_type_derived_from(type_a, type_b) {
            a_more_derived += 1;
        } else if ctx.is_type_derived_from(type_b, type_a) {
            b_more_derived += 1;
        }
    }

    // Only prefer if one is strictly more derived without the other being more derived elsewhere
    if a_more_derived > 0 && b_more_derived == 0 {
        return Some(a);
    }
    if b_more_derived > 0 && a_more_derived == 0 {
        return Some(b);
    }

    None
}

/// Count the number of exact (identity) matches in the conversions.
fn count_exact_matches(m: &OverloadMatch) -> usize {
    m.arg_conversions
        .iter()
        .filter(|c| c.as_ref().map(|conv| conv.is_exact()).unwrap_or(true))
        .count()
}

/// Build error for ambiguous overload.
fn ambiguous_overload_error(
    a: &OverloadMatch,
    b: &OverloadMatch,
    _all_candidates: &[TypeHash],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> CompilationError {
    let name = ctx
        .get_function(a.func_hash)
        .map(|f| f.def.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let format_candidate = |m: &OverloadMatch| -> String {
        ctx.get_function(m.func_hash)
            .map(|f| {
                let params: Vec<_> = f
                    .def
                    .params
                    .iter()
                    .map(|p| format_type(p.data_type.type_hash, ctx))
                    .collect();
                format!("{}({})", f.def.name, params.join(", "))
            })
            .unwrap_or_else(|| format!("{:?}", m.func_hash))
    };

    let candidates = format!("{} and {}", format_candidate(a), format_candidate(b));

    CompilationError::AmbiguousOverload {
        name,
        candidates,
        span,
    }
}

/// Format a type hash as a readable name.
fn format_type(hash: TypeHash, ctx: &CompilationContext<'_>) -> String {
    ctx.get_type(hash)
        .map(|e| e.qualified_name().to_string())
        .unwrap_or_else(|| format!("{:?}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversion::{Conversion, ConversionKind};

    fn make_match(hash: u64, cost: u32, exact_count: usize, total_args: usize) -> OverloadMatch {
        let mut conversions = Vec::with_capacity(total_args);

        for i in 0..total_args {
            if i < exact_count {
                conversions.push(Some(Conversion {
                    kind: ConversionKind::Identity,
                    cost: Conversion::COST_EXACT,
                    is_implicit: true,
                }));
            } else {
                conversions.push(Some(Conversion {
                    kind: ConversionKind::Primitive {
                        from: angelscript_core::primitives::INT32,
                        to: angelscript_core::primitives::INT64,
                    },
                    cost: Conversion::COST_PRIMITIVE_WIDENING,
                    is_implicit: true,
                }));
            }
        }

        OverloadMatch {
            func_hash: TypeHash::from_name(&format!("func_{}", hash)),
            arg_conversions: conversions,
            total_cost: cost,
        }
    }

    #[test]
    fn single_viable_returns_it() {
        let m = make_match(1, 0, 1, 1);
        let viable = vec![m.clone()];

        use angelscript_registry::SymbolRegistry;
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let result = find_best_match(&viable, &[], &ctx, Span::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().func_hash, m.func_hash);
    }

    #[test]
    fn lower_cost_wins() {
        let m1 = make_match(1, 5, 0, 2);
        let m2 = make_match(2, 2, 0, 2);
        let viable = vec![m1, m2.clone()];

        use angelscript_registry::SymbolRegistry;
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let result = find_best_match(&viable, &[], &ctx, Span::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().func_hash, m2.func_hash);
    }

    #[test]
    fn more_exact_matches_wins_tie() {
        // Same cost, but m1 has more exact matches
        let m1 = make_match(1, 2, 2, 3); // 2 exact, 1 conversion
        let m2 = make_match(2, 2, 1, 3); // 1 exact, 2 conversions
        let viable = vec![m1.clone(), m2];

        use angelscript_registry::SymbolRegistry;
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let result = find_best_match(&viable, &[], &ctx, Span::default());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().func_hash, m1.func_hash);
    }

    #[test]
    fn equal_cost_equal_exact_is_ambiguous() {
        let m1 = make_match(1, 2, 1, 2);
        let m2 = make_match(2, 2, 1, 2);
        let viable = vec![m1, m2];

        use angelscript_registry::SymbolRegistry;
        let registry = SymbolRegistry::with_primitives();
        let ctx = CompilationContext::new(&registry);

        let result = find_best_match(&viable, &[], &ctx, Span::default());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::AmbiguousOverload { .. }
        ));
    }

    #[test]
    fn non_template_preferred_over_template() {
        use angelscript_core::{
            DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, Visibility, primitives,
        };
        use angelscript_registry::SymbolRegistry;

        let mut registry = SymbolRegistry::with_primitives();

        // Non-template function: foo(int)
        let non_template_hash = TypeHash::from_function("foo_nontemplate", &[primitives::INT32]);
        let non_template_func = FunctionEntry::ffi(FunctionDef::new(
            non_template_hash,
            "foo".to_string(),
            vec![],
            vec![Param::new("x", DataType::simple(primitives::INT32))],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        ));
        registry.register_function(non_template_func).unwrap();

        // Template function: foo<T>(T)
        let template_param = TypeHash::from_name("T");
        let template_hash = TypeHash::from_function("foo_template", &[primitives::INT32]);
        let template_func = FunctionEntry::ffi(FunctionDef::new_template(
            template_hash,
            "foo".to_string(),
            vec![],
            vec![Param::new("x", DataType::simple(primitives::INT32))],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
            vec![template_param],
        ));
        registry.register_function(template_func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Both have same cost and same exact matches
        let m_non_template = OverloadMatch {
            func_hash: non_template_hash,
            arg_conversions: vec![Some(Conversion {
                kind: ConversionKind::Identity,
                cost: Conversion::COST_EXACT,
                is_implicit: true,
            })],
            total_cost: 0,
        };
        let m_template = OverloadMatch {
            func_hash: template_hash,
            arg_conversions: vec![Some(Conversion {
                kind: ConversionKind::Identity,
                cost: Conversion::COST_EXACT,
                is_implicit: true,
            })],
            total_cost: 0,
        };

        let viable = vec![m_non_template.clone(), m_template];
        let result = find_best_match(&viable, &[], &ctx, Span::default());

        assert!(result.is_ok());
        // Non-template should win
        assert_eq!(result.unwrap().func_hash, non_template_hash);
    }

    #[test]
    fn more_derived_parameter_wins_tie() {
        use angelscript_core::entries::TypeSource;
        use angelscript_core::{
            ClassEntry, DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, TypeKind,
            Visibility,
        };
        use angelscript_registry::SymbolRegistry;

        let mut registry = SymbolRegistry::with_primitives();

        // Create base class: Entity
        let entity_hash = TypeHash::from_name("Entity");
        let entity_class = ClassEntry::new(
            "Entity",
            vec![],
            "Entity",
            entity_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(entity_class.into()).unwrap();

        // Create derived class: Player : Entity
        let player_hash = TypeHash::from_name("Player");
        let player_class = ClassEntry::new(
            "Player",
            vec![],
            "Player",
            player_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_base(entity_hash);
        registry.register_type(player_class.into()).unwrap();

        // Function taking Entity: process(Entity)
        let func_entity_hash = TypeHash::from_function("process_entity", &[entity_hash]);
        let func_entity = FunctionEntry::ffi(FunctionDef::new(
            func_entity_hash,
            "process".to_string(),
            vec![],
            vec![Param::new("e", DataType::simple(entity_hash))],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        ));
        registry.register_function(func_entity).unwrap();

        // Function taking Player: process(Player)
        let func_player_hash = TypeHash::from_function("process_player", &[player_hash]);
        let func_player = FunctionEntry::ffi(FunctionDef::new(
            func_player_hash,
            "process".to_string(),
            vec![],
            vec![Param::new("p", DataType::simple(player_hash))],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        ));
        registry.register_function(func_player).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Both have same cost and same exact matches
        let m_entity = OverloadMatch {
            func_hash: func_entity_hash,
            arg_conversions: vec![Some(Conversion {
                kind: ConversionKind::Identity,
                cost: Conversion::COST_EXACT,
                is_implicit: true,
            })],
            total_cost: 0,
        };
        let m_player = OverloadMatch {
            func_hash: func_player_hash,
            arg_conversions: vec![Some(Conversion {
                kind: ConversionKind::Identity,
                cost: Conversion::COST_EXACT,
                is_implicit: true,
            })],
            total_cost: 0,
        };

        let viable = vec![m_entity, m_player.clone()];
        let result = find_best_match(&viable, &[], &ctx, Span::default());

        assert!(result.is_ok());
        // Player (more derived) should win
        assert_eq!(result.unwrap().func_hash, func_player_hash);
    }

    #[test]
    fn is_derived_from_direct() {
        use angelscript_core::entries::TypeSource;
        use angelscript_core::{ClassEntry, TypeKind};
        use angelscript_registry::SymbolRegistry;

        let mut registry = SymbolRegistry::with_primitives();

        let base_hash = TypeHash::from_name("Base");
        let base_class = ClassEntry::new(
            "Base",
            vec![],
            "Base",
            base_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(base_class.into()).unwrap();

        let derived_hash = TypeHash::from_name("Derived");
        let derived_class = ClassEntry::new(
            "Derived",
            vec![],
            "Derived",
            derived_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_base(base_hash);
        registry.register_type(derived_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        assert!(ctx.is_type_derived_from(derived_hash, base_hash));
        assert!(!ctx.is_type_derived_from(base_hash, derived_hash));
        assert!(!ctx.is_type_derived_from(base_hash, base_hash));
    }

    #[test]
    fn is_derived_from_transitive() {
        use angelscript_core::entries::TypeSource;
        use angelscript_core::{ClassEntry, TypeKind};
        use angelscript_registry::SymbolRegistry;

        let mut registry = SymbolRegistry::with_primitives();

        // Grandparent
        let grandparent_hash = TypeHash::from_name("Grandparent");
        let grandparent_class = ClassEntry::new(
            "Grandparent",
            vec![],
            "Grandparent",
            grandparent_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        );
        registry.register_type(grandparent_class.into()).unwrap();

        // Parent : Grandparent
        let parent_hash = TypeHash::from_name("Parent");
        let parent_class = ClassEntry::new(
            "Parent",
            vec![],
            "Parent",
            parent_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_base(grandparent_hash);
        registry.register_type(parent_class.into()).unwrap();

        // Child : Parent
        let child_hash = TypeHash::from_name("Child");
        let child_class = ClassEntry::new(
            "Child",
            vec![],
            "Child",
            child_hash,
            TypeKind::reference(),
            TypeSource::ffi_untyped(),
        )
        .with_base(parent_hash);
        registry.register_type(child_class.into()).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Child derives from Parent
        assert!(ctx.is_type_derived_from(child_hash, parent_hash));
        // Child derives from Grandparent (transitively)
        assert!(ctx.is_type_derived_from(child_hash, grandparent_hash));
        // Parent derives from Grandparent
        assert!(ctx.is_type_derived_from(parent_hash, grandparent_hash));
        // Not the other way around
        assert!(!ctx.is_type_derived_from(grandparent_hash, child_hash));
    }
}
