//! Overload resolution for function calls.
//!
//! This module implements the overload resolution algorithm that selects the best
//! matching function from a set of candidates based on argument types and conversion costs.
//!
//! ## Algorithm
//!
//! 1. Filter candidates by argument count (considering default parameters)
//! 2. Check argument type compatibility for each candidate
//! 3. Calculate total conversion cost for each viable candidate
//! 4. Rank candidates by cost and select the best match
//! 5. Detect and report ambiguous overloads when multiple candidates tie

mod operators;
mod ranking;

pub use operators::{OperatorResolution, resolve_binary_operator, resolve_unary_operator};
pub use ranking::find_best_match;

use angelscript_core::{CompilationError, DataType, Span, TypeHash};

use crate::context::CompilationContext;
use crate::conversion::{Conversion, find_conversion};

/// Result of successful overload resolution.
#[derive(Debug, Clone)]
pub struct OverloadMatch {
    /// The selected function hash.
    pub func_hash: TypeHash,
    /// Conversions needed for each argument (None = use default parameter).
    pub arg_conversions: Vec<Option<Conversion>>,
    /// Total conversion cost (lower is better).
    pub total_cost: u32,
}

/// Resolve an overloaded function call.
///
/// Given a list of candidate function hashes and the argument types provided,
/// selects the best matching function based on conversion costs.
///
/// # Arguments
///
/// * `candidates` - Function hashes to consider (from `ctx.resolve_function()`)
/// * `arg_types` - Types of the arguments at the call site
/// * `ctx` - Compilation context for type lookups
/// * `span` - Source location for error reporting
///
/// # Returns
///
/// * `Ok(OverloadMatch)` - The best matching function with conversion info
/// * `Err(CompilationError)` - No match found or ambiguous overload
pub fn resolve_overload(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OverloadMatch, CompilationError> {
    if candidates.is_empty() {
        return Err(CompilationError::Internal {
            message: "No candidates for overload resolution".to_string(),
        });
    }

    // Fast path: single candidate
    if candidates.len() == 1 {
        return try_single_candidate(candidates[0], arg_types, ctx, span);
    }

    // Filter to viable candidates
    let viable: Vec<_> = candidates
        .iter()
        .filter_map(|hash| try_match_candidate(*hash, arg_types, ctx))
        .collect();

    if viable.is_empty() {
        return Err(no_matching_overload_error(candidates, arg_types, ctx, span));
    }

    // Find best match by cost
    find_best_match(&viable, candidates, ctx, span)
}

/// Try to match a single candidate (fast path).
fn try_single_candidate(
    func_hash: TypeHash,
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> Result<OverloadMatch, CompilationError> {
    match try_match_candidate(func_hash, arg_types, ctx) {
        Some(m) => Ok(m),
        None => Err(no_matching_overload_error(
            &[func_hash],
            arg_types,
            ctx,
            span,
        )),
    }
}

/// Try to match arguments against a candidate function.
///
/// Returns `Some(OverloadMatch)` if the arguments can be converted to the
/// parameter types, `None` if they're incompatible.
fn try_match_candidate(
    func_hash: TypeHash,
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
) -> Option<OverloadMatch> {
    let func = ctx.get_function(func_hash)?;
    let params = &func.def.params;

    // Check argument count (considering defaults)
    let required_params = params.iter().filter(|p| !p.has_default).count();
    let max_params = params.len();

    if arg_types.len() < required_params || arg_types.len() > max_params {
        return None;
    }

    // Check each argument can convert to parameter type
    let mut arg_conversions = Vec::with_capacity(arg_types.len());
    let mut total_cost = 0u32;

    for (arg, param) in arg_types.iter().zip(params.iter()) {
        match find_conversion(arg, &param.data_type, ctx) {
            Some(conv) if conv.is_implicit => {
                total_cost = total_cost.saturating_add(conv.cost);
                arg_conversions.push(Some(conv));
            }
            _ => return None, // No implicit conversion available
        }
    }

    // Fill in None for default parameters not provided
    for _ in arg_types.len()..params.len() {
        arg_conversions.push(None);
    }

    Some(OverloadMatch {
        func_hash,
        arg_conversions,
        total_cost,
    })
}

/// Build error for no matching overload.
fn no_matching_overload_error(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
) -> CompilationError {
    let name = candidates
        .first()
        .and_then(|h| ctx.get_function(*h))
        .map(|f| f.def.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let args = arg_types
        .iter()
        .map(|dt| format_type_name(dt.type_hash, ctx))
        .collect::<Vec<_>>()
        .join(", ");

    CompilationError::NoMatchingOverload { name, args, span }
}

/// Format a type hash as a readable name.
fn format_type_name(hash: TypeHash, ctx: &CompilationContext<'_>) -> String {
    ctx.get_type(hash)
        .map(|e| e.qualified_name().to_string())
        .unwrap_or_else(|| format!("{:?}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use angelscript_core::{
        DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, Visibility, primitives,
    };
    use angelscript_registry::SymbolRegistry;

    fn setup_registry_with_primitives() -> SymbolRegistry {
        SymbolRegistry::with_primitives()
    }

    fn make_function(name: &str, params: Vec<Param>) -> FunctionEntry {
        let param_types: Vec<_> = params.iter().map(|p| p.data_type.type_hash).collect();
        let hash = TypeHash::from_function(name, &param_types);
        let def = FunctionDef::new(
            hash,
            name.to_string(),
            vec![],
            params,
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        FunctionEntry::ffi(def)
    }

    fn int_param(name: &str) -> Param {
        Param::new(name.to_string(), DataType::simple(primitives::INT32))
    }

    fn int64_param(name: &str) -> Param {
        Param::new(name.to_string(), DataType::simple(primitives::INT64))
    }

    fn float_param(name: &str) -> Param {
        Param::new(name.to_string(), DataType::simple(primitives::FLOAT))
    }

    fn double_param(name: &str) -> Param {
        Param::new(name.to_string(), DataType::simple(primitives::DOUBLE))
    }

    #[test]
    fn single_candidate_exact_match() {
        let mut registry = setup_registry_with_primitives();
        let func = make_function("foo", vec![int_param("x")]);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);
        let arg_types = vec![DataType::simple(primitives::INT32)];

        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert_eq!(m.total_cost, 0); // Exact match
        assert!(m.arg_conversions[0].as_ref().unwrap().is_exact());
    }

    #[test]
    fn single_candidate_with_widening() {
        let mut registry = setup_registry_with_primitives();
        let func = make_function("foo", vec![int64_param("x")]);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Pass int32 to int64 parameter
        let arg_types = vec![DataType::simple(primitives::INT32)];

        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert!(m.total_cost > 0); // Widening has cost
    }

    #[test]
    fn single_candidate_no_match() {
        let mut registry = setup_registry_with_primitives();
        let func = make_function("foo", vec![int_param("x")]);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Pass int64 to int32 parameter (narrowing, not implicit)
        let arg_types = vec![DataType::simple(primitives::INT64)];

        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::NoMatchingOverload { .. }
        ));
    }

    #[test]
    fn multiple_candidates_select_exact() {
        let mut registry = setup_registry_with_primitives();

        // foo(int)
        let func_int = make_function("foo", vec![int_param("x")]);
        let hash_int = func_int.def.func_hash;
        registry.register_function(func_int).unwrap();

        // foo(float)
        let func_float = make_function("foo", vec![float_param("x")]);
        let hash_float = func_float.def.func_hash;
        registry.register_function(func_float).unwrap();

        let ctx = CompilationContext::new(&registry);
        let arg_types = vec![DataType::simple(primitives::INT32)];

        let result = resolve_overload(&[hash_int, hash_float], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        // Should select foo(int) as exact match
        assert_eq!(m.func_hash, hash_int);
        assert_eq!(m.total_cost, 0);
    }

    #[test]
    fn multiple_candidates_same_cost_is_ambiguous() {
        let mut registry = setup_registry_with_primitives();

        // foo(int64)
        let func_int64 = make_function("foo", vec![int64_param("x")]);
        let hash_int64 = func_int64.def.func_hash;
        registry.register_function(func_int64).unwrap();

        // foo(double)
        let func_double = make_function("foo", vec![double_param("x")]);
        let hash_double = func_double.def.func_hash;
        registry.register_function(func_double).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Both int32->int64 and int32->double are widening with same cost
        let arg_types = vec![DataType::simple(primitives::INT32)];

        let result = resolve_overload(
            &[hash_int64, hash_double],
            &arg_types,
            &ctx,
            Span::default(),
        );

        // Both have same cost and same number of exact matches (0), so it's ambiguous
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::AmbiguousOverload { .. }
        ));
    }

    #[test]
    fn wrong_argument_count() {
        let mut registry = setup_registry_with_primitives();
        let func = make_function("foo", vec![int_param("x"), int_param("y")]);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Only one argument for two-parameter function
        let arg_types = vec![DataType::simple(primitives::INT32)];

        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_err());
    }

    #[test]
    fn empty_candidates_is_error() {
        let registry = setup_registry_with_primitives();
        let ctx = CompilationContext::new(&registry);

        let result = resolve_overload(&[], &[], &ctx, Span::default());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompilationError::Internal { .. }
        ));
    }
}
