//! Overload resolution.
//!
//! This module handles function and operator overload resolution,
//! including finding the best matching function for given argument types.

use crate::codegen::Instruction;
use crate::lexer::Span;
use crate::semantic::{
    DataType, OperatorBehavior, SemanticErrorKind, VOID_TYPE,
};
use crate::semantic::types::type_def::FunctionId;

use super::{ExprContext, FunctionCompiler};

impl<'src, 'ast> FunctionCompiler<'src, 'ast> {
    pub(super) fn try_binary_operator_overload(
        &mut self,
        operator: OperatorBehavior,
        reverse_operator: OperatorBehavior,
        left_type: &DataType,
        right_type: &DataType,
        _span: Span,
    ) -> Option<DataType> {
        // Try left operand's operator first
        if let Some(func_id) = self.registry.find_operator_method(left_type.type_id, operator) {
            self.bytecode.emit(Instruction::Call(func_id.as_u32()));
            let func = self.registry.get_function(func_id);
            return Some(func.return_type.clone());
        }

        // Try right operand's reverse operator
        if let Some(func_id) = self.registry.find_operator_method(right_type.type_id, reverse_operator) {
            // For reverse operators, arguments are swapped: right.opAdd_r(left)
            // Stack already has: [left, right]
            // We need: [right, left]
            self.bytecode.emit(Instruction::Swap);
            self.bytecode.emit(Instruction::Call(func_id.as_u32()));
            let func = self.registry.get_function(func_id);
            return Some(func.return_type.clone());
        }

        None
    }

    /// Tries to find and call an operator overload for a unary operation.
    ///
    /// Returns Some(result_type) if operator overload was found and emitted,
    /// None if no overload exists (caller should try primitive operation).
    pub(super) fn try_unary_operator_overload(
        &mut self,
        operator: OperatorBehavior,
        operand_type: &DataType,
        _span: Span,
    ) -> Option<DataType> {
        if let Some(func_id) = self.registry.find_operator_method(operand_type.type_id, operator) {
            self.bytecode.emit(Instruction::Call(func_id.as_u32()));
            let func = self.registry.get_function(func_id);
            return Some(func.return_type.clone());
        }
        None
    }

