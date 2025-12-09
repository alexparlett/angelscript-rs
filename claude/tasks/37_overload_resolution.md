# Task 37: Overload Resolution

## Overview

Implement the overload resolution algorithm that selects the best matching function from a set of candidates based on argument types and conversion costs.

## Goals

1. Use `ctx.get_function_overloads(name)` to get candidates from both registries
2. Filter candidates by argument count (considering defaults)
3. Check argument type compatibility
4. Rank candidates by total conversion cost
5. Detect and report ambiguous overloads
6. Handle operator overload resolution

## Dependencies

- Task 33: Compilation Context (provides `get_function_overloads()`)
- Task 36: Conversion System

## Files to Create

```
crates/angelscript-compiler/src/
├── overload/
│   ├── mod.rs             # Main overload resolution
│   ├── candidate.rs       # Candidate filtering
│   ├── ranking.rs         # Cost-based ranking
│   └── operators.rs       # Operator overload resolution
└── lib.rs
```

## Detailed Implementation

### 1. Main Overload Resolution (overload/mod.rs)

```rust
use angelscript_core::{DataType, TypeHash};

use crate::context::CompilationContext;
use crate::conversion::{find_conversion, Conversion};
use crate::error::{CompileError, Result};

mod candidate;
mod ranking;
pub mod operators;

/// Result of overload resolution.
#[derive(Debug)]
pub struct OverloadMatch {
    /// The selected function hash
    pub func_hash: TypeHash,
    /// Conversions needed for each argument
    pub arg_conversions: Vec<Option<Conversion>>,
    /// Total conversion cost
    pub total_cost: u32,
}

/// Resolve overloaded function call.
/// Uses ctx.get_function_overloads() which searches both unit and global registries.
pub fn resolve_overload(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
) -> Result<OverloadMatch> {
    if candidates.is_empty() {
        return Err(CompileError::Internal {
            message: "No candidates for overload resolution".to_string(),
        });
    }

    // Fast path: single candidate
    if candidates.len() == 1 {
        return try_single_candidate(candidates[0], arg_types, ctx);
    }

    // Filter to viable candidates
    let viable: Vec<_> = candidates
        .iter()
        .filter_map(|hash| try_match_candidate(*hash, arg_types, ctx))
        .collect();

    if viable.is_empty() {
        return Err(no_matching_overload_error(candidates, arg_types, ctx));
    }

    // Find best match by cost
    let best = ranking::find_best_match(&viable)?;

    Ok(best)
}

/// Try to match a single candidate.
fn try_single_candidate(
    func_hash: TypeHash,
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
) -> Result<OverloadMatch> {
    match try_match_candidate(func_hash, arg_types, ctx) {
        Some(m) => Ok(m),
        None => Err(no_matching_overload_error(&[func_hash], arg_types, ctx)),
    }
}

/// Try to match arguments against a candidate function.
fn try_match_candidate(
    func_hash: TypeHash,
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
) -> Option<OverloadMatch> {
    let func = ctx.get_function(func_hash)?;
    let params = func.def().params();

    // Check argument count (considering defaults)
    let required_params = params.iter().filter(|p| !p.has_default()).count();
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
                total_cost += conv.cost;
                arg_conversions.push(Some(conv));
            }
            _ => return None,  // No implicit conversion available
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

fn no_matching_overload_error(
    candidates: &[TypeHash],
    arg_types: &[DataType],
    ctx: &CompilationContext<'_>,
) -> CompileError {
    let name = candidates
        .first()
        .and_then(|h| ctx.get_function(*h))
        .map(|f| f.def().name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let args = arg_types
        .iter()
        .map(|dt| format!("{:?}", dt.type_hash))
        .collect::<Vec<_>>()
        .join(", ");

    CompileError::NoMatchingOverload {
        name,
        args,
        span: Default::default(),  // Caller should set span
    }
}
```

### 2. Ranking (overload/ranking.rs)

