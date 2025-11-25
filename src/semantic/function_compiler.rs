//! Function body compilation and type checking.
//!
//! This module implements Pass 2b of semantic analysis: compiling function bodies.
//! It performs type checking on expressions and statements, tracks local variables,
//! and emits bytecode.


use crate::ast::{
    expr::{
        AssignExpr, BinaryExpr, CallExpr, CastExpr, Expr, IdentExpr, IndexExpr, InitListExpr,
        LambdaExpr, LiteralExpr, LiteralKind, MemberAccess, MemberExpr, ParenExpr, PostfixExpr,
        TernaryExpr, UnaryExpr,
    },
    stmt::{
        Block, BreakStmt, ContinueStmt, DoWhileStmt, ExprStmt, ForeachStmt, ForInit, ForStmt,
        IfStmt, ReturnStmt, Stmt, SwitchStmt, TryCatchStmt, VarDeclStmt, WhileStmt,
    },
    types::TypeExpr,
    AssignOp, BinaryOp, PostfixOp, UnaryOp,
};
use crate::lexer::Span;
use crate::semantic::{
    BytecodeEmitter, CompiledBytecode, DataType, FunctionId, Instruction, LocalScope, Registry,
    SemanticError, SemanticErrorKind, TypeDef, TypeId, BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE, INT32_TYPE,
    INT64_TYPE, VOID_TYPE,
};

/// Result of compiling a single function.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    /// The compiled bytecode
    pub bytecode: CompiledBytecode,

    /// Errors encountered during compilation
    pub errors: Vec<SemanticError>,
}

/// Compiles a single function body (type checking + bytecode generation).
///
/// This structure maintains the compilation state for a single function.
/// It is created fresh for each function and discarded after compilation.
#[derive(Debug)]
pub struct FunctionCompiler<'src, 'ast> {
    /// Global registry (read-only) - contains all type information
    registry: &'ast Registry,

    /// Local variables for this function only
    local_scope: LocalScope,

    /// Bytecode emitter
    bytecode: BytecodeEmitter,

    /// Current function's return type
    return_type: DataType,

    /// Errors encountered during compilation
    errors: Vec<SemanticError>,

    /// Phantom data for source lifetime
    _phantom: std::marker::PhantomData<&'src ()>,
}