    /// Validates reference parameters against their arguments.
    ///
    /// Checks that &out and &inout arguments are mutable lvalues.
    /// &in parameters can accept any value (lvalue or rvalue).
    pub(super) fn validate_reference_parameters(
        &mut self,
        func_def: &crate::semantic::types::registry::FunctionDef,
        arg_contexts: &[ExprContext],
        call_args: &[crate::ast::expr::Argument<'src, 'ast>],
        _span: Span,
    ) -> Option<()> {
        use crate::semantic::types::RefModifier;

        // Iterate through parameters and check reference modifiers
        for (i, param_type) in func_def.params.iter().enumerate() {
            // Skip if we don't have an argument for this parameter
            if i >= arg_contexts.len() {
                continue;
            }

            let arg_ctx = &arg_contexts[i];

            // Void expressions cannot be passed as arguments
            if arg_ctx.data_type.type_id == VOID_TYPE {
                self.error(
                    SemanticErrorKind::VoidExpression,
                    call_args[i].span,
                    format!("cannot pass void expression as argument {}", i + 1),
                );
                return None;
            }

            match param_type.ref_modifier {
                RefModifier::None => {
                    // No reference, any value is fine
                }
                RefModifier::In => {
                    // &in accepts any value (lvalue or rvalue)
                    // The compiler will create a temporary if needed
                }
                RefModifier::Out | RefModifier::InOut => {
                    // &out and &inout require mutable lvalues
                    if !arg_ctx.is_lvalue {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            call_args[i].span,
                            format!(
                                "parameter {} with '{}' modifier requires an lvalue, found rvalue",
                                i + 1,
                                if param_type.ref_modifier == RefModifier::Out { "&out" } else { "&inout" }
                            ),
                        );
                        return None;
                    }

                    if !arg_ctx.is_mutable {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            call_args[i].span,
                            format!(
                                "parameter {} with '{}' modifier requires a mutable lvalue, found const lvalue",
                                i + 1,
                                if param_type.ref_modifier == RefModifier::Out { "&out" } else { "&inout" }
                            ),
                        );
                        return None;
                    }
                }
            }
        }

        Some(())
    }

    /// Finds the best matching function overload for the given arguments.
    ///
    /// Returns the FunctionId of the best match, or None if no match found.
    pub(super) fn find_best_function_overload(
        &mut self,
        candidates: &[FunctionId],
        arg_types: &[DataType],
        span: Span,
    ) -> Option<(FunctionId, Vec<Option<crate::semantic::Conversion>>)> {
        // Filter candidates by argument count first (considering default parameters)
        let count_matched: Vec<_> = candidates.iter().copied()
            .filter(|&func_id| {
                let func_def = self.registry.get_function(func_id);
                // Calculate minimum required params (total - defaults with values)
                let default_count = func_def.default_args.iter().filter(|a| a.is_some()).count();
                let min_params = func_def.params.len() - default_count;
                let max_params = func_def.params.len();
                // Accept if arg count is within valid range
                arg_types.len() >= min_params && arg_types.len() <= max_params
            })
            .collect();

        if count_matched.is_empty() {
            self.error(
                SemanticErrorKind::WrongArgumentCount,
                span,
                format!(
                    "no overload found with {} argument(s)",
                    arg_types.len()
                ),
            );
            return None;
        }

        // Find exact match first (all types match exactly)
        for &func_id in &count_matched {
            let func_def = self.registry.get_function(func_id);

            // Check if all parameters match exactly (considering identity conversions)
            let mut conversions = Vec::with_capacity(arg_types.len());
            let mut is_exact = true;

            for (param, arg) in func_def.params.iter().zip(arg_types.iter()) {
                if let Some(conversion) = arg.can_convert_to(param, self.registry) {
                    if conversion.cost == 0 {
                        // Identity or trivial conversion
                        conversions.push(if matches!(conversion.kind, crate::semantic::ConversionKind::Identity) {
                            None
                        } else {
                            Some(conversion)
                        });
                    } else {
                        // Non-identity conversion needed
                        is_exact = false;
                        break;
                    }
                } else {
                    // No conversion available
                    is_exact = false;
                    break;
                }
            }

            if is_exact {
                return Some((func_id, conversions));
            }
        }

        // If no exact match, find best match with implicit conversions
        // Rank by total conversion cost
        let mut best_match: Option<(FunctionId, Vec<Option<crate::semantic::Conversion>>, u32)> = None;

        for &func_id in &count_matched {
            let func_def = self.registry.get_function(func_id);
            let mut conversions = Vec::with_capacity(arg_types.len());
            let mut total_cost = 0u32;
            let mut all_convertible = true;

            for (param_type, arg_type) in func_def.params.iter().zip(arg_types.iter()) {
                if param_type.type_id == arg_type.type_id {
                    // Exact match - no conversion needed
                    conversions.push(None);
                } else if let Some(conversion) = arg_type.can_convert_to(param_type, self.registry) {
                    if !conversion.is_implicit {
                        // Explicit conversion required - not valid for function calls
                        all_convertible = false;
                        break;
                    }
                    total_cost += conversion.cost;
                    conversions.push(Some(conversion));
                } else {
                    // No conversion available
                    all_convertible = false;
                    break;
                }
            }

            if all_convertible {
                // Update best match if this is better (lower cost)
                if let Some((_, _, best_cost)) = best_match {
                    if total_cost < best_cost {
                        best_match = Some((func_id, conversions, total_cost));
                    }
                } else {
                    best_match = Some((func_id, conversions, total_cost));
                }
            }
        }

        if let Some((func_id, conversions, _)) = best_match {
            Some((func_id, conversions))
        } else {
            self.error(
                SemanticErrorKind::TypeMismatch,
                span,
                "no matching overload found for given argument types".to_string(),
            );
            None
        }
    }
}
