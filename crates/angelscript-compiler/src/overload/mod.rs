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

mod ranking;

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
    resolve_overload_with_const(candidates, arg_types, ctx, span, None)
}

/// Resolve a method overload with const-correctness handling.
///
/// For const objects, only const methods are considered.
/// For mutable objects, non-const methods are preferred but const methods
/// are allowed as fallback.
///
/// # Arguments
///
/// * `candidates` - Method hashes to consider
/// * `arg_types` - Types of the arguments at the call site
/// * `ctx` - Compilation context for type lookups
/// * `span` - Source location for error reporting
/// * `is_const_object` - Whether the object being called on is const
pub fn resolve_method_overload(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
    is_const_object: bool,
) -> Result<OverloadMatch, CompilationError> {
    resolve_overload_with_const(candidates, arg_types, ctx, span, Some(is_const_object))
}

/// Internal overload resolution with optional const-correctness handling.
fn resolve_overload_with_const(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
    span: Span,
    is_const_object: Option<bool>,
) -> Result<OverloadMatch, CompilationError> {
    if candidates.is_empty() {
        return Err(CompilationError::Internal {
            message: "No candidates for overload resolution".to_string(),
        });
    }

    // Filter candidates by const-correctness if applicable
    let filtered_candidates: Vec<TypeHash> = if let Some(is_const) = is_const_object {
        if is_const {
            // Const object: only const methods are valid
            candidates
                .iter()
                .filter(|&&hash| ctx.get_function(hash).is_some_and(|f| f.def.is_const()))
                .copied()
                .collect()
        } else {
            // Mutable object: prefer non-const, but allow const as fallback
            let non_const: Vec<TypeHash> = candidates
                .iter()
                .filter(|&&hash| ctx.get_function(hash).is_some_and(|f| !f.def.is_const()))
                .copied()
                .collect();
            if !non_const.is_empty() {
                non_const
            } else {
                candidates.to_vec()
            }
        }
    } else {
        candidates.to_vec()
    };

    if filtered_candidates.is_empty() {
        return Err(CompilationError::CannotModifyConst {
            message: "no const method available for const object".to_string(),
            span,
        });
    }

    // Fast path: single candidate
    if filtered_candidates.len() == 1 {
        return try_single_candidate(filtered_candidates[0], arg_types, ctx, span);
    }

    // Filter to viable candidates
    let viable: Vec<_> = filtered_candidates
        .iter()
        .filter_map(|hash| try_match_candidate(*hash, arg_types, ctx))
        .collect();

    if viable.is_empty() {
        return Err(no_matching_overload_error(
            &filtered_candidates,
            arg_types,
            ctx,
            span,
        ));
    }

    // Find best match by cost
    find_best_match(&viable, &filtered_candidates, ctx, span)
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
    let is_variadic = func.def.is_variadic;

    // Check argument count (considering defaults and variadic)
    let required_params = params.iter().filter(|p| !p.has_default).count();
    let max_params = params.len();

    // Too few arguments
    if arg_types.len() < required_params {
        return None;
    }

    // Too many arguments (unless variadic)
    if !is_variadic && arg_types.len() > max_params {
        return None;
    }

    // Check each argument can convert to parameter type
    let mut arg_conversions = Vec::with_capacity(arg_types.len());
    let mut total_cost = 0u32;

    for (arg, param) in arg_types.iter().zip(params.iter()) {
        let conv = find_conversion(arg, &param.data_type, ctx, true)?;
        total_cost = total_cost.saturating_add(conv.cost);
        arg_conversions.push(Some(conv));
    }

    // For variadic functions, handle extra arguments beyond the declared params.
    // AngelScript variadics are always typed - either a specific type or `?` (VARIABLE_PARAM).
    // The variadic type is the last parameter's type.
    if is_variadic && arg_types.len() > params.len() {
        let variadic_type = params.last().map(|p| p.data_type);

        for arg in arg_types.iter().skip(params.len()) {
            match variadic_type {
                Some(vt) if vt.type_hash == angelscript_core::primitives::VARIABLE_PARAM => {
                    // Variable type (`?`) - accept any type
                    arg_conversions.push(Some(Conversion::identity()));
                }
                Some(vt) => {
                    // Typed variadic - must convert to the variadic type
                    let conv = find_conversion(arg, &vt, ctx, true)?;
                    total_cost = total_cost.saturating_add(conv.cost);
                    arg_conversions.push(Some(conv));
                }
                None => {
                    // Shouldn't happen - variadic without params
                    return None;
                }
            }
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
        CompilationError, DataType, FunctionDef, FunctionEntry, FunctionTraits, Param, Visibility,
        primitives,
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

    fn int16_param(name: &str) -> Param {
        Param::new(name.to_string(), DataType::simple(primitives::INT16))
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
    fn single_candidate_narrowing_allowed() {
        // AngelScript allows implicit narrowing conversions
        let mut registry = setup_registry_with_primitives();
        let func = make_function("foo", vec![int_param("x")]);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Pass int64 to int32 parameter - narrowing is allowed implicitly in AngelScript
        let arg_types = vec![DataType::simple(primitives::INT64)];

        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert!(m.total_cost > 0); // Narrowing has a cost
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

        // foo(int16)
        let func_int16 = make_function("foo", vec![int16_param("x")]);
        let hash_int16 = func_int16.def.func_hash;
        registry.register_function(func_int16).unwrap();

        // foo(int32)
        let func_int32 = make_function("foo", vec![int_param("x")]);
        let hash_int32 = func_int32.def.func_hash;
        registry.register_function(func_int32).unwrap();

        let ctx = CompilationContext::new(&registry);
        // Both int8->int16 and int8->int32 are signed widening with same cost
        let arg_types = vec![DataType::simple(primitives::INT8)];

        let result = resolve_overload(&[hash_int16, hash_int32], &arg_types, &ctx, Span::default());

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
        match result.unwrap_err() {
            CompilationError::NoMatchingOverload { .. } => {}
            other => panic!(
                "Expected NoMatchingOverload error for wrong argument count, got: {:?}",
                other
            ),
        }
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

    #[test]
    fn default_parameter_allows_fewer_args() {
        let mut registry = setup_registry_with_primitives();

        // foo(int x, int y = 10) - second param has default
        let param_types: Vec<_> = vec![primitives::INT32, primitives::INT32];
        let hash = TypeHash::from_function("foo", &param_types);
        let def = FunctionDef::new(
            hash,
            "foo".to_string(),
            vec![],
            vec![
                Param::new("x", DataType::simple(primitives::INT32)),
                Param::with_default("y", DataType::simple(primitives::INT32)),
            ],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let func = FunctionEntry::ffi(def);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with only one argument - should succeed because y has default
        let arg_types = vec![DataType::simple(primitives::INT32)];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert_eq!(m.arg_conversions.len(), 2);
        assert!(m.arg_conversions[0].is_some()); // First arg has conversion
        assert!(m.arg_conversions[1].is_none()); // Second arg uses default
    }

    #[test]
    fn default_parameter_with_all_args_provided() {
        let mut registry = setup_registry_with_primitives();

        // foo(int x, int y = 10)
        let param_types: Vec<_> = vec![primitives::INT32, primitives::INT32];
        let hash = TypeHash::from_function("foo", &param_types);
        let def = FunctionDef::new(
            hash,
            "foo".to_string(),
            vec![],
            vec![
                Param::new("x", DataType::simple(primitives::INT32)),
                Param::with_default("y", DataType::simple(primitives::INT32)),
            ],
            DataType::void(),
            None,
            FunctionTraits::default(),
            false,
            Visibility::Public,
        );
        let func = FunctionEntry::ffi(def);
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with both arguments
        let arg_types = vec![
            DataType::simple(primitives::INT32),
            DataType::simple(primitives::INT32),
        ];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.arg_conversions.len(), 2);
        assert!(m.arg_conversions[0].is_some()); // Both args have conversions
        assert!(m.arg_conversions[1].is_some());
    }

    fn make_variadic_function(name: &str, params: Vec<Param>) -> FunctionEntry {
        let param_types: Vec<_> = params.iter().map(|p| p.data_type.type_hash).collect();
        let hash = TypeHash::from_function(name, &param_types);
        let mut def = FunctionDef::new(
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
        def.is_variadic = true;
        FunctionEntry::ffi(def)
    }

    #[test]
    fn variadic_function_accepts_exact_required_params() {
        let mut registry = setup_registry_with_primitives();

        // print(string format, ?& ...) - format param + variadic of any type
        let func = make_variadic_function(
            "print",
            vec![
                Param::new("format", DataType::simple(primitives::STRING)),
                Param::new("args", DataType::simple(primitives::VARIABLE_PARAM)),
            ],
        );
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with format + one variadic arg (matches the 2 params exactly)
        let arg_types = vec![
            DataType::simple(primitives::STRING),
            DataType::simple(primitives::INT32),
        ];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert_eq!(m.arg_conversions.len(), 2);
        assert!(m.arg_conversions[0].as_ref().unwrap().is_exact());
        // VARIABLE_PARAM uses VarArg conversion (not exact, but still valid)
        assert!(m.arg_conversions[1].is_some());
    }

    #[test]
    fn variadic_function_accepts_extra_args() {
        let mut registry = setup_registry_with_primitives();

        // print(string format, ?& ...) - format param + variadic of any type
        let func = make_variadic_function(
            "print",
            vec![
                Param::new("format", DataType::simple(primitives::STRING)),
                Param::new("args", DataType::simple(primitives::VARIABLE_PARAM)),
            ],
        );
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with format + 2 extra variadic args
        let arg_types = vec![
            DataType::simple(primitives::STRING),
            DataType::simple(primitives::INT32),
            DataType::simple(primitives::FLOAT),
        ];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_ok());
        let m = result.unwrap();
        assert_eq!(m.func_hash, func_hash);
        assert_eq!(m.arg_conversions.len(), 3);
        // First is the format parameter
        assert!(m.arg_conversions[0].as_ref().unwrap().is_exact());
        // Extra variadic args get identity conversions (any type accepted)
        assert!(m.arg_conversions[1].is_some());
        assert!(m.arg_conversions[2].is_some());
    }

    #[test]
    fn variadic_function_rejects_fewer_than_required() {
        let mut registry = setup_registry_with_primitives();

        // print(string format, ...) - one required param, variadic after
        let func = make_variadic_function(
            "print",
            vec![Param::new("format", DataType::simple(primitives::STRING))],
        );
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with zero arguments - should fail because format is required
        let arg_types: Vec<DataType> = vec![];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());

        assert!(result.is_err());
        match result.unwrap_err() {
            CompilationError::NoMatchingOverload { name, .. } => {
                assert_eq!(name, "print");
            }
            other => panic!("Expected NoMatchingOverload, got: {:?}", other),
        }
    }

    #[test]
    fn variadic_function_with_multiple_required_params() {
        let mut registry = setup_registry_with_primitives();

        // format(string fmt, int precision, ...) - two required params
        let func = make_variadic_function(
            "format",
            vec![
                Param::new("fmt", DataType::simple(primitives::STRING)),
                Param::new("precision", DataType::simple(primitives::INT32)),
            ],
        );
        let func_hash = func.def.func_hash;
        registry.register_function(func).unwrap();

        let ctx = CompilationContext::new(&registry);

        // Call with exactly 2 required args - should succeed
        let arg_types = vec![
            DataType::simple(primitives::STRING),
            DataType::simple(primitives::INT32),
        ];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());
        assert!(result.is_ok());

        // Call with 2 required + 1 extra - should succeed
        let arg_types = vec![
            DataType::simple(primitives::STRING),
            DataType::simple(primitives::INT32),
            DataType::simple(primitives::FLOAT),
        ];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());
        assert!(result.is_ok());

        // Call with only 1 arg - should fail (needs 2 required)
        let arg_types = vec![DataType::simple(primitives::STRING)];
        let result = resolve_overload(&[func_hash], &arg_types, &ctx, Span::default());
        assert!(result.is_err());
    }
}
