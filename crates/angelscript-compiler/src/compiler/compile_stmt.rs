//! Statement compilation — compiles AST statements to bytecode.

use angelscript_core::opcode::OpCode;
use angelscript_core::PrimitiveType;
use angelscript_parser::ast::{self, StmtKind, VarInit};

use super::{Compiler, LoopContext, PrimCategory};

impl<'a> Compiler<'a> {
    /// Compile a statement.
    pub(crate) fn compile_statement(&mut self, stmt: &ast::Stmt) {
        match &stmt.kind {
            StmtKind::Block(stmts) => self.compile_block(stmts),
            StmtKind::VarDecl(var) => self.compile_var_decl(var),
            StmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.compile_if(condition, then_body, else_body.as_deref()),
            StmtKind::For {
                init,
                condition,
                increment,
                body,
            } => self.compile_for(init.as_deref(), condition.as_ref(), increment, body),
            StmtKind::While { condition, body } => self.compile_while(condition, body),
            StmtKind::DoWhile { body, condition } => self.compile_do_while(body, condition),
            StmtKind::Return(expr) => self.compile_return(expr.as_ref()),
            StmtKind::Break => self.compile_break(),
            StmtKind::Continue => self.compile_continue(),
            StmtKind::ExprStatement(expr) => {
                self.compile_expression(expr);
            }
            StmtKind::Switch { expr, cases } => self.compile_switch(expr, cases),
            StmtKind::TryCatch {
                try_body,
                catch_body,
            } => self.compile_try_catch(try_body, catch_body),
            StmtKind::Empty => { /* no-op */ }
        }
    }

    fn compile_block(&mut self, stmts: &[ast::Stmt]) {
        self.variables.push_scope();
        for stmt in stmts {
            self.compile_statement(stmt);
        }
        self.variables.pop_scope();
    }

    fn compile_var_decl(&mut self, var: &ast::VarDecl) {
        let data_type = crate::builder::resolve_type_expr_to_data_type(&var.type_expr);

        for declarator in &var.declarators {
            let offset = self
                .variables
                .alloc_variable(&declarator.name, data_type.clone());

            // Compile initializer if present.
            if let Some(init) = &declarator.init {
                match init {
                    VarInit::Expr(expr) => {
                        let result = self.compile_expression(expr);

                        // Convert if needed and copy to variable.
                        self.emit_conversion(&result.data_type, &data_type, result.stack_offset);

                        let prim = data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                        let category = Self::primitive_category(prim);
                        let cpy_op = match category {
                            PrimCategory::Int64 | PrimCategory::Double => OpCode::CpyVtoV8,
                            _ => OpCode::CpyVtoV4,
                        };
                        self.bytecode.emit_ww(cpy_op, offset, result.stack_offset);
                    }
                    VarInit::InitList(_list) => {
                        self.error("init lists not yet supported in compilation");
                    }
                    VarInit::ConstructorArgs(_args) => {
                        self.error("constructor args not yet supported in compilation");
                    }
                }
            } else {
                // Zero-initialize.
                let prim = data_type.as_primitive().unwrap_or(PrimitiveType::Int32);
                let category = Self::primitive_category(prim);
                match category {
                    PrimCategory::Int64 | PrimCategory::Double => {
                        self.bytecode.emit_qw(OpCode::SetV8, 0);
                        self.bytecode.emit_w(OpCode::CpyRtoV8, offset);
                    }
                    _ => {
                        self.bytecode.emit_dw(OpCode::SetV4, 0);
                        self.bytecode.emit_w(OpCode::CpyRtoV4, offset);
                    }
                }
            }
        }
    }

    fn compile_if(
        &mut self,
        condition: &ast::Expr,
        then_body: &ast::Stmt,
        else_body: Option<&ast::Stmt>,
    ) {
        let cond = self.compile_expression(condition);

        // Load condition into register.
        self.bytecode.emit_w(OpCode::CpyVtoR4, cond.stack_offset);

        if let Some(else_stmt) = else_body {
            let else_label = self.bytecode.new_label();
            let end_label = self.bytecode.new_label();

            // Jump to else if condition is zero (false).
            self.bytecode.emit_jump(OpCode::JZ, else_label);

            // Then branch.
            self.compile_statement(then_body);
            self.bytecode.emit_jump(OpCode::JMP, end_label);

            // Else branch.
            self.bytecode.place_label(else_label);
            self.compile_statement(else_stmt);

            self.bytecode.place_label(end_label);
        } else {
            let end_label = self.bytecode.new_label();

            // Jump past the body if condition is false.
            self.bytecode.emit_jump(OpCode::JZ, end_label);

            self.compile_statement(then_body);

            self.bytecode.place_label(end_label);
        }
    }

    fn compile_while(&mut self, condition: &ast::Expr, body: &ast::Stmt) {
        let loop_start = self.bytecode.new_label();
        let loop_end = self.bytecode.new_label();

        self.loop_stack.push(LoopContext {
            break_label: loop_end,
            continue_label: loop_start,
        });

        self.bytecode.place_label(loop_start);

        // Evaluate condition.
        let cond = self.compile_expression(condition);
        self.bytecode.emit_w(OpCode::CpyVtoR4, cond.stack_offset);
        self.bytecode.emit_jump(OpCode::JZ, loop_end);

        // Body.
        self.compile_statement(body);
        self.bytecode.emit_jump(OpCode::JMP, loop_start);

        self.bytecode.place_label(loop_end);
        self.loop_stack.pop();
    }

