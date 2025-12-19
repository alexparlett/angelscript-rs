//! Foreach loop compilation.
//!
//! Handles foreach loops over collections that implement the opFor* behaviors.

use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, TypeHash};
use angelscript_parser::ast::ForeachStmt;

use crate::bytecode::OpCode;
use crate::type_resolver::TypeResolver;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a foreach loop.
    ///
    /// Foreach loops iterate over collections that implement the opFor* behaviors:
    /// - `opForBegin()` - Initialize iteration state
    /// - `opForEnd(state)` - Check if iteration is complete (returns bool)
    /// - `opForNext(state)` - Advance to next element
    /// - `opForValue(state)` - Get current value (or opForValue0, opForValue1, etc.)
    ///
    /// Bytecode layout (matches AngelScript semantics):
    /// ```text
    /// [collection expression]
    /// Dup
    /// CallMethod opForBegin
    /// SetLocal __iter_state
    ///
    /// loop_start:
    /// GetLocal __collection
    /// GetLocal __iter_state
    /// CallMethod opForEnd
    /// JumpIfTrue -> exit
    /// Pop (result of opForEnd)
    ///
    /// GetLocal __collection
    /// GetLocal __iter_state
    /// CallMethod opForValue
    /// SetLocal item_var
    ///
    /// [body]
    ///
    /// GetLocal __collection
    /// GetLocal __iter_state
    /// CallMethod opForNext      <- update happens after body
    /// SetLocal __iter_state
    ///
    /// Loop -> loop_start
    /// exit:
    /// Pop (result of opForEnd)
    /// [cleanup]
    /// ```
    ///
    /// This matches the AngelScript equivalent:
    /// ```text
    /// for(auto @it = container.opForBegin(); !container.opForEnd(it); @it = container.opForNext(it))
    /// {
    ///     auto val = container.opForValue(it);
    ///     ...
    /// }
    /// ```
    pub fn compile_foreach<'ast>(&mut self, foreach: &ForeachStmt<'ast>) -> Result<()> {
        let span = foreach.span;

        // Push scope for iteration variables and hidden state
        self.ctx.push_local_scope();

        // Compile collection expression
        let collection_info = {
            let mut expr_compiler = self.expr_compiler();
            expr_compiler.infer(foreach.expr)?
        };

        // Look up foreach behaviors on the collection type
        let foreach_behaviors =
            self.get_foreach_behaviors(&collection_info.data_type, foreach.vars.len(), span)?;

        // Store collection in hidden local (needed for opFor calls)
        let collection_slot = self.ctx.declare_local(
            "__foreach_collection".to_string(),
            collection_info.data_type,
            true, // const
            span,
        )?;
        self.ctx.mark_local_initialized("__foreach_collection");
        self.emitter.emit_dup(); // Dup for storing
        self.emitter.emit_set_local(collection_slot);

        // Call opForBegin to get initial state
        self.emitter
            .emit_call_method(foreach_behaviors.for_begin, 0);

        // Store state in hidden local
        let state_slot = self.ctx.declare_local(
            "__foreach_state".to_string(),
            foreach_behaviors.state_type,
            false, // mutable
            span,
        )?;
        self.ctx.mark_local_initialized("__foreach_state");
        self.emitter.emit_set_local(state_slot);

        // Declare iteration variables
        let mut var_slots = Vec::with_capacity(foreach.vars.len());
        for (i, var) in foreach.vars.iter().enumerate() {
            // Resolve type
            let var_type = TypeResolver::new(self.ctx).resolve(&var.ty)?;
            let expected_type = &foreach_behaviors.value_types[i];

            // Check type compatibility
            if var_type != *expected_type {
                return Err(CompilationError::TypeMismatch {
                    message: format!(
                        "foreach variable type mismatch: expected {:?}, found {:?}",
                        expected_type.type_hash, var_type.type_hash
                    ),
                    span: var.span,
                });
            }

            let slot = self.ctx.declare_local(
                var.name.name.to_string(),
                var_type,
                false, // mutable
                var.span,
            )?;
            var_slots.push(slot);
        }

        // Loop start - use deferred continue target since continue should
        // jump to opForNext (update), not back to condition
        let loop_start = self.emitter.current_offset();
        self.emitter.enter_loop_deferred();

        // Check if iteration is complete: opForEnd(state) -> bool
        self.emitter.emit_get_local(collection_slot);
        self.emitter.emit_get_local(state_slot);
        self.emitter.emit_call_method(foreach_behaviors.for_end, 1);

        // Exit if done
        let exit_jump = self.emitter.emit_jump(OpCode::JumpIfTrue);

        // Pop the opForEnd result
        self.emitter.emit_pop();

        // Get values and store in iteration variables BEFORE advancing
        // (opForNext is the loop "update", called after body)
        if foreach.vars.len() == 1 {
            // Single value: opForValue(state) -> value
            self.emitter.emit_get_local(collection_slot);
            self.emitter.emit_get_local(state_slot);
            self.emitter
                .emit_call_method(foreach_behaviors.for_value[0], 1);
            self.emitter.emit_set_local(var_slots[0]);
            self.ctx.mark_local_initialized(foreach.vars[0].name.name);
        } else {
            // Multiple values: opForValue0(state), opForValue1(state), etc.
            for (i, (slot, var)) in var_slots.iter().zip(foreach.vars.iter()).enumerate() {
                self.emitter.emit_get_local(collection_slot);
                self.emitter.emit_get_local(state_slot);
                self.emitter
                    .emit_call_method(foreach_behaviors.for_value[i], 1);
                self.emitter.emit_set_local(*slot);
                self.ctx.mark_local_initialized(var.name.name);
            }
        }

        // Compile body
        self.compile(foreach.body)?;

        // Set continue target to the update (opForNext)
        // This patches any continue statements in the body
        let update_start = self.emitter.current_offset();
        self.emitter.set_continue_target(update_start);

        // Advance to next: opForNext(state) -> new_state
        // This is the loop "update" expression, called after body
        self.emitter.emit_get_local(collection_slot);
        self.emitter.emit_get_local(state_slot);
        self.emitter.emit_call_method(foreach_behaviors.for_next, 1);
        self.emitter.emit_set_local(state_slot);

        // Loop back
        self.emitter.emit_loop(loop_start);

        // Exit target
        self.emitter.patch_jump(exit_jump);
        self.emitter.emit_pop(); // Pop opForEnd result

        // Exit loop context
        self.emitter.exit_loop();

        // Pop scope and cleanup handles
        let exiting_vars = self.ctx.pop_local_scope();
        for var in exiting_vars {
            if var.data_type.is_handle {
                let release = self.get_release_behavior(var.data_type.type_hash, span)?;
                self.emitter.emit_get_local(var.slot);
                self.emitter.emit_release(release);
            }
        }

        Ok(())
    }

    /// Get foreach behaviors for a collection type.
    fn get_foreach_behaviors(
        &self,
        collection_type: &DataType,
        var_count: usize,
        span: Span,
    ) -> Result<ForeachBehaviors> {
        let type_entry = self
            .ctx
            .get_type(collection_type.type_hash)
            .ok_or_else(|| CompilationError::Other {
                message: format!("type '{:?}' does not exist", collection_type.type_hash),
                span,
            })?;

        let class = type_entry
            .as_class()
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' is not a class and cannot be iterated",
                    type_entry.qualified_name()
                ),
                span,
            })?;

        // Look up opForBegin
        let for_begin = class
            .behaviors
            .get_operator(OperatorBehavior::OpForBegin)
            .and_then(|ops| ops.first().copied())
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' does not support foreach (missing opForBegin)",
                    class.name
                ),
                span,
            })?;

        // Get state type from opForBegin return type
        let for_begin_func =
            self.ctx
                .get_function(for_begin)
                .ok_or_else(|| CompilationError::Other {
                    message: "opForBegin function not found".to_string(),
                    span,
                })?;
        let state_type = for_begin_func.def.return_type;

        // Look up opForEnd
        let for_end = class
            .behaviors
            .get_operator(OperatorBehavior::OpForEnd)
            .and_then(|ops| ops.first().copied())
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' does not support foreach (missing opForEnd)",
                    class.name
                ),
                span,
            })?;

        // Look up opForNext
        let for_next = class
            .behaviors
            .get_operator(OperatorBehavior::OpForNext)
            .and_then(|ops| ops.first().copied())
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' does not support foreach (missing opForNext)",
                    class.name
                ),
                span,
            })?;

        // Look up opForValue(s)
        let mut for_value = Vec::new();
        let mut value_types = Vec::new();

        if var_count == 1 {
            // Single value: use opForValue
            let func_hash = class
                .behaviors
                .get_operator(OperatorBehavior::OpForValue)
                .and_then(|ops| ops.first().copied())
                .ok_or_else(|| CompilationError::Other {
                    message: format!(
                        "type '{}' does not support foreach (missing opForValue)",
                        class.name
                    ),
                    span,
                })?;
            for_value.push(func_hash);

            let func = self
                .ctx
                .get_function(func_hash)
                .ok_or_else(|| CompilationError::Other {
                    message: "opForValue function not found".to_string(),
                    span,
                })?;
            value_types.push(func.def.return_type);
        } else {
            // Multiple values: use opForValue0, opForValue1, etc. (dynamic limit)
            if var_count > 256 {
                return Err(CompilationError::Other {
                    message: "foreach supports at most 256 iteration variables".to_string(),
                    span,
                });
            }

            for i in 0..var_count {
                let op = OperatorBehavior::OpForValueN(i as u8);
                let func_hash = class
                    .behaviors
                    .get_operator(op)
                    .and_then(|ops| ops.first().copied())
                    .ok_or_else(|| CompilationError::Other {
                        message: format!(
                            "type '{}' does not support foreach with {} variables (missing opForValue{})",
                            class.name, var_count, i
                        ),
                        span,
                    })?;
                for_value.push(func_hash);

                let func =
                    self.ctx
                        .get_function(func_hash)
                        .ok_or_else(|| CompilationError::Other {
                            message: format!("opForValue{} function not found", i),
                            span,
                        })?;
                value_types.push(func.def.return_type);
            }
        }

        Ok(ForeachBehaviors {
            for_begin,
            for_end,
            for_next,
            for_value,
            state_type,
            value_types,
        })
    }
}