impl<'src, 'ast> FunctionCompiler<'src, 'ast> {
    /// Creates a new function compiler.
    ///
    /// # Parameters
    ///
    /// - `registry`: The complete type registry from Pass 2a
    /// - `return_type`: The expected return type for this function
    pub fn new(registry: &'ast Registry, return_type: DataType) -> Self {
        Self {
            registry,
            local_scope: LocalScope::new(),
            bytecode: BytecodeEmitter::new(),
            return_type,
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Compiles a function body.
    ///
    /// This is a convenience method for compiling a complete function with parameters.
    pub fn compile_block(
        registry: &'ast Registry,
        return_type: DataType,
        params: &[(String, DataType)],
        body: &'ast Block<'src, 'ast>,
    ) -> CompiledFunction {
        let mut compiler = Self::new(registry, return_type);

        // Enter function scope
        compiler.local_scope.enter_scope();

        // Declare parameters as local variables
        for (name, param_type) in params {
            compiler
                .local_scope
                .declare_variable_auto(name.clone(), param_type.clone(), true);
        }

        // Compile the function body
        compiler.visit_block(body);

        // Exit function scope
        compiler.local_scope.exit_scope();

        // Ensure function returns properly
        if compiler.return_type.type_id != VOID_TYPE {
            // Non-void function should have explicit return
            // (In a complete implementation, we'd do control flow analysis)
            compiler.bytecode.emit(Instruction::ReturnVoid);
        } else {
            compiler.bytecode.emit(Instruction::ReturnVoid);
        }

        CompiledFunction {
            bytecode: compiler.bytecode.finish(),
            errors: compiler.errors,
        }
    }

    /// Records an error.
    fn error(&mut self, kind: SemanticErrorKind, span: Span, message: impl Into<String>) {
        self.errors
            .push(SemanticError::new(kind, span, message));
    }

    /// Visits a block of statements.
    fn visit_block(&mut self, block: &'ast Block<'src, 'ast>) {
        self.local_scope.enter_scope();

        for stmt in block.stmts {
            self.visit_stmt(stmt);
        }

        self.local_scope.exit_scope();
    }

    /// Visits a statement.
    fn visit_stmt(&mut self, stmt: &'ast Stmt<'src, 'ast>) {
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
    fn visit_expr_stmt(&mut self, expr_stmt: &ExprStmt<'src, 'ast>) {
        if let Some(expr) = expr_stmt.expr {
            let _ = self.check_expr(expr);
            // Expression result is discarded
            self.bytecode.emit(Instruction::Pop);
        }
    }

    /// Visits a variable declaration statement.
    fn visit_var_decl(&mut self, var_decl: &VarDeclStmt<'src, 'ast>) {
        // Resolve the type
        let var_type = match self.resolve_type_expr(&var_decl.ty) {
            Some(ty) => ty,
            None => return, // Error already recorded
        };

        for var in var_decl.vars {
            // Check initializer if present
            if let Some(init) = var.init {
                let init_type = match self.check_expr(init) {
                    Some(ty) => ty,
                    None => continue, // Error already recorded
                };

                if !self.is_assignable(&init_type, &var_type) {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        var.span,
                        format!(
                            "cannot initialize variable of type '{}' with value of type '{}'",
                            self.type_name(&var_type),
                            self.type_name(&init_type)
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
    fn visit_return(&mut self, ret: &ReturnStmt<'src, 'ast>) {
        if let Some(value) = ret.value {
            // Check return value type
            let value_type = match self.check_expr(value) {
                Some(ty) => ty,
                None => return, // Error already recorded
            };

            if !self.is_assignable(&value_type, &self.return_type) {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    ret.span,
                    format!(
                        "cannot return value of type '{}' from function with return type '{}'",
                        self.type_name(&value_type),
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
    fn visit_break(&mut self, brk: &BreakStmt) {
        if self.bytecode.emit_break().is_none() {
            self.error(
                SemanticErrorKind::BreakOutsideLoop,
                brk.span,
                "break statement must be inside a loop or switch",
            );
        }
    }

    /// Visits a continue statement.
    fn visit_continue(&mut self, cont: &ContinueStmt) {
        if self.bytecode.emit_continue().is_none() {
            self.error(
                SemanticErrorKind::ContinueOutsideLoop,
                cont.span,
                "continue statement must be inside a loop",
            );
        }
    }

    /// Visits an if statement.
    fn visit_if(&mut self, if_stmt: &'ast IfStmt<'src, 'ast>) {
        // Check condition
        if let Some(cond_type) = self.check_expr(if_stmt.condition)
            && cond_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    if_stmt.condition.span(),
                    format!(
                        "if condition must be bool, found '{}'",
                        self.type_name(&cond_type)
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
    fn visit_while(&mut self, while_stmt: &'ast WhileStmt<'src, 'ast>) {
        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Check condition
        if let Some(cond_type) = self.check_expr(while_stmt.condition)
            && cond_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    while_stmt.condition.span(),
                    format!(
                        "while condition must be bool, found '{}'",
                        self.type_name(&cond_type)
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
    fn visit_do_while(&mut self, do_while: &'ast DoWhileStmt<'src, 'ast>) {
        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Compile body
        self.visit_stmt(do_while.body);

        // Check condition
        if let Some(cond_type) = self.check_expr(do_while.condition)
            && cond_type.type_id != BOOL_TYPE {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    do_while.condition.span(),
                    format!(
                        "do-while condition must be bool, found '{}'",
                        self.type_name(&cond_type)
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
    fn visit_for(&mut self, for_stmt: &'ast ForStmt<'src, 'ast>) {
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
            if let Some(cond_type) = self.check_expr(condition)
                && cond_type.type_id != BOOL_TYPE {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        condition.span(),
                        format!(
                            "for condition must be bool, found '{}'",
                            self.type_name(&cond_type)
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
    fn visit_foreach(&mut self, foreach: &'ast ForeachStmt<'src, 'ast>) {
        // Type check the iterable expression
        let iterable_type = match self.check_expr(foreach.expr) {
            Some(ty) => ty,
            None => return, // Error already recorded
        };

        // Check that the expression is iterable (array type)
        let typedef = self.registry.get_type(iterable_type.type_id);
        let element_type = match typedef {
            TypeDef::TemplateInstance { template, sub_types } => {
                if *template == self.registry.array_template {
                    // For array<T>, element type is first sub_type
                    if let Some(elem_type) = sub_types.first() {
                        elem_type.clone()
                    } else {
                        self.error(
                            SemanticErrorKind::InternalError,
                            foreach.span,
                            "array template has no element type".to_string(),
                        );
                        return;
                    }
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        foreach.expr.span(),
                        format!(
                            "type '{}' is not iterable in foreach loop",
                            self.type_name(&iterable_type)
                        ),
                    );
                    return;
                }
            }
            _ => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    foreach.expr.span(),
                    format!(
                        "type '{}' does not support foreach iteration",
                        self.type_name(&iterable_type)
                    ),
                );
                return;
            }
        };

        // Enter new scope for loop variables
        self.local_scope.enter_scope();

        // Declare loop variables
        for var in foreach.vars {
            // Resolve the variable's type
            if let Some(var_type) = self.resolve_type_expr(&var.ty) {
                // Check that variable type matches element type
                // For simplicity, we require exact match
                // A complete implementation would allow compatible types
                if var_type.type_id != element_type.type_id {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        var.span,
                        format!(
                            "foreach variable type '{}' does not match iterable element type '{}'",
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

        // Emit foreach loop setup (simplified bytecode)
        // In a real implementation, this would:
        // 1. Initialize iterator
        // 2. Check if more elements
        // 3. Load element into variable(s)
        // 4. Execute body
        // 5. Advance iterator
        // 6. Jump back to check

        let loop_start = self.bytecode.current_position();
        self.bytecode.enter_loop(loop_start);

        // Compile body
        self.visit_stmt(foreach.body);

        // Jump back to loop start
        let current_pos = self.bytecode.current_position();
        let offset = (loop_start as i32) - (current_pos as i32) - 1;
        self.bytecode.emit(Instruction::Jump(offset));

        // Exit loop
        let end_pos = self.bytecode.current_position();
        self.bytecode.exit_loop(end_pos);

        // Exit scope
        self.local_scope.exit_scope();
    }

    /// Visits a switch statement.
    fn visit_switch(&mut self, switch: &'ast SwitchStmt<'src, 'ast>) {
        // Type check the switch expression
        let switch_type = match self.check_expr(switch.expr) {
            Some(ty) => ty,
            None => return, // Error already recorded
        };

        // Switch expressions must be integer or enum types
        if !self.is_integer(&switch_type) {
            self.error(
                SemanticErrorKind::TypeMismatch,
                switch.expr.span(),
                format!(
                    "switch expression must be integer type, found '{}'",
                    self.type_name(&switch_type)
                ),
            );
            return;
        }

        // Track case values to detect duplicates
        let _case_values: std::collections::HashSet<i64> = std::collections::HashSet::new();
        let mut has_default = false;
        let mut case_jump_positions = Vec::new();

        // Emit jump table setup (simplified - real implementation would be more complex)
        for case in switch.cases {
            if case.is_default() {
                if has_default {
                    self.error(
                        SemanticErrorKind::DuplicateDeclaration,
                        case.span,
                        "switch statement can only have one default case".to_string(),
                    );
                }
                has_default = true;
            } else {
                // Check case values
                for value_expr in case.values {
                    // Type check the case value
                    if let Some(value_type) = self.check_expr(value_expr) {
                        // Case value must match switch type
                        if value_type.type_id != switch_type.type_id {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                value_expr.span(),
                                format!(
                                    "case value type '{}' does not match switch type '{}'",
                                    self.type_name(&value_type),
                                    self.type_name(&switch_type)
                                ),
                            );
                        }

                        // TODO: Check for duplicate case values
                        // This would require evaluating constant expressions
                        // For now, we skip this check
                    }
                }
            }

            // Emit case label (placeholder)
            let case_pos = self.bytecode.current_position();
            case_jump_positions.push(case_pos);

            // Compile case statements
            for stmt in case.stmts {
                self.visit_stmt(stmt);
            }
        }

        // Emit switch end
        // In a real implementation, this would patch all jump positions
    }

    /// Visits a try-catch statement.
    fn visit_try_catch(&mut self, try_catch: &'ast TryCatchStmt<'src, 'ast>) {
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

    /// Type checks an expression and returns its type.
    ///
    /// Returns None if type checking failed (error already recorded).
    fn check_expr(&mut self, expr: &'ast Expr<'src, 'ast>) -> Option<DataType> {
        match expr {
            Expr::Literal(lit) => self.check_literal(lit),
            Expr::Ident(ident) => self.check_ident(ident),
            Expr::Binary(binary) => self.check_binary(binary),
            Expr::Unary(unary) => self.check_unary(unary),
            Expr::Assign(assign) => self.check_assign(assign),
            Expr::Ternary(ternary) => self.check_ternary(ternary),
            Expr::Call(call) => self.check_call(call),
            Expr::Index(index) => self.check_index(index),
            Expr::Member(member) => self.check_member(member),
            Expr::Postfix(postfix) => self.check_postfix(postfix),
            Expr::Cast(cast) => self.check_cast(cast),
            Expr::Lambda(lambda) => self.check_lambda(lambda),
            Expr::InitList(init_list) => self.check_init_list(init_list),
            Expr::Paren(paren) => self.check_paren(paren),
        }
    }

    /// Type checks a literal expression.
    fn check_literal(&mut self, lit: &LiteralExpr) -> Option<DataType> {
        let type_id = match &lit.kind {
            LiteralKind::Int(_) => INT64_TYPE,
            LiteralKind::Float(_) => FLOAT_TYPE,
            LiteralKind::Double(_) => DOUBLE_TYPE,
            LiteralKind::Bool(_) => BOOL_TYPE,
            LiteralKind::String(s) => {
                let idx = self.bytecode.add_string_constant(s.clone());
                self.bytecode.emit(Instruction::PushString(idx));
                // STRING_TYPE is TypeId(16)
                return Some(DataType::simple(TypeId(16)));
            }
            LiteralKind::Null => {
                self.bytecode.emit(Instruction::PushNull);
                return Some(DataType::simple(VOID_TYPE));
            }
        };

        // Emit bytecode for literal
        match &lit.kind {
            LiteralKind::Int(i) => self.bytecode.emit(Instruction::PushInt(*i)),
            LiteralKind::Float(f) => self.bytecode.emit(Instruction::PushFloat(*f)),
            LiteralKind::Double(d) => self.bytecode.emit(Instruction::PushDouble(*d)),
            LiteralKind::Bool(b) => self.bytecode.emit(Instruction::PushBool(*b)),
            _ => unreachable!(),
        };

        Some(DataType::simple(type_id))
    }

    /// Type checks an identifier expression.
    fn check_ident(&mut self, ident: &IdentExpr<'src, 'ast>) -> Option<DataType> {
        let name = ident.ident.name;

        // Check local variables first
        if let Some(local_var) = self.local_scope.lookup(name) {
            let offset = local_var.stack_offset;
            self.bytecode.emit(Instruction::LoadLocal(offset));
            return Some(local_var.data_type.clone());
        }

        // Check global variables in registry
        if let Some(global_var) = self.registry.lookup_global_var(name) {
            // Emit load global instruction (using string constant for name)
            let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
            self.bytecode.emit(Instruction::LoadGlobal(name_idx));
            return Some(global_var.data_type.clone());
        }

        // Not found in locals or globals
        self.error(
            SemanticErrorKind::UndefinedVariable,
            ident.span,
            format!("variable '{}' is not defined", name),
        );
        None
    }

    /// Type checks a binary expression.
    fn check_binary(&mut self, binary: &BinaryExpr<'src, 'ast>) -> Option<DataType> {
        let left_type = self.check_expr(binary.left)?;
        let right_type = self.check_expr(binary.right)?;

        // Check if operation is valid for these types
        let result_type = self.check_binary_op(binary.op, &left_type, &right_type, binary.span)?;

        // Emit bytecode for operation
        let instr = match binary.op {
            BinaryOp::Add => Instruction::Add,
            BinaryOp::Sub => Instruction::Sub,
            BinaryOp::Mul => Instruction::Mul,
            BinaryOp::Div => Instruction::Div,
            BinaryOp::Mod => Instruction::Mod,
            BinaryOp::Pow => Instruction::Pow,
            BinaryOp::BitwiseAnd => Instruction::BitAnd,
            BinaryOp::BitwiseOr => Instruction::BitOr,
            BinaryOp::BitwiseXor => Instruction::BitXor,
            BinaryOp::ShiftLeft => Instruction::ShiftLeft,
            BinaryOp::ShiftRight => Instruction::ShiftRight,
            BinaryOp::ShiftRightUnsigned => Instruction::ShiftRightUnsigned,
            BinaryOp::LogicalAnd => Instruction::LogicalAnd,
            BinaryOp::LogicalOr => Instruction::LogicalOr,
            BinaryOp::LogicalXor => Instruction::LogicalXor,
            BinaryOp::Equal => Instruction::Equal,
            BinaryOp::NotEqual => Instruction::NotEqual,
            BinaryOp::Less => Instruction::LessThan,
            BinaryOp::LessEqual => Instruction::LessEqual,
            BinaryOp::Greater => Instruction::GreaterThan,
            BinaryOp::GreaterEqual => Instruction::GreaterEqual,
            BinaryOp::Is | BinaryOp::NotIs => {
                // Type comparison operators
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    binary.span,
                    "is/!is operators are not yet implemented",
                );
                return None;
            }
        };

        self.bytecode.emit(instr);
        Some(result_type)
    }

    /// Checks if a binary operation is valid and returns the result type.
    fn check_binary_op(
        &mut self,
        op: BinaryOp,
        left: &DataType,
        right: &DataType,
        span: Span,
    ) -> Option<DataType> {
        // For simplicity, we'll use basic type rules
        // In a complete implementation, this would be more sophisticated

        match op {
            // Arithmetic operators: require numeric types
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => {
                if self.is_numeric(left) && self.is_numeric(right) {
                    // Result is the "larger" type
                    Some(self.promote_numeric(left, right))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires numeric operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Bitwise operators: require integer types
            BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::ShiftRightUnsigned => {
                if self.is_integer(left) && self.is_integer(right) {
                    Some(self.promote_numeric(left, right))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires integer operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Logical operators: require bool types
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => {
                if left.type_id == BOOL_TYPE && right.type_id == BOOL_TYPE {
                    Some(DataType::simple(BOOL_TYPE))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        span,
                        format!(
                            "operator '{}' requires bool operands, found '{}' and '{}'",
                            op,
                            self.type_name(left),
                            self.type_name(right)
                        ),
                    );
                    None
                }
            }

            // Comparison operators: result is bool
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                // Allow comparison of compatible types
                Some(DataType::simple(BOOL_TYPE))
            }

            // Type comparison
            BinaryOp::Is | BinaryOp::NotIs => Some(DataType::simple(BOOL_TYPE)),
        }
    }

    /// Type checks a unary expression.
    fn check_unary(&mut self, unary: &UnaryExpr<'src, 'ast>) -> Option<DataType> {
        let operand_type = self.check_expr(unary.operand)?;

        let result_type = match unary.op {
            UnaryOp::Neg => {
                if self.is_numeric(&operand_type) {
                    self.bytecode.emit(Instruction::Negate);
                    operand_type
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '-' requires numeric operand, found '{}'",
                            self.type_name(&operand_type)
                        ),
                    );
                    return None;
                }
            }

            UnaryOp::LogicalNot => {
                if operand_type.type_id == BOOL_TYPE {
                    self.bytecode.emit(Instruction::Not);
                    operand_type
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '!' requires bool operand, found '{}'",
                            self.type_name(&operand_type)
                        ),
                    );
                    return None;
                }
            }

            UnaryOp::BitwiseNot => {
                if self.is_integer(&operand_type) {
                    self.bytecode.emit(Instruction::BitNot);
                    operand_type
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '~' requires integer operand, found '{}'",
                            self.type_name(&operand_type)
                        ),
                    );
                    return None;
                }
            }

            UnaryOp::Plus => {
                // Unary + is a no-op for numeric types
                if self.is_numeric(&operand_type) {
                    operand_type
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '+' requires numeric operand, found '{}'",
                            self.type_name(&operand_type)
                        ),
                    );
                    return None;
                }
            }

            UnaryOp::PreInc => {
                self.bytecode.emit(Instruction::PreIncrement);
                operand_type
            }
            UnaryOp::PreDec => {
                self.bytecode.emit(Instruction::PreDecrement);
                operand_type
            }

            UnaryOp::HandleOf => {
                // @ operator - handle reference
                // For now, just return the operand type
                operand_type
            }
        };

        Some(result_type)
    }

    /// Type checks an assignment expression.
    fn check_assign(&mut self, assign: &AssignExpr<'src, 'ast>) -> Option<DataType> {
        use AssignOp::*;

        match assign.op {
            Assign => {
                // Simple assignment: target = value
                let target_type = self.check_expr(assign.target)?;
                let value_type = self.check_expr(assign.value)?;

                // Check if value is assignable to target
                if !self.is_assignable(&value_type, &target_type) {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "cannot assign value of type '{}' to variable of type '{}'",
                            self.type_name(&value_type),
                            self.type_name(&target_type)
                        ),
                    );
                }

                Some(target_type)
            }

            // Compound assignment operators: desugar to binary op + assign
            // e.g., x += 5  =>  x = x + 5
            AddAssign | SubAssign | MulAssign | DivAssign | ModAssign | PowAssign |
            AndAssign | OrAssign | XorAssign | ShlAssign | ShrAssign |
            UshrAssign => {
                // Check target first (this is what we're assigning to)
                let target_type = self.check_expr(assign.target)?;

                // Check value (RHS)
                let value_type = self.check_expr(assign.value)?;

                // Determine the binary operator equivalent
                let binary_op = match assign.op {
                    AddAssign => BinaryOp::Add,
                    SubAssign => BinaryOp::Sub,
                    MulAssign => BinaryOp::Mul,
                    DivAssign => BinaryOp::Div,
                    ModAssign => BinaryOp::Mod,
                    PowAssign => BinaryOp::Pow,
                    AndAssign => BinaryOp::BitwiseAnd,
                    OrAssign => BinaryOp::BitwiseOr,
                    XorAssign => BinaryOp::BitwiseXor,
                    ShlAssign => BinaryOp::ShiftLeft,
                    ShrAssign => BinaryOp::ShiftRight,
                    UshrAssign => BinaryOp::ShiftRightUnsigned,
                    _ => unreachable!(),
                };

                // Perform the binary operation type checking
                // This validates that the operation is valid for these types
                let result_type = self.check_binary_op(
                    binary_op,
                    &target_type,
                    &value_type,
                    assign.span,
                )?;

                // Result should be assignable back to target
                if !self.is_assignable(&result_type, &target_type) {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "compound assignment result type '{}' is not assignable to target type '{}'",
                            self.type_name(&result_type),
                            self.type_name(&target_type)
                        ),
                    );
                }

                // Emit the corresponding binary operation instruction
                let instr = match binary_op {
                    BinaryOp::Add => Instruction::Add,
                    BinaryOp::Sub => Instruction::Sub,
                    BinaryOp::Mul => Instruction::Mul,
                    BinaryOp::Div => Instruction::Div,
                    BinaryOp::Mod => Instruction::Mod,
                    BinaryOp::Pow => Instruction::Pow,
                    BinaryOp::BitwiseAnd => Instruction::BitAnd,
                    BinaryOp::BitwiseOr => Instruction::BitOr,
                    BinaryOp::BitwiseXor => Instruction::BitXor,
                    BinaryOp::ShiftLeft => Instruction::ShiftLeft,
                    BinaryOp::ShiftRight => Instruction::ShiftRight,
                    BinaryOp::ShiftRightUnsigned => Instruction::ShiftRightUnsigned,
                    _ => unreachable!(),
                };
                self.bytecode.emit(instr);

                Some(target_type)
            }
        }
    }

    /// Type checks a ternary expression.
    fn check_ternary(&mut self, ternary: &TernaryExpr<'src, 'ast>) -> Option<DataType> {
        // Check condition
        let cond_type = self.check_expr(ternary.condition)?;
        if cond_type.type_id != BOOL_TYPE {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.condition.span(),
                format!(
                    "ternary condition must be bool, found '{}'",
                    self.type_name(&cond_type)
                ),
            );
        }

        // Check both branches
        let then_type = self.check_expr(ternary.then_expr)?;
        let else_type = self.check_expr(ternary.else_expr)?;

        // Both branches should have compatible types
        // For simplicity, we'll require exact match
        if !self.is_assignable(&then_type, &else_type) {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.span,
                format!(
                    "ternary branches have incompatible types: '{}' and '{}'",
                    self.type_name(&then_type),
                    self.type_name(&else_type)
                ),
            );
        }

        Some(then_type)
    }

    /// Type checks a function call.
    fn check_call(&mut self, call: &CallExpr<'src, 'ast>) -> Option<DataType> {
        // Type check all arguments first
        let mut arg_types = Vec::with_capacity(call.args.len());
        for arg in call.args {
            let arg_type = self.check_expr(arg.value)?;
            arg_types.push(arg_type);
        }

        // Determine what we're calling
        // For now, we only handle simple identifier calls like: foo(args)
        // More complex cases like (someExpr)(args) would need additional handling
        match call.callee {
            Expr::Ident(ident_expr) => {
                // Build qualified name (handling scope if present)
                let func_name = if let Some(scope) = ident_expr.scope {
                    let scope_parts: Vec<&str> = scope.segments.iter().map(|id| id.name).collect();
                    format!("{}::{}", scope_parts.join("::"), ident_expr.ident.name)
                } else {
                    ident_expr.ident.name.to_string()
                };

                // Look up all functions with this name
                let candidates = self.registry.lookup_functions(&func_name);

                if candidates.is_empty() {
                    self.error(
                        SemanticErrorKind::UndefinedVariable,
                        call.span,
                        format!("undefined function '{}'", func_name),
                    );
                    return None;
                }

                // Find best matching overload
                let matching_func = self.find_best_function_overload(
                    candidates,
                    &arg_types,
                    call.span,
                )?;

                let func_def = self.registry.get_function(matching_func);

                // Emit call instruction
                self.bytecode.emit(Instruction::Call {
                    function_id: matching_func.as_u32(),
                    arg_count: arg_types.len() as u32,
                });

                Some(func_def.return_type.clone())
            }
            _ => {
                // Complex call expression (e.g., function pointer, lambda)
                let _callee_type = self.check_expr(call.callee)?;

                self.error(
                    SemanticErrorKind::NotCallable,
                    call.span,
                    "complex call expressions are not yet fully implemented",
                );
                None
            }
        }
    }

    /// Type checks an index expression.
    fn check_index(&mut self, index: &IndexExpr<'src, 'ast>) -> Option<DataType> {
        let object_type = self.check_expr(index.object)?;

        // Check that the object is indexable (array type)
        let typedef = self.registry.get_type(object_type.type_id);

        let element_type = match typedef {
            TypeDef::TemplateInstance { template, sub_types } => {
                // Check if this is an array template instance
                if *template == self.registry.array_template {
                    // For array<T>, the element type is the first sub_type
                    if let Some(elem_type) = sub_types.first() {
                        elem_type.clone()
                    } else {
                        self.error(
                            SemanticErrorKind::InternalError,
                            index.span,
                            "array template has no element type".to_string(),
                        );
                        return None;
                    }
                } else if *template == self.registry.dict_template {
                    // For dictionary<K,V>, the element type is the second sub_type (value type)
                    if let Some(value_type) = sub_types.get(1) {
                        value_type.clone()
                    } else {
                        self.error(
                            SemanticErrorKind::InternalError,
                            index.span,
                            "dictionary template has invalid structure".to_string(),
                        );
                        return None;
                    }
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        index.span,
                        format!("type '{}' is not indexable", self.type_name(&object_type)),
                    );
                    return None;
                }
            }
            _ => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    index.span,
                    format!("type '{}' does not support indexing", self.type_name(&object_type)),
                );
                return None;
            }
        };

        // Check all indices - should be integer type for arrays
        // (For dictionaries, the index type should match the key type, but simplified for now)
        for idx_item in index.indices {
            let idx_type = self.check_expr(idx_item.index)?;

            // For arrays, index must be integer
            if !self.is_integer(&idx_type) {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    idx_item.span,
                    format!(
                        "array index must be integer type, found '{}'",
                        self.type_name(&idx_type)
                    ),
                );
            }
        }

        // Emit index instruction
        self.bytecode.emit(Instruction::Index);

        Some(element_type)
    }

    /// Type checks a member access expression.
    fn check_member(&mut self, member: &MemberExpr<'src, 'ast>) -> Option<DataType> {
        let object_type = self.check_expr(member.object)?;

        // Check that the object is a class/interface type
        let typedef = self.registry.get_type(object_type.type_id);

        match &member.member {
            MemberAccess::Field(field_name) => {
                // Look up the field in the class
                match typedef {
                    TypeDef::Class { fields, .. } => {
                        // Find the field by name and get its index
                        if let Some((field_index, field_def)) = fields.iter().enumerate().find(|(_, f)| f.name == field_name.name) {
                            // Emit load field instruction (using field index)
                            self.bytecode.emit(Instruction::LoadField(field_index as u32));

                            // If the object is const, the field should also be const
                            let mut field_type = field_def.data_type.clone();
                            if object_type.is_const || object_type.is_handle_to_const {
                                field_type.is_const = true;
                            }

                            Some(field_type)
                        } else {
                            self.error(
                                SemanticErrorKind::UndefinedField,
                                member.span,
                                format!(
                                    "type '{}' has no field '{}'",
                                    self.type_name(&object_type),
                                    field_name.name
                                ),
                            );
                            None
                        }
                    }
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            member.span,
                            format!(
                                "type '{}' does not support field access",
                                self.type_name(&object_type)
                            ),
                        );
                        None
                    }
                }
            }
            MemberAccess::Method { name, args } => {
                // Type check all arguments first
                let mut arg_types = Vec::with_capacity(args.len());
                for arg in *args {
                    let arg_type = self.check_expr(arg.value)?;
                    arg_types.push(arg_type);
                }

                // Verify the object is a class type
                match typedef {
                    TypeDef::Class { .. } => {
                        // Look up methods with this name
                        // Methods are stored with qualified names like "ClassName::methodName"
                        let class_name = typedef.name();
                        let method_name = format!("{}::{}", class_name, name.name);

                        let candidates = self.registry.lookup_functions(&method_name);

                        if candidates.is_empty() {
                            self.error(
                                SemanticErrorKind::UndefinedMethod,
                                member.span,
                                format!(
                                    "type '{}' has no method '{}'",
                                    self.type_name(&object_type),
                                    name.name
                                ),
                            );
                            return None;
                        }

                        // Filter by const-correctness first
                        let is_const_object = object_type.is_const || object_type.is_handle_to_const;

                        let const_filtered: Vec<_> = if is_const_object {
                            // Const objects can only call const methods
                            candidates.iter().copied()
                                .filter(|&func_id| {
                                    let func_def = self.registry.get_function(func_id);
                                    func_def.traits.is_const
                                })
                                .collect()
                        } else {
                            // Non-const objects can call both const and non-const methods
                            candidates.to_vec()
                        };

                        if const_filtered.is_empty() {
                            self.error(
                                SemanticErrorKind::InvalidOperation,
                                member.span,
                                format!(
                                    "no const method '{}' found for const object of type '{}'",
                                    name.name,
                                    self.type_name(&object_type)
                                ),
                            );
                            return None;
                        }

                        // Find best matching overload from const-filtered candidates
                        let matching_method = self.find_best_function_overload(
                            &const_filtered,
                            &arg_types,
                            member.span,
                        )?;

                        let func_def = self.registry.get_function(matching_method);

                        // Emit method call instruction
                        self.bytecode.emit(Instruction::CallMethod {
                            method_id: matching_method.as_u32(),
                            arg_count: arg_types.len() as u32,
                        });

                        Some(func_def.return_type.clone())
                    }
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            member.span,
                            format!(
                                "type '{}' does not support method calls",
                                self.type_name(&object_type)
                            ),
                        );
                        None
                    }
                }
            }
        }
    }

    /// Type checks a postfix expression.
    fn check_postfix(&mut self, postfix: &PostfixExpr<'src, 'ast>) -> Option<DataType> {
        let operand_type = self.check_expr(postfix.operand)?;

        match postfix.op {
            PostfixOp::PostInc => {
                self.bytecode.emit(Instruction::PostIncrement);
            }
            PostfixOp::PostDec => {
                self.bytecode.emit(Instruction::PostDecrement);
            }
        }

        Some(operand_type)
    }

    /// Type checks a cast expression.
    fn check_cast(&mut self, cast: &CastExpr<'src, 'ast>) -> Option<DataType> {
        let _expr_type = self.check_expr(cast.expr)?;
        let target_type = self.resolve_type_expr(&cast.target_type)?;

        self.bytecode.emit(Instruction::Cast(target_type.type_id));
        Some(target_type)
    }

    /// Type checks a lambda expression.
    fn check_lambda(&mut self, lambda: &LambdaExpr<'src, 'ast>) -> Option<DataType> {
        self.error(
            SemanticErrorKind::InternalError,
            lambda.span,
            "lambda expressions are not yet implemented in Phase 2b",
        );
        None
    }

    /// Type checks an initializer list.
    fn check_init_list(&mut self, init_list: &InitListExpr<'src, 'ast>) -> Option<DataType> {
        self.error(
            SemanticErrorKind::InternalError,
            init_list.span,
            "initializer lists are not yet implemented in Phase 2b",
        );
        None
    }

    /// Type checks a parenthesized expression.
    fn check_paren(&mut self, paren: &ParenExpr<'src, 'ast>) -> Option<DataType> {
        self.check_expr(paren.expr)
    }

    /// Resolves a type expression to a DataType.
    fn resolve_type_expr(&mut self, type_expr: &TypeExpr<'src, 'ast>) -> Option<DataType> {
        // Simplified type resolution
        // In a complete implementation, this would use the TypeCompiler's logic
        let type_name = format!("{}", type_expr.base);

        if let Some(type_id) = self.registry.lookup_type(&type_name) {
            // TODO: Handle type modifiers and template arguments
            Some(DataType::simple(type_id))
        } else {
            self.error(
                SemanticErrorKind::UndefinedType,
                type_expr.span,
                format!("undefined type '{}'", type_name),
            );
            None
        }
    }

    /// Checks if a value type is assignable to a target type.
    fn is_assignable(&self, value: &DataType, target: &DataType) -> bool {
        // Simplified assignability check
        // In a complete implementation, this would handle:
        // - Implicit conversions
        // - Inheritance relationships
        // - Handle conversions
        value.type_id == target.type_id
    }

    /// Checks if a type is numeric.
    fn is_numeric(&self, ty: &DataType) -> bool {
        matches!(
            ty.type_id,
            INT32_TYPE | INT64_TYPE | FLOAT_TYPE | DOUBLE_TYPE
        )
    }

    /// Checks if a type is an integer type.
    fn is_integer(&self, ty: &DataType) -> bool {
        matches!(ty.type_id, INT32_TYPE | INT64_TYPE)
    }

    /// Promotes two numeric types to their common type.
    fn promote_numeric(&self, left: &DataType, right: &DataType) -> DataType {
        // Simplified promotion rules
        if left.type_id == DOUBLE_TYPE || right.type_id == DOUBLE_TYPE {
            DataType::simple(DOUBLE_TYPE)
        } else if left.type_id == FLOAT_TYPE || right.type_id == FLOAT_TYPE {
            DataType::simple(FLOAT_TYPE)
        } else if left.type_id == INT64_TYPE || right.type_id == INT64_TYPE {
            DataType::simple(INT64_TYPE)
        } else {
            DataType::simple(INT32_TYPE)
        }
    }

    /// Gets a human-readable name for a type.
    fn type_name(&self, ty: &DataType) -> String {
        let type_def = self.registry.get_type(ty.type_id);
        type_def.name().to_string()
    }

    /// Finds the best matching function overload for the given arguments.
    ///
    /// Returns the FunctionId of the best match, or None if no match found.
    fn find_best_function_overload(
        &mut self,
        candidates: &[FunctionId],
        arg_types: &[DataType],
        span: Span,
    ) -> Option<FunctionId> {
        // Filter candidates by argument count first
        let count_matched: Vec<_> = candidates.iter().copied()
            .filter(|&func_id| {
                let func_def = self.registry.get_function(func_id);
                func_def.params.len() == arg_types.len()
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
            let exact_match = func_def.params.iter().zip(arg_types.iter())
                .all(|(param, arg)| param.type_id == arg.type_id);

            if exact_match {
                return Some(func_id);
            }
        }

        // If no exact match, try to find a compatible match with implicit conversions
        // For now, simplified: just take the first one with matching count
        // A complete implementation would rank by conversion quality
        let first_match = count_matched.first().copied();

        if first_match.is_none() {
            self.error(
                SemanticErrorKind::TypeMismatch,
                span,
                "no matching overload found for given argument types".to_string(),
            );
        }

        first_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{DataType, Registry};

    fn create_test_registry() -> Registry {
        Registry::new()
    }

    #[test]
    fn new_compiler_initializes() {
        let registry = create_test_registry();
        let return_type = DataType::simple(VOID_TYPE);
        let compiler = FunctionCompiler::<'_, '_>::new(&registry, return_type);

        assert_eq!(compiler.errors.len(), 0);
        assert_eq!(compiler.return_type.type_id, VOID_TYPE);
    }

    // More tests will be added as we implement the compiler
}
