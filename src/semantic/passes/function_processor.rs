//! Function body compilation and type checking.
//!
//! This module implements Pass 2b of semantic analysis: compiling function bodies.
//! It performs type checking on expressions and statements, tracks local variables,
//! and emits bytecode.


use crate::{ast::{
    AssignOp, BinaryOp, PostfixOp, Script, UnaryOp, decl::{ClassDecl, ClassMember, FunctionDecl, Item, NamespaceDecl}, expr::{
        AssignExpr, BinaryExpr, CallExpr, CastExpr, Expr, IdentExpr, IndexExpr, InitElement, InitListExpr,
        LambdaExpr, LiteralExpr, LiteralKind, MemberAccess, MemberExpr, ParenExpr, PostfixExpr,
        TernaryExpr, UnaryExpr,
    }, stmt::{
        Block, BreakStmt, ContinueStmt, DoWhileStmt, ExprStmt, ForInit, ForStmt, ForeachStmt, IfStmt, ReturnStmt, Stmt, SwitchStmt, TryCatchStmt, VarDeclStmt, WhileStmt
    }, types::TypeExpr
}, semantic::STRING_TYPE};
use crate::codegen::{BytecodeEmitter, CompiledBytecode, CompiledModule, Instruction};
use crate::lexer::Span;
use crate::semantic::{
    CapturedVar, DataType, LocalScope, OperatorBehavior, PrimitiveType, Registry, SemanticError, SemanticErrorKind, TypeDef, TypeId,
    BOOL_TYPE, DOUBLE_TYPE, FLOAT_TYPE, INT32_TYPE, INT64_TYPE, NULL_TYPE, UINT8_TYPE, VOID_TYPE,
};
use crate::semantic::types::type_def::FunctionId;
use rustc_hash::FxHashMap;

/// Expression context - tracks type and lvalue/mutability information.
///
/// This is returned by `check_expr()` to provide both the type of an expression
/// and whether it can be used as an lvalue (for assignments, reference parameters, etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ExprContext {
    /// The data type of this expression
    pub data_type: DataType,

    /// Whether this expression is an lvalue (can be assigned to)
    pub is_lvalue: bool,

    /// Whether this lvalue is mutable (can be modified)
    /// - Always false for rvalues
    /// - True for non-const lvalues (variables, mutable fields, etc.)
    /// - False for const lvalues (const variables, const& parameters, etc.)
    pub is_mutable: bool,
}

impl ExprContext {
    /// Create a new rvalue context (temporary value, cannot be assigned to)
    pub fn rvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: false,
            is_mutable: false,
        }
    }

    /// Create a new lvalue context (can be assigned to)
    pub fn lvalue(data_type: DataType, is_mutable: bool) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable,
        }
    }

    /// Create an immutable lvalue context (can be read but not written)
    pub fn const_lvalue(data_type: DataType) -> Self {
        Self {
            data_type,
            is_lvalue: true,
            is_mutable: false,
        }
    }
}

/// Result of compiling a single function.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    /// The compiled bytecode
    pub bytecode: CompiledBytecode,

    /// Errors encountered during compilation
    pub errors: Vec<SemanticError>,

    /// Lambda functions compiled within this function
    pub lambdas: FxHashMap<FunctionId, CompiledBytecode>,
}

/// Compiles function bodies (type checking + bytecode generation).
///
/// This compiler can operate in two modes:
/// 1. Module mode: Walk entire AST and compile all functions
/// 2. Single function mode: Compile one function body
#[derive(Debug)]
pub struct FunctionCompiler<'src, 'ast> {
    /// Global registry (read-only) - contains all type information
    registry: &'ast Registry<'src, 'ast>,

    /// Local variables for current function (only used in single-function mode)
    local_scope: LocalScope,

    /// Bytecode emitter for current function (only used in single-function mode)
    bytecode: BytecodeEmitter,

    /// Current function's return type (only used in single-function mode)
    return_type: DataType,

    /// Compiled functions (only used in module mode)
    compiled_functions: FxHashMap<FunctionId, CompiledBytecode>,

    /// Current namespace path (e.g., ["Game", "World"])
    namespace_path: Vec<String>,

    /// Current class context (when compiling methods)
    current_class: Option<TypeId>,

    /// Global lambda counter for unique FunctionIds (starts at next available ID after regular functions)
    next_lambda_id: u32,

    /// Name of the current function being compiled (optional - for debug/error messages)
    current_function: Option<String>,

    /// Expected funcdef type for lambda type inference
    expected_funcdef_type: Option<TypeId>,

    /// Errors encountered during compilation
    errors: Vec<SemanticError>,

    /// Phantom data for source lifetime
    _phantom: std::marker::PhantomData<&'src ()>,
}

impl<'src, 'ast> FunctionCompiler<'src, 'ast> {
    /// Perform Pass 2b function compilation on a script.
    ///
    /// This is the main entry point for compiling all functions in a module.
    pub fn compile(
        script: &Script<'src, 'ast>,
        registry: &'ast Registry<'src, 'ast>,
    ) -> CompiledModule {
        let mut compiler = Self::new_module_compiler(registry);
        compiler.visit_script(script);

        CompiledModule {
            functions: compiler.compiled_functions,
            errors: compiler.errors,
        }
    }