```rust
use super::OverloadMatch;
use crate::error::{CompileError, Result};

/// Find the best match from viable candidates.
pub fn find_best_match(viable: &[OverloadMatch]) -> Result<OverloadMatch> {
    assert!(!viable.is_empty());

    if viable.len() == 1 {
        return Ok(viable[0].clone());
    }

    // Sort by total cost
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

        return Err(CompileError::AmbiguousOverload {
            name: "function".to_string(),  // Caller should set
            candidates: vec![
                format!("{:?}", best.func_hash),
                format!("{:?}", second.func_hash),
            ],
            span: Default::default(),
        });
    }

    Ok(best.clone())
}

/// Try to break a tie between two candidates with equal cost.
fn break_tie<'a>(a: &'a OverloadMatch, b: &'a OverloadMatch) -> Option<&'a OverloadMatch> {
    // Prefer exact matches over conversions
    let a_exact = a.arg_conversions.iter()
        .filter(|c| c.as_ref().map(|c| c.is_exact()).unwrap_or(true))
        .count();
    let b_exact = b.arg_conversions.iter()
        .filter(|c| c.as_ref().map(|c| c.is_exact()).unwrap_or(true))
        .count();

    if a_exact > b_exact {
        return Some(a);
    }
    if b_exact > a_exact {
        return Some(b);
    }

    // Prefer non-template over template instantiation
    // (would need additional info in OverloadMatch)

    None  // Truly ambiguous
}
```

### 3. Operator Resolution (overload/operators.rs)

