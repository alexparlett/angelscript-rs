//! Statement compilation.
//!
//! This module contains all `visit_*` methods for compiling statements
//! and emitting bytecode.

use crate::ast::{
    expr::{Expr, LiteralKind},
    stmt::{
        Block, BreakStmt, ContinueStmt, DoWhileStmt, ExprStmt, ForInit, ForStmt, ForeachStmt,
        IfStmt, ReturnStmt, Stmt, SwitchStmt, TryCatchStmt, VarDeclStmt, WhileStmt,
    },
    types::{TypeBase, TypeSuffix},
};
use crate::codegen::Instruction;
use crate::semantic::{
    ConstEvaluator, ConstValue, OperatorBehavior,
    SemanticErrorKind, TypeDef, BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE,
    NULL_TYPE, VOID_TYPE, STRING_TYPE,
};
use crate::semantic::types::type_def::FunctionId;
use rustc_hash::FxHashSet;

use super::{FunctionCompiler, SwitchCategory, SwitchCaseKey};

impl<'ast> FunctionCompiler<'ast> {
    pub(super) fn visit_block(&mut self, block: &'ast Block<'ast>) {
        self.local_scope.enter_scope();

        for stmt in block.stmts {
            self.visit_stmt(stmt);
        }

        self.local_scope.exit_scope();
    }

    /// Visits a statement.
    pub(super) fn visit_stmt(&mut self, stmt: &'ast Stmt<'ast>) {
        match stmt {
            Stmt::Expr(expr_stmt) => self.visit_expr_stmt(expr_stmt),
            Stmt::VarDecl(var_decl) => self.visit_var_decl(var_decl),
            Stmt::Return(ret) => self.visit_return(ret),
            Stmt::Break(brk) => self.visit_break(brk),
            Stmt::Continue(cont) => self.visit_continue(cont),
            Stmt::Block(block) => self.visit_block(block),
            Stmt::If(if_stmt) => self.visit_if(if_stmt),
            Stmt::While(while_stmt) => self.visit_while(while_stmt),
            Stmt::DoWhile(do_while) => self.visit_do_while(do_while),
            Stmt::For(for_stmt) => self.visit_for(for_stmt),
            Stmt::Foreach(foreach) => self.visit_foreach(foreach),
            Stmt::Switch(switch) => self.visit_switch(switch),
            Stmt::TryCatch(try_catch) => self.visit_try_catch(try_catch),
        }
    }

    /// Visits an expression statement.
    pub(super) fn visit_expr_stmt(&mut self, expr_stmt: &ExprStmt<'ast>) {
        if let Some(expr) = expr_stmt.expr {
            let _ = self.check_expr(expr);
            // Expression result is discarded
            self.bytecode.emit(Instruction::Pop);
        }
    }

    /// Visits a variable declaration statement.
    pub(super) fn visit_var_decl(&mut self, var_decl: &VarDeclStmt<'ast>) {
        // Check if this is an auto type declaration
        let is_auto = matches!(var_decl.ty.base, TypeBase::Auto);

        // For non-auto types, resolve the type upfront
        let base_var_type = if is_auto {
            None
        } else {
            match self.resolve_type_expr(&var_decl.ty) {
                Some(ty) => {
                    // Void type cannot be used for variables
                    if ty.type_id == VOID_TYPE {
                        self.error(
                            SemanticErrorKind::VoidExpression,
                            var_decl.ty.span,
                            "cannot declare variable of type 'void'",
                        );
                        return;
                    }
                    Some(ty)
                }
                None => return, // Error already recorded
            }
        };

        for var in var_decl.vars {
            // Determine the variable type (either from declaration or inferred from initializer)
            let var_type = if is_auto {
                // Auto type requires an initializer
                let init = match var.init {
                    Some(init) => init,
                    None => {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            var.span,
                            "cannot use 'auto' without an initializer",
                        );
                        continue;
                    }
                };

                // Evaluate the initializer to infer the type
                let init_ctx = match self.check_expr(init) {
                    Some(ctx) => ctx,
                    None => continue, // Error already recorded
                };

                // Cannot infer void type
                if init_ctx.data_type.type_id == VOID_TYPE {
                    self.error(
                        SemanticErrorKind::VoidExpression,
                        var.span,
                        "cannot infer type from void expression",
                    );
                    continue;
                }

                // Build the inferred type, applying modifiers from the auto declaration
                let mut inferred_type = init_ctx.data_type.clone();

                // Apply const from "const auto"
                if var_decl.ty.is_const {
                    inferred_type.is_const = true;
                }

                // Apply handle from "auto@"
                for suffix in var_decl.ty.suffixes {
                    match suffix {
                        TypeSuffix::Handle { is_const } => {
                            // If the initializer isn't already a handle, make it one
                            if !inferred_type.is_handle {
                                inferred_type.is_handle = true;
                            }
                            // Apply const handle if specified
                            if *is_const {
                                inferred_type.is_const = true;
                            }
                        }
                        TypeSuffix::Array => {
                            // auto[] doesn't make sense - the array type should come from initializer
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                var_decl.ty.span,
                                "cannot use array suffix with 'auto'; type is inferred from initializer",
                            );
                            continue;
                        }
                    }
                }

                // Declare the variable with inferred type
                let offset = self.local_scope.declare_variable_auto(
                    var.name.name.to_string(),
                    inferred_type,
                    true,
                );

                // Store the initializer value
                self.bytecode.emit(Instruction::StoreLocal(offset));

                // Continue to next variable (we've already handled the initializer)
                continue;
            } else {
                base_var_type.clone().unwrap()
            };

