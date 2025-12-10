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
        if let Some(winner) = break_tie(best, second) {
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
/// Tie-breaking rules:
/// 1. Prefer more exact matches over conversions
/// 2. (Future) Prefer non-template over template instantiation
fn break_tie<'a>(a: &'a OverloadMatch, b: &'a OverloadMatch) -> Option<&'a OverloadMatch> {
    // Count exact matches (identity conversions)
    let a_exact = count_exact_matches(a);
    let b_exact = count_exact_matches(b);

    if a_exact > b_exact {
        return Some(a);
    }
    if b_exact > a_exact {
        return Some(b);
    }

    // TODO: Additional tie-breakers:
    // - Prefer non-template over template instantiation
    // - Prefer more derived class in inheritance hierarchy

    None // Truly ambiguous
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
}