/// Resolved foreach behaviors for a collection type.
struct ForeachBehaviors {
    /// opForBegin function hash
    for_begin: TypeHash,
    /// opForEnd function hash
    for_end: TypeHash,
    /// opForNext function hash
    for_next: TypeHash,
    /// opForValue function hash(es) - one per iteration variable
    for_value: Vec<TypeHash>,
    /// Type of the iteration state
    state_type: DataType,
    /// Types of the iteration values
    value_types: Vec<DataType>,
}

#[cfg(test)]
mod tests {
    // Note: Full tests require a registry with types that implement opFor* behaviors.
    // For now, we test the error case when foreach is used on a non-iterable type.

    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::Span;
    use angelscript_parser::ast::{
        Block, Expr, Ident, LiteralExpr, LiteralKind, PrimitiveType, Stmt, TypeExpr,
    };
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn foreach_on_primitive_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // foreach (int x : 42) {} - should fail (int is not iterable)
        let vars = arena.alloc_slice_copy(&[angelscript_parser::ast::ForeachVar {
            ty: TypeExpr::primitive(PrimitiveType::Int, Span::default()),
            name: Ident::new("x", Span::default()),
            span: Span::default(),
        }]);

        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let body = arena.alloc(Stmt::Block(Block {
            stmts: &[],
            span: Span::default(),
        }));

        let foreach = ForeachStmt {
            vars,
            expr,
            body,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_foreach(&foreach);

        // Should fail because int is not a class type
        assert!(result.is_err());
    }
}