            // Check if variable type is a funcdef (for function handle inference)
            let is_funcdef = matches!(
                self.registry.get_type(var_type.type_id),
                TypeDef::Funcdef { .. }
            );

            // Check initializer if present
            if let Some(init) = var.init {
                // Set expected funcdef type for function handle inference
                if is_funcdef && var_type.is_handle {
                    self.expected_funcdef_type = Some(var_type.type_id);
                }

                // Set expected init list type for empty init lists
                // If the target is an array<T>, the element type is T
                if let TypeDef::TemplateInstance { template, sub_types, .. } = self.registry.get_type(var_type.type_id)
                    && *template == self.registry.array_template && !sub_types.is_empty() {
                        self.expected_init_list_type = Some(sub_types[0].clone());
                    }

                let init_ctx = match self.check_expr(init) {
                    Some(ctx) => ctx,
                    None => {
                        self.expected_funcdef_type = None;
                        self.expected_init_list_type = None;
                        continue; // Error already recorded
                    }
                };

                // Clear expected types
                self.expected_funcdef_type = None;
                self.expected_init_list_type = None;

                // Check if initializer can be converted to variable type and emit conversion if needed
                if let Some(conversion) = init_ctx.data_type.can_convert_to(&var_type, self.registry) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            var.span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' (explicit cast required)",
                                self.type_name(&init_ctx.data_type),
                                self.type_name(&var_type)
                            ),
                        );
                    } else {
                        // Emit conversion instruction if needed
                        self.emit_conversion(&conversion);
                    }
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        var.span,
                        format!(
                            "cannot initialize variable of type '{}' with value of type '{}'",
                            self.type_name(&var_type),
                            self.type_name(&init_ctx.data_type)
                        ),
                    );
                }
            }

            // Declare the variable
            let offset = self.local_scope.declare_variable_auto(
                var.name.name.to_string(),
                var_type.clone(),
                true,
            );

            // Store the initializer value if present
            if var.init.is_some() {
                self.bytecode.emit(Instruction::StoreLocal(offset));
            }
        }
    }

    /// Visits a return statement.
    pub(super) fn visit_return(&mut self, ret: &ReturnStmt<'ast>) {
        if let Some(value) = ret.value {
            // Set expected funcdef type if return type is a funcdef
            // This allows lambda inference in return statements
            // Note: funcdef types are always handles, so we just check the type
            let type_def = self.registry.get_type(self.return_type.type_id);
            if matches!(type_def, TypeDef::Funcdef { .. }) {
                self.expected_funcdef_type = Some(self.return_type.type_id);
            }

            // Check return value type
            let value_ctx = match self.check_expr(value) {
                Some(ctx) => ctx,
                None => {
                    self.expected_funcdef_type = None;
                    return; // Error already recorded
                }
            };

            self.expected_funcdef_type = None;

            // Cannot return a void expression
            if value_ctx.data_type.type_id == VOID_TYPE {
                self.error(
                    SemanticErrorKind::VoidExpression,
                    ret.span,
                    "cannot return a void expression",
                );
                return;
            }

            // Check if value can be converted to return type and emit conversion if needed
            if let Some(conversion) = value_ctx.data_type.can_convert_to(&self.return_type, self.registry) {
                if !conversion.is_implicit {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        ret.span,
                        format!(
                            "cannot implicitly convert '{}' to '{}' (explicit cast required)",
                            self.type_name(&value_ctx.data_type),
                            self.type_name(&self.return_type)
                        ),
                    );
                } else {
                    // Emit conversion instruction if needed
                    self.emit_conversion(&conversion);
                }
            } else {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    ret.span,
                    format!(
                        "cannot return value of type '{}' from function with return type '{}'",
                        self.type_name(&value_ctx.data_type),
                        self.type_name(&self.return_type)
                    ),
                );
            }

            self.bytecode.emit(Instruction::Return);
        } else {
            // Void return
            if self.return_type.type_id != VOID_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    ret.span,
                    format!(
                        "cannot return void from function with return type '{}'",
                        self.type_name(&self.return_type)
                    ),
                );
            }

            self.bytecode.emit(Instruction::ReturnVoid);
        }
    }

    /// Visits a break statement.
    pub(super) fn visit_break(&mut self, brk: &BreakStmt) {
        if self.bytecode.emit_break().is_none() {
            self.error(
                SemanticErrorKind::BreakOutsideLoop,
                brk.span,
                "break statement must be inside a loop or switch",
            );
        }
    }

    /// Visits a continue statement.
    pub(super) fn visit_continue(&mut self, cont: &ContinueStmt) {
        if self.bytecode.emit_continue().is_none() {
            self.error(
                SemanticErrorKind::ContinueOutsideLoop,
                cont.span,
                "continue statement must be inside a loop",
            );
        }
    }

    /// Visits an if statement.
    pub(super) fn visit_if(&mut self, if_stmt: &'ast IfStmt<'ast>) {
        // Check condition
        if let Some(cond_ctx) = self.check_expr(if_stmt.condition)
            && cond_ctx.data_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    if_stmt.condition.span(),
                    format!(
                        "if condition must be bool, found '{}'",
                        self.type_name(&cond_ctx.data_type)
                    ),
                );
            }

        // Emit conditional jump
        let jump_to_else = self.bytecode.emit(Instruction::JumpIfFalse(0));

        // Compile then branch
        self.visit_stmt(if_stmt.then_stmt);

        if let Some(else_stmt) = if_stmt.else_stmt {
            // Jump over else branch
            let jump_to_end = self.bytecode.emit(Instruction::Jump(0));

            // Patch jump to else
            let else_pos = self.bytecode.current_position();
            self.bytecode.patch_jump(jump_to_else, else_pos);

            // Compile else branch
            self.visit_stmt(else_stmt);

            // Patch jump to end
            let end_pos = self.bytecode.current_position();
            self.bytecode.patch_jump(jump_to_end, end_pos);
        } else {
            // Patch jump to end
            let end_pos = self.bytecode.current_position();
            self.bytecode.patch_jump(jump_to_else, end_pos);
        }
    }

    /// Visits a while loop.
    pub(super) fn visit_while(&mut self, while_stmt: &'ast WhileStmt<'ast>) {
        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Check condition
        if let Some(cond_ctx) = self.check_expr(while_stmt.condition)
            && cond_ctx.data_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    while_stmt.condition.span(),
                    format!(
                        "while condition must be bool, found '{}'",
                        self.type_name(&cond_ctx.data_type)
                    ),
                );
            }

        // Jump out of loop if condition is false
        let jump_to_end = self.bytecode.emit(Instruction::JumpIfFalse(0));

        // Compile body
        self.visit_stmt(while_stmt.body);

        // Jump back to start
        let current_pos = self.bytecode.current_position();
        let offset = (loop_start as i32) - (current_pos as i32) - 1;
        self.bytecode.emit(Instruction::Jump(offset));

        // Patch jump to end
        let end_pos = self.bytecode.current_position();
        self.bytecode.patch_jump(jump_to_end, end_pos);
        self.bytecode.exit_loop(end_pos);
    }

    /// Visits a do-while loop.
    pub(super) fn visit_do_while(&mut self, do_while: &'ast DoWhileStmt<'ast>) {
        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Compile body
        self.visit_stmt(do_while.body);

        // Check condition
        if let Some(cond_ctx) = self.check_expr(do_while.condition)
            && cond_ctx.data_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    do_while.condition.span(),
                    format!(
                        "do-while condition must be bool, found '{}'",
                        self.type_name(&cond_ctx.data_type)
                    ),
                );
            }

        // Jump back to start if condition is true
        let current_pos = self.bytecode.current_position();
        let offset = (loop_start as i32) - (current_pos as i32) - 1;
        self.bytecode.emit(Instruction::JumpIfTrue(offset));

        let end_pos = self.bytecode.current_position();
        self.bytecode.exit_loop(end_pos);
    }

    /// Visits a for loop.
    pub(super) fn visit_for(&mut self, for_stmt: &'ast ForStmt<'ast>) {
        // Enter scope for loop (init variables live in loop scope)
        self.local_scope.enter_scope();

        // Compile initializer
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::VarDecl(var_decl) => self.visit_var_decl(var_decl),
                ForInit::Expr(expr) => {
                    let _ = self.check_expr(expr);
                    self.bytecode.emit(Instruction::Pop);
                }
            }
        }

        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Check condition
        let jump_to_end = if let Some(condition) = for_stmt.condition {
            if let Some(cond_ctx) = self.check_expr(condition)
                && cond_ctx.data_type.type_id != BOOL_TYPE {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        condition.span(),
                        format!(
                            "for condition must be bool, found '{}'",
                            self.type_name(&cond_ctx.data_type)
                        ),
                    );
                }
            Some(self.bytecode.emit(Instruction::JumpIfFalse(0)))
        } else {
            None
        };

        // Compile body
        self.visit_stmt(for_stmt.body);

        // Compile update expressions
        for update_expr in for_stmt.update {
            let _ = self.check_expr(update_expr);
            self.bytecode.emit(Instruction::Pop);
        }

        // Jump back to start
        let current_pos = self.bytecode.current_position();
        let offset = (loop_start as i32) - (current_pos as i32) - 1;
        self.bytecode.emit(Instruction::Jump(offset));

        // Patch jump to end
        let end_pos = self.bytecode.current_position();
        if let Some(jump_pos) = jump_to_end {
            self.bytecode.patch_jump(jump_pos, end_pos);
        }
        self.bytecode.exit_loop(end_pos);

        // Exit loop scope
        self.local_scope.exit_scope();
    }

    /// Visits a foreach loop.
    pub(super) fn visit_foreach(&mut self, foreach: &'ast ForeachStmt<'ast>) {
        // Type check the container expression
        let container_ctx = match self.check_expr(foreach.expr) {
            Some(ctx) => ctx,
            None => return, // Error already recorded
        };

        let container_type_id = container_ctx.data_type.type_id;

        // Check for required foreach operators
        let begin_func_id = match self.registry.find_operator_method(container_type_id, OperatorBehavior::OpForBegin) {
            Some(func_id) => func_id,
            None => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    foreach.expr.span(),
                    format!(
                        "type '{}' does not support foreach iteration (missing opForBegin)",
                        self.type_name(&container_ctx.data_type)
                    ),
                );
                return;
            }
        };

        let end_func_id = match self.registry.find_operator_method(container_type_id, OperatorBehavior::OpForEnd) {
            Some(func_id) => func_id,
            None => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    foreach.expr.span(),
                    format!(
                        "type '{}' does not support foreach iteration (missing opForEnd)",
                        self.type_name(&container_ctx.data_type)
                    ),
                );
                return;
            }
        };

        let next_func_id = match self.registry.find_operator_method(container_type_id, OperatorBehavior::OpForNext) {
            Some(func_id) => func_id,
            None => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    foreach.expr.span(),
                    format!(
                        "type '{}' does not support foreach iteration (missing opForNext)",
                        self.type_name(&container_ctx.data_type)
                    ),
                );
                return;
            }
        };

        // Validate operator signatures
        let begin_func = self.registry.get_function(begin_func_id);
        if !begin_func.params.is_empty() {
            self.error(
                SemanticErrorKind::InvalidOperation,
                foreach.expr.span(),
                format!("opForBegin must have no parameters, found {}", begin_func.params.len()),
            );
            return;
        }

        let end_func = self.registry.get_function(end_func_id);
        if end_func.params.len() != 1 || end_func.return_type.type_id != self.registry.bool_type {
            self.error(
                SemanticErrorKind::InvalidOperation,
                foreach.expr.span(),
                "opForEnd must have signature (iterator) -> bool".to_string(),
            );
            return;
        }

        let next_func = self.registry.get_function(next_func_id);
        if next_func.params.len() != 1 {
            self.error(
                SemanticErrorKind::InvalidOperation,
                foreach.expr.span(),
                "opForNext must have signature (iterator) -> iterator".to_string(),
            );
            return;
        }

        // Determine value operators based on number of variables
        let num_vars = foreach.vars.len();
        let value_func_ids: Vec<FunctionId> = if num_vars == 1 {
            // Try opForValue first, fall back to opForValue0
            if let Some(func_id) = self.registry.find_operator_method(container_type_id, OperatorBehavior::OpForValue) {
                vec![func_id]
            } else if let Some(func_id) = self.registry.find_operator_method(container_type_id, OperatorBehavior::OpForValue0) {
                vec![func_id]
            } else {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    foreach.expr.span(),
                    "foreach requires opForValue or opForValue0 operator".to_string(),
                );
                return;
            }
        } else {
            // Multiple variables: need opForValue0, opForValue1, etc.
            let mut operators = Vec::new();
            for i in 0..num_vars {
                let op_behavior = match i {
                    0 => OperatorBehavior::OpForValue0,
                    1 => OperatorBehavior::OpForValue1,
                    2 => OperatorBehavior::OpForValue2,
                    3 => OperatorBehavior::OpForValue3,
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            foreach.span,
                            format!("foreach supports at most 4 variables, found {}", num_vars),
                        );
                        return;
                    }
                };

                if let Some(func_id) = self.registry.find_operator_method(container_type_id, op_behavior) {
                    operators.push(func_id);
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        foreach.expr.span(),
                        format!("foreach requires opForValue{} operator", i),
                    );
                    return;
                }
            }
            operators
        };

        // Enter new scope for loop variables
        self.local_scope.enter_scope();

        // Declare and type-check loop variables
        for (i, var) in foreach.vars.iter().enumerate() {
            let value_func = self.registry.get_function(value_func_ids[i]);

            if value_func.params.len() != 1 {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    var.span,
                    format!("opForValue{} must have exactly 1 parameter (iterator)", i),
                );
                continue;
            }

            let element_type = value_func.return_type.clone();

            // Resolve the variable's type
            if let Some(var_type) = self.resolve_type_expr(&var.ty) {
                // Check if element type can be converted to variable type
                if let Some(_conversion) = element_type.can_convert_to(&var_type, self.registry) {
                    // Type is compatible
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        var.span,
                        format!(
                            "foreach variable type '{}' is not compatible with element type '{}'",
                            self.type_name(&var_type),
                            self.type_name(&element_type)
                        ),
                    );
                }

                // Declare the loop variable
                self.local_scope
                    .declare_variable_auto(var.name.name.to_string(), var_type, false);
            }
        }

        // Emit foreach loop bytecode:
        //   container_var = <container expression>
        //   it = container_var.opForBegin()
        // loop_start:
        //   if container_var.opForEnd(it): goto loop_end
        //   var0 = container_var.opForValue0(it)
        //   var1 = container_var.opForValue1(it)  // if multiple vars
        //   ... body ...
        //   it = container_var.opForNext(it)
        //   goto loop_start
        // loop_end:

        // Container is already on stack from check_expr above
        // Store container in a temporary local variable to avoid re-evaluating
        let container_offset = self.local_scope.declare_variable_auto(
            format!("$container_{}_{}", foreach.span.line, foreach.span.col),
            container_ctx.data_type.clone(),
            false,
        );
        self.bytecode.emit(Instruction::StoreLocal(container_offset));

        // Call container.opForBegin() to get initial iterator
        // Stack: [] -> [iterator]
        self.bytecode.emit(Instruction::LoadLocal(container_offset));
        self.bytecode.emit(Instruction::Call(begin_func_id.as_u32()));

        // Store iterator in a local variable
        let iterator_type = begin_func.return_type.clone();
        let iterator_offset = self.local_scope.declare_variable_auto(
            format!("$it_{}_{}", foreach.span.line, foreach.span.col),
            iterator_type,
            false,
        );
        self.bytecode.emit(Instruction::StoreLocal(iterator_offset));

        // Loop start: check if iteration is complete
        let loop_start = self.bytecode.current_position();

        // Call container.opForEnd(iterator)
        // Stack: [] -> [bool]
        self.bytecode.emit(Instruction::LoadLocal(container_offset));
        self.bytecode.emit(Instruction::LoadLocal(iterator_offset));
        self.bytecode.emit(Instruction::Call(end_func_id.as_u32()));

        // If true (iteration done), jump to loop_end
        let end_jump_pos = self.bytecode.emit(Instruction::JumpIfTrue(0)); // Placeholder

        self.bytecode.enter_loop(loop_start);

        // Load values into loop variables
        for (i, var) in foreach.vars.iter().enumerate() {
            let value_func_id = value_func_ids[i];
            let value_func = self.registry.get_function(value_func_id);

            // Call container.opForValue#(iterator)
            // Stack: [] -> [value]
            self.bytecode.emit(Instruction::LoadLocal(container_offset));
            self.bytecode.emit(Instruction::LoadLocal(iterator_offset));
            self.bytecode.emit(Instruction::Call(value_func_id.as_u32()));

            // Apply conversion if needed
            if let Some(var_local) = self.local_scope.lookup(var.name.name) {
                let element_type = value_func.return_type.clone();
                let var_offset = var_local.stack_offset; // Extract offset before mutable borrow
                let var_type = var_local.data_type.clone();

                if let Some(conversion) = element_type.can_convert_to(&var_type, self.registry)
                    && conversion.is_implicit {
                        self.emit_conversion(&conversion);
                    }

                // Store value in loop variable
                self.bytecode.emit(Instruction::StoreLocal(var_offset));
            }
        }

        // Compile body
        self.visit_stmt(foreach.body);

        // Advance iterator: it = container.opForNext(it)
        // Stack: [] -> [new_iterator]
        self.bytecode.emit(Instruction::LoadLocal(container_offset));
        self.bytecode.emit(Instruction::LoadLocal(iterator_offset));
        self.bytecode.emit(Instruction::Call(next_func_id.as_u32()));
        self.bytecode.emit(Instruction::StoreLocal(iterator_offset));

        // Jump back to loop start
        let current_pos = self.bytecode.current_position();
        let offset = (loop_start as i32) - (current_pos as i32) - 1;
        self.bytecode.emit(Instruction::Jump(offset));

        // Patch the end jump
        let end_pos = self.bytecode.current_position();
        self.bytecode.patch_jump(end_jump_pos, end_pos);

        // Exit loop
        self.bytecode.exit_loop(end_pos);

        // Exit scope (cleans up container, iterator, and loop variables)
        self.local_scope.exit_scope();
    }

    /// Visits a switch statement.
    ///
    /// Bytecode generation strategy:
    /// 1. Evaluate switch expression and store in temp variable
    /// 2. Emit dispatch table: for each case value, compare and jump if match
    /// 3. Jump to default case (or end if no default)
    /// 4. Emit case bodies with fallthrough semantics
    /// 5. Break statements jump to switch end
    ///
    /// Supports: integers, enums, bool, float, double, string, and handle types.
    /// Handle types support type pattern matching (case ClassName:) and null checks.
    pub(super) fn visit_switch(&mut self, switch: &'ast SwitchStmt<'ast>) {
        // Type check the switch expression
        let switch_ctx = match self.check_expr(switch.expr) {
            Some(ctx) => ctx,
            None => return, // Error already recorded
        };

        // Determine switch category
        let switch_category = match self.get_switch_category(&switch_ctx.data_type) {
            Some(cat) => cat,
            None => {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    switch.expr.span(),
                    format!(
                        "switch expression must be integer, enum, bool, float, string, or handle type, found '{}'",
                        self.type_name(&switch_ctx.data_type)
                    ),
                );
                return;
            }
        };

        // Enter a new scope for the switch temp variable
        self.local_scope.enter_scope();

        // Store switch expression value in a temporary variable
        // The expression value is already on the stack from check_expr
        let switch_offset = self.local_scope.declare_variable_auto(
            format!("$switch_{}_{}", switch.span.line, switch.span.col),
            switch_ctx.data_type.clone(),
            false,
        );
        self.bytecode.emit(Instruction::StoreLocal(switch_offset));

        // Track case values to detect duplicates (now supports all types)
        let mut case_values: FxHashSet<SwitchCaseKey> = FxHashSet::default();
        let mut default_case_index: Option<usize> = None;

        // First pass: find default case and check for duplicate case values
        for (case_idx, case) in switch.cases.iter().enumerate() {
            if case.is_default() {
                if default_case_index.is_some() {
                    self.error(
                        SemanticErrorKind::DuplicateDeclaration,
                        case.span,
                        "switch statement can only have one default case".to_string(),
                    );
                }
                default_case_index = Some(case_idx);
            } else {
                for value_expr in case.values {
                    // Check for type pattern (only valid for Handle category)
                    if switch_category == SwitchCategory::Handle
                        && let Some(type_id) = self.try_resolve_type_pattern(value_expr) {
                            let key = SwitchCaseKey::Type(type_id);
                            if !case_values.insert(key) {
                                let type_name = self.registry.get_type(type_id).name();
                                self.error(
                                    SemanticErrorKind::DuplicateDeclaration,
                                    value_expr.span(),
                                    format!("duplicate case type: {}", type_name),
                                );
                            }
                            continue;
                        }

                    // Check for null literal
                    if let Expr::Literal(lit) = value_expr
                        && matches!(lit.kind, LiteralKind::Null) {
                            if switch_category != SwitchCategory::Handle {
                                self.error(
                                    SemanticErrorKind::TypeMismatch,
                                    value_expr.span(),
                                    "case null is only valid for handle types".to_string(),
                                );
                            } else if !case_values.insert(SwitchCaseKey::Null) {
                                self.error(
                                    SemanticErrorKind::DuplicateDeclaration,
                                    value_expr.span(),
                                    "duplicate case null".to_string(),
                                );
                            }
                            continue;
                        }

                    // Evaluate constant value for duplicate detection
                    let evaluator = ConstEvaluator::new(self.registry);
                    if let Some(const_value) = evaluator.eval(value_expr) {
                        let key = match &const_value {
                            ConstValue::Int(i) => SwitchCaseKey::Int(*i),
                            ConstValue::UInt(u) => SwitchCaseKey::Int(*u as i64),
                            ConstValue::Float(f) => SwitchCaseKey::Float(f.to_bits()),
                            ConstValue::Bool(b) => SwitchCaseKey::Bool(*b),
                            ConstValue::String(s) => SwitchCaseKey::String(s.clone()),
                        };

                        if !case_values.insert(key) {
                            let display = match &const_value {
                                ConstValue::Int(i) => format!("{}", i),
                                ConstValue::UInt(u) => format!("{}", u),
                                ConstValue::Float(f) => format!("{}", f),
                                ConstValue::Bool(b) => format!("{}", b),
                                ConstValue::String(s) => format!("\"{}\"", s),
                            };
                            self.error(
                                SemanticErrorKind::DuplicateDeclaration,
                                value_expr.span(),
                                format!("duplicate case value: {}", display),
                            );
                        }
                    }
                }
            }
        }

        // Enter switch context to allow break statements
        self.bytecode.enter_switch();

        // Collect jump positions for each case (one per case, not per case value)
        // Each entry is (case_index, jump_position) for patching later
        let mut case_jumps: Vec<(usize, usize)> = Vec::new();

        // Emit dispatch table: compare switch value against each case value
        for (case_idx, case) in switch.cases.iter().enumerate() {
            if !case.is_default() {
                // For each case value (handles case 1: case 2: ... syntax)
                for value_expr in case.values {
                    // Check for type pattern (only valid for Handle category)
                    if switch_category == SwitchCategory::Handle
                        && let Some(type_id) = self.try_resolve_type_pattern(value_expr) {
                            // Emit: LoadLocal(switch_offset), IsInstanceOf(type_id), JumpIfTrue
                            self.bytecode.emit(Instruction::LoadLocal(switch_offset));
                            self.bytecode.emit(Instruction::IsInstanceOf(type_id));
                            let jump_pos = self.bytecode.emit(Instruction::JumpIfTrue(0));
                            case_jumps.push((case_idx, jump_pos));
                            continue;
                        }

                    // Check for null literal
                    if let Expr::Literal(lit) = value_expr
                        && matches!(lit.kind, LiteralKind::Null) {
                            // Emit: LoadLocal(switch_offset), PushNull, Equal, JumpIfTrue
                            self.bytecode.emit(Instruction::LoadLocal(switch_offset));
                            self.bytecode.emit(Instruction::PushNull);
                            self.bytecode.emit(Instruction::Equal);
                            let jump_pos = self.bytecode.emit(Instruction::JumpIfTrue(0));
                            case_jumps.push((case_idx, jump_pos));
                            continue;
                        }

                    // Load switch value
                    self.bytecode.emit(Instruction::LoadLocal(switch_offset));

                    // Emit case value expression and type check
                    if let Some(value_ctx) = self.check_expr(value_expr) {
                        // Check type compatibility based on switch category
                        let types_compatible = match switch_category {
                            SwitchCategory::Integer => {
                                value_ctx.data_type.type_id == switch_ctx.data_type.type_id
                                    || (self.is_integer(&value_ctx.data_type) && self.is_integer(&switch_ctx.data_type))
                            }
                            SwitchCategory::Bool => value_ctx.data_type.type_id == BOOL_TYPE,
                            SwitchCategory::Float => {
                                value_ctx.data_type.type_id == FLOAT_TYPE
                                    || value_ctx.data_type.type_id == DOUBLE_TYPE
                                    || self.is_integer(&value_ctx.data_type)
                            }
                            SwitchCategory::String => value_ctx.data_type.type_id == STRING_TYPE,
                            SwitchCategory::Handle => {
                                value_ctx.data_type.type_id == NULL_TYPE
                                    || (value_ctx.data_type.is_handle
                                        && value_ctx.data_type.type_id == switch_ctx.data_type.type_id)
                            }
                        };

                        if !types_compatible {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                value_expr.span(),
                                format!(
                                    "case value type '{}' does not match switch type '{}'",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(&switch_ctx.data_type)
                                ),
                            );
                        }
                    }

                    // Emit comparison based on switch category
                    match switch_category {
                        SwitchCategory::String => {
                            // String comparison via opEquals
                            if let Some(func_id) = self.registry.find_operator_method(
                                switch_ctx.data_type.type_id,
                                OperatorBehavior::OpEquals,
                            ) {
                                self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                            } else {
                                // Fallback to primitive Equal (shouldn't happen for strings)
                                self.bytecode.emit(Instruction::Equal);
                            }
                        }
                        _ => {
                            // Primitive equality for int, bool, float, handle identity
                            self.bytecode.emit(Instruction::Equal);
                        }
                    }

                    // Jump if equal (placeholder offset, will be patched)
                    let jump_pos = self.bytecode.emit(Instruction::JumpIfTrue(0));
                    case_jumps.push((case_idx, jump_pos));
                }
            }
        }

        // Jump to default case if exists, otherwise jump to end
        let default_jump_pos = self.bytecode.emit(Instruction::Jump(0));

        // Track case body positions for patching jumps
        let mut case_body_positions: Vec<usize> = Vec::with_capacity(switch.cases.len());

        // Emit case bodies (in order, with fallthrough)
        for case in switch.cases {
            // Record position of this case body
            let body_pos = self.bytecode.current_position();
            case_body_positions.push(body_pos);

            // Compile case statements
            for stmt in case.stmts {
                self.visit_stmt(stmt);
            }
            // Fallthrough: no jump at end of case (unless break was used)
        }

        // Switch end position
        let switch_end = self.bytecode.current_position();

        // Patch all case value jumps to their case body positions
        for (case_idx, jump_pos) in case_jumps {
            let target = case_body_positions[case_idx];
            self.bytecode.patch_jump(jump_pos, target);
        }

        // Patch default jump
        if let Some(default_idx) = default_case_index {
            let target = case_body_positions[default_idx];
            self.bytecode.patch_jump(default_jump_pos, target);
        } else {
            // No default case, jump to switch end
            self.bytecode.patch_jump(default_jump_pos, switch_end);
        }

        // Exit switch context and patch all break statements to switch end
        self.bytecode.exit_switch(switch_end);

        // Exit the switch scope
        self.local_scope.exit_scope();
    }

    /// Visits a try-catch statement.
    pub(super) fn visit_try_catch(&mut self, try_catch: &'ast TryCatchStmt<'ast>) {
        // Emit try block start (marks exception handler boundary)
        let _try_start = self.bytecode.current_position();
        self.bytecode.emit(Instruction::TryStart);

        // Compile try block
        for stmt in try_catch.try_block.stmts {
            self.visit_stmt(stmt);
        }

        // Emit jump over catch block (if no exception)
        let jump_over_catch = self.bytecode.emit(Instruction::Jump(0)); // Placeholder offset

        // Mark try block end and catch block start
        let _catch_start = self.bytecode.current_position();
        self.bytecode.emit(Instruction::TryEnd);
        self.bytecode.emit(Instruction::CatchStart);

        // Compile catch block
        for stmt in try_catch.catch_block.stmts {
            self.visit_stmt(stmt);
        }

        // Emit catch block end
        self.bytecode.emit(Instruction::CatchEnd);

        // Patch jump over catch block
        let end_pos = self.bytecode.current_position();
        self.bytecode.patch_jump(jump_over_catch, end_pos);
    }
}