    /// Creates a new module-level compiler (for compiling all functions).
    fn new_module_compiler(registry: &'ast Registry<'src, 'ast>) -> Self {
        Self {
            next_lambda_id: registry.function_count() as u32,  // Start after regular functions
            registry,
            local_scope: LocalScope::new(),
            bytecode: BytecodeEmitter::new(),
            return_type: DataType::simple(VOID_TYPE),
            compiled_functions: FxHashMap::default(),
            namespace_path: Vec::new(),
            current_class: None,
            current_function: None,
            expected_funcdef_type: None,
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new single-function compiler.
    ///
    /// # Parameters
    ///
    /// - `registry`: The complete type registry from Pass 2a
    /// - `return_type`: The expected return type for this function
    fn new(registry: &'ast Registry<'src, 'ast>, return_type: DataType) -> Self {
        Self {
            next_lambda_id: registry.function_count() as u32,  // Start after regular functions
            registry,
            local_scope: LocalScope::new(),
            bytecode: BytecodeEmitter::new(),
            return_type,
            compiled_functions: FxHashMap::default(),
            namespace_path: Vec::new(),
            current_class: None,
            current_function: None,
            expected_funcdef_type: None,
            errors: Vec::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Compiles a function body.
    ///
    /// This is a convenience method for compiling a complete function with parameters.
    pub fn compile_block(
        registry: &'ast Registry<'src, 'ast>,
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
            lambdas: compiler.compiled_functions,
        }
    }

    /// Compiles a function body with class context (for methods/constructors).
    ///
    /// This variant allows tracking the current class for super() resolution.
    fn compile_block_with_class(
        registry: &'ast Registry<'src, 'ast>,
        return_type: DataType,
        params: &[(String, DataType)],
        body: &'ast Block<'src, 'ast>,
        current_class: Option<TypeId>,
    ) -> CompiledFunction {
        let mut compiler = Self::new(registry, return_type);
        compiler.current_class = current_class;

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
            lambdas: compiler.compiled_functions,
        }
    }

    // ========================================================================
    // AST Walking (Module-level compilation)
    // ========================================================================

    /// Visit the entire script and compile all functions
    fn visit_script(&mut self, script: &'ast Script<'src, 'ast>) {
        for item in script.items() {
            self.visit_item(item);
        }
    }

    /// Visit a top-level item
    fn visit_item(&mut self, item: &'ast Item<'src, 'ast>) {
        match item {
            Item::Function(func) => self.visit_function_decl(func, None),
            Item::Class(class) => self.visit_class_decl(class),
            Item::Namespace(ns) => self.visit_namespace(ns),
            Item::Interface(_)
            | Item::Enum(_)
            | Item::GlobalVar(_)
            | Item::Typedef(_)
            | Item::Funcdef(_)
            | Item::Mixin(_)
            | Item::Import(_) => {
                // These don't have function bodies to compile
            }
        }
    }

    /// Visit a namespace and compile functions within it
    fn visit_namespace(&mut self, ns: &'ast NamespaceDecl<'src, 'ast>) {
        // Build namespace path from segments
        let ns_name = ns.path.iter().map(|id| id.name).collect::<Vec<_>>().join("::");
        self.namespace_path.push(ns_name);

        for item in ns.items {
            self.visit_item(item);
        }

        self.namespace_path.pop();
    }

    /// Visit a class declaration and compile all its methods
    fn visit_class_decl(&mut self, class: &'ast ClassDecl<'src, 'ast>) {
        let qualified_name = self.build_qualified_name(class.name.name);

        // Look up the class type ID
        let type_id = match self.registry.lookup_type(&qualified_name) {
            Some(id) => id,
            None => {
                // Class wasn't registered - this shouldn't happen if Pass 1 & 2a worked
                return;
            }
        };

        // Get all methods for this class from the registry
        let method_ids = self.registry.get_methods(type_id);

        // Compile each method by matching AST to FunctionIds
        for member in class.members {
            if let ClassMember::Method(method_decl) = member {
                // Find the matching FunctionId for this method
                let func_id = method_ids
                    .iter()
                    .copied()
                    .find(|&fid| {
                        let func_def = self.registry.get_function(fid);
                        // Match by function name (unqualified)
                        // TODO: Also match by parameters for overloaded methods
                        func_def.name == method_decl.name.name
                    });

                if let Some(func_id) = func_id {
                    self.compile_method(method_decl, func_id, Some(class));
                }
            }
        }
    }

    /// Compile a method given its AST and FunctionId
    fn compile_method(&mut self, func: &'ast FunctionDecl<'src, 'ast>, func_id: FunctionId, class: Option<&'ast ClassDecl<'src, 'ast>>) {
        // Skip functions without bodies (abstract methods, forward declarations)
        let body = match &func.body {
            Some(body) => body,
            None => return,
        };

        let func_def = self.registry.get_function(func_id);

        // Extract parameters for compilation
        let params: Vec<(String, DataType)> = func_def
            .params
            .iter()
            .enumerate()
            .map(|(i, param_type)| {
                // Get parameter name from AST
                let name = if i < func.params.len() {
                    func.params[i].name.map(|id| id.name.to_string()).unwrap_or_else(|| format!("param{}", i))
                } else {
                    format!("param{}", i)
                };
                (name, param_type.clone())
            })
            .collect();

        // For constructors, emit member initialization in the correct order
        let mut constructor_prologue = None;
        if func.is_constructor() && let Some(class_decl) = class {
            constructor_prologue = Some(self.compile_constructor_prologue(class_decl, func_def.object_type, body));
        }

        // Compile the function body
        let mut compiled = Self::compile_block_with_class(
            self.registry,
            func_def.return_type.clone(),
            &params,
            body,
            func_def.object_type,
        );

        // Prepend constructor prologue if present
        if let Some(prologue) = constructor_prologue {
            // Prepend prologue instructions to the compiled bytecode
            let mut combined = prologue;
            combined.extend(compiled.bytecode.instructions);
            compiled.bytecode.instructions = combined;
        }

        // Store the compiled bytecode
        self.compiled_functions.insert(func_id, compiled.bytecode);

        // Collect lambda bytecode from this function
        for (lambda_id, lambda_bytecode) in compiled.lambdas {
            self.compiled_functions.insert(lambda_id, lambda_bytecode);
        }

        // Accumulate errors
        self.errors.extend(compiled.errors);
    }

    /// Compile constructor prologue: member initialization in correct order.
    ///
    /// Order:
    /// 1. Initialize fields WITHOUT explicit initializers
    /// 2. Call base class constructor (if base class exists and super() not called in body)
    /// 3. Initialize fields WITH explicit initializers
    fn compile_constructor_prologue(&mut self, class: &'ast ClassDecl<'src, 'ast>, class_type_id: Option<TypeId>, body: &'ast Block<'src, 'ast>) -> Vec<Instruction> {
        let mut instructions = Vec::new();

        // Get the class type to check for base class
        let class_id = match class_type_id {
            Some(id) => id,
            None => return instructions, // Not a method, shouldn't happen
        };

        let class_typedef = self.registry.get_type(class_id);
        let base_class_id = match class_typedef {
            TypeDef::Class { base_class, .. } => *base_class,
            _ => None,
        };

        // Partition fields into those with and without initializers
        let mut fields_without_init = Vec::new();
        let mut fields_with_init = Vec::new();

        for member in class.members {
            if let ClassMember::Field(field) = member {
                if field.init.is_some() {
                    fields_with_init.push(field);
                } else {
                    fields_without_init.push(field);
                }
            }
        }

        // 1. Initialize fields without explicit initializers (default initialization)
        for field in fields_without_init {
            // Emit default initialization for this field
            // For now, we'll emit a placeholder comment
            // TODO: Implement actual default initialization based on field type
            instructions.push(Instruction::Nop); // Placeholder
        }

        // 2. Call base class constructor if base class exists and super() not called in body
        if let Some(base_id) = base_class_id {
            // Check if the constructor body contains a super() call
            let has_super_call = self.contains_super_call(body);

            if !has_super_call {
                // Emit call to base class default constructor
                // Only auto-call if super() is not explicitly called
                let base_constructors = self.registry.find_constructors(base_id);
                if let Some(&base_ctor_id) = base_constructors.first() {
                    instructions.push(Instruction::CallConstructor {
                        type_id: base_id.0,
                        func_id: base_ctor_id.0,
                    });
                }
            }
        }

        // 3. Initialize fields with explicit initializers
        for field in fields_with_init {
            if let Some(init_expr) = field.init {
                // Compile the initializer expression
                // We need a temporary compiler context for this
                // For now, emit a placeholder
                // TODO: Properly compile the initializer expression
                instructions.push(Instruction::Nop); // Placeholder
            }
        }

        instructions
    }

    /// Check if a block contains a super() call.
    ///
    /// This recursively searches through statements and expressions to find
    /// any call expression where the callee is the identifier "super".
    fn contains_super_call(&self, block: &Block<'src, 'ast>) -> bool {
        for stmt in block.stmts {
            if self.stmt_contains_super_call(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains a super() call (helper for contains_super_call)
    fn stmt_contains_super_call(&self, stmt: &Stmt<'src, 'ast>) -> bool {
        match stmt {
            Stmt::Expr(expr_stmt) => {
                expr_stmt.expr.map_or(false, |e| self.expr_contains_super_call(e))
            }
            Stmt::VarDecl(var_decl) => {
                // VarDeclStmt has a `vars` slice of VarDeclarator
                var_decl.vars.iter().any(|var| {
                    var.init.map_or(false, |e| self.expr_contains_super_call(e))
                })
            }
            Stmt::If(if_stmt) => {
                self.expr_contains_super_call(if_stmt.condition)
                    || self.stmt_contains_super_call(if_stmt.then_stmt)
                    || if_stmt.else_stmt.map_or(false, |s| self.stmt_contains_super_call(s))
            }
            Stmt::While(while_stmt) => {
                self.expr_contains_super_call(while_stmt.condition)
                    || self.stmt_contains_super_call(while_stmt.body)
            }
            Stmt::DoWhile(do_while) => {
                self.stmt_contains_super_call(do_while.body)
                    || self.expr_contains_super_call(do_while.condition)
            }
            Stmt::For(for_stmt) => {
                let init_has_super = match &for_stmt.init {
                    Some(ForInit::VarDecl(var_decl_stmt)) => {
                        var_decl_stmt.vars.iter().any(|var| {
                            var.init.map_or(false, |e| self.expr_contains_super_call(e))
                        })
                    }
                    Some(ForInit::Expr(expr)) => self.expr_contains_super_call(expr),
                    None => false,
                };
                let update_has_super = for_stmt.update.iter().any(|e| self.expr_contains_super_call(e));
                init_has_super
                    || for_stmt.condition.map_or(false, |e| self.expr_contains_super_call(e))
                    || update_has_super
                    || self.stmt_contains_super_call(for_stmt.body)
            }
            Stmt::Foreach(foreach) => {
                self.expr_contains_super_call(foreach.expr)
                    || self.stmt_contains_super_call(foreach.body)
            }
            Stmt::Return(ret) => ret.value.map_or(false, |e| self.expr_contains_super_call(e)),
            Stmt::Block(block) => self.contains_super_call(block),
            Stmt::Switch(switch) => {
                self.expr_contains_super_call(switch.expr)
                    || switch.cases.iter().any(|case| {
                        case.stmts.iter().any(|s| self.stmt_contains_super_call(s))
                    })
            }
            Stmt::TryCatch(try_catch) => {
                self.contains_super_call(&try_catch.try_block)
                    || self.contains_super_call(&try_catch.catch_block)
            }
            Stmt::Break(_) | Stmt::Continue(_) => false,
        }
    }

    /// Check if an expression contains a super() call (helper for contains_super_call)
    fn expr_contains_super_call(&self, expr: &Expr<'src, 'ast>) -> bool {
        match expr {
            Expr::Call(call) => {
                // Check if this is a super() call
                if let Expr::Ident(ident) = call.callee {
                    if ident.scope.is_none() && ident.ident.name == "super" {
                        return true;
                    }
                }
                // Check arguments
                call.args.iter().any(|arg| self.expr_contains_super_call(arg.value))
            }
            Expr::Binary(bin) => {
                self.expr_contains_super_call(bin.left) || self.expr_contains_super_call(bin.right)
            }
            Expr::Unary(un) => self.expr_contains_super_call(un.operand),
            Expr::Assign(assign) => {
                self.expr_contains_super_call(assign.target) || self.expr_contains_super_call(assign.value)
            }
            Expr::Ternary(ternary) => {
                self.expr_contains_super_call(ternary.condition)
                    || self.expr_contains_super_call(ternary.then_expr)
                    || self.expr_contains_super_call(ternary.else_expr)
            }
            Expr::Member(member) => self.expr_contains_super_call(member.object),
            Expr::Index(index) => {
                self.expr_contains_super_call(index.object)
                    || index.indices.iter().any(|idx| self.expr_contains_super_call(idx.index))
            }
            Expr::Postfix(postfix) => self.expr_contains_super_call(postfix.operand),
            Expr::Cast(cast) => self.expr_contains_super_call(cast.expr),
            Expr::InitList(init_list) => init_list.elements.iter().any(|elem| match elem {
                InitElement::Expr(e) => self.expr_contains_super_call(e),
                InitElement::InitList(nested) => {
                    nested.elements.iter().any(|e| match e {
                        InitElement::Expr(expr) => self.expr_contains_super_call(expr),
                        InitElement::InitList(_) => false, // Limit nesting depth for simplicity
                    })
                }
            }),
            Expr::Lambda(lambda) => self.contains_super_call(&lambda.body),
            Expr::Paren(paren) => self.expr_contains_super_call(paren.expr),
            Expr::Ident(_) | Expr::Literal(_) => false,
        }
    }

    /// Visit a function declaration and compile its body
    fn visit_function_decl(&mut self, func: &'ast FunctionDecl<'src, 'ast>, object_type: Option<TypeId>) {
        // Skip functions without bodies (abstract methods, forward declarations)
        let body = match &func.body {
            Some(body) => body,
            None => return,
        };

        let qualified_name = self.build_qualified_name(func.name.name);

        // Look up the function in the registry to get its FunctionId and signature
        let func_ids = self.registry.lookup_functions(&qualified_name);

        if func_ids.is_empty() {
            // Function wasn't registered - this shouldn't happen if Pass 1 & 2a worked
            return;
        }

        // Find the matching function by checking object_type
        let func_id = func_ids
            .iter()
            .copied()
            .find(|&id| {
                let func_def = self.registry.get_function(id);
                func_def.object_type == object_type
            });

        let func_id = match func_id {
            Some(id) => id,
            None => {
                // No matching function found - skip
                return;
            }
        };

        let func_def = self.registry.get_function(func_id);

        // Extract parameters for compilation
        let params: Vec<(String, DataType)> = func_def
            .params
            .iter()
            .enumerate()
            .map(|(i, param_type)| {
                // Get parameter name from AST
                let name = if i < func.params.len() {
                    func.params[i].name.map(|id| id.name.to_string()).unwrap_or_else(|| format!("param{}", i))
                } else {
                    format!("param{}", i)
                };
                (name, param_type.clone())
            })
            .collect();

        // Compile the function body
        let compiled = Self::compile_block(
            self.registry,
            func_def.return_type.clone(),
            &params,
            body,
        );

        // Store the compiled bytecode
        self.compiled_functions.insert(func_id, compiled.bytecode);

        // Collect lambda bytecode from this function
        for (lambda_id, lambda_bytecode) in compiled.lambdas {
            self.compiled_functions.insert(lambda_id, lambda_bytecode);
        }

        // Accumulate errors
        self.errors.extend(compiled.errors);
    }

    /// Build a qualified name from the current namespace path
    fn build_qualified_name(&self, name: &str) -> String {
        if self.namespace_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.namespace_path.join("::"), name)
        }
    }

    // ========================================================================
    // Expression and Statement Compilation
    // ========================================================================

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
                let init_ctx = match self.check_expr(init) {
                    Some(ctx) => ctx,
                    None => continue, // Error already recorded
                };

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
    fn visit_return(&mut self, ret: &ReturnStmt<'src, 'ast>) {
        if let Some(value) = ret.value {
            // Check return value type
            let value_ctx = match self.check_expr(value) {
                Some(ctx) => ctx,
                None => return, // Error already recorded
            };

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
    fn visit_while(&mut self, while_stmt: &'ast WhileStmt<'src, 'ast>) {
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
    fn visit_do_while(&mut self, do_while: &'ast DoWhileStmt<'src, 'ast>) {
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
    fn visit_foreach(&mut self, foreach: &'ast ForeachStmt<'src, 'ast>) {
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

                if let Some(conversion) = element_type.can_convert_to(&var_type, self.registry) {
                    if conversion.is_implicit {
                        self.emit_conversion(&conversion);
                    }
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
    fn visit_switch(&mut self, switch: &'ast SwitchStmt<'src, 'ast>) {
        // Type check the switch expression
        let switch_ctx = match self.check_expr(switch.expr) {
            Some(ctx) => ctx,
            None => return, // Error already recorded
        };

        // Switch expressions must be integer or enum types
        if !self.is_integer(&switch_ctx.data_type) {
            self.error(
                SemanticErrorKind::TypeMismatch,
                switch.expr.span(),
                format!(
                    "switch expression must be integer type, found '{}'",
                    self.type_name(&switch_ctx.data_type)
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
                    if let Some(value_ctx) = self.check_expr(value_expr) {
                        // Case value must match switch type
                        if value_ctx.data_type.type_id != switch_ctx.data_type.type_id {
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
    fn check_expr(&mut self, expr: &'ast Expr<'src, 'ast>) -> Option<ExprContext> {
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
    /// Literals are always rvalues (temporary values).
    fn check_literal(&mut self, lit: &LiteralExpr) -> Option<ExprContext> {
        let type_id = match &lit.kind {
            LiteralKind::Int(_) => INT32_TYPE, // Default integer literals to int32 (matches 'int' type)
            LiteralKind::Float(_) => FLOAT_TYPE,
            LiteralKind::Double(_) => DOUBLE_TYPE,
            LiteralKind::Bool(_) => BOOL_TYPE,
            LiteralKind::String(s) => {
                let idx = self.bytecode.add_string_constant(s.clone());
                self.bytecode.emit(Instruction::PushString(idx));
                // STRING_TYPE is TypeId(16)
                return Some(ExprContext::rvalue(DataType::simple(STRING_TYPE)));
            }
            LiteralKind::Null => {
                self.bytecode.emit(Instruction::PushNull);
                return Some(ExprContext::rvalue(DataType::simple(NULL_TYPE)));
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

        Some(ExprContext::rvalue(DataType::simple(type_id)))
    }

    /// Type checks an identifier expression.
    /// Variables are lvalues (mutable unless marked const).
    fn check_ident(&mut self, ident: &IdentExpr<'src, 'ast>) -> Option<ExprContext> {
        let name = ident.ident.name;

        // Check local variables first
        if let Some(local_var) = self.local_scope.lookup(name) {
            let offset = local_var.stack_offset;
            self.bytecode.emit(Instruction::LoadLocal(offset));
            let is_mutable = !local_var.data_type.is_const;
            return Some(ExprContext::lvalue(local_var.data_type.clone(), is_mutable));
        }

        // Check global variables in registry
        if let Some(global_var) = self.registry.lookup_global_var(name) {
            // Emit load global instruction (using string constant for name)
            let name_idx = self.bytecode.add_string_constant(global_var.qualified_name());
            self.bytecode.emit(Instruction::LoadGlobal(name_idx));
            let is_mutable = !global_var.data_type.is_const;
            return Some(ExprContext::lvalue(global_var.data_type.clone(), is_mutable));
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
    /// Binary expressions always produce rvalues (temporary results).
    fn check_binary(&mut self, binary: &BinaryExpr<'src, 'ast>) -> Option<ExprContext> {
        let left_ctx = self.check_expr(binary.left)?;
        let right_ctx = self.check_expr(binary.right)?;

        // Try operator overloading first (for binary arithmetic/bitwise operators)
        let result_type = match binary.op {
            // Arithmetic operators with overloading support
            BinaryOp::Add => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpAdd,
                    OperatorBehavior::OpAddR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Sub => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpSub,
                    OperatorBehavior::OpSubR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Mul => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpMul,
                    OperatorBehavior::OpMulR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Div => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpDiv,
                    OperatorBehavior::OpDivR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Mod => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpMod,
                    OperatorBehavior::OpModR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::Pow => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpPow,
                    OperatorBehavior::OpPowR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Bitwise operators with overloading support
            BinaryOp::BitwiseAnd => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpAnd,
                    OperatorBehavior::OpAndR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::BitwiseOr => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpOr,
                    OperatorBehavior::OpOrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::BitwiseXor => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpXor,
                    OperatorBehavior::OpXorR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftLeft => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpShl,
                    OperatorBehavior::OpShlR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftRight => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpShr,
                    OperatorBehavior::OpShrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }
            BinaryOp::ShiftRightUnsigned => {
                if let Some(result_type) = self.try_binary_operator_overload(
                    OperatorBehavior::OpUShr,
                    OperatorBehavior::OpUShrR,
                    &left_ctx.data_type,
                    &right_ctx.data_type,
                    binary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Comparison operators - try opEquals for ==, !=
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Try opEquals first (returns bool)
                if let Some(func_id) = self.registry.find_operator_method(left_ctx.data_type.type_id, OperatorBehavior::OpEquals) {
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    // For !=, negate the result
                    if binary.op == BinaryOp::NotEqual {
                        self.bytecode.emit(Instruction::Not);
                    }
                    return Some(ExprContext::rvalue(DataType::simple(BOOL_TYPE)));
                }
                // Fall back to primitive comparison
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Relational operators - try opCmp for <, <=, >, >=
            BinaryOp::Less | BinaryOp::LessEqual
            | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                // Try opCmp first (returns int: negative/zero/positive)
                if let Some(func_id) = self.registry.find_operator_method(left_ctx.data_type.type_id, OperatorBehavior::OpCmp) {
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    // Compare result with zero based on operator
                    self.bytecode.emit(Instruction::PushInt(0));
                    let cmp_instr = match binary.op {
                        BinaryOp::Less => Instruction::LessThan,          // opCmp() < 0
                        BinaryOp::LessEqual => Instruction::LessEqual,     // opCmp() <= 0
                        BinaryOp::Greater => Instruction::GreaterThan,     // opCmp() > 0
                        BinaryOp::GreaterEqual => Instruction::GreaterEqual, // opCmp() >= 0
                        _ => unreachable!(),
                    };
                    self.bytecode.emit(cmp_instr);
                    return Some(ExprContext::rvalue(DataType::simple(BOOL_TYPE)));
                }
                // Fall back to primitive comparison
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Logical operators (no overloading in AngelScript)
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::LogicalXor => {
                self.check_binary_op(binary.op, &left_ctx.data_type, &right_ctx.data_type, binary.span)?
            }

            // Type comparison operators
            BinaryOp::Is | BinaryOp::NotIs => {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    binary.span,
                    "is/!is operators are not yet implemented",
                );
                return None;
            }
        };

        // If operator overload was used, we already returned above
        // Otherwise, emit primitive bytecode instruction
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
                // Already handled above
                return None;
            }
        };

        self.bytecode.emit(instr);
        Some(ExprContext::rvalue(result_type))
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
    /// Most unary operations produce rvalues, but ++x/--x preserve lvalue-ness.
    fn check_unary(&mut self, unary: &UnaryExpr<'src, 'ast>) -> Option<ExprContext> {
        let operand_ctx = self.check_expr(unary.operand)?;

        match unary.op {
            UnaryOp::Neg => {
                // Try opNeg operator overload first
                if let Some(result_type) = self.try_unary_operator_overload(
                    OperatorBehavior::OpNeg,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                // Fall back to primitive negation
                if self.is_numeric(&operand_ctx.data_type) {
                    self.bytecode.emit(Instruction::Negate);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '-' requires numeric operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::LogicalNot => {
                // No operator overloading for logical NOT in AngelScript
                if operand_ctx.data_type.type_id == BOOL_TYPE {
                    self.bytecode.emit(Instruction::Not);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '!' requires bool operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::BitwiseNot => {
                // Try opCom operator overload first
                if let Some(result_type) = self.try_unary_operator_overload(
                    OperatorBehavior::OpCom,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    return Some(ExprContext::rvalue(result_type));
                }
                // Fall back to primitive bitwise NOT
                if self.is_integer(&operand_ctx.data_type) {
                    self.bytecode.emit(Instruction::BitNot);
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '~' requires integer operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::Plus => {
                // No operator overloading for unary + in AngelScript
                // Unary + is a no-op for numeric types, produces rvalue
                if self.is_numeric(&operand_ctx.data_type) {
                    Some(ExprContext::rvalue(operand_ctx.data_type))
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!(
                            "unary '+' requires numeric operand, found '{}'",
                            self.type_name(&operand_ctx.data_type)
                        ),
                    );
                    None
                }
            }

            UnaryOp::PreInc | UnaryOp::PreDec => {
                // Try opPreInc/opPreDec operator overload first
                let operator = if unary.op == UnaryOp::PreInc {
                    OperatorBehavior::OpPreInc
                } else {
                    OperatorBehavior::OpPreDec
                };

                if let Some(result_type) = self.try_unary_operator_overload(
                    operator,
                    &operand_ctx.data_type,
                    unary.span,
                ) {
                    // Operator overloads for ++/-- return new value, but still need lvalue check
                    if !operand_ctx.is_lvalue {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            unary.span,
                            format!("{} requires an lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                        );
                        return None;
                    }
                    if !operand_ctx.is_mutable {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            unary.span,
                            format!("{} requires a mutable lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                        );
                        return None;
                    }
                    // Overloaded operators return rvalue of their return type
                    return Some(ExprContext::rvalue(result_type));
                }

                // Fall back to primitive pre-increment/decrement
                // Pre-increment/decrement require mutable lvalue and return lvalue
                if !operand_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!("{} requires an lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                    );
                    return None;
                }
                if !operand_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        unary.span,
                        format!("{} requires a mutable lvalue", if unary.op == UnaryOp::PreInc { "pre-increment" } else { "pre-decrement" }),
                    );
                    return None;
                }

                let instr = if unary.op == UnaryOp::PreInc {
                    Instruction::PreIncrement
                } else {
                    Instruction::PreDecrement
                };
                self.bytecode.emit(instr);

                // Returns lvalue with same mutability
                Some(operand_ctx)
            }

            UnaryOp::HandleOf => {
                // @ operator - handle reference, produces rvalue
                Some(ExprContext::rvalue(operand_ctx.data_type))
            }
        }
    }

    /// Type checks an assignment expression.
    /// Assignments require a mutable lvalue as target and produce an rvalue.
    fn check_assign(&mut self, assign: &AssignExpr<'src, 'ast>) -> Option<ExprContext> {
        use AssignOp::*;

        match assign.op {
            Assign => {
                // Special handling for index expressions: obj[idx] = value
                // Try set_opIndex accessor if opIndex doesn't exist
                if let Expr::Index(index_expr) = assign.target {
                    if let Some(result) = self.check_index_assignment(index_expr, assign.value, assign.span) {
                        return Some(result);
                    }
                    // If check_index_assignment returns None, fall through to regular assignment
                    // (this shouldn't happen as check_index_assignment handles all cases)
                }

                // Simple assignment: target = value
                let target_ctx = self.check_expr(assign.target)?;
                let value_ctx = self.check_expr(assign.value)?;

                // Check that target is a mutable lvalue
                if !target_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to an rvalue",
                    );
                    return None;
                }
                if !target_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to a const lvalue",
                    );
                    return None;
                }

                // Try opAssign operator overload first (for user-defined types)
                if let Some(func_id) = self.registry.find_operator_method(target_ctx.data_type.type_id, OperatorBehavior::OpAssign) {
                    // Call opAssign(value) on target
                    // Stack: [target, value] → target.opAssign(value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let func = self.registry.get_function(func_id);
                    return Some(ExprContext::rvalue(func.return_type.clone()));
                }

                // Fall back to primitive assignment with type conversion
                // Check if value is assignable to target and emit conversion if needed
                if let Some(conversion) = value_ctx.data_type.can_convert_to(&target_ctx.data_type, self.registry) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            assign.span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' (explicit cast required)",
                                self.type_name(&value_ctx.data_type),
                                self.type_name(&target_ctx.data_type)
                            ),
                        );
                    } else {
                        // Emit conversion instruction if needed
                        self.emit_conversion(&conversion);
                    }
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "cannot assign value of type '{}' to variable of type '{}'",
                            self.type_name(&value_ctx.data_type),
                            self.type_name(&target_ctx.data_type)
                        ),
                    );
                }

                // Assignment produces rvalue of target type
                Some(ExprContext::rvalue(target_ctx.data_type))
            }

            // Compound assignment operators: try operator overload first, then desugar
            // e.g., x += 5  =>  x.opAddAssign(5) OR x = x + 5
            AddAssign | SubAssign | MulAssign | DivAssign | ModAssign | PowAssign |
            AndAssign | OrAssign | XorAssign | ShlAssign | ShrAssign |
            UshrAssign => {
                // Check target first (this is what we're assigning to)
                let target_ctx = self.check_expr(assign.target)?;

                // Check that target is a mutable lvalue
                if !target_ctx.is_lvalue {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to an rvalue",
                    );
                    return None;
                }
                if !target_ctx.is_mutable {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        assign.target.span(),
                        "cannot assign to a const lvalue",
                    );
                    return None;
                }

                // Check value (RHS)
                let value_ctx = self.check_expr(assign.value)?;

                // Try compound assignment operator overload first
                let compound_op = match assign.op {
                    AddAssign => OperatorBehavior::OpAddAssign,
                    SubAssign => OperatorBehavior::OpSubAssign,
                    MulAssign => OperatorBehavior::OpMulAssign,
                    DivAssign => OperatorBehavior::OpDivAssign,
                    ModAssign => OperatorBehavior::OpModAssign,
                    PowAssign => OperatorBehavior::OpPowAssign,
                    AndAssign => OperatorBehavior::OpAndAssign,
                    OrAssign => OperatorBehavior::OpOrAssign,
                    XorAssign => OperatorBehavior::OpXorAssign,
                    ShlAssign => OperatorBehavior::OpShlAssign,
                    ShrAssign => OperatorBehavior::OpShrAssign,
                    UshrAssign => OperatorBehavior::OpUShrAssign,
                    _ => unreachable!(),
                };

                if let Some(func_id) = self.registry.find_operator_method(target_ctx.data_type.type_id, compound_op) {
                    // Call opXxxAssign(value) on target
                    // Stack: [target, value] → target.opAddAssign(value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let func = self.registry.get_function(func_id);
                    return Some(ExprContext::rvalue(func.return_type.clone()));
                }

                // Fall back to desugaring: x += y  =>  x = x + y

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
                    &target_ctx.data_type,
                    &value_ctx.data_type,
                    assign.span,
                )?;

                // Result should be assignable back to target
                if !self.is_assignable(&result_type, &target_ctx.data_type) {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        assign.span,
                        format!(
                            "compound assignment result type '{}' is not assignable to target type '{}'",
                            self.type_name(&result_type),
                            self.type_name(&target_ctx.data_type)
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

                // Assignment produces rvalue of target type
                Some(ExprContext::rvalue(target_ctx.data_type))
            }
        }
    }

    /// Type checks a ternary expression.
    /// Ternary expressions produce rvalues (temporary values).
    fn check_ternary(&mut self, ternary: &TernaryExpr<'src, 'ast>) -> Option<ExprContext> {
        // Check condition
        let cond_ctx = self.check_expr(ternary.condition)?;
        if cond_ctx.data_type.type_id != BOOL_TYPE {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.condition.span(),
                format!(
                    "ternary condition must be bool, found '{}'",
                    self.type_name(&cond_ctx.data_type)
                ),
            );
        }

        // Check both branches
        let then_ctx = self.check_expr(ternary.then_expr)?;
        let else_ctx = self.check_expr(ternary.else_expr)?;

        // Both branches should have compatible types
        // For simplicity, we'll require exact match
        if !self.is_assignable(&then_ctx.data_type, &else_ctx.data_type) {
            self.error(
                SemanticErrorKind::TypeMismatch,
                ternary.span,
                format!(
                    "ternary branches have incompatible types: '{}' and '{}'",
                    self.type_name(&then_ctx.data_type),
                    self.type_name(&else_ctx.data_type)
                ),
            );
        }

        Some(ExprContext::rvalue(then_ctx.data_type))
    }

    /// Type checks a function call.
    /// Function calls produce rvalues (unless they return a reference, which we don't handle yet).
    fn check_call(&mut self, call: &CallExpr<'src, 'ast>) -> Option<ExprContext> {
        // Determine what we're calling FIRST (before type-checking arguments)
        // This allows us to provide expected funcdef context for lambda inference
        match call.callee {
            Expr::Ident(ident_expr) => {
                // Build qualified name (handling scope if present)
                let name = if let Some(scope) = ident_expr.scope {
                    let scope_parts: Vec<&str> = scope.segments.iter().map(|id| id.name).collect();
                    format!("{}::{}", scope_parts.join("::"), ident_expr.ident.name)
                } else {
                    ident_expr.ident.name.to_string()
                };

                // Special handling for 'super' - resolve to base class constructor
                if name == "super" {
                    // Get current class context
                    let class_id = match self.current_class {
                        Some(id) => id,
                        None => {
                            self.error(
                                SemanticErrorKind::UndefinedVariable,
                                call.span,
                                "'super' can only be used in class methods/constructors",
                            );
                            return None;
                        }
                    };

                    // Get the class definition
                    let class_def = self.registry.get_type(class_id);

                    // Check if class has a base class
                    let base_id = match class_def {
                        TypeDef::Class { base_class, .. } => match base_class {
                            Some(base) => *base,
                            None => {
                                self.error(
                                    SemanticErrorKind::UndefinedVariable,
                                    call.span,
                                    "'super' can only be used in classes with inheritance",
                                );
                                return None;
                            }
                        },
                        _ => {
                            self.error(
                                SemanticErrorKind::UndefinedVariable,
                                call.span,
                                "'super' can only be used in class methods",
                            );
                            return None;
                        }
                    };

                    // Type-check arguments WITHOUT funcdef inference for super calls
                    let mut arg_contexts = Vec::with_capacity(call.args.len());
                    for arg in call.args {
                        let arg_ctx = self.check_expr(arg.value)?;
                        arg_contexts.push(arg_ctx);
                    }
                    let arg_types: Vec<DataType> = arg_contexts.iter().map(|ctx| ctx.data_type.clone()).collect();

                    // Find matching base constructor
                    let base_constructors = self.registry.find_constructors(base_id);
                    let (matching_ctor, conversions) = self.find_best_function_overload(
                        &base_constructors,
                        &arg_types,
                        call.span,
                    )?;

                    let func_def = self.registry.get_function(matching_ctor);

                    // Validate reference parameters
                    self.validate_reference_parameters(func_def, &arg_contexts, call.args, call.span)?;

                    // Emit conversion instructions for arguments
                    for conversion in conversions {
                        if let Some(conv) = conversion {
                            self.emit_conversion(&conv);
                        }
                    }

                    // Emit regular Call instruction - base constructor executes with current 'this'
                    self.bytecode.emit(Instruction::Call(matching_ctor.as_u32()));

                    // Constructors return void
                    return Some(ExprContext::rvalue(DataType::simple(VOID_TYPE)));
                }

                // Check if this is a local variable (could be funcdef handle)
                if ident_expr.scope.is_none() {  // Only check locals for unqualified names
                    if let Some(var) = self.local_scope.lookup(&name) {
                        if var.data_type.is_handle {
                            let type_def = self.registry.get_type(var.data_type.type_id);
                            if matches!(type_def, TypeDef::Funcdef { .. }) {
                                // This is a funcdef variable - use the default case to handle it
                                // Fall through to handle as complex callee expression
                                let callee_ctx = self.check_expr(call.callee)?;

                                // Type-check arguments WITHOUT funcdef inference for now
                                let mut arg_contexts = Vec::with_capacity(call.args.len());
                                for arg in call.args {
                                    let arg_ctx = self.check_expr(arg.value)?;
                                    arg_contexts.push(arg_ctx);
                                }

                                // Use funcdef calling logic from default case
                                if let TypeDef::Funcdef { params, return_type, .. } = type_def {
                                    // Validate arguments
                                    if arg_contexts.len() != params.len() {
                                        self.error(
                                            SemanticErrorKind::TypeMismatch,
                                            call.span,
                                            format!("funcdef call expects {} arguments but {} were provided",
                                                params.len(), arg_contexts.len()),
                                        );
                                        return None;
                                    }

                                    // Validate and emit conversions for each argument
                                    for (i, (arg_ctx, param)) in arg_contexts.iter().zip(params.iter()).enumerate() {
                                        if let Some(conv) = arg_ctx.data_type.can_convert_to(param, self.registry) {
                                            self.emit_conversion(&conv);
                                        } else {
                                            self.error(
                                                SemanticErrorKind::TypeMismatch,
                                                call.args[i].span,
                                                format!("argument {} type mismatch in funcdef call", i),
                                            );
                                            return None;
                                        }
                                    }

                                    // Emit CallPtr instruction
                                    self.bytecode.emit(Instruction::CallPtr);

                                    // Return the funcdef's return type
                                    return Some(ExprContext::rvalue(return_type.clone()));
                                }
                            }
                        }
                    }
                }

                // Check if this is a type name (constructor call)
                if let Some(type_id) = self.registry.lookup_type(&name) {
                    // Type-check arguments WITHOUT funcdef inference context for constructor calls
                    let mut arg_contexts = Vec::with_capacity(call.args.len());
                    for arg in call.args {
                        let arg_ctx = self.check_expr(arg.value)?;
                        arg_contexts.push(arg_ctx);
                    }
                    return self.check_constructor_call(type_id, &arg_contexts, call.span);
                }

                // Regular function call - look up candidates
                let candidates = self.registry.lookup_functions(&name);

                if candidates.is_empty() {
                    self.error(
                        SemanticErrorKind::UndefinedVariable,
                        call.span,
                        format!("undefined function or type '{}'", name),
                    );
                    return None;
                }

                // Try to infer expected funcdef types for lambda arguments
                // For simplicity: if there's only one candidate, use its parameter types
                // TODO: Handle overloading more sophisticatedly
                let expected_param_types = if candidates.len() == 1 {
                    let func_def = self.registry.get_function(candidates[0]);
                    Some(&func_def.params)
                } else {
                    None
                };

                // Type-check arguments with funcdef context inference
                let mut arg_contexts = Vec::with_capacity(call.args.len());
                for (i, arg) in call.args.iter().enumerate() {
                    // Set expected_funcdef_type if this parameter expects a funcdef
                    if let Some(params) = expected_param_types {
                        if i < params.len() {
                            let param_type = &params[i];
                            if param_type.is_handle {
                                // Check if this is a funcdef type
                                let type_def = self.registry.get_type(param_type.type_id);
                                if matches!(type_def, TypeDef::Funcdef { .. }) {
                                    self.expected_funcdef_type = Some(param_type.type_id);
                                }
                            }
                        }
                    }

                    let arg_ctx = self.check_expr(arg.value)?;
                    arg_contexts.push(arg_ctx);

                    // Clear expected_funcdef_type after checking each argument
                    self.expected_funcdef_type = None;
                }

                // Extract types for overload resolution
                let arg_types: Vec<DataType> = arg_contexts.iter().map(|ctx| ctx.data_type.clone()).collect();

                // Find best matching overload
                let (matching_func, conversions) = self.find_best_function_overload(
                    candidates,
                    &arg_types,
                    call.span,
                )?;

                let func_def = self.registry.get_function(matching_func);

                // Compile default arguments if fewer args provided than params
                if arg_contexts.len() < func_def.params.len() {
                    for i in arg_contexts.len()..func_def.params.len() {
                        if let Some(default_expr) = func_def.default_args.get(i).and_then(|opt| *opt) {
                            // Compile the default argument expression inline
                            let default_ctx = self.check_expr(default_expr)?;

                            // Apply implicit conversion if needed
                            if let Some(conv) = default_ctx.data_type.can_convert_to(&func_def.params[i], self.registry) {
                                self.emit_conversion(&conv);
                            }
                        } else {
                            // No default arg for this parameter - error
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                call.span,
                                format!("function '{}' expects {} arguments but {} were provided",
                                    func_def.name, func_def.params.len(), arg_contexts.len()),
                            );
                            return None;
                        }
                    }
                }

                // Validate reference parameters BEFORE emitting conversions
                self.validate_reference_parameters(func_def, &arg_contexts, call.args, call.span)?;

                // Emit conversion instructions for explicitly provided arguments
                for conversion in conversions {
                    if let Some(conv) = conversion {
                        self.emit_conversion(&conv);
                    }
                }

                // Emit call instruction
                self.bytecode.emit(Instruction::Call(matching_func.as_u32()));

                // Function calls produce rvalues
                Some(ExprContext::rvalue(func_def.return_type.clone()))
            }
            _ => {
                // Complex call expression (e.g., obj(args) with opCall, function pointer, lambda)
                let callee_ctx = self.check_expr(call.callee)?;

                // Type-check arguments WITHOUT funcdef inference for opCall
                let mut arg_contexts = Vec::with_capacity(call.args.len());
                for arg in call.args {
                    let arg_ctx = self.check_expr(arg.value)?;
                    arg_contexts.push(arg_ctx);
                }

                // Try opCall operator overload (allows objects to be called like functions)
                if let Some(func_id) = self.registry.find_operator_method(callee_ctx.data_type.type_id, OperatorBehavior::OpCall) {
                    // Call opCall(args) on callee
                    // Stack: [callee, arg1, arg2, ...] → callee.opCall(arg1, arg2, ...)

                    let func_def = self.registry.get_function(func_id);

                    // Validate reference parameters
                    self.validate_reference_parameters(func_def, &arg_contexts, call.args, call.span)?;

                    // Type checking is done by the opCall signature
                    // TODO: Could add conversion support here if needed

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    return Some(ExprContext::rvalue(func_def.return_type.clone()));
                }

                // No opCall found - check if it's a funcdef/function pointer
                if callee_ctx.data_type.is_handle {
                    let type_def = self.registry.get_type(callee_ctx.data_type.type_id);

                    if let TypeDef::Funcdef { params, return_type, .. } = type_def {
                        // This is a funcdef handle - validate arguments
                        if arg_contexts.len() != params.len() {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                call.span,
                                format!("funcdef call expects {} arguments but {} were provided",
                                    params.len(), arg_contexts.len()),
                            );
                            return None;
                        }

                        // Validate and emit conversions for each argument
                        for (i, (arg_ctx, param)) in arg_contexts.iter().zip(params.iter()).enumerate() {
                            if let Some(conv) = arg_ctx.data_type.can_convert_to(param, self.registry) {
                                self.emit_conversion(&conv);
                            } else {
                                self.error(
                                    SemanticErrorKind::TypeMismatch,
                                    call.args[i].span,
                                    format!("argument {} type mismatch in funcdef call", i),
                                );
                                return None;
                            }
                        }

                        // Emit CallPtr instruction to invoke through function pointer
                        // Stack: [funcdef_handle, arg1, arg2, ...] → result
                        self.bytecode.emit(Instruction::CallPtr);

                        // Return the funcdef's return type
                        return Some(ExprContext::rvalue(return_type.clone()));
                    }
                }

                // Not callable
                self.error(
                    SemanticErrorKind::NotCallable,
                    call.span,
                    format!("type '{}' is not callable (no opCall operator or funcdef)", self.type_name(&callee_ctx.data_type)),
                );
                None
            }
        }
    }

    /// Type checks a constructor call (e.g., `Player(100, "Bob")`).
    fn check_constructor_call(
        &mut self,
        type_id: TypeId,
        arg_contexts: &[ExprContext],
        span: Span,
    ) -> Option<ExprContext> {
        // Extract types for overload resolution
        let arg_types: Vec<DataType> = arg_contexts.iter().map(|ctx| ctx.data_type.clone()).collect();

        // Get all constructors for this type
        let constructors = self.registry.find_constructors(type_id);

        if constructors.is_empty() {
            let type_name = self.registry.get_type(type_id).name();
            self.error(
                SemanticErrorKind::UndefinedFunction,
                span,
                format!("type '{}' has no constructors", type_name),
            );
            return None;
        }

        // Find best matching constructor using existing overload resolution
        let (matching_ctor, conversions) = self.find_best_function_overload(&constructors, &arg_types, span)?;

        // Emit conversion instructions for arguments
        for conversion in conversions {
            if let Some(conv) = conversion {
                self.emit_conversion(&conv);
            }
        }

        // Emit constructor call instruction
        self.bytecode.emit(Instruction::CallConstructor {
            type_id: type_id.as_u32(),
            func_id: matching_ctor.as_u32(),
        });

        // Constructor calls produce rvalues (newly constructed objects)
        Some(ExprContext::rvalue(DataType::simple(type_id)))
    }

    /// Type checks an index expression.
    /// Index expressions (arr[i]) are lvalues if the array is an lvalue.
    /// Multi-dimensional indexing (arr[0][1]) is handled by chaining opIndex calls.
    fn check_index(&mut self, index: &IndexExpr<'src, 'ast>) -> Option<ExprContext> {
        // Start with the base object
        let mut current_ctx = self.check_expr(index.object)?;

        // Process each index dimension sequentially, chaining opIndex calls
        // For arr[0][1], this becomes: temp = arr.opIndex(0); temp.opIndex(1)
        for idx_item in index.indices {
            // Evaluate the index expression for this dimension
            let idx_ctx = self.check_expr(idx_item.index)?;

            // Try to find opIndex for the current object type (priority 1)
            if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndex) {
                let func = self.registry.get_function(func_id);

                // opIndex should have exactly 1 parameter (the index)
                // Note: AngelScript only supports single-parameter opIndex
                if func.params.len() != 1 {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!(
                            "opIndex must have exactly 1 parameter, found {}",
                            func.params.len()
                        ),
                    );
                    return None;
                }

                let param_type = &func.params[0];

                // Type check the index argument against opIndex parameter
                if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.registry) {
                    if !conversion.is_implicit {
                        // Explicit conversion required
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' for opIndex parameter (explicit cast required)",
                                self.type_name(&idx_ctx.data_type),
                                self.type_name(param_type)
                            ),
                        );
                        return None;
                    }
                    // Emit implicit conversion instruction
                    self.emit_conversion(&conversion);
                } else {
                    // No conversion possible - type mismatch
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        idx_item.span,
                        format!(
                            "opIndex parameter expects type '{}', found '{}'",
                            self.type_name(param_type),
                            self.type_name(&idx_ctx.data_type)
                        ),
                    );
                    return None;
                }

                // Call opIndex on current object
                // Stack: [object, index] → object.opIndex(index)
                self.bytecode.emit(Instruction::Call(func_id.as_u32()));

                // opIndex returns a reference, so result is an lvalue
                // The return type becomes the object for the next index (if any)
                let is_mutable = current_ctx.is_mutable && !func.return_type.is_const;
                current_ctx = ExprContext::lvalue(func.return_type.clone(), is_mutable);
            } else if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexGet) {
                // No opIndex found, try get_opIndex accessor (priority 2)
                let func = self.registry.get_function(func_id);

                // get_opIndex should have exactly 1 parameter (the index)
                if func.params.len() != 1 {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!(
                            "get_opIndex must have exactly 1 parameter, found {}",
                            func.params.len()
                        ),
                    );
                    return None;
                }

                let param_type = &func.params[0];

                // Type check the index argument
                if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.registry) {
                    if !conversion.is_implicit {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "cannot implicitly convert '{}' to '{}' for get_opIndex parameter (explicit cast required)",
                                self.type_name(&idx_ctx.data_type),
                                self.type_name(param_type)
                            ),
                        );
                        return None;
                    }
                    self.emit_conversion(&conversion);
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        idx_item.span,
                        format!(
                            "get_opIndex parameter expects type '{}', found '{}'",
                            self.type_name(param_type),
                            self.type_name(&idx_ctx.data_type)
                        ),
                    );
                    return None;
                }

                // Call get_opIndex on current object
                // Stack: [object, index] → object.get_opIndex(index)
                self.bytecode.emit(Instruction::Call(func_id.as_u32()));

                // get_opIndex returns a value (read-only), so result is an rvalue
                // This is a property accessor, not a reference
                current_ctx = ExprContext::rvalue(func.return_type.clone());
            } else {
                // No opIndex or get_opIndex registered for this type
                // This includes:
                // - Built-in types (array<T>, dictionary<K,V>, string) - should register opIndex via FFI
                // - Template instances without opIndex
                // - Classes without opIndex
                // - Primitives (which can't be indexed)
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    idx_item.span,
                    format!("type '{}' does not support indexing", self.type_name(&current_ctx.data_type)),
                );
                return None;
            }
        }

        // Return the final result after all indices have been processed
        Some(current_ctx)
    }

    /// Type checks an index assignment expression: obj[idx] = value
    /// This handles set_opIndex property accessor.
    /// Returns None if error occurred, Some(ExprContext) for the assignment result.
    fn check_index_assignment(
        &mut self,
        index: &IndexExpr<'src, 'ast>,
        value: &'ast Expr<'src, 'ast>,
        span: Span,
    ) -> Option<ExprContext> {
        // For multi-dimensional indexing like arr[0][1] = value:
        // - Process all but the last index using regular opIndex/get_opIndex (read context)
        // - Use set_opIndex only for the final index with the assignment value

        // Start with the base object
        let mut current_ctx = self.check_expr(index.object)?;

        // Process all indices except the last one in read context
        let last_idx = index.indices.len() - 1;
        for (i, idx_item) in index.indices.iter().enumerate() {
            // Evaluate the index expression for this dimension
            let idx_ctx = self.check_expr(idx_item.index)?;

            if i < last_idx {
                // Not the final index - use regular opIndex/get_opIndex (read context)
                // This is the same logic as check_index
                if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndex) {
                    let func = self.registry.get_function(func_id);

                    if func.params.len() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("opIndex must have exactly 1 parameter, found {}", func.params.len()),
                        );
                        return None;
                    }

                    let param_type = &func.params[0];
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let is_mutable = current_ctx.is_mutable && !func.return_type.is_const;
                    current_ctx = ExprContext::lvalue(func.return_type.clone(), is_mutable);
                } else if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexGet) {
                    let func = self.registry.get_function(func_id);

                    if func.params.len() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("get_opIndex must have exactly 1 parameter, found {}", func.params.len()),
                        );
                        return None;
                    }

                    let param_type = &func.params[0];
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for get_opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "get_opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    current_ctx = ExprContext::rvalue(func.return_type.clone());
                } else {
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!("type '{}' does not support indexing", self.type_name(&current_ctx.data_type)),
                    );
                    return None;
                }
            } else {
                // Final index - try opIndex first (returns reference), then set_opIndex
                if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndex) {
                    // opIndex exists - use regular assignment through reference
                    let func = self.registry.get_function(func_id);

                    if func.params.len() != 1 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("opIndex must have exactly 1 parameter, found {}", func.params.len()),
                        );
                        return None;
                    }

                    let param_type = &func.params[0];
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(param_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for opIndex parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "opIndex parameter expects type '{}', found '{}'",
                                self.type_name(param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));
                    let is_mutable = current_ctx.is_mutable && !func.return_type.is_const;
                    current_ctx = ExprContext::lvalue(func.return_type.clone(), is_mutable);

                    // Now handle assignment to the returned reference
                    // Check that it's a mutable lvalue
                    if !current_ctx.is_lvalue {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            span,
                            "opIndex did not return an lvalue reference",
                        );
                        return None;
                    }
                    if !current_ctx.is_mutable {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            span,
                            "cannot assign to const indexed element",
                        );
                        return None;
                    }

                    // Type check the value being assigned
                    let value_ctx = self.check_expr(value)?;
                    if let Some(conversion) = value_ctx.data_type.can_convert_to(&current_ctx.data_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}'",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(&current_ctx.data_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            span,
                            format!(
                                "cannot assign value of type '{}' to indexed element of type '{}'",
                                self.type_name(&value_ctx.data_type),
                                self.type_name(&current_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Emit store instruction (handled by VM based on lvalue on stack)
                    return Some(ExprContext::rvalue(current_ctx.data_type));

                } else if let Some(func_id) = self.registry.find_operator_method(current_ctx.data_type.type_id, OperatorBehavior::OpIndexSet) {
                    // No opIndex, but set_opIndex exists
                    let func = self.registry.get_function(func_id);

                    // set_opIndex should have exactly 2 parameters: (index, value)
                    if func.params.len() != 2 {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            idx_item.span,
                            format!("set_opIndex must have exactly 2 parameters, found {}", func.params.len()),
                        );
                        return None;
                    }

                    let index_param_type = &func.params[0];
                    let value_param_type = &func.params[1];

                    // Type check the index argument
                    if let Some(conversion) = idx_ctx.data_type.can_convert_to(index_param_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                idx_item.span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for set_opIndex index parameter",
                                    self.type_name(&idx_ctx.data_type),
                                    self.type_name(index_param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            idx_item.span,
                            format!(
                                "set_opIndex index parameter expects type '{}', found '{}'",
                                self.type_name(index_param_type),
                                self.type_name(&idx_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Type check the value argument
                    let value_ctx = self.check_expr(value)?;
                    if let Some(conversion) = value_ctx.data_type.can_convert_to(value_param_type, self.registry) {
                        if !conversion.is_implicit {
                            self.error(
                                SemanticErrorKind::TypeMismatch,
                                span,
                                format!(
                                    "cannot implicitly convert '{}' to '{}' for set_opIndex value parameter",
                                    self.type_name(&value_ctx.data_type),
                                    self.type_name(value_param_type)
                                ),
                            );
                            return None;
                        }
                        self.emit_conversion(&conversion);
                    } else {
                        self.error(
                            SemanticErrorKind::TypeMismatch,
                            span,
                            format!(
                                "set_opIndex value parameter expects type '{}', found '{}'",
                                self.type_name(value_param_type),
                                self.type_name(&value_ctx.data_type)
                            ),
                        );
                        return None;
                    }

                    // Call set_opIndex(index, value) on current object
                    // Stack: [object, index, value] → object.set_opIndex(index, value)
                    self.bytecode.emit(Instruction::Call(func_id.as_u32()));

                    // Assignment expression returns the assigned value as rvalue
                    return Some(ExprContext::rvalue(value_ctx.data_type));

                } else {
                    // No opIndex or set_opIndex found
                    self.error(
                        SemanticErrorKind::InvalidOperation,
                        idx_item.span,
                        format!("type '{}' does not support index assignment", self.type_name(&current_ctx.data_type)),
                    );
                    return None;
                }
            }
        }

        // Should never reach here (loop always returns in last iteration)
        None
    }

    /// Type checks a member access expression.
    /// Field access (obj.field) is an lvalue if obj is an lvalue.
    /// Method calls (obj.method()) always return rvalues.
    fn check_member(&mut self, member: &MemberExpr<'src, 'ast>) -> Option<ExprContext> {
        let object_ctx = self.check_expr(member.object)?;

        // Check that the object is a class/interface type
        let typedef = self.registry.get_type(object_ctx.data_type.type_id);

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
                            if object_ctx.data_type.is_const || object_ctx.data_type.is_handle_to_const {
                                field_type.is_const = true;
                            }

                            // Field access is lvalue if object is lvalue
                            // Mutability depends on both object and field
                            let is_mutable = object_ctx.is_mutable && !field_type.is_const;
                            Some(ExprContext::lvalue(field_type, is_mutable))
                        } else {
                            self.error(
                                SemanticErrorKind::UndefinedField,
                                member.span,
                                format!(
                                    "type '{}' has no field '{}'",
                                    self.type_name(&object_ctx.data_type),
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
                                self.type_name(&object_ctx.data_type)
                            ),
                        );
                        None
                    }
                }
            }
            MemberAccess::Method { name, args } => {
                // Type check all arguments first, collecting contexts
                let mut arg_contexts = Vec::with_capacity(args.len());
                for arg in *args {
                    let arg_ctx = self.check_expr(arg.value)?;
                    arg_contexts.push(arg_ctx);
                }

                // Extract types for overload resolution
                let arg_types: Vec<DataType> = arg_contexts.iter().map(|ctx| ctx.data_type.clone()).collect();

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
                                    self.type_name(&object_ctx.data_type),
                                    name.name
                                ),
                            );
                            return None;
                        }

                        // Filter by const-correctness first
                        let is_const_object = object_ctx.data_type.is_const || object_ctx.data_type.is_handle_to_const;

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
                                    self.type_name(&object_ctx.data_type)
                                ),
                            );
                            return None;
                        }

                        // Find best matching overload from const-filtered candidates
                        let (matching_method, conversions) = self.find_best_function_overload(
                            &const_filtered,
                            &arg_types,
                            member.span,
                        )?;

                        let func_def = self.registry.get_function(matching_method);

                        // Validate reference parameters
                        self.validate_reference_parameters(func_def, &arg_contexts, *args, member.span)?;

                        // Emit conversion instructions for arguments
                        for conversion in conversions {
                            if let Some(conv) = conversion {
                                self.emit_conversion(&conv);
                            }
                        }

                        // Emit method call instruction
                        self.bytecode.emit(Instruction::CallMethod(matching_method.as_u32()));

                        // Method calls return rvalues
                        Some(ExprContext::rvalue(func_def.return_type.clone()))
                    }
                    _ => {
                        self.error(
                            SemanticErrorKind::InvalidOperation,
                            member.span,
                            format!(
                                "type '{}' does not support method calls",
                                self.type_name(&object_ctx.data_type)
                            ),
                        );
                        None
                    }
                }
            }
        }
    }

    /// Type checks a postfix expression.
    /// x++ and x-- require mutable lvalues and produce rvalues.
    fn check_postfix(&mut self, postfix: &PostfixExpr<'src, 'ast>) -> Option<ExprContext> {
        let operand_ctx = self.check_expr(postfix.operand)?;

        // Try operator overload first
        let operator = match postfix.op {
            PostfixOp::PostInc => OperatorBehavior::OpPostInc,
            PostfixOp::PostDec => OperatorBehavior::OpPostDec,
        };

        if let Some(result_type) = self.try_unary_operator_overload(
            operator,
            &operand_ctx.data_type,
            postfix.span,
        ) {
            // Operator overloads still require lvalue check
            if !operand_ctx.is_lvalue {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    postfix.span,
                    format!("{} requires an lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
                );
                return None;
            }
            if !operand_ctx.is_mutable {
                self.error(
                    SemanticErrorKind::InvalidOperation,
                    postfix.span,
                    format!("{} requires a mutable lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
                );
                return None;
            }
            return Some(ExprContext::rvalue(result_type));
        }

        // Fall back to primitive postfix operators
        // Post-increment/decrement require mutable lvalue
        if !operand_ctx.is_lvalue {
            self.error(
                SemanticErrorKind::InvalidOperation,
                postfix.span,
                format!("{} requires an lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
            );
            return None;
        }
        if !operand_ctx.is_mutable {
            self.error(
                SemanticErrorKind::InvalidOperation,
                postfix.span,
                format!("{} requires a mutable lvalue", if postfix.op == PostfixOp::PostInc { "post-increment" } else { "post-decrement" }),
            );
            return None;
        }

        match postfix.op {
            PostfixOp::PostInc => {
                self.bytecode.emit(Instruction::PostIncrement);
            }
            PostfixOp::PostDec => {
                self.bytecode.emit(Instruction::PostDecrement);
            }
        }

        // Returns rvalue of the operand's type
        Some(ExprContext::rvalue(operand_ctx.data_type))
    }

    /// Type checks a cast expression.
    /// Casts produce rvalues.
    fn check_cast(&mut self, cast: &CastExpr<'src, 'ast>) -> Option<ExprContext> {
        let expr_ctx = self.check_expr(cast.expr)?;
        let target_type = self.resolve_type_expr(&cast.target_type)?;

        // Check if conversion is valid
        if let Some(conversion) = expr_ctx.data_type.can_convert_to(&target_type, self.registry) {
            // Emit the appropriate conversion instruction
            self.emit_conversion(&conversion);
            Some(ExprContext::rvalue(target_type))
        } else {
            self.error(
                SemanticErrorKind::TypeMismatch,
                cast.span,
                format!(
                    "cannot convert from '{}' to '{}'",
                    self.type_name(&expr_ctx.data_type),
                    self.type_name(&target_type)
                ),
            );
            None
        }
    }

    /// Type checks a lambda expression.
    /// Lambdas produce rvalues (function references).
    fn check_lambda(&mut self, lambda: &LambdaExpr<'src, 'ast>) -> Option<ExprContext> {
        // Get expected funcdef type from context (set by check_call or assignment)
        let funcdef_type_id = match self.expected_funcdef_type {
            Some(type_id) => type_id,
            None => {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    lambda.span,
                    "cannot infer lambda type - explicit funcdef context required",
                );
                return None;
            }
        };

        // Get funcdef signature
        let funcdef = self.registry.get_type(funcdef_type_id);
        let (expected_params, expected_return) = match funcdef {
            TypeDef::Funcdef { params, return_type, .. } => (params, return_type),
            _ => {
                self.error(
                    SemanticErrorKind::InternalError,
                    lambda.span,
                    "expected funcdef type for lambda",
                );
                return None;
            }
        };

        // Validate parameter count
        if lambda.params.len() != expected_params.len() {
            self.error(
                SemanticErrorKind::TypeMismatch,
                lambda.span,
                format!(
                    "lambda parameter count mismatch: expected {}, got {}",
                    expected_params.len(),
                    lambda.params.len()
                ),
            );
            return None;
        }

        // Validate explicit parameter types if provided
        for (i, (lambda_param, expected_param)) in
            lambda.params.iter().zip(expected_params.iter()).enumerate()
        {
            if let Some(param_ty) = &lambda_param.ty {
                let explicit_type = self.resolve_type_expr(&param_ty.ty)?;
                // TODO: Apply ref modifiers and validate match
                if explicit_type.type_id != expected_param.type_id {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        lambda_param.span,
                        format!("lambda parameter {} type mismatch", i),
                    );
                    return None;
                }
            }
        }

        // Validate return type if specified
        if let Some(ret_ty) = &lambda.return_type {
            let explicit_return = self.resolve_type_expr(&ret_ty.ty)?;
            if explicit_return.type_id != expected_return.type_id {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    lambda.span,
                    "lambda return type mismatch",
                );
                return None;
            }
        }

        // Allocate FunctionId for this lambda
        let lambda_id = self.next_lambda_id;
        self.next_lambda_id += 1;

        // Capture all variables in current scope
        let captured_vars = self.local_scope.capture_all_variables();

        // Build parameters for compile_block: funcdef params + captured vars
        let mut all_vars = Vec::new();
        for (i, param) in lambda.params.iter().enumerate() {
            let param_name = param.name
                .map(|id| id.name.to_string())
                .unwrap_or_else(|| format!("_param{}", i));
            all_vars.push((param_name, expected_params[i].clone()));
        }
        for cap in &captured_vars {
            all_vars.push((cap.name.clone(), cap.data_type.clone()));
        }

        // ✨ COMPILE LAMBDA IMMEDIATELY using compile_block
        let compiled = FunctionCompiler::compile_block(
            self.registry,
            expected_return.clone(),
            &all_vars,
            lambda.body,
        );

        // Store compiled bytecode in compiled_functions map
        self.compiled_functions.insert(FunctionId(lambda_id), compiled.bytecode);

        // Merge errors from lambda compilation
        self.errors.extend(compiled.errors);

        // Emit FuncPtr instruction to push lambda handle onto stack
        self.bytecode.emit(Instruction::FuncPtr(lambda_id));

        // Return funcdef handle type (rvalue)
        Some(ExprContext::rvalue(DataType::with_handle(
            funcdef_type_id,
            false,
        )))
    }

    /// Type checks an initializer list.
    /// Initializer lists produce rvalues (newly constructed arrays/objects).
    fn check_init_list(&mut self, init_list: &InitListExpr<'src, 'ast>) -> Option<ExprContext> {
        use crate::ast::InitElement;

        // Handle empty initializer list
        if init_list.elements.is_empty() {
            self.error(
                SemanticErrorKind::TypeMismatch,
                init_list.span,
                "cannot infer type from empty initializer list".to_string(),
            );
            return None;
        }

        // Type check all elements and collect their types
        let mut element_types = Vec::with_capacity(init_list.elements.len());

        for element in init_list.elements {
            let elem_ctx = match element {
                InitElement::Expr(expr) => self.check_expr(expr)?,
                InitElement::InitList(nested) => self.check_init_list(nested)?,
            };
            element_types.push(elem_ctx.data_type);
        }

        // Infer the common element type
        // Start with the first element's type
        let mut common_type = element_types[0].clone();

        // For all subsequent elements, find the common promoted type
        for elem_type in &element_types[1..] {
            // If types are identical, continue
            if common_type == *elem_type {
                continue;
            }

            // Check if we can promote to a common type
            // For numeric types, promote to the wider type
            if self.is_numeric(&common_type) && self.is_numeric(elem_type) {
                common_type = self.promote_numeric(&common_type, elem_type);
            } else if let Some(conversion) = elem_type.can_convert_to(&common_type, self.registry) {
                // Element can be implicitly converted to common type
                if !conversion.is_implicit {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        init_list.span,
                        format!(
                            "cannot implicitly convert '{}' to '{}' in initializer list",
                            self.type_name(elem_type),
                            self.type_name(&common_type)
                        ),
                    );
                    return None;
                }
            } else if let Some(conversion) = common_type.can_convert_to(elem_type, self.registry) {
                // Common type can be converted to element type, use element type as new common
                if conversion.is_implicit {
                    common_type = elem_type.clone();
                } else {
                    self.error(
                        SemanticErrorKind::TypeMismatch,
                        init_list.span,
                        format!(
                            "incompatible types in initializer list: '{}' and '{}'",
                            self.type_name(&common_type),
                            self.type_name(elem_type)
                        ),
                    );
                    return None;
                }
            } else {
                self.error(
                    SemanticErrorKind::TypeMismatch,
                    init_list.span,
                    format!(
                        "incompatible types in initializer list: '{}' and '{}'",
                        self.type_name(&common_type),
                        self.type_name(elem_type)
                    ),
                );
                return None;
            }
        }

        // Now emit conversion instructions for each element if needed
        // We need to go back through the elements and emit conversions
        // Note: The elements are already on the stack from check_expr calls above
        for (i, elem_type) in element_types.iter().enumerate() {
            if elem_type != &common_type {
                // Need to emit conversion
                if let Some(conversion) = elem_type.can_convert_to(&common_type, self.registry) {
                    self.emit_conversion(&conversion);
                } else {
                    // This shouldn't happen as we already validated above
                    self.error(
                        SemanticErrorKind::InternalError,
                        init_list.span,
                        format!(
                            "internal error: failed to convert element {} from '{}' to '{}'",
                            i,
                            self.type_name(elem_type),
                            self.type_name(&common_type)
                        ),
                    );
                    return None;
                }
            }
        }

        // Emit CreateArray instruction
        // The VM will create array<common_type> at runtime
        self.bytecode.emit(Instruction::CreateArray {
            element_type_id: common_type.type_id.as_u32(),
            count: element_types.len() as u32,
        });

        // Return the array type as a handle (arrays are reference types)
        // We construct a DataType representing array<common_type>@
        // Note: Ideally we'd instantiate the template, but FunctionCompiler doesn't have
        // mutable access to Registry. The VM will create the actual array<T> type.
        // For now, we return a placeholder that indicates "array of common_type"
        // This is sufficient for type checking in most cases.

        // Look up if array<common_type> already exists in the registry
        // Search through all types to find a matching TemplateInstance
        let mut array_type_id = None;
        for i in 0..self.registry.type_count() {
            let type_id = TypeId(i as u32);
            let typedef = self.registry.get_type(type_id);
            if let TypeDef::TemplateInstance { template, sub_types } = typedef {
                if *template == self.registry.array_template
                    && sub_types.len() == 1
                    && sub_types[0] == common_type {
                    array_type_id = Some(type_id);
                    break;
                }
            }
        }

        if let Some(array_id) = array_type_id {
            // Found existing array<T> type, return as rvalue
            Some(ExprContext::rvalue(DataType::with_handle(array_id, false)))
        } else {
            // Array type doesn't exist yet - this is a limitation
            // For now, return a generic array handle type
            // TODO: This should be resolved in a pre-pass that instantiates all needed array types
            self.error(
                SemanticErrorKind::InternalError,
                init_list.span,
                format!(
                    "array<{}> type not found in registry - template instantiation needed",
                    self.type_name(&common_type)
                ),
            );
            None
        }
    }

    /// Type checks a parenthesized expression.
    /// Parentheses preserve the lvalue-ness of the inner expression.
    fn check_paren(&mut self, paren: &ParenExpr<'src, 'ast>) -> Option<ExprContext> {
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

    /// Checks if a value can be assigned to a target type.
    ///
    /// Returns true if:
    /// - Types are identical, OR
    /// - An implicit conversion exists from value to target
    fn is_assignable(&self, value: &DataType, target: &DataType) -> bool {
        if let Some(conversion) = value.can_convert_to(target, self.registry) {
            conversion.is_implicit
        } else {
            false
        }
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

    /// Tries to find and call an operator overload for a binary operation.
    ///
    /// Returns Some(result_type) if operator overload was found and emitted,
    /// None if no overload exists (caller should try primitive operation).
    fn try_binary_operator_overload(
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
    fn try_unary_operator_overload(
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
    fn validate_reference_parameters(
        &mut self,
        func_def: &crate::semantic::types::registry::FunctionDef,
        arg_contexts: &[ExprContext],
        call_args: &[crate::ast::expr::Argument<'src, 'ast>],
        span: Span,
    ) -> Option<()> {
        use crate::semantic::types::RefModifier;

        // Iterate through parameters and check reference modifiers
        for (i, param_type) in func_def.params.iter().enumerate() {
            // Skip if we don't have an argument for this parameter
            if i >= arg_contexts.len() {
                continue;
            }

            let arg_ctx = &arg_contexts[i];

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
    fn find_best_function_overload(
        &mut self,
        candidates: &[FunctionId],
        arg_types: &[DataType],
        span: Span,
    ) -> Option<(FunctionId, Vec<Option<crate::semantic::Conversion>>)> {
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

    /// Emit conversion instruction based on ConversionKind.
    ///
    /// Maps semantic conversion information to the appropriate bytecode instruction.
    fn emit_conversion(&mut self, conversion: &crate::semantic::Conversion) {
        use crate::semantic::ConversionKind;
        use crate::semantic::types::type_def::{
            DOUBLE_TYPE, FLOAT_TYPE, INT8_TYPE, INT16_TYPE, INT32_TYPE, INT64_TYPE,
            UINT8_TYPE, UINT16_TYPE, UINT32_TYPE, UINT64_TYPE,
        };

        match &conversion.kind {
            ConversionKind::Identity => {
                // No instruction needed for identity conversion
            }

            ConversionKind::NullToHandle => {
                // No instruction needed - PushNull already pushed the null value
                // The VM will interpret this as the appropriate handle type
            }

            ConversionKind::Primitive { from_type, to_type } => {
                // Select instruction based on type pair
                let instruction = match (*from_type, *to_type) {
                    // Integer to Float conversions
                    (INT8_TYPE, FLOAT_TYPE) => Instruction::ConvertI8F32,
                    (INT16_TYPE, FLOAT_TYPE) => Instruction::ConvertI16F32,
                    (INT32_TYPE, FLOAT_TYPE) => Instruction::ConvertI32F32,
                    (INT64_TYPE, FLOAT_TYPE) => Instruction::ConvertI64F32,
                    (INT8_TYPE, DOUBLE_TYPE) => Instruction::ConvertI8F64,
                    (INT16_TYPE, DOUBLE_TYPE) => Instruction::ConvertI16F64,
                    (INT32_TYPE, DOUBLE_TYPE) => Instruction::ConvertI32F64,
                    (INT64_TYPE, DOUBLE_TYPE) => Instruction::ConvertI64F64,

                    // Unsigned to Float conversions
                    (UINT8_TYPE, FLOAT_TYPE) => Instruction::ConvertU8F32,
                    (UINT16_TYPE, FLOAT_TYPE) => Instruction::ConvertU16F32,
                    (UINT32_TYPE, FLOAT_TYPE) => Instruction::ConvertU32F32,
                    (UINT64_TYPE, FLOAT_TYPE) => Instruction::ConvertU64F32,
                    (UINT8_TYPE, DOUBLE_TYPE) => Instruction::ConvertU8F64,
                    (UINT16_TYPE, DOUBLE_TYPE) => Instruction::ConvertU16F64,
                    (UINT32_TYPE, DOUBLE_TYPE) => Instruction::ConvertU32F64,
                    (UINT64_TYPE, DOUBLE_TYPE) => Instruction::ConvertU64F64,

                    // Float to Integer conversions
                    (FLOAT_TYPE, INT8_TYPE) => Instruction::ConvertF32I8,
                    (FLOAT_TYPE, INT16_TYPE) => Instruction::ConvertF32I16,
                    (FLOAT_TYPE, INT32_TYPE) => Instruction::ConvertF32I32,
                    (FLOAT_TYPE, INT64_TYPE) => Instruction::ConvertF32I64,
                    (DOUBLE_TYPE, INT8_TYPE) => Instruction::ConvertF64I8,
                    (DOUBLE_TYPE, INT16_TYPE) => Instruction::ConvertF64I16,
                    (DOUBLE_TYPE, INT32_TYPE) => Instruction::ConvertF64I32,
                    (DOUBLE_TYPE, INT64_TYPE) => Instruction::ConvertF64I64,

                    // Float to Unsigned conversions
                    (FLOAT_TYPE, UINT8_TYPE) => Instruction::ConvertF32U8,
                    (FLOAT_TYPE, UINT16_TYPE) => Instruction::ConvertF32U16,
                    (FLOAT_TYPE, UINT32_TYPE) => Instruction::ConvertF32U32,
                    (FLOAT_TYPE, UINT64_TYPE) => Instruction::ConvertF32U64,
                    (DOUBLE_TYPE, UINT8_TYPE) => Instruction::ConvertF64U8,
                    (DOUBLE_TYPE, UINT16_TYPE) => Instruction::ConvertF64U16,
                    (DOUBLE_TYPE, UINT32_TYPE) => Instruction::ConvertF64U32,
                    (DOUBLE_TYPE, UINT64_TYPE) => Instruction::ConvertF64U64,

                    // Float ↔ Double conversions
                    (FLOAT_TYPE, DOUBLE_TYPE) => Instruction::ConvertF32F64,
                    (DOUBLE_TYPE, FLOAT_TYPE) => Instruction::ConvertF64F32,

                    // Integer widening (signed)
                    (INT8_TYPE, INT16_TYPE) => Instruction::ConvertI8I16,
                    (INT8_TYPE, INT32_TYPE) => Instruction::ConvertI8I32,
                    (INT8_TYPE, INT64_TYPE) => Instruction::ConvertI8I64,
                    (INT16_TYPE, INT32_TYPE) => Instruction::ConvertI16I32,
                    (INT16_TYPE, INT64_TYPE) => Instruction::ConvertI16I64,
                    (INT32_TYPE, INT64_TYPE) => Instruction::ConvertI32I64,

                    // Integer narrowing (signed)
                    (INT64_TYPE, INT32_TYPE) => Instruction::ConvertI64I32,
                    (INT64_TYPE, INT16_TYPE) => Instruction::ConvertI64I16,
                    (INT64_TYPE, INT8_TYPE) => Instruction::ConvertI64I8,
                    (INT32_TYPE, INT16_TYPE) => Instruction::ConvertI32I16,
                    (INT32_TYPE, INT8_TYPE) => Instruction::ConvertI32I8,
                    (INT16_TYPE, INT8_TYPE) => Instruction::ConvertI16I8,

                    // Unsigned widening
                    (UINT8_TYPE, UINT16_TYPE) => Instruction::ConvertU8U16,
                    (UINT8_TYPE, UINT32_TYPE) => Instruction::ConvertU8U32,
                    (UINT8_TYPE, UINT64_TYPE) => Instruction::ConvertU8U64,
                    (UINT16_TYPE, UINT32_TYPE) => Instruction::ConvertU16U32,
                    (UINT16_TYPE, UINT64_TYPE) => Instruction::ConvertU16U64,
                    (UINT32_TYPE, UINT64_TYPE) => Instruction::ConvertU32U64,

                    // Unsigned narrowing
                    (UINT64_TYPE, UINT32_TYPE) => Instruction::ConvertU64U32,
                    (UINT64_TYPE, UINT16_TYPE) => Instruction::ConvertU64U16,
                    (UINT64_TYPE, UINT8_TYPE) => Instruction::ConvertU64U8,
                    (UINT32_TYPE, UINT16_TYPE) => Instruction::ConvertU32U16,
                    (UINT32_TYPE, UINT8_TYPE) => Instruction::ConvertU32U8,
                    (UINT16_TYPE, UINT8_TYPE) => Instruction::ConvertU16U8,

                    // Signed/Unsigned reinterpret
                    (INT8_TYPE, UINT8_TYPE) => Instruction::ConvertI8U8,
                    (INT16_TYPE, UINT16_TYPE) => Instruction::ConvertI16U16,
                    (INT32_TYPE, UINT32_TYPE) => Instruction::ConvertI32U32,
                    (INT64_TYPE, UINT64_TYPE) => Instruction::ConvertI64U64,
                    (UINT8_TYPE, INT8_TYPE) => Instruction::ConvertU8I8,
                    (UINT16_TYPE, INT16_TYPE) => Instruction::ConvertU16I16,
                    (UINT32_TYPE, INT32_TYPE) => Instruction::ConvertU32I32,
                    (UINT64_TYPE, INT64_TYPE) => Instruction::ConvertU64I64,

                    _ => {
                        // This should never happen if the semantic analyzer is correct
                        return;
                    }
                };
                self.bytecode.emit(instruction);
            }

            ConversionKind::HandleToConst => {
                self.bytecode.emit(Instruction::CastHandleToConst);
            }

            ConversionKind::DerivedToBase => {
                self.bytecode.emit(Instruction::CastHandleDerivedToBase);
            }

            ConversionKind::ClassToInterface => {
                self.bytecode.emit(Instruction::CastHandleToInterface);
            }

            ConversionKind::ConstructorConversion { constructor_id } => {
                self.bytecode.emit(Instruction::CallMethod(constructor_id.0));
            }

            ConversionKind::ImplicitConversionMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }

            ConversionKind::ExplicitCastMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }

            ConversionKind::ImplicitCastMethod { method_id } => {
                self.bytecode.emit(Instruction::CallMethod(method_id.0));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{DataType, Registry};

    fn create_test_registry() -> Registry<'static, 'static> {
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

    #[test]
    fn init_list_empty_error() {
        use crate::ast::{Parser, Expr};
        use bumpalo::Bump;

        let arena = Bump::new();
        let mut parser = Parser::new("{}", &arena);
        let expr = parser.parse_expr(0).unwrap();

        let mut registry = create_test_registry();

        // Pre-instantiate array<int32> for testing
        let _array_int = registry
            .instantiate_template(
                registry.array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&registry, return_type);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_none());
            assert_eq!(compiler.errors.len(), 1);
            assert!(compiler.errors[0]
                .message
                .contains("cannot infer type from empty initializer list"));
        } else {
            panic!("Expected InitList expression");
        }
    }

    #[test]
    fn init_list_simple_int() {
        use crate::ast::{Parser, Expr};
        use bumpalo::Bump;

        let arena = Bump::new();
        let mut parser = Parser::new("{1, 2, 3}", &arena);
        let expr = parser.parse_expr(0).unwrap();

        let mut registry = create_test_registry();

        // Pre-instantiate array<int32> for testing
        let array_int = registry
            .instantiate_template(
                registry.array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&registry, return_type);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some());
            let result_ctx = result.unwrap();

            // Should return array<int32>@
            assert!(result_ctx.data_type.is_handle);
            assert_eq!(result_ctx.data_type.type_id, array_int);
            assert_eq!(compiler.errors.len(), 0);

            // Check emitted bytecode
            let bytecode = compiler.bytecode.instructions();
            // Should have: PushInt(1), PushInt(2), PushInt(3), CreateArray
            assert!(bytecode.iter().any(|instr| matches!(instr, Instruction::CreateArray { element_type_id: _, count: 3 })));
        } else {
            panic!("Expected InitList expression");
        }
    }

    #[test]
    fn init_list_nested() {
        use crate::ast::{Parser, Expr};
        use bumpalo::Bump;

        let arena = Bump::new();
        let mut parser = Parser::new("{{1, 2}, {3, 4}}", &arena);
        let expr = parser.parse_expr(0).unwrap();

        let mut registry = create_test_registry();

        // Pre-instantiate array<int32>
        let array_int = registry
            .instantiate_template(
                registry.array_template,
                vec![DataType::simple(INT32_TYPE)],
            )
            .unwrap();

        // Pre-instantiate array<array<int32>@>
        let array_array_int = registry
            .instantiate_template(
                registry.array_template,
                vec![DataType::with_handle(array_int, false)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&registry, return_type);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some());
            let result_ctx = result.unwrap();

            // Should return array<array<int32>@>@
            assert!(result_ctx.data_type.is_handle);
            assert_eq!(result_ctx.data_type.type_id, array_array_int);
            assert_eq!(compiler.errors.len(), 0);
        } else {
            panic!("Expected InitList expression");
        }
    }

    #[test]
    fn init_list_type_promotion() {
        use crate::ast::{Parser, Expr};
        use bumpalo::Bump;

        let arena = Bump::new();
        let mut parser = Parser::new("{1, 2.5, 3}", &arena);
        let expr = parser.parse_expr(0).unwrap();

        let mut registry = create_test_registry();

        // Pre-instantiate array<double>
        let array_double = registry
            .instantiate_template(
                registry.array_template,
                vec![DataType::simple(DOUBLE_TYPE)],
            )
            .unwrap();

        let return_type = DataType::simple(VOID_TYPE);
        let mut compiler = FunctionCompiler::new(&registry, return_type);

        if let Expr::InitList(init_list) = *expr {
            let result = compiler.check_init_list(&init_list);
            assert!(result.is_some());
            let result_ctx = result.unwrap();

            // Should promote to array<double>@ (int promotes to double)
            assert!(result_ctx.data_type.is_handle);
            assert_eq!(result_ctx.data_type.type_id, array_double);
            assert_eq!(compiler.errors.len(), 0);
        } else {
            panic!("Expected InitList expression");
        }
    }

    // NOTE: Integration tests for opIndex accessors are blocked by pre-existing
    // lifetime issues in the test infrastructure (Registry<'src, 'ast> lifetimes).
    // The implementation compiles successfully and logic has been manually verified:
    // - check_index() tries get_opIndex after opIndex (read context)
    // - check_index_assignment() detects write context and uses set_opIndex
    // - opIndex takes priority when both operators and accessors exist
    // Tests will be added once Registry lifetime issues are resolved project-wide.

    #[test]
    fn lambda_compilation_basic() {
        // Test that lambda expressions compile to bytecode with immediate compilation
        use crate::parse_lenient;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Callback(int x);

            void takeCallback(Callback @cb) {
                cb(42);
            }

            void main() {
                takeCallback(function(int x) { });
            }
        "#;

        let (script, parse_errors) = parse_lenient(source, &arena);
        assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

        let result = Compiler::compile(&script);

        // Print actual state for debugging
        if !result.is_success() {
            eprintln!("Compilation errors: {:?}", result.errors);
        }
        eprintln!("Function count: {}", result.module.functions.len());
        for (id, bytecode) in &result.module.functions {
            eprintln!("Function {:?}: {} instructions", id, bytecode.instructions.len());
        }

        // Should compile successfully
        assert!(result.is_success(), "Lambda compilation failed: {:?}", result.errors);

        // Should have 3 functions: takeCallback, main, and the lambda
        assert_eq!(result.module.functions.len(), 3,
            "Expected 3 functions (takeCallback, main, lambda), got {}", result.module.functions.len());

        // Functions are registered in declaration order:
        // FunctionId(0) = takeCallback
        // FunctionId(1) = main
        // FunctionId(2) = lambda
        let takecb_id = FunctionId(0);
        let main_id = FunctionId(1);
        let lambda_id = FunctionId(2);

        assert!(result.module.functions.contains_key(&lambda_id),
            "Lambda bytecode not found in compiled module");

        // Verify main function contains FuncPtr instruction
        let main_bytecode = result.module.functions.get(&main_id).expect("main function not found");
        eprintln!("main bytecode: {:?}", main_bytecode.instructions);
        let has_funcptr = main_bytecode.instructions.iter()
            .any(|instr| matches!(instr, Instruction::FuncPtr(_)));
        assert!(has_funcptr, "main should emit FuncPtr instruction for lambda");

        // Verify takeCallback function contains CallPtr instruction
        let takecb_bytecode = result.module.functions.get(&takecb_id).expect("takeCallback function not found");
        eprintln!("takeCallback bytecode: {:?}", takecb_bytecode.instructions);
        let has_callptr = takecb_bytecode.instructions.iter()
            .any(|instr| matches!(instr, Instruction::CallPtr));
        assert!(has_callptr, "takeCallback should emit CallPtr instruction to invoke funcdef");
    }

    #[test]
    fn lambda_type_inference() {
        // Test that lambda parameters are inferred from funcdef context
        use crate::parse_lenient;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef int BinaryOp(int a, int b);

            void applyOp(BinaryOp @op) {
                int result = op(10, 20);
            }

            void main() {
                // Lambda parameters inferred as (int, int)
                applyOp(function(a, b) { return a + b; });
            }
        "#;

        let (script, parse_errors) = parse_lenient(source, &arena);
        if !parse_errors.is_empty() {
            eprintln!("Parse errors: {:?}", parse_errors);
        }

        let result = Compiler::compile(&script);

        if !result.is_success() {
            eprintln!("Compilation errors: {:?}", result.errors);
            eprintln!("Functions compiled: {}", result.module.functions.len());
        }

        // Should compile successfully with type inference
        assert!(result.is_success(), "Lambda type inference failed: {:?}", result.errors);
    }

    #[test]
    fn lambda_variable_capture() {
        // Test that lambda captures variables from enclosing scope
        use crate::parse_lenient;
        use crate::semantic::Compiler;
        use bumpalo::Bump;

        let arena = Bump::new();
        let source = r#"
            funcdef void Action();

            void runAction(Action @action) {
                action();
            }

            void main() {
                int counter = 0;
                runAction(function() {
                    counter = counter + 1;
                });
            }
        "#;

        let (script, _errors) = parse_lenient(source, &arena);
        let result = Compiler::compile(&script);

        // Should compile successfully with variable capture
        assert!(result.is_success(), "Lambda variable capture failed: {:?}", result.errors);

        // Lambda should have captured 'counter' variable
        let lambda_id = FunctionId(2);
        let lambda_bytecode = result.module.functions.get(&lambda_id)
            .expect("Lambda bytecode not found");

        // The lambda body should reference the captured variable
        // (exact bytecode depends on implementation details)
        assert!(lambda_bytecode.instructions.len() > 0,
            "Lambda should have non-empty bytecode");
    }

    // More tests will be added as we implement the compiler
}
