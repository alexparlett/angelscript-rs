//! Switch statement compilation.
//!
//! Handles switch statements with case values and default case.

use angelscript_core::{CompilationError, DataType, OperatorBehavior, Span, primitives};
use angelscript_parser::ast::{Expr, SwitchStmt, UnaryOp};

use crate::bytecode::OpCode;
use crate::emit::JumpLabel;

use super::{Result, StmtCompiler};

impl<'a, 'ctx, 'pool> StmtCompiler<'a, 'ctx, 'pool> {
    /// Compile a switch statement.
    ///
    /// Switch statements compare an expression against case values and execute
    /// the matching case body. Fall-through is supported (no implicit break).
    ///
    /// Bytecode layout:
    /// ```text
    /// [switch expression]
    /// Dup; [case1 value]; Equal; JumpIfTrue -> case1_body; Pop
    /// Dup; [case2 value]; Equal; JumpIfTrue -> case2_body; Pop
    /// ...
    /// Pop (switch expr)
    /// Jump -> default_body (or end if no default)
    ///
    /// case1_body:
    /// Pop (true comparison result)
    /// [statements]
    /// (fall-through or break)
    ///
    /// case2_body:
    /// Pop
    /// [statements]
    /// ...
    ///
    /// default_body:
    /// [statements]
    ///
    /// end:
    /// ```
    pub fn compile_switch<'ast>(&mut self, switch: &SwitchStmt<'ast>) -> Result<()> {
        let span = switch.span;

        // Compile switch expression
        let switch_info = {
            let mut expr_compiler = self.expr_compiler();
            expr_compiler.infer(switch.expr)?
        };

        // Validate switch type - must be primitive or have opEquals
        let equals_method = self.validate_switch_type(&switch_info.data_type, span)?;

        // Enter switch context (for break handling)
        self.emitter.enter_switch();

        // Collect case jumps: (jump_label, case_index)
        let mut case_jumps: Vec<(JumpLabel, usize)> = Vec::new();
        let mut default_index: Option<usize> = None;

        // Find default case and emit comparison jumps
        for (i, case) in switch.cases.iter().enumerate() {
            if case.is_default() {
                if default_index.is_some() {
                    return Err(CompilationError::Other {
                        message: "multiple default cases in switch".to_string(),
                        span: case.span,
                    });
                }
                default_index = Some(i);
            } else {
                // Emit comparison for each case value
                for value in case.values.iter() {
                    // Validate that case value is a compile-time constant
                    self.validate_case_constant(value, value.span())?;

                    // Dup switch value for comparison
                    self.emitter.emit_dup();

                    // Compile case value (emits bytecode and verifies type compatibility)
                    let mut expr_compiler = self.expr_compiler();
                    expr_compiler.check(value, &switch_info.data_type)?;

                    // Emit equality check
                    self.emit_equality(&switch_info.data_type, equals_method)?;

                    // Jump to case body if equal
                    let jump = self.emitter.emit_jump(OpCode::JumpIfTrue);
                    case_jumps.push((jump, i));

                    // Pop comparison result (false path)
                    self.emitter.emit_pop();
                }
            }
        }

        // Pop switch value (after all comparisons)
        self.emitter.emit_pop();

        // Jump to default or end
        let default_jump = if default_index.is_some() {
            Some(self.emitter.emit_jump(OpCode::Jump))
        } else {
            // No default - jump to end (will be patched by exit_switch)
            self.emitter.emit_break().ok();
            None
        };

        // Emit case bodies
        for (i, case) in switch.cases.iter().enumerate() {
            // Patch jumps that target this case
            for (jump, target_i) in &case_jumps {
                if *target_i == i {
                    self.emitter.patch_jump(*jump);
                    // Pop comparison result (true path)
                    self.emitter.emit_pop();
                }
            }

            // Patch default jump if this is the default case
            if Some(i) == default_index
                && let Some(jump) = default_jump
            {
                self.emitter.patch_jump(jump);
            }

            // Compile case statements
            for stmt in case.stmts.iter() {
                self.compile(stmt)?;
            }

            // Fall through to next case (no implicit break)
        }

        // Exit switch context (patches break jumps)
        self.emitter.exit_switch();

        Ok(())
    }

    /// Validate that the switch expression type is valid.
    ///
    /// Returns the opEquals method hash if the type is an object type.
    fn validate_switch_type(
        &self,
        data_type: &DataType,
        span: angelscript_core::Span,
    ) -> Result<Option<angelscript_core::TypeHash>> {
        // Primitives are always valid
        if data_type.is_primitive() {
            return Ok(None);
        }

        // Objects need opEquals
        let type_entry =
            self.ctx
                .get_type(data_type.type_hash)
                .ok_or_else(|| CompilationError::Other {
                    message: format!("unknown type for switch: {:?}", data_type.type_hash),
                    span,
                })?;

        let class = type_entry
            .as_class()
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' cannot be used in switch",
                    type_entry.qualified_name()
                ),
                span,
            })?;

        // Look up opEquals
        let equals = class
            .behaviors
            .get_operator(OperatorBehavior::OpEquals)
            .and_then(|ops| ops.first().copied())
            .ok_or_else(|| CompilationError::Other {
                message: format!(
                    "type '{}' does not support switch (missing opEquals)",
                    class.name
                ),
                span,
            })?;

        Ok(Some(equals))
    }

    /// Validate that a case value is a compile-time constant.
    ///
    /// Only literals and const variables are allowed in switch cases.
    /// This prevents side effects and enables potential future optimizations
    /// like jump tables.
    fn validate_case_constant(&self, expr: &Expr<'_>, span: Span) -> Result<()> {
        match expr {
            // Literals are always constant
            Expr::Literal(_) => Ok(()),

            // Identifiers are allowed if they refer to const variables
            Expr::Ident(ident) => {
                let name = ident.ident.name;

                // Check if it's a local const variable
                if let Some(var) = self.ctx.get_local(name) {
                    if var.is_const {
                        return Ok(());
                    }
                    return Err(CompilationError::Other {
                        message: format!(
                            "case value '{}' must be const; non-const variables are not allowed",
                            name
                        ),
                        span,
                    });
                }

                // Check if it's a global const variable
                if let Some(hash) = self.ctx.resolve_global(name)
                    && let Some(global) = self.ctx.get_global_entry(hash)
                {
                    if global.is_const {
                        return Ok(());
                    }
                    return Err(CompilationError::Other {
                        message: format!(
                            "case value '{}' must be const; non-const globals are not allowed",
                            name
                        ),
                        span,
                    });
                }

                // Could be an enum value or other constant - allow it
                // The type checker will catch invalid references
                Ok(())
            }

            // Parenthesized expressions - check inner
            Expr::Paren(paren) => self.validate_case_constant(paren.expr, paren.span),

            // Allow unary operators on constants (for expressions like -1, +1, ~0xFF, -CONST_VAL)
            Expr::Unary(unary)
                if matches!(unary.op, UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitwiseNot) =>
            {
                // Recursively validate the operand (can be literal or const variable)
                self.validate_case_constant(unary.operand, span)
            }

            // Everything else is not allowed
            _ => Err(CompilationError::Other {
                message: "case values must be literals or const variables".to_string(),
                span,
            }),
        }
    }

    /// Emit equality comparison.
    fn emit_equality(
        &mut self,
        data_type: &DataType,
        equals_method: Option<angelscript_core::TypeHash>,
    ) -> Result<()> {
        if data_type.is_primitive() {
            // Use primitive equality
            match data_type.type_hash {
                h if h == primitives::INT8 || h == primitives::INT16 || h == primitives::INT32 => {
                    self.emitter.emit(OpCode::EqI32);
                }
                h if h == primitives::INT64 => {
                    self.emitter.emit(OpCode::EqI64);
                }
                h if h == primitives::UINT8
                    || h == primitives::UINT16
                    || h == primitives::UINT32 =>
                {
                    self.emitter.emit(OpCode::EqI32);
                }
                h if h == primitives::UINT64 => {
                    self.emitter.emit(OpCode::EqI64);
                }
                h if h == primitives::FLOAT => {
                    self.emitter.emit(OpCode::EqF32);
                }
                h if h == primitives::DOUBLE => {
                    self.emitter.emit(OpCode::EqF64);
                }
                h if h == primitives::BOOL => {
                    self.emitter.emit(OpCode::EqBool);
                }
                _ => {
                    // Fall back to opEquals for other types
                    if let Some(method) = equals_method {
                        self.emitter.emit_call_method(method, 1);
                    } else {
                        return Err(CompilationError::Other {
                            message: "cannot compare this type".to_string(),
                            span: angelscript_core::Span::default(),
                        });
                    }
                }
            }
        } else {
            // Call opEquals method
            if let Some(method) = equals_method {
                self.emitter.emit_call_method(method, 1);
            } else {
                return Err(CompilationError::Other {
                    message: "type does not support equality comparison".to_string(),
                    span: angelscript_core::Span::default(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ConstantPool;
    use crate::context::CompilationContext;
    use crate::emit::BytecodeEmitter;
    use angelscript_core::Span;
    use angelscript_parser::ast::{BreakStmt, Expr, LiteralExpr, LiteralKind, Stmt, SwitchCase};
    use angelscript_registry::SymbolRegistry;
    use bumpalo::Bump;

    fn create_test_context() -> (SymbolRegistry, ConstantPool) {
        (SymbolRegistry::with_primitives(), ConstantPool::new())
    }

    #[test]
    fn switch_empty() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) {}
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let switch = SwitchStmt {
            expr,
            cases: &[],
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        // Should have switch value + pop + break jump (patched to end)
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_single_case() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case 42: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let case_value: &Expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        // Should compile without errors
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_with_default() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { default: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let default_case = SwitchCase {
            values: &[], // Empty values = default
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[default_case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_multiple_defaults_error() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { default: break; default: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let default1 = SwitchCase {
            values: &[],
            stmts,
            span: Span::default(),
        };
        let default2 = SwitchCase {
            values: &[],
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[default1, default2]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_switch(&switch);

        assert!(result.is_err());
    }

    #[test]
    fn switch_case_and_default() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case 1: break; default: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let case_value: &Expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case1 = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let default_case = SwitchCase {
            values: &[],
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case1, default_case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_multiple_case_values() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case 1: case 2: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let case_value1: &Expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));
        let case_value2: &Expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(2),
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value1, case_value2]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_bool() {
        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (true) { case true: break; }
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));

        let case_value: &Expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Bool(true),
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_case_function_call_error() {
        use angelscript_parser::ast::{CallExpr, Ident, IdentExpr};

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case foo(): break; } - should fail (function call not allowed)
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        // Create a function call expression: foo()
        let callee = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("foo", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let case_value: &Expr = arena.alloc(Expr::Call(arena.alloc(CallExpr {
            callee,
            args: &[],
            span: Span::default(),
        })));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_switch(&switch);

        // Should fail because function calls are not allowed in case values
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{:?}", err).contains("literals or const variables"),
            "Error should mention literals or const variables"
        );
    }

    #[test]
    fn switch_case_non_const_var_error() {
        use angelscript_core::primitives;
        use angelscript_parser::ast::{Ident, IdentExpr};

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a non-const local variable
        ctx.declare_local(
            "x".to_string(),
            DataType::simple(primitives::INT32),
            false, // NOT const
            Span::default(),
        )
        .unwrap();
        ctx.mark_local_initialized("x");

        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case x: break; } - should fail (x is not const)
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let case_value: &Expr = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("x", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        let result = compiler.compile_switch(&switch);

        // Should fail because x is not const
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{:?}", err).contains("must be const"),
            "Error should mention 'must be const'"
        );
    }

    #[test]
    fn switch_case_const_var_allowed() {
        use angelscript_core::primitives;
        use angelscript_parser::ast::{Ident, IdentExpr};

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a const local variable
        ctx.declare_local(
            "CONST_VAL".to_string(),
            DataType::simple(primitives::INT32),
            true, // IS const
            Span::default(),
        )
        .unwrap();
        ctx.mark_local_initialized("CONST_VAL");

        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case CONST_VAL: break; } - should succeed
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        let case_value: &Expr = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("CONST_VAL", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        // Should succeed because CONST_VAL is const
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_case_negative_literal_allowed() {
        use angelscript_parser::ast::UnaryExpr;

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();
        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case -1: break; } - should succeed (negative literal)
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        // Create -1: unary minus on literal 1
        let operand = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(1),
            span: Span::default(),
        }));
        let case_value: &Expr = arena.alloc(Expr::Unary(arena.alloc(UnaryExpr {
            op: UnaryOp::Neg,
            operand,
            span: Span::default(),
        })));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        // Should succeed because -1 is allowed (unary minus on literal)
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }

    #[test]
    fn switch_case_negated_const_var_allowed() {
        use angelscript_core::primitives;
        use angelscript_parser::ast::{Ident, IdentExpr, UnaryExpr};

        let arena = Bump::new();
        let (registry, mut constants) = create_test_context();
        let mut ctx = CompilationContext::new(&registry);
        ctx.begin_function();

        // Declare a const local variable
        ctx.declare_local(
            "CONST_VAL".to_string(),
            DataType::simple(primitives::INT32),
            true, // IS const
            Span::default(),
        )
        .unwrap();
        ctx.mark_local_initialized("CONST_VAL");

        let mut emitter = BytecodeEmitter::new(&mut constants);

        // switch (42) { case -CONST_VAL: break; } - should succeed (negated const var)
        let expr = arena.alloc(Expr::Literal(LiteralExpr {
            kind: LiteralKind::Int(42),
            span: Span::default(),
        }));

        // Create -CONST_VAL: unary minus on const identifier
        let operand = arena.alloc(Expr::Ident(IdentExpr {
            scope: None,
            ident: Ident::new("CONST_VAL", Span::default()),
            type_args: &[],
            span: Span::default(),
        }));
        let case_value: &Expr = arena.alloc(Expr::Unary(arena.alloc(UnaryExpr {
            op: UnaryOp::Neg,
            operand,
            span: Span::default(),
        })));
        let values = arena.alloc_slice_copy(&[case_value]);

        let break_stmt = Stmt::Break(BreakStmt {
            span: Span::default(),
        });
        let stmts = arena.alloc_slice_copy(&[break_stmt]);

        let case = SwitchCase {
            values,
            stmts,
            span: Span::default(),
        };
        let cases = arena.alloc_slice_copy(&[case]);

        let switch = SwitchStmt {
            expr,
            cases,
            span: Span::default(),
        };

        let mut compiler = StmtCompiler::new(&mut ctx, &mut emitter, DataType::void(), None);
        // Should succeed because -CONST_VAL is a const expression
        compiler.compile_switch(&switch).unwrap();

        let chunk = emitter.finish();
        assert!(chunk.len() > 0);
    }
}