```rust
use angelscript_core::{DataType, TypeHash};
use angelscript_parser::ast::BinaryOp;

use crate::context::CompilationContext;
use crate::conversion::find_conversion;
use crate::error::Result;
use super::OverloadMatch;

/// Result of operator resolution.
#[derive(Debug)]
pub enum OperatorResolution {
    /// Built-in primitive operation
    Primitive {
        opcode: crate::bytecode::OpCode,
        result_type: DataType,
    },
    /// User-defined operator method
    Method {
        method_hash: TypeHash,
        on_left: bool,  // true = left.op(right), false = right.op_r(left)
        arg_conversion: Option<crate::conversion::Conversion>,
        result_type: DataType,
    },
}

/// Resolve binary operator.
pub fn resolve_binary_operator(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
    ctx: &CompilationContext<'_>,
) -> Result<OperatorResolution> {
    // 1. Try primitive operation
    if let Some(resolution) = try_primitive_operator(left, right, op) {
        return Ok(resolution);
    }

    // 2. Try left.opXxx(right)
    let method_name = operator_method_name(op);
    if let Some(resolution) = try_method_operator(left, right, &method_name, true, ctx) {
        return Ok(resolution);
    }

    // 3. Try right.opXxx_r(left) - reverse operator
    let reverse_name = format!("{}_r", method_name);
    if let Some(resolution) = try_method_operator(right, left, &reverse_name, false, ctx) {
        return Ok(resolution);
    }

    Err(crate::error::CompileError::NoOperator {
        op: format!("{:?}", op),
        left: format!("{:?}", left.type_hash),
        right: format!("{:?}", right.type_hash),
        span: Default::default(),
    })
}

/// Try to resolve as primitive operation.
fn try_primitive_operator(
    left: &DataType,
    right: &DataType,
    op: BinaryOp,
) -> Option<OperatorResolution> {
    use crate::bytecode::OpCode;
    use angelscript_core::primitives;

    // Both must be same primitive type (or convertible)
    if left.type_hash != right.type_hash {
        return None;
    }

    let opcode = match (left.type_hash, op) {
        // i32 operations
        (primitives::INT32, BinaryOp::Add) => OpCode::AddI32,
        (primitives::INT32, BinaryOp::Sub) => OpCode::SubI32,
        (primitives::INT32, BinaryOp::Mul) => OpCode::MulI32,
        (primitives::INT32, BinaryOp::Div) => OpCode::DivI32,
        (primitives::INT32, BinaryOp::Mod) => OpCode::ModI32,
        (primitives::INT32, BinaryOp::Lt) => OpCode::LtI32,
        (primitives::INT32, BinaryOp::Le) => OpCode::LeI32,
        (primitives::INT32, BinaryOp::Gt) => OpCode::GtI32,
        (primitives::INT32, BinaryOp::Ge) => OpCode::GeI32,
        (primitives::INT32, BinaryOp::Eq) => OpCode::EqI32,

        // i64 operations
        (primitives::INT64, BinaryOp::Add) => OpCode::AddI64,
        (primitives::INT64, BinaryOp::Sub) => OpCode::SubI64,
        (primitives::INT64, BinaryOp::Mul) => OpCode::MulI64,
        (primitives::INT64, BinaryOp::Div) => OpCode::DivI64,
        (primitives::INT64, BinaryOp::Mod) => OpCode::ModI64,

        // f32 operations
        (primitives::FLOAT, BinaryOp::Add) => OpCode::AddF32,
        (primitives::FLOAT, BinaryOp::Sub) => OpCode::SubF32,
        (primitives::FLOAT, BinaryOp::Mul) => OpCode::MulF32,
        (primitives::FLOAT, BinaryOp::Div) => OpCode::DivF32,

        // f64 operations
        (primitives::DOUBLE, BinaryOp::Add) => OpCode::AddF64,
        (primitives::DOUBLE, BinaryOp::Sub) => OpCode::SubF64,
        (primitives::DOUBLE, BinaryOp::Mul) => OpCode::MulF64,
        (primitives::DOUBLE, BinaryOp::Div) => OpCode::DivF64,

        // Bitwise
        (primitives::INT32, BinaryOp::BitAnd) => OpCode::BitAnd,
        (primitives::INT32, BinaryOp::BitOr) => OpCode::BitOr,
        (primitives::INT32, BinaryOp::BitXor) => OpCode::BitXor,
        (primitives::INT32, BinaryOp::Shl) => OpCode::Shl,
        (primitives::INT32, BinaryOp::Shr) => OpCode::Shr,

        // Boolean
        (primitives::BOOL, BinaryOp::Eq) => OpCode::EqBool,

        _ => return None,
    };

    let result_type = match op {
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge |
        BinaryOp::Eq | BinaryOp::Ne => DataType::simple(primitives::BOOL),
        _ => *left,  // Arithmetic produces same type
    };

    Some(OperatorResolution::Primitive { opcode, result_type })
}

/// Try to resolve as method call.
fn try_method_operator(
    object: &DataType,
    arg: &DataType,
    method_name: &str,
    on_left: bool,
    ctx: &CompilationContext<'_>,
) -> Option<OperatorResolution> {
    let methods = ctx.find_methods(object.type_hash, method_name);

    for method_hash in methods {
        let func = ctx.get_function(method_hash)?;
        let def = func.def();

        // Must have exactly one parameter
        if def.params.len() != 1 {
            continue;
        }

        // Check if argument can convert to parameter
        if let Some(conv) = find_conversion(arg, &def.params[0].data_type, ctx) {
            if conv.is_implicit {
                return Some(OperatorResolution::Method {
                    method_hash,
                    on_left,
                    arg_conversion: Some(conv),
                    result_type: def.return_type,
                });
            }
        }
    }

    None
}

fn operator_method_name(op: BinaryOp) -> String {
    match op {
        BinaryOp::Add => "opAdd",
        BinaryOp::Sub => "opSub",
        BinaryOp::Mul => "opMul",
        BinaryOp::Div => "opDiv",
        BinaryOp::Mod => "opMod",
        BinaryOp::Eq => "opEquals",
        BinaryOp::Ne => "opEquals",  // Negated result
        BinaryOp::Lt => "opCmp",
        BinaryOp::Le => "opCmp",
        BinaryOp::Gt => "opCmp",
        BinaryOp::Ge => "opCmp",
        BinaryOp::BitAnd => "opAnd",
        BinaryOp::BitOr => "opOr",
        BinaryOp::BitXor => "opXor",
        BinaryOp::Shl => "opShl",
        BinaryOp::Shr => "opShr",
        _ => "opUnknown",
    }.to_string()
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_candidate_exact_match() {
        // Setup function taking int
        // Call with int
        // Should match with cost 0
    }

    #[test]
    fn single_candidate_with_conversion() {
        // Setup function taking int64
        // Call with int32
        // Should match with widening cost
    }

    #[test]
    fn multiple_candidates_best_match() {
        // Setup: foo(int), foo(float)
        // Call with int
        // Should select foo(int) as exact match
    }

    #[test]
    fn ambiguous_overload() {
        // Setup: foo(int, float), foo(float, int)
        // Call with (int, int)
        // Should report ambiguous
    }

    #[test]
    fn binary_operator_primitive() {
        // 1 + 2 should resolve to AddI32
    }

    #[test]
    fn binary_operator_method() {
        // vector + vector should find opAdd method
    }
}
```

## Acceptance Criteria

- [ ] Single candidate resolution works
- [ ] Multiple candidates ranked by conversion cost
- [ ] Exact matches preferred over conversions
- [ ] Ambiguous overloads detected and reported
- [ ] Default parameters handled correctly
- [ ] Primitive operators resolved to opcodes
- [ ] User-defined operators (opAdd, etc.) found
- [ ] Reverse operators (opAdd_r) checked
- [ ] All tests pass

## Next Phase

Task 38: Registration Pass - AST walker to register types and functions