    fn compile_do_while(&mut self, body: &ast::Stmt, condition: &ast::Expr) {
        let loop_start = self.bytecode.new_label();
        let loop_end = self.bytecode.new_label();
        let continue_label = self.bytecode.new_label();

        self.loop_stack.push(LoopContext {
            break_label: loop_end,
            continue_label,
        });

        self.bytecode.place_label(loop_start);

        // Body executes at least once.
        self.compile_statement(body);

        // Continue target is just before condition.
        self.bytecode.place_label(continue_label);

        // Evaluate condition.
        let cond = self.compile_expression(condition);
        self.bytecode.emit_w(OpCode::CpyVtoR4, cond.stack_offset);
        self.bytecode.emit_jump(OpCode::JNZ, loop_start);

        self.bytecode.place_label(loop_end);
        self.loop_stack.pop();
    }

    fn compile_for(
        &mut self,
        init: Option<&ast::Stmt>,
        condition: Option<&ast::Expr>,
        increment: &[ast::Expr],
        body: &ast::Stmt,
    ) {
        self.variables.push_scope();

        // Init.
        if let Some(init_stmt) = init {
            self.compile_statement(init_stmt);
        }

        let loop_start = self.bytecode.new_label();
        let loop_end = self.bytecode.new_label();
        let continue_label = self.bytecode.new_label();

        self.loop_stack.push(LoopContext {
            break_label: loop_end,
            continue_label,
        });

        self.bytecode.place_label(loop_start);

        // Condition (optional — if absent, loop forever).
        if let Some(cond_expr) = condition {
            let cond = self.compile_expression(cond_expr);
            self.bytecode.emit_w(OpCode::CpyVtoR4, cond.stack_offset);
            self.bytecode.emit_jump(OpCode::JZ, loop_end);
        }

        // Body.
        self.compile_statement(body);

        // Continue target: right before increment.
        self.bytecode.place_label(continue_label);

        // Increment.
        for inc_expr in increment {
            self.compile_expression(inc_expr);
        }

        self.bytecode.emit_jump(OpCode::JMP, loop_start);

        self.bytecode.place_label(loop_end);
        self.loop_stack.pop();
        self.variables.pop_scope();
    }

    fn compile_return(&mut self, expr: Option<&ast::Expr>) {
        if let Some(ret_expr) = expr {
            let result = self.compile_expression(ret_expr);

            // Convert to function return type if needed.
            let return_type = &self.current_function.signature.return_type;
            self.emit_conversion(&result.data_type, return_type, result.stack_offset);

            // Copy result to the register (return value convention).
            let prim = return_type.as_primitive().unwrap_or(PrimitiveType::Int32);
            let category = Self::primitive_category(prim);
            match category {
                PrimCategory::Int64 | PrimCategory::Double => {
                    self.bytecode.emit_w(OpCode::CpyVtoR8, result.stack_offset);
                }
                _ => {
                    self.bytecode.emit_w(OpCode::CpyVtoR4, result.stack_offset);
                }
            }
        }

        self.bytecode.emit_op(OpCode::RET);
    }

    fn compile_break(&mut self) {
        if let Some(ctx) = self.loop_stack.last() {
            let label = ctx.break_label;
            self.bytecode.emit_jump(OpCode::JMP, label);
        } else {
            self.error("break statement outside of loop");
        }
    }

    fn compile_continue(&mut self) {
        if let Some(ctx) = self.loop_stack.last() {
            let label = ctx.continue_label;
            self.bytecode.emit_jump(OpCode::JMP, label);
        } else {
            self.error("continue statement outside of loop");
        }
    }

    fn compile_switch(&mut self, expr: &ast::Expr, cases: &[ast::CaseClause]) {
        let switch_val = self.compile_expression(expr);
        let end_label = self.bytecode.new_label();
        let mut case_labels = Vec::new();
        let mut default_label = None;

        // Allocate labels for each case.
        for case in cases {
            let label = self.bytecode.new_label();
            case_labels.push(label);
            if matches!(case.label, ast::CaseLabel::Default) {
                default_label = Some(label);
            }
        }

        // Emit comparison jumps for each non-default case.
        for (i, case) in cases.iter().enumerate() {
            if let ast::CaseLabel::Expr(case_expr) = &case.label {
                let case_val = self.compile_expression(case_expr);

                // Compare switch value with case value.
                self.bytecode
                    .emit_ww(OpCode::CMPi, switch_val.stack_offset, case_val.stack_offset);
                // Jump to case body if equal (CMPi returns 0 for equal).
                self.bytecode.emit_jump(OpCode::JZ, case_labels[i]);
            }
        }

        // Jump to default if no match, or to end if no default.
        if let Some(def) = default_label {
            self.bytecode.emit_jump(OpCode::JMP, def);
        } else {
            self.bytecode.emit_jump(OpCode::JMP, end_label);
        }

        // Emit case bodies (fall-through between cases).
        for (i, case) in cases.iter().enumerate() {
            self.bytecode.place_label(case_labels[i]);
            for stmt in &case.body {
                self.compile_statement(stmt);
            }
            // Note: no automatic break — AngelScript switch falls through
            // unless break is explicit.
        }

        self.bytecode.place_label(end_label);
    }

    fn compile_try_catch(&mut self, try_body: &ast::Stmt, catch_body: &ast::Stmt) {
        // Basic try-catch structure:
        // - Emit TryBlock marker with offset to catch handler.
        // - Emit try body.
        // - Jump past catch.
        // - Emit catch body.
        let catch_label = self.bytecode.new_label();
        let end_label = self.bytecode.new_label();

        // TryBlock marker (the VM uses this to register an exception handler).
        self.bytecode.emit_jump(OpCode::TryBlock, catch_label);

        // Try body.
        self.compile_statement(try_body);
        self.bytecode.emit_jump(OpCode::JMP, end_label);

        // Catch body.
        self.bytecode.place_label(catch_label);
        self.compile_statement(catch_body);

        self.bytecode.place_label(end_label);
    }
}
